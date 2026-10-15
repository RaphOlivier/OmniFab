//! Erreurs typées de forge-io (ADR-0003 : codes stables `OF-IO-NNNN`,
//! registre central `docs/error-codes.md`).

use forge_geom::GeomError;
use thiserror::Error;

/// Erreur d'import/export. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum IoError {
    /// Entrée plus volumineuse que le budget d'octets du parseur
    /// (OF-IO-0001).
    #[error("OF-IO-0001 : entrée trop volumineuse : {found} octets, budget {max}")]
    TooLarge {
        /// Taille constatée, en octets.
        found: usize,
        /// Budget d'octets du parseur.
        max: usize,
    },
    /// STL binaire structurellement invalide (OF-IO-0002).
    #[error("OF-IO-0002 : STL binaire invalide : {reason}")]
    InvalidBinary {
        /// Explication.
        reason: String,
    },
    /// Nombre de triangles au-delà du budget du parseur (OF-IO-0003).
    #[error("OF-IO-0003 : budget de triangles dépassé : {found}, budget {max}")]
    TooManyTriangles {
        /// Nombre de triangles constaté ou annoncé.
        found: u64,
        /// Budget de triangles.
        max: u64,
    },
    /// STL ASCII hors du sous-ensemble accepté (OF-IO-0004).
    #[error("OF-IO-0004 : STL ASCII invalide (ligne {line}) : {reason}")]
    InvalidAscii {
        /// Numéro de la ligne fautive, en partant de 1 (0 : fin de
        /// fichier).
        line: usize,
        /// Explication.
        reason: String,
    },
    /// Format non reconnu (OF-IO-0005).
    #[error("OF-IO-0005 : format non reconnu (ni STL binaire ni STL ASCII)")]
    UnknownFormat,
    /// Géométrie invalide à l'import, ex. coordonnée non finie
    /// (OF-IO-0006).
    #[error("OF-IO-0006 : géométrie invalide à l'import : {0}")]
    InvalidGeometry(#[from] GeomError),
    /// SVG invalide : document refusé par le parseur usvg (OF-IO-0007).
    #[error("OF-IO-0007 : SVG invalide : {raison}")]
    InvalidSvg {
        /// Explication du refus.
        raison: String,
    },
    /// Budget vectoriel dépassé : tracés ou sommets au-delà des
    /// budgets du parseur SVG/DXF (OF-IO-0008).
    #[error("OF-IO-0008 : budget dépassé : {found} {unite}, budget {max}")]
    TooManyPaths {
        /// Nombre constaté.
        found: u64,
        /// Budget correspondant.
        max: u64,
        /// Unité comptée (« tracés » ou « sommets »).
        unite: &'static str,
    },
    /// DXF invalide : document refusé par le parseur `dxf` (OF-IO-0009).
    #[error("OF-IO-0009 : DXF invalide : {raison}")]
    InvalidDxf {
        /// Explication du refus.
        raison: String,
    },
}

impl IoError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "OF-IO-0001",
            Self::InvalidBinary { .. } => "OF-IO-0002",
            Self::TooManyTriangles { .. } => "OF-IO-0003",
            Self::InvalidAscii { .. } => "OF-IO-0004",
            Self::UnknownFormat => "OF-IO-0005",
            Self::InvalidGeometry(_) => "OF-IO-0006",
            Self::InvalidSvg { .. } => "OF-IO-0007",
            Self::TooManyPaths { .. } => "OF-IO-0008",
            Self::InvalidDxf { .. } => "OF-IO-0009",
        }
    }
}
