//! Erreurs typées de forge-geom (ADR-0003 : codes stables `OF-GEO-NNNN`,
//! registre central `docs/error-codes.md`).

use forge_units::Length;
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
    /// Discrétisation impossible : tolérance nulle, négative ou non
    /// finie, arc à rayon nul, négatif ou non fini, angle non fini
    /// (OF-GEO-0004).
    #[error("OF-GEO-0004 : discrétisation impossible — {raison}")]
    DiscretisationImpossible {
        /// Détail du refus.
        raison: &'static str,
    },
    /// Budget de discrétisation dépassé : profondeur de subdivision
    /// ou nombre de points au-delà des bornes publiques (OF-GEO-0005).
    #[error("OF-GEO-0005 : budget de discrétisation dépassé — {raison}")]
    BudgetDiscretisationDepasse {
        /// Détail du dépassement.
        raison: &'static str,
    },
    /// Chaînage de contour rompu : l'arrivée d'un segment et le
    /// départ du suivant sont écartés de plus de la tolérance
    /// géométrique (OF-GEO-0006).
    #[error("OF-GEO-0006 : chaînage rompu au segment {index} — écart {ecart}")]
    ChainageRompu {
        /// Indice du segment qui ne s'enchaîne pas.
        index: usize,
        /// Écart mesuré entre l'arrivée et le départ.
        ecart: Length,
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
            Self::DiscretisationImpossible { .. } => "OF-GEO-0004",
            Self::BudgetDiscretisationDepasse { .. } => "OF-GEO-0005",
            Self::ChainageRompu { .. } => "OF-GEO-0006",
        }
    }
}
