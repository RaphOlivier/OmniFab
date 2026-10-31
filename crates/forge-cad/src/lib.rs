//! CAO paramétrique OmniFab — consolidation du noyau OCCT (jalon 3,
//! phase A, ADR-0023).
//!
//! Le binding opencascade-rs est isolé ENTIÈREMENT derrière le module
//! interne `occt` : aucun type OCCT n'apparaît dans l'API publique (même
//! frontière que le shim planegcs, ADR-0013). OCCT 7.8 builtin
//! épinglée par le lock (checkpoint \[J3] du 2026-10-26, ADR-0023 D1 amendé).
//!
//! Unités strictes à la frontière (verrou n°5) : les dimensions passent en
//! [`Length`], jamais de `f64` nu dans l'API publique.

#![deny(missing_docs)]

pub mod erreur;
mod occt;

use forge_units::Length;

pub use erreur::CadError;

/// Budgets de ressources des parseurs CAO (patron ADR-0009 : un fichier
/// hostile ne gèle jamais l'interface — tout dépassement est refusé AVANT
/// le premier octet confié à OCCT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CadBudgets {
    /// Taille maximale en octets d'un fichier importé.
    pub octets_max: usize,
    /// Nombre maximal de formes acceptées dans un fichier.
    pub formes_max: usize,
}

impl Default for CadBudgets {
    fn default() -> Self {
        Self {
            octets_max: 256 * 1024 * 1024,
            formes_max: 10_000,
        }
    }
}

/// Maillage tessellé d'un solide — coordonnées en millimètres.
#[derive(Debug, Clone, PartialEq)]
pub struct Maillage {
    /// Sommets `[x, y, z]` en millimètres.
    pub sommets: Vec<[f64; 3]>,
    /// Normales par sommet, issues de la tessellation OCCT.
    pub normales: Vec<[f64; 3]>,
    /// Triangles, par indices de sommets.
    pub triangles: Vec<[u32; 3]>,
}

/// Solide CAO opaque : le type OCCT sous-jacent ne fuit jamais.
#[derive(Debug)]
pub struct Solide {
    forme: occt::Forme,
}

impl Solide {
    /// Boîte alignée sur les axes, coin à l'origine.
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si la construction OCCT échoue.
    pub fn boite(largeur: Length, profondeur: Length, hauteur: Length) -> Result<Self, CadError> {
        Ok(Self {
            forme: occt::Forme::boite(
                largeur.millimeters(),
                profondeur.millimeters(),
                hauteur.millimeters(),
            ),
        })
    }

    /// Cylindre vertical de rayon `rayon`, base centrée en (x, y) sur
    /// z = 0, de hauteur `hauteur` vers +Z (perçage : le soustraire).
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si la construction OCCT échoue.
    pub fn cylindre_centre(
        rayon: Length,
        x: Length,
        y: Length,
        hauteur: Length,
    ) -> Result<Self, CadError> {
        Ok(Self {
            forme: occt::Forme::cylindre_centre(
                rayon.millimeters(),
                x.millimeters(),
                y.millimeters(),
                hauteur.millimeters(),
            ),
        })
    }

    /// Prisme : face construite sur le contour fermé, extrudée de
    /// `hauteur` vers +Z (vraies courbes OCCT — ADR-0024 D2).
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si le contour est dégénéré.
    pub fn prisme(contour: &[(Length, Length)], hauteur: Length) -> Result<Self, CadError> {
        let points: Vec<glam::DVec3> = contour
            .iter()
            .map(|(x, y)| glam::dvec3(x.millimeters(), y.millimeters(), 0.0))
            .collect();
        let forme =
            occt::Forme::prisme(points, hauteur.millimeters()).map_err(|e| CadError::Occt {
                operation: format!("extrusion du contour : {e}"),
            })?;
        Ok(Self { forme })
    }

    /// Soustraction booléenne `self - autre` (poche, perçage).
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si le booléen OCCT échoue.
    pub fn soustraire(&self, autre: &Solide) -> Result<Self, CadError> {
        Ok(Self {
            forme: occt::Forme::soustraire(&self.forme, &autre.forme),
        })
    }

    /// Union booléenne `self + autre`.
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si le booléen OCCT échoue.
    pub fn unir(&self, autre: &Solide) -> Result<Self, CadError> {
        Ok(Self {
            forme: occt::Forme::unir(&self.forme, &autre.forme),
        })
    }

    /// Tessellation du solide ; `tolerance` est l'écart maximal accepté
    /// entre une surface et son maillage (millimètres).
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si la tessellation échoue.
    pub fn tesseller(&self, tolerance: Length) -> Result<Maillage, CadError> {
        let maille = self
            .forme
            .tesseller(tolerance.millimeters())
            .map_err(|e| CadError::Occt {
                operation: format!("tessellation : {e}"),
            })?;
        let sommets: Vec<[f64; 3]> = maille.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
        let normales: Vec<[f64; 3]> = maille.normals.iter().map(|n| [n.x, n.y, n.z]).collect();
        let triangles: Vec<[u32; 3]> = maille
            .indices
            .chunks(3)
            .filter(|t| t.len() == 3)
            .map(|t| [t[0] as u32, t[1] as u32, t[2] as u32])
            .collect();
        Ok(Maillage {
            sommets,
            normales,
            triangles,
        })
    }

