//! Test d'intégration des signalements A4-04 de l'audit n°4 : le
//! diagnostic signale la troncature de ses listes de tags (plafond ABI
//! de 32) au lieu de couper en silence, et le statut de résolution 3
//! du solveur (SuccessfulSolutionInvalid) est une erreur typée, pas un
//! « code ABI (exception C++ rattrapée) » trompeur.

use forge_gcs::{Algorithme, Contrainte, GcsError, Solveur2D, Systeme2D};
use forge_units::Length;

/// 40 contraintes de distance identiques entre deux points libres :
/// une seule équation indépendante, les autres sont redondantes — la
/// liste dépasse le plafond GCS_MAX_TAGS (32) du shim et le diagnostic
/// doit le signaler (A4-04).
#[test]
fn diagnostic_signale_la_troncature_des_listes() -> Result<(), GcsError> {
    let mut solveur = Systeme2D::new()?;
    let a = solveur.ajouter_point(Length::from_millimeters(0.0), Length::from_millimeters(0.0))?;
    let b = solveur.ajouter_point(
        Length::from_millimeters(10.0),
        Length::from_millimeters(0.0),
    )?;
    for _ in 0..40 {
        solveur.ajouter_contrainte(Contrainte::Distance {
            p1: a,
            p2: b,
            valeur: Length::from_millimeters(10.0),
        })?;
    }
    let diagnostic = solveur.diagnostic(Algorithme::DogLeg)?;
    assert!(
        diagnostic.tronque(),
        "40 contraintes redondantes : la troncature à 32 doit être signalée"
    );
    assert!(
        diagnostic.redondantes().len() <= 32,
        "une liste tronquée ne dépasse jamais le plafond ABI"
    );
    Ok(())
}

/// Le statut 3 du solveur (SuccessfulSolutionInvalid chez planegcs :
/// résolution réussie mais géométrie invalide) doit être une erreur
/// typée OF-GCS-0005, jamais le fourre-tout AbiInvalide (A4-04).
#[test]
fn statut_solution_invalide_est_une_erreur_typee() {
    let erreur = GcsError::SolutionInvalide {
        algorithme: Algorithme::DogLeg,
    };
    assert_eq!(erreur.code(), "OF-GCS-0005");
    assert!(
        erreur.to_string().contains("SuccessfulSolutionInvalid"),
        "le message doit nommer le statut planegcs : {erreur}"
    );
}
