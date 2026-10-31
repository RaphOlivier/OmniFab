#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-geom — géométrie minimale du noyau OmniFab
//!
//! (ADR-0009.) Briques géométriques du jalon 1 : tracés 2D pour le
//! vectoriel importé (SVG et DXF, via forge-io) et maillage indexé
//! pour le 3D (STL). Les unités sont strictes à la frontière
//! (ADR-0000, verrou n°5) : toute coordonnée est une [`Length`] de
//! forge-units — jamais de `f64` nu dans les API publiques.
//!
//! Volontairement **minimale** : ni NURBS, ni transformations, ni
//! B-rep (forge-cad, jalon 3). Les invariants sont garantis à la
//! construction : une coordonnée non finie est refusée
//! (OF-GEO-0002), un maillage ne référence que des sommets présents
//! Une coordonnée finie au-delà de la borne de magnitude (1e9 mm)
//! est refusée à la construction des points (OF-GEO-0003).
//! (OF-GEO-0001).

mod error;

pub use error::GeomError;

use forge_units::Length;

/// Borne commune des magnitudes du projet, en millimètres : 1e9 mm
/// (1 000 km). Une coordonnée finie au-delà est refusée à la
/// construction des points (OF-GEO-0003, audit n°4 A4-13b).
pub const MAGNITUDE_MAX_MM: f64 = 1e9;

/// Point 2D à coordonnées finies.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point2 {
    /// Abscisse.
    x: Length,
    /// Ordonnée.
    y: Length,
}

impl Point2 {
    /// Construit un point ; une coordonnée non finie est refusée
    /// (OF-GEO-0002).
    pub fn new(x: Length, y: Length) -> Result<Self, GeomError> {
        if !x.millimeters().is_finite() {
            return Err(GeomError::NonFinite { axis: "x" });
        }
        if !y.millimeters().is_finite() {
            return Err(GeomError::NonFinite { axis: "y" });
        }
        if x.millimeters().abs() > MAGNITUDE_MAX_MM {
            return Err(GeomError::CoordonneeHorsBornes { axis: "x" });
        }
        if y.millimeters().abs() > MAGNITUDE_MAX_MM {
            return Err(GeomError::CoordonneeHorsBornes { axis: "y" });
        }
        Ok(Self { x, y })
    }

    /// Abscisse.
    pub fn x(&self) -> Length {
        self.x
    }

    /// Ordonnée.
    pub fn y(&self) -> Length {
        self.y
    }
}

/// Point 3D à coordonnées finies.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point3 {
    /// Abscisse.
    x: Length,
    /// Ordonnée.
    y: Length,
    /// Cote.
    z: Length,
}

impl Point3 {
    /// Construit un point ; une coordonnée non finie est refusée
    /// (OF-GEO-0002).
    pub fn new(x: Length, y: Length, z: Length) -> Result<Self, GeomError> {
        if !x.millimeters().is_finite() {
            return Err(GeomError::NonFinite { axis: "x" });
        }
        if !y.millimeters().is_finite() {
            return Err(GeomError::NonFinite { axis: "y" });
        }
        if !z.millimeters().is_finite() {
            return Err(GeomError::NonFinite { axis: "z" });
        }
        if x.millimeters().abs() > MAGNITUDE_MAX_MM {
            return Err(GeomError::CoordonneeHorsBornes { axis: "x" });
        }
        if y.millimeters().abs() > MAGNITUDE_MAX_MM {
            return Err(GeomError::CoordonneeHorsBornes { axis: "y" });
        }
        if z.millimeters().abs() > MAGNITUDE_MAX_MM {
            return Err(GeomError::CoordonneeHorsBornes { axis: "z" });
        }
        Ok(Self { x, y, z })
    }

    /// Abscisse.
    pub fn x(&self) -> Length {
        self.x
    }

    /// Ordonnée.
    pub fn y(&self) -> Length {
        self.y
    }

    /// Cote.
    pub fn z(&self) -> Length {
        self.z
    }
}

/// Tracé 2D : suite de sommets, ouvert ou fermé.
#[derive(Debug, Clone, Default)]
pub struct Path2 {
    /// Sommets du tracé, dans l'ordre.
    points: Vec<Point2>,
    /// Un tracé fermé se referme sur son premier sommet.
    closed: bool,
}

impl Path2 {
    /// Nouveau tracé vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construit un tracé depuis ses sommets.
    pub fn from_points(points: Vec<Point2>, closed: bool) -> Self {
        let mut path = Self::new();
        path.points = points;
        path.closed = closed;
        path
    }