    /// Volume du solide en millimètres cubes, par tessellation.
    ///
    /// Les faces planes sont exactes au f64 près ; les faces courbes portent
    /// l'erreur de tessellation, qui diminue avec `tolerance`. La bascule
    /// vers `BRepGProp::VolumeProperties` (exacte, y compris sur les
    /// faces courbes) est planifiée au lot 2 de la phase A : elle exige un
    /// pont direct opencascade-sys.
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si la tessellation échoue.
    pub fn volume_mm3(&self, tolerance: Length) -> Result<f64, CadError> {
        let maillage = self.tesseller(tolerance)?;
        let mut volume = 0.0;
        for t in &maillage.triangles {
            let a = maillage.sommets[t[0] as usize];
            let b = maillage.sommets[t[1] as usize];
            let c = maillage.sommets[t[2] as usize];
            let produit_mixte = a[0] * (b[1] * c[2] - b[2] * c[1])
                - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]);
            volume += produit_mixte / 6.0;
        }
        Ok(volume)
    }
}

/// Écrit dans un fichier temporaire via `ecrire`, lit les octets, efface
/// le fichier — OCCT travaille par fichiers (le lot 1 n'expose pas de
/// flux mémoire).
fn octets_via_fichier_temporaire(
    suffixe: &str,
    ecrire: impl FnOnce(&std::path::Path) -> Result<(), CadError>,
) -> Result<Vec<u8>, CadError> {
    let nom = format!("omnifab-cad-{}{suffixe}", uuid::Uuid::now_v7().simple());
    let chemin = std::env::temp_dir().join(nom);
    let octets = match ecrire(&chemin) {
        Ok(()) => std::fs::read(&chemin).map_err(|e| CadError::Occt {
            operation: format!("lecture du fichier temporaire : {e}"),
        }),
        Err(erreur) => Err(erreur),
    };
    let _ = std::fs::remove_file(&chemin);
    octets
}

/// Import/export STEP (ADR-0023 D4 : forge-cad, jamais forge-io — la
/// frontière ADR-0009 interdit à forge-io de dépendre d'OCCT).
pub mod step {
    use super::{CadBudgets, CadError, Solide};

    /// Exporte les solides en octets STEP.
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si l'écriture OCCT échoue.
    pub fn exporter(solides: &[Solide]) -> Result<Vec<u8>, CadError> {
        let formes: Vec<&crate::occt::Forme> = solides.iter().map(|s| &s.forme).collect();
        super::octets_via_fichier_temporaire(".step", |chemin| {
            crate::occt::Forme::ecrire_step_tous(&formes, chemin).map_err(|e| CadError::Occt {
                operation: format!("export STEP : {e}"),
            })
        })
    }

    /// Importe les octets STEP (le lot 1 lit le fichier comme une forme
    /// unique : composés inclus).
    ///
    /// # Erreurs
    /// [`CadError::Budget`] (OF-CAD-0004) avant le premier octet confié à
    /// OCCT ; [`CadError::Illisible`] (OF-CAD-0003) si le lecteur refuse.
    pub fn importer(octets: &[u8], budgets: &CadBudgets) -> Result<Vec<Solide>, CadError> {
        if octets.len() > budgets.octets_max {
            return Err(CadError::Budget {
                limite: format!("octets_max = {}", budgets.octets_max),
            });
        }
        if budgets.formes_max < 1 {
            return Err(CadError::Budget {
                limite: format!("formes_max = {}", budgets.formes_max),
            });
        }
        let nom = format!("omnifab-cad-{}.step", uuid::Uuid::now_v7().simple());
        let chemin = std::env::temp_dir().join(nom);
        if let Err(e) = std::fs::write(&chemin, octets) {
            let _ = std::fs::remove_file(&chemin);
            return Err(CadError::Occt {
                operation: format!("écriture du fichier temporaire : {e}"),
            });
        }
        let lecture = crate::occt::Forme::lire_step(&chemin);
        let _ = std::fs::remove_file(&chemin);
        let forme = lecture.map_err(|_| CadError::Illisible {
            formate: "STEP".to_string(),
            raison: "refusé par le lecteur OCCT".to_string(),
        })?;
        Ok(vec![Solide { forme }])
    }
}

/// Import/export BREP textuel (format natif OCCT — ADR-0023 D4).
pub mod brep {
    use super::{CadBudgets, CadError, Solide};

