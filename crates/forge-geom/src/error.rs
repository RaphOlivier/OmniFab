//! Erreurs typées de forge-geom (ADR-0003 : codes stables `OF-GEO-NNNN`,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreur de géométrie. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum GeomError {
    /// Triangle référençant un sommet absent du maillage (OF-GEO-0001).
    #[error("OF-GEO-0001 : indice de sommet {index} hors du maillage ({count} sommets)")]
    VertexOutOfBounds {
        /// Indice fautif.
        index: u32,
        /// Nombre de sommets du maillage.
        count: usize,
    },
    /// Coordonnée non finie, refusée à la construction (OF-GEO-0002).
    #[error("OF-GEO-0002 : coordonnée {axis} non finie")]
    NonFinite {
        /// Axe fautif (« x », « y » ou « z »).
        axis: &'static str,
    },
    /// Coordonnée finie au-delà de la borne de magnitude, refusée à
    /// la construction (OF-GEO-0003, audit n°4 A4-13b).
    #[error("OF-GEO-0003 : coordonnée {axis} au-delà de la borne de magnitude")]
    CoordonneeHorsBornes {
        /// Axe fautif (« x », « y » ou « z »).
        axis: &'static str,
    },
}

impl GeomError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::VertexOutOfBounds { .. } => "OF-GEO-0001",
            Self::NonFinite { .. } => "OF-GEO-0002",
            Self::CoordonneeHorsBornes { .. } => "OF-GEO-0003",
        }
    }
}