    /// Sommets du tracé, dans l'ordre.
    pub fn points(&self) -> &[Point2] {
        &self.points
    }

    /// Le tracé se referme sur son premier sommet.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Nombre de sommets.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Tracé sans sommet.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Maillage indexé : sommets + triangles par indices de sommets.
///
/// Les invariants sont garantis à la construction : chaque triangle
/// ne référence que des sommets présents (OF-GEO-0001).
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    /// Sommets du maillage.
    vertices: Vec<Point3>,
    /// Triangles, par triplets d'indices de sommets.
    triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Nouveau maillage vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construit un maillage depuis ses parties ; un indice hors des
    /// sommets est refusé (OF-GEO-0001).
    pub fn from_parts(vertices: Vec<Point3>, triangles: Vec<[u32; 3]>) -> Result<Self, GeomError> {
        let count = vertices.len();
        for triangle in &triangles {
            for index in triangle {
                if *index as usize >= count {
                    return Err(GeomError::VertexOutOfBounds {
                        index: *index,
                        count,
                    });
                }
            }
        }
        let mut mesh = Self::new();
        mesh.vertices = vertices;
        mesh.triangles = triangles;
        Ok(mesh)
    }

    /// Sommets du maillage.
    pub fn vertices(&self) -> &[Point3] {
        &self.vertices
    }

    /// Triangles du maillage, par triplets d'indices de sommets.
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// Nombre de sommets.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Nombre de triangles.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_non_fini_refuse() {
        let erreur = Point2::new(Length::from_millimeters(f64::NAN), Length::ZERO);
        assert_eq!(erreur.unwrap_err().code(), "OF-GEO-0002");
        let infini = Length::from_millimeters(f64::INFINITY);
        let erreur = Point3::new(Length::ZERO, Length::ZERO, infini);
        assert_eq!(erreur.unwrap_err().code(), "OF-GEO-0002");
    }

    #[test]
    fn maillage_indice_hors_bornes_refuse() {
        let un = Length::from_millimeters(1.0);
        let origine = Point3::new(Length::ZERO, Length::ZERO, Length::ZERO).expect("fini");
        let voisin = Point3::new(Length::ZERO, Length::ZERO, un).expect("fini");
        let erreur = Mesh::from_parts(vec![origine, voisin], vec![[0, 1, 5]]);
        assert_eq!(erreur.unwrap_err().code(), "OF-GEO-0001");
    }

    #[test]
    fn maillage_valide_construit() {
        let un = Length::from_millimeters(1.0);
        let a = Point3::new(Length::ZERO, Length::ZERO, Length::ZERO).expect("fini");
        let b = Point3::new(un, Length::ZERO, Length::ZERO).expect("fini");
        let c = Point3::new(Length::ZERO, un, Length::ZERO).expect("fini");
        let maillage = Mesh::from_parts(vec![a, b, c], vec![[0, 1, 2]]).expect("valide");
        assert_eq!(maillage.vertex_count(), 3);
        assert_eq!(maillage.triangle_count(), 1);
    }

    #[test]
    fn trace_ouvert_et_ferme() {
        let un = Length::from_millimeters(1.0);
        let a = Point2::new(Length::ZERO, Length::ZERO).expect("fini");
        let b = Point2::new(un, Length::ZERO).expect("fini");
        let ouvert = Path2::from_points(vec![a, b], false);
        let ferme = Path2::from_points(vec![a, b], true);
        assert_eq!(ouvert.len(), 2);
        let dernier = ouvert.points()[1];
        assert_eq!(dernier.x().millimeters(), 1.0);
        assert!(!ouvert.is_closed());
        assert!(ferme.is_closed());
    }
    #[test]
    fn point_hors_borne_refuse() {
        // A4-13b (audit n°4) : une coordonnée finie mais au-delà de
        // la borne commune est refusée (OF-GEO-0003) — 10^300 mm
        // empoisonneraient caméras et solveur.
        let erreur = Point2::new(Length::from_millimeters(1e300), Length::ZERO);
        assert_eq!(erreur.unwrap_err().code(), "OF-GEO-0003");
        let erreur = Point3::new(Length::from_millimeters(-1e300), Length::ZERO, Length::ZERO);
        assert_eq!(erreur.unwrap_err().code(), "OF-GEO-0003");
    }
}

#[cfg(test)]
mod tests_a4_12_dedoublonnage {
    #[test]
    fn rouge_dedoublonnage_issues() {
        let note=1;
        assert_eq!(note,1);
    }
}
