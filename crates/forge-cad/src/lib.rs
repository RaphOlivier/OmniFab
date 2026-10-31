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
use forge_units::TOLERANCE_MM;

pub use erreur::CadError;

/// Nombre maximal de points d'un contour de prisme (convention « entrées
/// bornées », v12) : les contrôles de distinctness et d'auto-intersection
/// sont en O(n²) — le plafond borne le coût avant tout travail géométrique.
pub const MAX_POINTS_CONTOUR: usize = 4096;

/// Valide qu'une dimension est finie et strictement positive — le noyau
/// OCCT n'apporte aucune garde (spike phase A, cas 4) : jamais d'entrée
/// brute confiée au pont.
fn valider_dimension(nom: &str, valeur: Length) -> Result<(), CadError> {
    let mm = valeur.millimeters();
    if !mm.is_finite() || mm <= 0.0 {
        return Err(CadError::FormeInvalide {
            operation: format!("dimension {nom} : {mm} mm, finie > 0 requise"),
        });
    }
    Ok(())
}

/// Valide qu'une coordonnée de position est finie (signe libre).
fn valider_coordonnee(nom: &str, valeur: Length) -> Result<(), CadError> {
    if !valeur.millimeters().is_finite() {
        return Err(CadError::FormeInvalide {
            operation: format!("coordonnée {nom} non finie"),
        });
    }
    Ok(())
}

/// Valide le contour d'un prisme AVANT le pont OCCT (audit n°3 A3-08) :
/// 3 points distincts minimum à [`TOLERANCE_MM`], coordonnées finies,
/// arêtes cycliques non nulles (fermeture comprise — le contour est
/// fermé implicitement, ne pas répéter le premier point), aire de lacet
/// non nulle (orientation libre). Les contours auto-sécants — deux
/// arêtes non adjacentes qui se croisent ou se touchent — sont
/// refusés (dette lot 1) : l'extrusion d'une face repliée donne un
/// solide invalide sans erreur OCCT.
fn valider_contour(contour: &[(Length, Length)]) -> Result<(), CadError> {
    let n = contour.len();
    if n < 3 {
        return Err(CadError::FormeInvalide {
            operation: format!("contour : {n} points, 3 minimum"),
        });
    }
    if n > MAX_POINTS_CONTOUR {
        return Err(CadError::FormeInvalide {
            operation: format!("contour : {n} points, {MAX_POINTS_CONTOUR} maximum"),
        });
    }
    let points: Vec<(f64, f64)> = contour
        .iter()
        .map(|(x, y)| (x.millimeters(), y.millimeters()))
        .collect();
    for (x, y) in &points {
        if !x.is_finite() || !y.is_finite() {
            return Err(CadError::FormeInvalide {
                operation: "coordonnée du contour non finie".to_string(),
            });
        }
    }
    // arêtes cycliques non nulles : distance consécutive au-dessus de la
    // tolérance, fermeture dernier → premier comprise.
    for i in 0..n {
        let (x1, y1) = points[i];
        let (x2, y2) = points[(i + 1) % n];
        if (x2 - x1).hypot(y2 - y1) <= TOLERANCE_MM {
            return Err(CadError::FormeInvalide {
                operation: format!("arête nulle entre les points {i} et {}", (i + 1) % n),
            });
        }
    }
    // au moins 3 points distincts (le contrôle d'arêtes n'écarte que les
    // doublons consécutifs) — O(n²) borné par le plafond.
    let mut distincts: Vec<(f64, f64)> = Vec::new();
    for (x, y) in &points {
        let inedit = distincts
            .iter()
            .all(|(dx, dy)| (x - dx).hypot(y - dy) > TOLERANCE_MM);
        if inedit {
            distincts.push((*x, *y));
        }
    }
    if distincts.len() < 3 {
        return Err(CadError::FormeInvalide {
            operation: format!("contour : {} points distincts, 3 minimum", distincts.len()),
        });
    }
    // aire du lacet : nulle si tous les points sont alignés.
    let mut aire = 0.0;
    for i in 0..n {
        let (x1, y1) = points[i];
        let (x2, y2) = points[(i + 1) % n];
        aire += x1 * y2 - x2 * y1;
    }
    if aire.abs() < f64::EPSILON {
        return Err(CadError::FormeInvalide {
            operation: "aire du contour nulle : points alignés".to_string(),
        });
    }
    // contours auto-sécants (dette lot 1) : deux arêtes NON ADJACENTES
    // qui se croisent ou se touchent — les paires adjacentes partagent
    // un sommet sain par construction et sont exclues. O(n²) borné par
    // le plafond, comme le contrôle de distinctness.
    for i in 0..n {
        for j in (i + 2)..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let debut_a = points[i];
            let fin_a = points[(i + 1) % n];
            let debut_b = points[j];
            let fin_b = points[(j + 1) % n];
            if segments_se_touchent(debut_a, fin_a, debut_b, fin_b) {
                return Err(CadError::FormeInvalide {
                    operation: format!("contour auto-sécant : arêtes {i} et {j} se croisent"),
                });
            }
        }
    }
    Ok(())
}

