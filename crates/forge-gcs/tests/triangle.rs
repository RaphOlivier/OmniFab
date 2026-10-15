//! Test d'intégration du spike planegcs (ADR-0013, phase A).
//!
//! Croquis de référence : un triangle rectangle entièrement contraint
//! (p1 ancré à l'origine, base horizontale de 20 mm, côté vertical de
//! 15 mm). Critères de sortie de la phase A :
//! résolution exacte à la tolérance du projet (1e-6 mm) sur la matrice
//! CI multi-OS, et reproductibilité des positions (FreeCAD #33399).

use forge_gcs::{
    Algorithme, Contrainte, GcsError, IdPoint, Solveur2D, Systeme2D,
};
use forge_units::{Length, TOLERANCE_MM};

/// Construit le croquis de référence ; renvoie (p1, p2, p3).
///
/// Inconnues : p2 et p3 (p1 est fixé). Contraintes : l1 horizontale,
/// |p1p2| = 20 mm, l1 ⊥ l2, |p2p3| = 15 mm — soit 4 contraintes pour
/// 4 inconnues : degrés de liberté attendus, 0.
fn construire_triangle(
    solveur: &mut impl Solveur2D,
) -> Result<(IdPoint, IdPoint, IdPoint), GcsError> {
    let p1 = solveur.ajouter_point(
        Length::from_millimeters(0.0),
        Length::from_millimeters(0.0),
    )?;
    // Positions initiales proches de la solution attendue : le solveur
    // converge vers (20, 0) et (20, 15).
    let p2 = solveur.ajouter_point(
        Length::from_millimeters(19.0),
        Length::from_millimeters(0.4),
    )?;
    let p3 = solveur.ajouter_point(
        Length::from_millimeters(19.0),
        Length::from_millimeters(14.0),
    )?;
    let l1 = solveur.ajouter_ligne(p1, p2)?;
    let l2 = solveur.ajouter_ligne(p2, p3)?;
    let _l3 = solveur.ajouter_ligne(p3, p1)?;

    solveur.fixer_point(p1)?;
    solveur.ajouter_contrainte(Contrainte::Horizontale { ligne: l1 })?;
    solveur.ajouter_contrainte(Contrainte::Distance {
        p1,
        p2,
        valeur: Length::from_millimeters(20.0),
    })?;
    solveur.ajouter_contrainte(Contrainte::Perpendiculaires {
        ligne1: l1,
        ligne2: l2,
    })?;
    solveur.ajouter_contrainte(Contrainte::Distance {
        p1: p2,
        p2: p3,
        valeur: Length::from_millimeters(15.0),
    })?;
    Ok((p1, p2, p3))
}

#[test]
fn triangle_resout_positions_exactes() -> Result<(), GcsError> {
    let mut solveur = Systeme2D::new()?;
    let (_p1, p2, p3) = construire_triangle(&mut solveur)?;

    solveur.resoudre(Algorithme::DogLeg)?;

    let (x2, y2) = solveur.position_point(p2)?;
    let (x3, y3) = solveur.position_point(p3)?;
    assert!(
        (x2.millimeters() - 20.0).abs() <= TOLERANCE_MM,
        "p2.x attendu 20 mm, trouvé {} mm",
        x2.millimeters()
    );
    assert!(
        (y2.millimeters() - 0.0).abs() <= TOLERANCE_MM,
        "p2.y attendu 0 mm, trouvé {} mm",
        y2.millimeters()
    );
    assert!(
        (x3.millimeters() - 20.0).abs() <= TOLERANCE_MM,
        "p3.x attendu 20 mm, trouvé {} mm",
        x3.millimeters()
    );
    assert!(
        (y3.millimeters() - 15.0).abs() <= TOLERANCE_MM,
        "p3.y attendu 15 mm, trouvé {} mm",
        y3.millimeters()
    );

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

#[test]
fn deux_croquis_identiques_donnent_positions_identiques() -> Result<(), GcsError> {
    // Reproductibilité inter-exécutions (FreeCAD #33399) : deux
    // systèmes construits et résolus à l'identique donnent des
    // positions égales à la tolérance près.
    let mut a = Systeme2D::new()?;
    let mut b = Systeme2D::new()?;
    let (_pa1, pa2, pa3) = construire_triangle(&mut a)?;
    let (_pb1, pb2, pb3) = construire_triangle(&mut b)?;

    a.resoudre(Algorithme::DogLeg)?;
    b.resoudre(Algorithme::DogLeg)?;

    for (pa, pb) in [(pa2, pb2), (pa3, pb3)] {
        let (xa, ya) = a.position_point(pa)?;
        let (xb, yb) = b.position_point(pb)?;
        assert!(
            (xa.millimeters() - xb.millimeters()).abs() <= TOLERANCE_MM,
            "positions x divergentes : {} puis {}",
            xa.millimeters(),
            xb.millimeters()
        );
        assert!(
            (ya.millimeters() - yb.millimeters()).abs() <= TOLERANCE_MM,
            "positions y divergentes : {} puis {}",
            ya.millimeters(),
            yb.millimeters()
        );
    }

    // Deux résolutions successives du même système : idempotence.
    a.resoudre(Algorithme::DogLeg)?;
    let (x, y) = a.position_point(pa2)?;
    assert!(
        (x.millimeters() - 20.0).abs() <= TOLERANCE_MM,
        "re-résolution : p2.x attendu 20 mm, trouvé {} mm",
        x.millimeters()
    );
    assert!(
        (y.millimeters() - 0.0).abs() <= TOLERANCE_MM,
        "re-résolution : p2.y attendu 0 mm, trouvé {} mm",
        y.millimeters()
    );
    Ok(())
}

/// Garde anti-valeurs non finies : les newtypes de forge-units
/// refusent NaN/inf **avant** l'ABI C (convention « jamais de f64 nu ») ;
/// le shim ABI garde sa propre vérification en défense en profondeur
/// (OF-GCS-0004 si jamais un chemin futur la contourne).
#[test]
#[should_panic(expected = "fini")]
fn valeur_non_finie_est_refusee() {
    let mut solveur = Systeme2D::new().expect("systeme creable");
    let _ = solveur.ajouter_point(
        Length::from_millimeters(f64::NAN),
        Length::from_millimeters(0.0),
    );
}
