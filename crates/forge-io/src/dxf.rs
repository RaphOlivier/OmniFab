//! Import/export DXF (ADR-0009) : parseur crate `dxf`, budgets
//! intégrés.
//!
//! Convention d'unités : une unité = 1 mm ; le z est ignoré (tracés
//! 2D). Les entités géométriques du jalon 1 sont lues : `LINE`,
//! `LWPOLYLINE`, `POLYLINE` (bulges discrétisés en arcs),
//! `CIRCLE`, `ARC` (cercles et arcs discrétisés) ; les entités
//! non géométriques (textes, cotes, blocs…) sont ignorées par
//! conception (ADR-0009). À l'export, chaque segment devient une
//! entité `LINE` au z nul.
//!
//! Sécurité (ADR-0009) : tout dépassement de [`DxfBudgets`] est une
//! erreur typée, levée avant tout traitement ultérieur.

use crate::IoError;
use dxf::entities::{Arc, Circle, Entity, EntityType, Line};
use forge_geom::{Path2, Point2};
use forge_units::Length;

/// Nombre de sommets de la discrétisation d'un cercle.
pub const POINTS_CERCLE: usize = 64;

/// Nombre de segments de la discrétisation d'un arc.
pub const SEGMENTS_ARC: usize = 32;

/// Budgets de ressources du parseur DXF (ADR-0009).
///
/// La sécurité ne doit pas brider les fonctions : chaque budget est
/// public et réglable par l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxfBudgets {
    /// Taille maximale acceptée en octets (défaut : 256 Mio).
    pub max_bytes: usize,
    /// Nombre maximal de tracés accepté (défaut : 100 000).
    pub max_paths: usize,
    /// Nombre maximal de sommets accepté (défaut : 100 000).
    pub max_vertices: usize,
}

impl Default for DxfBudgets {
    /// Défauts : 256 Mio, 100 000 tracés, 100 000 sommets.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_paths: 100_000,
            max_vertices: 100_000,
        }
    }
}

/// Importe un DXF en tracés 2D. Convention : une unité = 1 mm ; le
/// z est ignoré. Les cercles deviennent des polygones fermés à
/// [`POINTS_CERCLE`] sommets, les arcs des polylignes ouvertes à
/// [`SEGMENTS_ARC`] segments ; les bulges des polylignes (arcs
/// codés par sommet, code 42, générés par FreeCAD à l'export)
/// deviennent des arcs discrétisés le long de la corde. Les
/// budgets protègent contre les documents démesurés (ADR-0009).
pub fn import(bytes: &[u8], budgets: &DxfBudgets) -> Result<Vec<Path2>, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    let mut curseur = std::io::Cursor::new(bytes);
    let dessin = dxf::Drawing::load(&mut curseur).map_err(|erreur| IoError::InvalidDxf {
        raison: erreur.to_string(),
    })?;
    let mut traces: Vec<Path2> = Vec::new();
    let mut total = 0usize;
    for entite in dessin.entities() {
        let mut points: Vec<Point2> = Vec::new();
        let mut ferme = false;
        let geometrique = match &entite.specific {
            EntityType::Line(ligne) => {
                points.push(sommet(ligne.p1.x, ligne.p1.y, &mut total, budgets)?);
                points.push(sommet(ligne.p2.x, ligne.p2.y, &mut total, budgets)?);
                true
            }
            EntityType::LwPolyline(polyligne) => {
                let sommets: Vec<(f64, f64, f64)> = polyligne
                    .vertices
                    .iter()
                    .map(|v| (v.x, v.y, v.bulge))
                    .collect();
                ferme = polyligne.is_closed();
                pousse_polyligne(&sommets, ferme, &mut points, &mut total, budgets)?;
                true
            }
            EntityType::Polyline(polyligne) => {
                let sommets: Vec<(f64, f64, f64)> = polyligne
                    .vertices()
                    .map(|v| (v.location.x, v.location.y, v.bulge))
                    .collect();
                pousse_polyligne(&sommets, false, &mut points, &mut total, budgets)?;
                true
            }
            EntityType::Circle(cercle) => {
                points = cercle_points(cercle, &mut total, budgets)?;
                ferme = true;
                true
            }
            EntityType::Arc(arc) => {
                points = arc_points(arc, &mut total, budgets)?;
                true
            }
            _ => false,
        };
        if !geometrique || points.len() < 2 {
            continue;
        }
        if traces.len() >= budgets.max_paths {
            return Err(IoError::TooManyPaths {
                found: traces.len() as u64 + 1,
                max: budgets.max_paths as u64,
                unite: "tracés",
            });
        }
        traces.push(Path2::from_points(points, ferme));
    }
    Ok(traces)
}

