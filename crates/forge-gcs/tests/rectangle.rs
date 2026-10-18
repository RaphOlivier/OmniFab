//! Croquis doré de non-régression « rectangle contraint » (ADR-0018,
//! lot 2 : critère de sortie n°2 de la phase D).
//!
//! Même croquis de référence que le noyau forge-session : quatre
//! lignes fermées, quatre contraintes H/V (deux horizontales, deux
//! verticales), deux distances (100 mm et 50 mm), un coin fixé à
//! l'origine. Les graines sont volontairement déformées : la
//! résolution doit retomber sur le rectangle exact à la tolérance du
//! projet (1e-6 mm), avec des degrés de liberté nuls. Aucun
//! changement de frontière : ce croquis ne consomme que l'API
//! publique de la crate (trait Solveur2D).

use forge_gcs::{Algorithme, Contrainte, GcsError, IdPoint, Solveur2D, Systeme2D};
use forge_units::{Length, TOLERANCE_MM};

/// Construit le rectangle doré ; renvoie les quatre coins (p1..p4).
///
/// Inconnues : p2, p3 et p4 (p1 est fixé). Contraintes : l1 et l3
/// horizontales, l2 et l4 verticales, |p1p2| = 100 mm, |p2p3| = 50 mm
/// — soit 6 contraintes pour 6 inconnues : degrés de liberté
/// attendus, 0. Les positions de départ sont déformées exprès : la
/// résolution doit redresser le rectangle, pas le préserver.
fn construire_rectangle(
    solveur: &mut impl Solveur2D,
) -> Result<(IdPoint, IdPoint, IdPoint, IdPoint), GcsError> {
    let p1 = solveur.ajouter_point(Length::from_millimeters(0.0), Length::from_millimeters(0.0))?;
    let p2 = solveur.ajouter_point(
        Length::from_millimeters(80.0),
        Length::from_millimeters(-3.0),
    )?;
    let p3 = solveur.ajouter_point(
        Length::from_millimeters(97.0),
        Length::from_millimeters(44.0),
    )?;
    let p4 = solveur.ajouter_point(
        Length::from_millimeters(-6.0),
        Length::from_millimeters(51.0),
    )?;
    let l1 = solveur.ajouter_ligne(p1, p2)?;
    let l2 = solveur.ajouter_ligne(p2, p3)?;
    let l3 = solveur.ajouter_ligne(p3, p4)?;
    let l4 = solveur.ajouter_ligne(p4, p1)?;

    solveur.fixer_point(p1)?;
    solveur.ajouter_contrainte(Contrainte::Horizontale { ligne: l1 })?;
    solveur.ajouter_contrainte(Contrainte::Horizontale { ligne: l3 })?;
    solveur.ajouter_contrainte(Contrainte::Verticale { ligne: l2 })?;
    solveur.ajouter_contrainte(Contrainte::Verticale { ligne: l4 })?;
    solveur.ajouter_contrainte(Contrainte::Distance {
        p1,
        p2,
        valeur: Length::from_millimeters(100.0),
    })?;
    solveur.ajouter_contrainte(Contrainte::Distance {
        p1: p2,
        p2: p3,
        valeur: Length::from_millimeters(50.0),
    })?;
    Ok((p1, p2, p3, p4))
}

/// Le rectangle doré est résolu exactement : positions à la tolérance
/// géométrique du projet (1e-6 mm), degrés de liberté nuls, ni conflit
/// ni redondance, reproductibilité garantie (critère de sortie n°2 de
/// l'ADR-0018).
#[test]
fn rectangle_resout_positions_exactes() -> Result<(), GcsError> {
    let mut solveur = Systeme2D::new()?;
    let (_p1, p2, p3, p4) = construire_rectangle(&mut solveur)?;

    solveur.resoudre(Algorithme::DogLeg)?;

    // Positions exactes : la résolution a redressé le rectangle
    // déformé en 100 mm × 50 mm ancré à l'origine.
    for (point, attendu_x, attendu_y) in [(p2, 100.0, 0.0), (p3, 100.0, 50.0), (p4, 0.0, 50.0)] {
        let (x, y) = solveur.position_point(point)?;
        assert!(
            (x.millimeters() - attendu_x).abs() <= TOLERANCE_MM,
            "x attendu {} mm, trouvé {} mm",
            attendu_x,
            x.millimeters()
        );
        assert!(
            (y.millimeters() - attendu_y).abs() <= TOLERANCE_MM,
            "y attendu {} mm, trouvé {} mm",
            attendu_y,
            y.millimeters()
        );
    }

    // Croquis entièrement contraint, sans conflit ni redondance, et
    // reproductible (garde FreeCAD #33399 de l'ADR-0013).
    let diagnostic = solveur.diagnostic(Algorithme::DogLeg)?;
    assert_eq!(diagnostic.dof(), 0, "degrés de liberté attendus : 0");
    assert!(
        diagnostic.conflictuelles().is_empty(),
        "contraintes conflictuelles inattendues"
    );
    assert!(
        diagnostic.redondantes().is_empty(),
        "contraintes redondantes inattendues"
    );
    assert!(
        diagnostic.reproductible(),
        "l'arène doit garantir la reproductibilité"
    );
    Ok(())
}
