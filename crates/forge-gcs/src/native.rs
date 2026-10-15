//! Implémentation native du trait [`Solveur2D`] (ADR-0013) : handle
//! opaque sur le shim ABI C de planegcs.
//!
//! Toute exception C++ est déjà rattrapée dans le shim ; ici, on
//! traduit uniquement les codes d'erreur ABI en [`GcsError`] typée.
//! Les valeurs de cote deviennent des paramètres **fixes** du solveur
//! (cf. `Contrainte`) : c'est ce que pilotera forge-values en phase E.

use crate::{
    Algorithme, Contrainte, Diagnostic, GcsError, IdArc, IdCercle, IdLigne, IdPoint, Solveur2D,
    StatutResolution, TagContrainte,
};
use forge_gcs_sys as sys;
use forge_units::{Angle, Length};

/// Système de contraintes natif : handle opaque sur le shim ABI C.
///
/// Le handle est libéré à la destruction (`gcs_detruire`). planegcs
/// n'étant pas thread-safe, le handle n'est ni [`Send`] ni [`Sync`] au
/// spike (usage monothread).
#[derive(Debug)]
pub struct Systeme2D {
    systeme: *mut sys::GcsSysteme,
}

impl Systeme2D {
    /// Crée un système de contraintes vide.
    pub fn new() -> Result<Self, GcsError> {
        let systeme = unsafe { sys::gcs_creer() };
        if systeme.is_null() {
            return Err(GcsError::AbiInvalide {
                contexte: "gcs_creer a renvoyé un handle nul".to_string(),
            });
        }
        Ok(Self { systeme })
    }

    /// Crée un paramètre fixe (valeur de cote) et renvoie son
    /// identifiant ABI. Usage interne du wrapper.
    fn param_valeur(&mut self, valeur: f64) -> Result<i32, GcsError> {
        let code = unsafe { sys::gcs_ajouter_parametre(self.systeme, valeur, 1) };
        if code < 0 {
            return Err(Self::traduire(code, "gcs_ajouter_parametre"));
        }
        Ok(code)
    }

    /// Traduit un code d'erreur ABI (négatif) en [`GcsError`].
    fn traduire(code: i32, contexte: &'static str) -> GcsError {
        if code == sys::GCS_ERR_ID_INCONNU {
            return GcsError::IdInconnu {
                contexte: contexte.to_string(),
            };
        }
        if code == sys::GCS_ERR_VALEUR {
            return GcsError::AbiInvalide {
                contexte: format!("{contexte} : valeur non finie refusée par le shim"),
            };
        }
        GcsError::AbiInvalide {
            contexte: format!("{contexte} : code ABI {code} (exception C++ rattrapée)"),
        }
    }
}

impl Drop for Systeme2D {
    fn drop(&mut self) {
        unsafe { sys::gcs_detruire(self.systeme) };
    }
}

impl Solveur2D for Systeme2D {
    fn ajouter_point(&mut self, x: Length, y: Length) -> Result<IdPoint, GcsError> {
        let code =
            unsafe { sys::gcs_ajouter_point(self.systeme, x.millimeters(), y.millimeters()) };
        if code < 0 {
            return Err(Self::traduire(code, "gcs_ajouter_point"));
        }
        Ok(IdPoint(code))
    }

    fn ajouter_ligne(&mut self, p1: IdPoint, p2: IdPoint) -> Result<IdLigne, GcsError> {
        let code = unsafe { sys::gcs_ajouter_ligne(self.systeme, p1.0, p2.0) };
        if code < 0 {
            return Err(Self::traduire(code, "gcs_ajouter_ligne"));
        }
        Ok(IdLigne(code))
    }

    fn ajouter_cercle(&mut self, centre: IdPoint, rayon: Length) -> Result<IdCercle, GcsError> {
        let code = unsafe { sys::gcs_ajouter_cercle(self.systeme, centre.0, rayon.millimeters()) };
        if code < 0 {
            return Err(Self::traduire(code, "gcs_ajouter_cercle"));
        }
        Ok(IdCercle(code))
    }

    fn ajouter_arc(
        &mut self,
        centre: IdPoint,
        debut: IdPoint,
        fin: IdPoint,
        rayon: Length,
        angle_debut: Angle,
        angle_fin: Angle,
    ) -> Result<IdArc, GcsError> {
        let code = unsafe {
            sys::gcs_ajouter_arc(
                self.systeme,
                centre.0,
                debut.0,
                fin.0,
                rayon.millimeters(),
                angle_debut.radians(),
                angle_fin.radians(),
            )
        };
        if code < 0 {
            return Err(Self::traduire(code, "gcs_ajouter_arc"));
        }
        Ok(IdArc(code))
    }

    fn fixer_point(&mut self, point: IdPoint) -> Result<(), GcsError> {
        let code = unsafe { sys::gcs_fixer_point(self.systeme, point.0, 1) };
        if code != sys::GCS_OK {
            return Err(Self::traduire(code, "gcs_fixer_point"));
        }
        Ok(())
    }