/// Exporte les tracés en DXF : chaque segment devient une entité
/// `LINE` (z nul, tracés 2D) ; un tracé fermé ajoute la ligne de
/// fermeture dernier → premier sommet.
pub fn export(traces: &[Path2]) -> Result<Vec<u8>, IoError> {
    let mut dessin = dxf::Drawing::new();
    for trace in traces {
        let points = trace.points();
        if points.len() < 2 {
            continue;
        }
        for fenetre in points.windows(2) {
            ajoute_ligne(&mut dessin, &fenetre[0], &fenetre[1]);
        }
        if trace.is_closed() && points.len() >= 3 {
            let premier = points.first().expect("au moins deux sommets");
            let dernier = points.last().expect("au moins deux sommets");
            ajoute_ligne(&mut dessin, premier, dernier);
        }
    }
    let mut sortie = Vec::new();
    if let Err(erreur) = dessin.save(&mut sortie) {
        return Err(IoError::InvalidDxf {
            raison: format!("écriture impossible : {erreur}"),
        });
    }
    Ok(sortie)
}

/// Construit un sommet 2D (z ignoré), sous garde de budget.
fn sommet(x: f64, y: f64, total: &mut usize, budgets: &DxfBudgets) -> Result<Point2, IoError> {
    if *total >= budgets.max_vertices {
        return Err(IoError::TooManyPaths {
            found: *total as u64 + 1,
            max: budgets.max_vertices as u64,
            unite: "sommets",
        });
    }
    let x_mm = Length::from_millimeters(x);
    let y_mm = Length::from_millimeters(y);
    let point = Point2::new(x_mm, y_mm).map_err(IoError::InvalidGeometry)?;
    *total += 1;
    Ok(point)
}