/// Aire signée du triangle (a, b, c) — nulle à l'epsilon près si
/// les trois points sont alignés.
fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Vrai si le point `p` — supposé aligné avec le segment — est
/// entre ses extrémités (boîte englobante).
fn entre(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

/// Vrai si les segments [a, b] et [c, d] se croisent ou se touchent
/// (test d'orientation classique ; l'epsilon d'alignement est en
/// mm², cohérent avec des aires de lacet). Réservé aux paires
/// d'arêtes NON ADJACENTES du contour fermé.
fn segments_se_touchent(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    const EPS_AIRE: f64 = 1e-12;
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    let opposes_1 = (o1 > EPS_AIRE && o2 < -EPS_AIRE) || (o1 < -EPS_AIRE && o2 > EPS_AIRE);
    let opposes_2 = (o3 > EPS_AIRE && o4 < -EPS_AIRE) || (o3 < -EPS_AIRE && o4 > EPS_AIRE);
    if opposes_1 && opposes_2 {
        return true;
    }
    // contacts : un sommet de l'autre segment est aligné ET entre
    // les extrémités du premier.
    if o1.abs() <= EPS_AIRE && entre(c, a, b) {
        return true;
    }
    if o2.abs() <= EPS_AIRE && entre(d, a, b) {
        return true;
    }
    if o3.abs() <= EPS_AIRE && entre(a, c, d) {
        return true;
    }
    if o4.abs() <= EPS_AIRE && entre(b, c, d) {
        return true;
    }
    false
}

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
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si une dimension n'est
    /// pas finie ou pas strictement positive ; [`CadError::Occt`]
    /// (OF-CAD-0001) si la construction OCCT échoue.
    pub fn boite(largeur: Length, profondeur: Length, hauteur: Length) -> Result<Self, CadError> {
        valider_dimension("largeur", largeur)?;
        valider_dimension("profondeur", profondeur)?;
        valider_dimension("hauteur", hauteur)?;
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
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si le rayon ou la
    /// hauteur n'est pas fini strictement positif, ou si une coordonnée
    /// du centre n'est pas finie ; [`CadError::Occt`] (OF-CAD-0001) si
    /// la construction OCCT échoue.
    pub fn cylindre_centre(
        rayon: Length,
        x: Length,
        y: Length,
        hauteur: Length,
    ) -> Result<Self, CadError> {
        valider_dimension("rayon", rayon)?;
        valider_coordonnee("x du centre", x)?;
        valider_coordonnee("y du centre", y)?;
        valider_dimension("hauteur", hauteur)?;
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
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si le contour est
    /// dégénéré (moins de 3 points distincts, arête nulle, coordonnée
    /// non finie, aire nulle, auto-sécant, hors plafond) ou si la hauteur
    /// n'est pas finie strictement positive ; [`CadError::Occt`] (OF-CAD-0001)
    /// si l'extrusion OCCT échoue.
    pub fn prisme(contour: &[(Length, Length)], hauteur: Length) -> Result<Self, CadError> {
        valider_contour(contour)?;
        valider_dimension("hauteur du prisme", hauteur)?;
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
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si le résultat est
    /// vide — opérandes identiques ou disjoints : jamais un `Ok`
    /// silencieux au volume nul (audit n°3 A3-08.b).
    pub fn soustraire(&self, autre: &Solide) -> Result<Self, CadError> {
        let forme = occt::Forme::soustraire(&self.forme, &autre.forme).map_err(|raison| {
            CadError::FormeInvalide {
                operation: format!("soustraction booléenne : {raison}"),
            }
        })?;
        Ok(Self { forme })
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
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si la tolérance n'est
    /// pas finie strictement positive ; [`CadError::Occt`] (OF-CAD-0001)
    /// si la tessellation échoue.
    pub fn tesseller(&self, tolerance: Length) -> Result<Maillage, CadError> {
        valider_dimension("tolérance de tessellation", tolerance)?;
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

    /// Volume du solide en millimètres cubes, par tessellation — rendu en
    /// valeur absolue : l'orientation des triangles de la tessellation
    /// ne fuit jamais dans le résultat (audit n°3 A3-08.d).
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
        Ok(volume.abs())
    }
}

/// Fichier temporaire privé (0o600 sur Unix), supprimé à la sortie du
/// scope — même sur panique ou erreur précoce (audit n°3 A3-08.f : le
/// 0o644 par défaut de `fs::write` laisse fuir la géométrie vers tout
/// utilisateur local de la machine).
struct FichierTemporaire {
    chemin: std::path::PathBuf,
}

impl FichierTemporaire {
    /// Réserve un nom unique et crée le fichier.
    fn creer(suffixe: &str) -> Result<Self, CadError> {
        let nom = format!("omnifab-cad-{}{suffixe}", uuid::Uuid::now_v7().simple());
        let chemin = std::env::temp_dir().join(nom);
        creer_fichier(&chemin)?;
        Ok(Self { chemin })
    }

    /// Chemin du fichier temporaire.
    fn chemin(&self) -> &std::path::Path {
        &self.chemin
    }
}

impl Drop for FichierTemporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.chemin);
    }
}

/// Crée le fichier exclusivement (échec s'il existe déjà), avec les
/// permissions 0o600 sur Unix.
fn creer_fichier(chemin: &std::path::Path) -> Result<(), CadError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(chemin)
        .map(|_| ())
        .map_err(|e| CadError::Occt {
            operation: format!("création du fichier temporaire : {e}"),
        })
}

