//! Import/export SVG (ADR-0009) : parseur `usvg`, budgets intégrés.
//!
//! Convention d'unités : une unité SVG = 1 mm. Le parseur `usvg`
//! n'exécute aucun script, ne charge aucune ressource externe et
//! n'accepte ni DTD ni XXE ; les formes de base (`rect`,
//! `circle`, …) sont normalisées en tracés par `usvg`. Les courbes
//! de Bézier (quadratiques et cubiques) sont aplaties en polylignes
//! à [`SEGMENTS_COURBE`] segments par courbe.
//!
//! Sécurité (ADR-0009) : tout dépassement de [`SvgBudgets`] est une
//! erreur typée, levée avant tout traitement ultérieur.

use crate::IoError;
use forge_geom::{Path2, Point2};
use forge_units::Length;
use usvg::tiny_skia_path::PathSegment;

/// Segments par courbe de Bézier à l'aplatissement.
pub const SEGMENTS_COURBE: usize = 16;

/// Budgets de ressources du parseur SVG (ADR-0009).
///
/// La sécurité ne doit pas brider les fonctions : chaque budget est
/// public et réglable par l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgBudgets {
    /// Taille maximale acceptée en octets (défaut : 256 Mio).
    pub max_bytes: usize,
    /// Nombre maximal de tracés accepté (défaut : 100 000).
    pub max_paths: usize,
    /// Nombre maximal de sommets accepté (défaut : 100 000).
    pub max_vertices: usize,
}

impl Default for SvgBudgets {
    /// Défauts : 256 Mio, 100 000 tracés, 100 000 sommets.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_paths: 100_000,
            max_vertices: 100_000,
        }
    }
}

/// Transformation identité, point de départ de la composition des
/// transformations de groupes.
const IDENTITE: usvg::Transform = usvg::Transform {
    sx: 1.0,
    kx: 0.0,
    ky: 0.0,
    sy: 1.0,
    tx: 0.0,
    ty: 0.0,
};

/// Importe un SVG en tracés 2D. Convention : une unité SVG = 1 mm ;
/// les transformations de groupes et de tracés sont composées, les
/// courbes sont aplaties en polylignes, les nœuds non géométriques
/// (images, textes) sont ignorés. Les budgets protègent contre les
/// documents démesurés (ADR-0009).
pub fn import(bytes: &[u8], budgets: &SvgBudgets) -> Result<Vec<Path2>, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    let options = usvg::Options::default();
    let arbre = usvg::Tree::from_data(bytes, &options).map_err(|erreur| {
        IoError::InvalidSvg {
            raison: erreur.to_string(),
        }
    })?;
    let mut traces: Vec<Path2> = Vec::new();
    let mut etat = Etat {
        chemins: 0,
        sommets: 0,
        budgets,
    };
    parcourir(arbre.root(), &IDENTITE, &mut traces, &mut etat)?;
    Ok(traces)
}

/// Exporte les tracés en SVG : chaque tracé devient un élément
/// `path` (`M`/`L` par sommet, `Z` si fermé), coordonnées en
/// millimètres. Le document de sortie est du SVG minimal, sans
/// métadonnées.
pub fn export(traces: &[Path2]) -> Result<Vec<u8>, IoError> {
    use std::fmt::Write as _;
    let mut sortie = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\">\n",
    );
    for trace in traces {
        let points = trace.points();
        if points.is_empty() {
            continue;
        }
        sortie.push_str("<path d=\"");
        for (index, point) in points.iter().enumerate() {
            if index > 0 {
                sortie.push(' ');
            }
            let marque = if index == 0 { 'M' } else { 'L' };
            write!(
                sortie,
                "{marque} {} {}",
                point.x().millimeters(),
                point.y().millimeters()
            )
            .expect("écriture en mémoire infaillible");
        }
        if trace.is_closed() {
            sortie.push_str(" Z");
        }
        sortie.push_str("\"/>\n");
    }
    sortie.push_str("</svg>\n");
    Ok(sortie.into_bytes())
}

/// Compteurs et budgets de l'import en cours.
struct Etat<'a> {
    /// Tracés déjà collectés.
    chemins: usize,
    /// Sommets déjà collectés.
    sommets: usize,
    /// Budgets de l'import.
    budgets: &'a SvgBudgets,
}