    fn ajouter_contrainte(&mut self, contrainte: Contrainte) -> Result<TagContrainte, GcsError> {
        let (code, contexte) = match contrainte {
            Contrainte::Coincidence { p1, p2 } => (
                unsafe { sys::gcs_contrainte_coincidence(self.systeme, p1.0, p2.0) },
                "gcs_contrainte_coincidence",
            ),
            Contrainte::Horizontale { ligne } => (
                unsafe { sys::gcs_contrainte_horizontale(self.systeme, ligne.0) },
                "gcs_contrainte_horizontale",
            ),
            Contrainte::Verticale { ligne } => (
                unsafe { sys::gcs_contrainte_verticale(self.systeme, ligne.0) },
                "gcs_contrainte_verticale",
            ),
            Contrainte::Distance { p1, p2, valeur } => {
                let v = self.param_valeur(valeur.millimeters())?;
                (
                    unsafe { sys::gcs_contrainte_distance(self.systeme, p1.0, p2.0, v) },
                    "gcs_contrainte_distance",
                )
            }
            Contrainte::Angle {
                ligne1,
                ligne2,
                valeur,
            } => {
                let v = self.param_valeur(valeur.radians())?;
                (
                    unsafe { sys::gcs_contrainte_angle_l2l(self.systeme, ligne1.0, ligne2.0, v) },
                    "gcs_contrainte_angle_l2l",
                )
            }
            Contrainte::MemeLongueur { ligne1, ligne2 } => (
                unsafe { sys::gcs_contrainte_meme_longueur(self.systeme, ligne1.0, ligne2.0) },
                "gcs_contrainte_meme_longueur",
            ),
            Contrainte::PointSurLigne { point, ligne } => (
                unsafe { sys::gcs_contrainte_point_sur_ligne(self.systeme, point.0, ligne.0) },
                "gcs_contrainte_point_sur_ligne",
            ),
            Contrainte::Paralleles { ligne1, ligne2 } => (
                unsafe { sys::gcs_contrainte_paralleles(self.systeme, ligne1.0, ligne2.0) },
                "gcs_contrainte_paralleles",
            ),
            Contrainte::Perpendiculaires { ligne1, ligne2 } => (
                unsafe { sys::gcs_contrainte_perpendiculaires(self.systeme, ligne1.0, ligne2.0) },
                "gcs_contrainte_perpendiculaires",
            ),
            Contrainte::RayonCercle { cercle, valeur } => {
                let v = self.param_valeur(valeur.millimeters())?;
                (
                    unsafe { sys::gcs_contrainte_rayon_cercle(self.systeme, cercle.0, v) },
                    "gcs_contrainte_rayon_cercle",
                )
            }
            Contrainte::PointSurCercle { point, cercle } => (
                unsafe { sys::gcs_contrainte_point_sur_cercle(self.systeme, point.0, cercle.0) },
                "gcs_contrainte_point_sur_cercle",
            ),
        };
        if code < 0 {
            return Err(Self::traduire(code, contexte));
        }
        Ok(TagContrainte(code))
    }

    fn resoudre(&mut self, algorithme: Algorithme) -> Result<StatutResolution, GcsError> {
        let code = unsafe { sys::gcs_resoudre(self.systeme, algorithme.code_abi()) };
        let statut = match code {
            x if x == sys::GCS_STATUT_SUCCES => StatutResolution::Succes,
            x if x == sys::GCS_STATUT_CONVERGE => StatutResolution::Converge,
            x if x == sys::GCS_STATUT_ECHEC => {
                return Err(GcsError::ResolutionEchouee { algorithme });
            }
            x => return Err(Self::traduire(x, "gcs_resoudre")),
        };
        // Garde ADR-0013 : reproductibilité obligatoire (FreeCAD
        // #33399) — l'arène doit maintenir des adresses croissantes.
        let ordonnes = unsafe { sys::gcs_params_ordonnes(self.systeme) };
        if ordonnes == 0 {
            return Err(GcsError::NonReproductible {
                contexte: "arène de paramètres débordée : adresses non croissantes".to_string(),
            });
        }
        Ok(statut)
    }

    fn position_point(&self, point: IdPoint) -> Result<(Length, Length), GcsError> {
        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;
        let code = unsafe { sys::gcs_lire_point(self.systeme, point.0, &mut x, &mut y) };
        if code != sys::GCS_OK {
            return Err(Self::traduire(code, "gcs_lire_point"));
        }
        if !x.is_finite() || !y.is_finite() {
            return Err(GcsError::AbiInvalide {
                contexte: "position de point non finie après résolution".to_string(),
            });
        }
        Ok((Length::from_millimeters(x), Length::from_millimeters(y)))
    }

    fn diagnostic(&mut self, algorithme: Algorithme) -> Result<Diagnostic, GcsError> {
        let mut brut = sys::GcsDiagnostic {
            dof: 0,
            params_ordonnes: 0,
            conflictuelles: [0; sys::GCS_MAX_TAGS as usize],
            conflictuelles_nombre: 0,
            redondantes: [0; sys::GCS_MAX_TAGS as usize],
            redondantes_nombre: 0,
            partiellement_redondantes: [0; sys::GCS_MAX_TAGS as usize],
            partiellement_redondantes_nombre: 0,
        };
        let code =
            unsafe { sys::gcs_diagnostiquer(self.systeme, algorithme.code_abi(), &mut brut) };
        if code != sys::GCS_OK {
            return Err(Self::traduire(code, "gcs_diagnostiquer"));
        }
        let extraire = |source: &[i32], nombre: i32| -> Vec<TagContrainte> {
            source
                .iter()
                .take(nombre.max(0) as usize)
                .map(|tag| TagContrainte(*tag))
                .collect()
        };
        Ok(Diagnostic {
            dof: brut.dof.max(0) as usize,
            reproductible: brut.params_ordonnes != 0,
            conflictuelles: extraire(&brut.conflictuelles, brut.conflictuelles_nombre),
            redondantes: extraire(&brut.redondantes, brut.redondantes_nombre),
            partiellement_redondantes: extraire(
                &brut.partiellement_redondantes,
                brut.partiellement_redondantes_nombre,
            ),
        })
    }
}