/// Écrit dans le fichier temporaire privé via `ecrire`, lit les octets —
/// le fichier est supprimé à la sortie du scope par la garde RAII, même
/// sur panique (le lot 1 d'OCCT travaille par fichiers, pas de flux
/// mémoire).
fn octets_via_fichier_temporaire(
    suffixe: &str,
    ecrire: impl FnOnce(&std::path::Path) -> Result<(), CadError>,
) -> Result<Vec<u8>, CadError> {
    let fichier = FichierTemporaire::creer(suffixe)?;
    ecrire(fichier.chemin())?;
    std::fs::read(fichier.chemin()).map_err(|e| CadError::Occt {
        operation: format!("lecture du fichier temporaire : {e}"),
    })
}

/// Import/export STEP (ADR-0023 D4 : forge-cad, jamais forge-io — la
/// frontière ADR-0009 interdit à forge-io de dépendre d'OCCT).
pub mod step {
    use super::{CadBudgets, CadError, FichierTemporaire, Solide};

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
    /// OCCT ; [`CadError::Illisible`] (OF-CAD-0003) si le lecteur refuse ;
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si la forme importée est
    /// vide ou non solide.
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
        let fichier = FichierTemporaire::creer(".step")?;
        if let Err(e) = std::fs::write(fichier.chemin(), octets) {
            return Err(CadError::Occt {
                operation: format!("écriture du fichier temporaire : {e}"),
            });
        }
        let lecture = crate::occt::Forme::lire_step(fichier.chemin());
        let forme = lecture.map_err(|_| CadError::Illisible {
            formate: "STEP".to_string(),
            raison: "refusé par le lecteur OCCT".to_string(),
        })?;
        if forme.est_vide() {
            return Err(CadError::FormeInvalide {
                operation: "import : forme vide ou non solide".to_string(),
            });
        }
        Ok(vec![Solide { forme }])
    }
}

/// Import/export BREP textuel (format natif OCCT — ADR-0023 D4).
pub mod brep {
    use super::{CadBudgets, CadError, FichierTemporaire, Solide};

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
    /// OCCT ; [`CadError::Illisible`] (OF-CAD-0003) si le lecteur refuse ;
    /// [`CadError::FormeInvalide`] (OF-CAD-0002) si la forme importée est
    /// vide ou non solide.
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
        let fichier = FichierTemporaire::creer(".brep")?;
        if let Err(e) = std::fs::write(fichier.chemin(), octets) {
            return Err(CadError::Occt {
                operation: format!("écriture du fichier temporaire : {e}"),
            });
        }
        let lecture = crate::occt::Forme::lire_brep(fichier.chemin());
        let forme = lecture.map_err(|_| CadError::Illisible {
            formate: "BREP".to_string(),
            raison: "refusé par le lecteur OCCT".to_string(),
        })?;
        if forme.est_vide() {
            return Err(CadError::FormeInvalide {
                operation: "import : forme vide ou non solide".to_string(),
            });
        }
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
    /// échoue — propriété conservée par la garde RAII.
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