    /// Exporte UN solide en octets BREP textuels.
    ///
    /// # Erreurs
    /// [`CadError::Occt`] (OF-CAD-0001) si l'écriture OCCT échoue.
    pub fn exporter(solide: &Solide) -> Result<Vec<u8>, CadError> {
        super::octets_via_fichier_temporaire(".brep", |chemin| {
            crate::occt::Forme::ecrire_brep(&solide.forme, chemin).map_err(|e| CadError::Occt {
                operation: format!("export BREP : {e}"),
            })
        })
    }

    /// Importe les octets BREP textuels (une forme par fichier).
    ///
    /// # Erreurs
    /// [`CadError::Budget`] (OF-CAD-0004) avant le premier octet confié à
    /// OCCT ; [`CadError::Illisible`] (OF-CAD-0003) si le lecteur refuse.
    pub fn importer(octets: &[u8], budgets: &CadBudgets) -> Result<Vec<Solide>, CadError> {
        if octets.len() > budgets.octets_max {
            return Err(CadError::Budget {
                limite: format!("octets_max = {}", budgets.octets_max),
            });
        }
        if budgets.formes_max < 1 {
            return Err(CadError::Budget {
                limite: format!("formes_max = {}", budgets.formes_max),
            });
        }
        let nom = format!("omnifab-cad-{}.brep", uuid::Uuid::now_v7().simple());
        let chemin = std::env::temp_dir().join(nom);
        if let Err(e) = std::fs::write(&chemin, octets) {
            let _ = std::fs::remove_file(&chemin);
            return Err(CadError::Occt {
                operation: format!("écriture du fichier temporaire : {e}"),
            });
        }
        let lecture = crate::occt::Forme::lire_brep(&chemin);
        let _ = std::fs::remove_file(&chemin);
        let forme = lecture.map_err(|_| CadError::Illisible {
            formate: "BREP".to_string(),
            raison: "refusé par le lecteur OCCT".to_string(),
        })?;
        Ok(vec![Solide { forme }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A3-08.f : le fichier temporaire est privé (0o600 sur Unix) — le
    /// 0o644 par défaut de `fs::write` laisse fuir la géométrie vers
    /// tout utilisateur local de la machine.
    #[cfg(unix)]
    #[test]
    fn fichier_temporaire_est_prive() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::{Arc, Mutex};

        let mode = Arc::new(Mutex::new(None));
        let vu = Arc::clone(&mode);
        octets_via_fichier_temporaire(".step", |chemin| {
            std::fs::write(chemin, b"").map_err(|e| CadError::Occt {
                operation: format!("écriture de test : {e}"),
            })?;
            let meta = std::fs::metadata(chemin).map_err(|e| CadError::Occt {
                operation: format!("lecture des permissions : {e}"),
            })?;
            *vu.lock().unwrap() = Some(meta.permissions().mode() & 0o777);
            Ok(())
        })
        .unwrap();
        assert_eq!(*mode.lock().unwrap(), Some(0o600));
    }

    /// A3-08.f : le fichier temporaire est supprimé quand l'écriture
    /// échoue — propriété à conserver pendant la migration vers la
    /// garde RAII.
    #[test]
    fn fichier_temporaire_supprime_quand_l_ecriture_echoue() {
        use std::sync::{Arc, Mutex};

        let fuite = Arc::new(Mutex::new(None));
        let vu = Arc::clone(&fuite);
        let resultat = octets_via_fichier_temporaire(".step", |chemin| {
            std::fs::write(chemin, b"").map_err(|e| CadError::Occt {
                operation: format!("écriture de test : {e}"),
            })?;
            *vu.lock().unwrap() = Some(chemin.to_path_buf());
            Err(CadError::FormeInvalide {
                operation: "échec simulé".to_string(),
            })
        });
        assert!(resultat.is_err());
        let chemin = fuite.lock().unwrap().take().expect("chemin capturé");
        assert!(!chemin.exists(), "le fichier temporaire doit être supprimé");
    }

    /// A3-08.f : le fichier temporaire est supprimé même sur une panique
    /// pendant l'écriture — un `remove_file` manuel fuit sur ce chemin.
    #[test]
    fn fichier_temporaire_supprime_apres_panique() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        use std::sync::{Arc, Mutex};

        let fuite = Arc::new(Mutex::new(None));
        let vu = Arc::clone(&fuite);
        let panique = catch_unwind(AssertUnwindSafe(|| {
            let _ = octets_via_fichier_temporaire(".brep", |chemin| {
                std::fs::write(chemin, b"").map_err(|e| CadError::Occt {
                    operation: format!("écriture de test : {e}"),
                })?;
                *vu.lock().unwrap() = Some(chemin.to_path_buf());
                panic!("panique simulée pendant l'écriture");
            });
        }));
        assert!(panique.is_err(), "la panique simulée doit être propagée");
        let chemin = fuite.lock().unwrap().take().expect("chemin capturé");
        assert!(!chemin.exists(), "le fichier temporaire doit être supprimé");
    }
}
