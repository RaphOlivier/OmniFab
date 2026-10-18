//! Croquis doré de non-régression « cercle contraint » (ADR-0018,
//! lot 2 : critère de sortie n°2 de la phase D).
//!
//! Cercle ancré par son centre fixé, porté par une cote de rayon,
//! et un point posé sur la circonférence. Les graines sont
//! volontairement distinctes de la solution (rayon initial 7,5 mm
//! pour une cote de 12,5 mm) : les contraintes doivent piloter la
//! géométrie. Un point sur un cercle garde exactement un degré de
//! liberté (son angle) : le diagnostic doit le rapporter, preuve que
//! les deux contraintes comptent sans conflit ni redondance. Aucun
//! changement de frontière : ce croquis ne consomme que l'API
//! publique de la crate (trait Solveur2D).

use forge_gcs::{Algorithme, Contrainte, GcsError, IdPoint, Solveur2D, Systeme2D};
use forge_units::{Length, TOLERANCE_MM};

/// Construit le cercle doré ; renvoie (centre, point).
///
/// Inconnues : le rayon du cercle et les deux coordonnées du point
/// (le centre est fixé). Contraintes : rayon = 12,5 mm, point sur le
/// cercle — soit 2 contraintes pour 3 inconnues : degrés de liberté
/// attendus, 1 (l'angle du point sur la circonférence).
fn construire_cercle(solveur: &mut impl Solveur2D) -> Result<(IdPoint, IdPoint), GcsError> {
    let centre = solveur.ajouter_point(
        Length::from_millimeters(10.0),
        Length::from_millimeters(12.0),
    )?;
    // Rayon initial distinct de la cote : la contrainte de rayon doit
    // rapprocher le cercle de la valeur dorée.
    let cercle = solveur.ajouter_cercle(centre, Length::from_millimeters(7.5))?;
    let point = solveur.ajouter_point(
        Length::from_millimeters(20.0),
        Length::from_millimeters(21.0),
    )?;

    solveur.fixer_point(centre)?;
    solveur.ajouter_contrainte(Contrainte::RayonCercle {
        cercle,
        valeur: Length::from_millimeters(12.5),
    })?;
    solveur.ajouter_contrainte(Contrainte::PointSurCercle { point, cercle })?;
    Ok((centre, point))
}

/// Le cercle doré est résolu : le point posé sur la circonférence
/// respecte la cote de rayon à la tolérance géométrique du projet
/// (1e-6 mm), et le croquis garde exactement un degré de liberté
/// (l'angle du point), sans conflit ni redondance, reproductibilité
/// garantie (critère de sortie n°2 de l'ADR-0018).
#[test]
fn cercle_resout_rayon_et_point_sur_circonference() -> Result<(), GcsError> {
    let mut solveur = Systeme2D::new()?;
    let (centre, point) = construire_cercle(&mut solveur)?;

    solveur.resoudre(Algorithme::DogLeg)?;

    // La cote de rayon pilote la géométrie : le point, contraint sur
    // le cercle, se retrouve exactement à 12,5 mm du centre ancré.
    let (xc, yc) = solveur.position_point(centre)?;
    let (xp, yp) = solveur.position_point(point)?;
    let dx = xp.millimeters() - xc.millimeters();
    let dy = yp.millimeters() - yc.millimeters();
    let distance = (dx * dx + dy * dy).sqrt();
    assert!(
        (distance - 12.5).abs() <= TOLERANCE_MM,
        "distance centre-point attendue 12.5 mm, trouvée {} mm",
        distance
    );

    // Un point sur un cercle garde exactement un degré de liberté :
    // son angle sur la circonférence.
    let diagnostic = solveur.diagnostic(Algorithme::DogLeg)?;
    assert_eq!(diagnostic.dof(), 1, "degrés de liberté attendus : 1");
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
