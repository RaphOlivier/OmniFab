//! Import/export DXF (ADR-0009) : parseur crate `dxf`, budgets
//! intégrés.
//!
//! Convention d'unités : une unité = 1 mm ; le z est ignoré (tracés
//! 2D). Les entités géométriques du jalon 1 sont lues : `LINE`,
//! `LWPOLYLINE`, `POLYLINE`, `CIRCLE`, `ARC` (cercles et arcs
//! discrétisés) ; les entités non géométriques (textes, cotes,
//! blocs…) sont ignorées par conception (ADR-0009). À l'export,
//! chaque segment devient une entité `LINE` au z nul.
//!
//! Sécurité (ADR-0009) : tout dépassement de [`DxfBudgets`] est une
//! erreur typée, levée avant tout traitement ultérieur.

use crate::IoError;
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
/// [`SEGMENTS_ARC`] segments. Les budgets protègent contre les
/// documents démesurés (ADR-0009).
pub fn import(bytes: &[u8], budgets: &DxfBudgets) -> Result<Vec<Path2>, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    let mut curseur = std::io::Cursor::new(bytes);
    let dessin = dxf::Drawing::load(&mut curseur).map_err(|erreur| {
        IoError::InvalidDxf {
            raison: erreur.to_string(),
        }
    })?;
    let mut traces: Vec<Path2> = Vec::new();
    let mut total = 0usize;
    for entite in dessin.entities() {
        let mut points: Vec<Point2> = Vec::new();
        let mut ferme = false;
        let geometrique = match &entite.entity_type {
            dxf::EntityType::Line(ligne) => {
                points.push(sommet(ligne.p1.x, ligne.p1.y, &mut total, budgets)?);
                points.push(sommet(ligne.p2.x, ligne.p2.y, &mut total, budgets)?);
                true
            }
            dxf::EntityType::LwPolyline(polyligne) => {
                for v in &polyligne.vertices {
                    points.push(sommet(v.x, v.y, &mut total, budgets)?);
                }
                ferme = polyligne.is_closed();
                true
            }
            dxf::EntityType::Polyline(polyligne) => {
                for v in polyligne.vertices() {
                    points.push(sommet(v.location.x, v.location.y, &mut total, budgets)?);
                }
                true
            }
            dxf::EntityType::Circle(cercle) => {
                points = cercle_points(cercle, &mut total, budgets)?;
                ferme = true;
                true
            }
            dxf::EntityType::Arc(arc) => {
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

/// Discrétise un cercle en polygone fermé à [`POINTS_CERCLE`]
/// sommets.
fn cercle_points(
    cercle: &dxf::Circle,
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
fn arc_points(
    arc: &dxf::Arc,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<Vec<Point2>, IoError> {
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
    dessin.add_entity(dxf::Entity::new(dxf::EntityType::Line(dxf::Line::new(p1, p2))));
}