/// Parcourt un groupe : compose la transformation parent avec
/// celle du groupe, puis descend dans les sous-groupes et collecte
/// les tracés. Les nœuds non géométriques sont ignorés.
fn parcourir(
    groupe: &usvg::Group,
    parent: &usvg::Transform,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    let composee = compose(parent, &groupe.transform);
    for noeud in groupe.children() {
        match noeud {
            usvg::Node::Group(sous_groupe) => parcourir(sous_groupe, &composee, traces, etat)?,
            usvg::Node::Path(chemin) => {
                let globale = compose(&composee, &chemin.transform);
                collecte(chemin.data(), &globale, traces, etat)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Collecte les segments d'un tracé : un sous-tracé par `MoveTo`,
/// fermé par `Close`, courbes aplaties, tout sous la transformation
/// donnée.
fn collecte(
    donnees: &usvg::tiny_skia_path::Path,
    transformation: &usvg::Transform,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    let mut points: Vec<Point2> = Vec::new();
    let mut courant_x = 0.0;
    let mut courant_y = 0.0;
    for segment in donnees.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                termine(&mut points, false, traces, etat)?;
                courant_x = point.x;
                courant_y = point.y;
                ajoute(point.x, point.y, transformation, &mut points, etat)?;
            }
            PathSegment::LineTo(point) => {
                courant_x = point.x;
                courant_y = point.y;
                ajoute(point.x, point.y, transformation, &mut points, etat)?;
            }
            PathSegment::QuadTo(controle, fin) => {
                aplatis_quadratique(
                    (courant_x, courant_y),
                    (controle.x, controle.y),
                    (fin.x, fin.y),
                    transformation,
                    &mut points,
                    etat,
                )?;
                courant_x = fin.x;
                courant_y = fin.y;
            }
            PathSegment::CubicTo(c1, c2, fin) => {
                aplatis_cubique(
                    (courant_x, courant_y),
                    (c1.x, c1.y),
                    (c2.x, c2.y),
                    (fin.x, fin.y),
                    transformation,
                    &mut points,
                    etat,
                )?;
                courant_x = fin.x;
                courant_y = fin.y;
            }
            PathSegment::Close => {
                termine(&mut points, true, traces, etat)?;
            }
        }
    }
    termine(&mut points, false, traces, etat)
}

/// Ajoute un sommet (espace transformé), sous garde de budget.
fn ajoute(
    x: f32,
    y: f32,
    transformation: &usvg::Transform,
    points: &mut Vec<Point2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    if etat.sommets >= etat.budgets.max_vertices {
        return Err(IoError::TooManyPaths {
            found: etat.sommets as u64 + 1,
            max: etat.budgets.max_vertices as u64,
            unite: "sommets",
        });
    }
    let (gx, gy) = applique(transformation, x, y);
    let gx_mm = Length::from_millimeters(gx);
    let gy_mm = Length::from_millimeters(gy);
    let point = Point2::new(gx_mm, gy_mm).map_err(IoError::InvalidGeometry)?;
    points.push(point);
    etat.sommets += 1;
    Ok(())
}

/// Termine le sous-tracé courant : un tracé d'au moins deux sommets
/// est conservé (sous garde de budget), sinon les points sont
/// abandonnés.
fn termine(
    points: &mut Vec<Point2>,
    ferme: bool,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    if points.len() >= 2 {
        if etat.chemins >= etat.budgets.max_paths {
            return Err(IoError::TooManyPaths {
                found: etat.chemins as u64 + 1,
                max: etat.budgets.max_paths as u64,
                unite: "tracés",
            });
        }
        traces.push(Path2::from_points(std::mem::take(points), ferme));
        etat.chemins += 1;
    } else {
        points.clear();
    }
    Ok(())
}

/// Aplatit une Bézier quadratique en [`SEGMENTS_COURBE`] sommets
/// (le point de départ est déjà collecté).
fn aplatis_quadratique(
    debut: (f32, f32),
    controle: (f32, f32),
    fin: (f32, f32),
    transformation: &usvg::Transform,
    points: &mut Vec<Point2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    for pas in 1..=SEGMENTS_COURBE {
        let t = pas as f32 / SEGMENTS_COURBE as f32;
        let u = 1.0 - t;
        let x = u * u * debut.0 + 2.0 * u * t * controle.0 + t * t * fin.0;
        let y = u * u * debut.1 + 2.0 * u * t * controle.1 + t * t * fin.1;
        ajoute(x, y, transformation, points, etat)?;
    }
    Ok(())
}

/// Aplatit une Bézier cubique en [`SEGMENTS_COURBE`] sommets (le
/// point de départ est déjà collecté).
fn aplatis_cubique(
    debut: (f32, f32),
    c1: (f32, f32),
    c2: (f32, f32),
    fin: (f32, f32),
    transformation: &usvg::Transform,
    points: &mut Vec<Point2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    for pas in 1..=SEGMENTS_COURBE {
        let t = pas as f32 / SEGMENTS_COURBE as f32;
        let u = 1.0 - t;
        let a = u * u * u;
        let b = 3.0 * u * u * t;
        let c = 3.0 * u * t * t;
        let d = t * t * t;
        let x = a * debut.0 + b * c1.0 + c * c2.0 + d * fin.0;
        let y = a * debut.1 + b * c1.1 + c * c2.1 + d * fin.1;
        ajoute(x, y, transformation, points, etat)?;
    }
    Ok(())
}

/// Compose deux transformations (parent appliqué après enfant).
fn compose(parent: &usvg::Transform, enfant: &usvg::Transform) -> usvg::Transform {
    usvg::Transform {
        sx: parent.sx * enfant.sx + parent.kx * enfant.ky,
        kx: parent.sx * enfant.kx + parent.kx * enfant.sy,
        ky: parent.ky * enfant.sx + parent.sy * enfant.ky,
        sy: parent.ky * enfant.kx + parent.sy * enfant.sy,
        tx: parent.sx * enfant.tx + parent.kx * enfant.ty + parent.tx,
        ty: parent.ky * enfant.tx + parent.sy * enfant.ty + parent.ty,
    }
}

/// Applique une transformation à un point (espace SVG vers espace
/// global, en `f64`).
fn applique(t: &usvg::Transform, x: f32, y: f32) -> (f64, f64) {
    ((t.sx * x + t.kx * y + t.tx) as f64, (t.ky * x + t.sy * y + t.ty) as f64)
}