/// Pousse les sommets d'une polyligne (x, y, bulge) dans `points`,
/// en discrétisant chaque segment porteur d'un bulge (arc) ; les
/// segments sans bulge restent des cordes droites. Sur une
/// polyligne fermée, le bulge du dernier sommet s'applique au
/// segment de fermeture vers le premier sommet.
fn pousse_polyligne(
    sommets: &[(f64, f64, f64)],
    ferme: bool,
    points: &mut Vec<Point2>,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<(), IoError> {
    for (i, (x, y, bulge)) in sommets.iter().enumerate() {
        points.push(sommet(*x, *y, total, budgets)?);
        if bulge.abs() <= 1e-12 {
            continue;
        }
        let prochain = if i + 1 < sommets.len() {
            Some(sommets[i + 1])
        } else if ferme && sommets.len() > 1 {
            Some(sommets[0])
        } else {
            None
        };
        if let Some((x2, y2, _)) = prochain {
            points.extend(bulge_intermediaires(
                (*x, *y),
                (x2, y2),
                *bulge,
                total,
                budgets,
            )?);
        }
    }
    Ok(())
}

/// Points intérieurs de la discrétisation d'un segment à bulge DXF
/// (les deux extrémités sont exclues : l'appelant pousse les
/// sommets). Mathématiques du bulge : angle inclus θ = 4·atan(b),
/// positif = sens trigonométrique ; rayon signé r = demi-corde /
/// sin(θ/2) ; centre = milieu de la corde décalé de r·cos(θ/2)
/// le long de la normale à gauche de la corde.
fn bulge_intermediaires(
    debut: (f64, f64),
    fin: (f64, f64),
    bulge: f64,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<Vec<Point2>, IoError> {
    let theta = 4.0 * bulge.atan();
    let dx = fin.0 - debut.0;
    let dy = fin.1 - debut.1;
    let corde = (dx * dx + dy * dy).sqrt();
    if corde < 1e-12 {
        return Ok(Vec::new());
    }
    let demi = corde / 2.0;
    let demi_theta = theta / 2.0;
    let r_signe = demi / demi_theta.sin();
    let h = r_signe * demi_theta.cos();
    let mx = (debut.0 + fin.0) / 2.0;
    let my = (debut.1 + fin.1) / 2.0;
    // Normale à gauche de la corde debut → fin.
    let nx = -dy / corde;
    let ny = dx / corde;
    let cx = mx + h * nx;
    let cy = my + h * ny;
    let rayon = r_signe.abs();
    let alpha = (debut.1 - cy).atan2(debut.0 - cx);
    let segments =
        ((theta.abs().to_degrees() * SEGMENTS_ARC as f64 / 360.0).ceil() as usize).clamp(2, 64);
    let mut points = Vec::with_capacity(segments - 1);
    for k in 1..segments {
        let phi = alpha + theta * k as f64 / segments as f64;
        let x = cx + rayon * phi.cos();
        let y = cy + rayon * phi.sin();
        points.push(sommet(x, y, total, budgets)?);
    }
    Ok(points)
}

/// Discrétise un cercle en polygone fermé à [`POINTS_CERCLE`]
/// sommets.
fn cercle_points(
    cercle: &Circle,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<Vec<Point2>, IoError> {
    let mut points = Vec::with_capacity(POINTS_CERCLE);
    for i in 0..POINTS_CERCLE {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / POINTS_CERCLE as f64;
        let x = cercle.center.x + cercle.radius * angle.cos();
        let y = cercle.center.y + cercle.radius * angle.sin();
        points.push(sommet(x, y, total, budgets)?);
    }
    Ok(points)
}

/// Discrétise un arc en polyligne ouverte à [`SEGMENTS_ARC`]
/// segments, de l'angle de départ à l'angle de fin (degrés, sens
/// trigonométrique).
fn arc_points(arc: &Arc, total: &mut usize, budgets: &DxfBudgets) -> Result<Vec<Point2>, IoError> {
    let mut span = arc.end_angle - arc.start_angle;
    if span <= 0.0 {
        span += 360.0;
    }
    let mut points = Vec::with_capacity(SEGMENTS_ARC + 1);
    for i in 0..=SEGMENTS_ARC {
        let degres = arc.start_angle + span * i as f64 / SEGMENTS_ARC as f64;
        let angle = degres.to_radians();
        let x = arc.center.x + arc.radius * angle.cos();
        let y = arc.center.y + arc.radius * angle.sin();
        points.push(sommet(x, y, total, budgets)?);
    }
    Ok(points)
}

/// Ajoute une entité `LINE` au dessin, z nul (tracés 2D).
fn ajoute_ligne(dessin: &mut dxf::Drawing, debut: &Point2, fin: &Point2) {
    let p1 = dxf::Point {
        x: debut.x().millimeters(),
        y: debut.y().millimeters(),
        z: 0.0,
    };
    let p2 = dxf::Point {
        x: fin.x().millimeters(),
        y: fin.y().millimeters(),
        z: 0.0,
    };
    dessin.add_entity(Entity::new(EntityType::Line(Line::new(p1, p2))));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DXF ASCII minimal : une LWPOLYLINE ouverte à deux sommets
    /// (0,0) et (10,0), le premier portant le bulge donné (code 42).
    fn dxf_bulge(bulge: &str) -> Vec<u8> {
        let texte = format!(
            "0
SECTION
2
ENTITIES
0
LWPOLYLINE
100
AcDbEntity
8
0
             100
AcDbPolyline
90
2
70
0
10
0.0
20
0.0
42
{bulge}
             10
10.0
20
0.0
0
ENDSEC
0
EOF
"
        );
        texte.into_bytes()
    }

    #[test]
    fn lwpolyline_sans_bulge_reste_un_segment() {
        let traces = import(&dxf_bulge("0.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        let points = traces[0].points();
        assert_eq!(points.len(), 2);
        assert!(points[0].x().millimeters().abs() < 1e-9);
        assert!(points[0].y().millimeters().abs() < 1e-9);
        assert!((points[1].x().millimeters() - 10.0).abs() < 1e-9);
        assert!(points[1].y().millimeters().abs() < 1e-9);
    }

    #[test]
    fn lwpolyline_bulge_positif_passe_sous_la_corde() {
        // bulge 1 = demi-cercle trigonométrique de (0,0) vers
        // (10,0) : centre (5,0), rayon 5, l'arc passe par (5, -5).
        let traces = import(&dxf_bulge("1.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        let points = traces[0].points();
        // 2 sommets + 15 points intérieurs (16 segments).
        assert_eq!(points.len(), 17);
        let passage = points.iter().any(|p| {
            (p.x().millimeters() - 5.0).abs() < 1e-6 && (p.y().millimeters() + 5.0).abs() < 1e-6
        });
        assert!(passage, "l'arc doit passer par (5, -5)");
    }

    #[test]
    fn lwpolyline_bulge_negatif_passe_au_dessus() {
        // bulge -1 = demi-cercle horaire de (0,0) vers (10,0) :
        // l'arc passe par (5, 5).
        let traces = import(&dxf_bulge("-1.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        let points = traces[0].points();
        assert_eq!(points.len(), 17);
        let passage = points.iter().any(|p| {
            (p.x().millimeters() - 5.0).abs() < 1e-6 && (p.y().millimeters() - 5.0).abs() < 1e-6
        });
        assert!(passage, "l'arc doit passer par (5, 5)");
    }
}
