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
    /// Arc de balayage nul : l'arc effondré (fin = début) n'est
    /// plus l'encodage du cercle complet — le cercle complet est
    /// le variant explicite `Segment2::Cercle` (OF-GEO-0007,
    /// ADR-0037 D1, A5-02 + A4-18 — audit n°5).
    #[error("OF-GEO-0007 : arc de balayage nul — le cercle complet est `Cercle`")]
    BalayageNul,
    /// Embranchement de chaînage : un sommet réunit plus de deux
    /// extrémités de segments après l'élagage des queues
    /// (OF-GEO-0008, ADR-0037 D2) — jamais de choix arbitraire de
    /// boucle.
    #[error("OF-GEO-0008 : embranchement au sommet ({x}, {y}) — {degre} extrémités, deux au plus")]
    EmbranchementContour {
        /// Abscisse du sommet en cause.
        x: Length,
        /// Ordonnée du sommet en cause.
        y: Length,
        /// Nombre d'extrémités réunies au sommet.
        degre: usize,
    },
    /// Plafond d'entités de chaînage dépassé : plus de
    /// `MAX_ENTITES_CHAINAGE` segments soumis au chaînage
    /// (OF-GEO-0009, ADR-0037 D2).
    #[error("OF-GEO-0009 : {entites} entités à chaîner — plafond public de {plafond}")]
    PlafondChainageDepasse {
        /// Nombre d'entités soumises au chaînage.
        entites: usize,
        /// Plafond public dépassé.
        plafond: usize,
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
            Self::BalayageNul => "OF-GEO-0007",
            Self::EmbranchementContour { .. } => "OF-GEO-0008",
            Self::PlafondChainageDepasse { .. } => "OF-GEO-0009",
        }
    }
}
