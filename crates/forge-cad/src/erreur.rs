//! Erreurs typées du domaine CAO (ADR-0003 : codes stables OF-CAD-NNNN,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreurs de la crate forge-cad (codes OF-CAD-0001..0004).
#[derive(Debug, Error)]
pub enum CadError {
    /// Échec générique d'une opération OCCT (OF-CAD-0001).
    #[error("échec OCCT pendant : {operation}")]
    Occt {
        /// L'opération CAO en cours quand OCCT a échoué.
        operation: String,
    },
    /// Forme nulle ou invalide (OF-CAD-0002).
    #[error("forme nulle ou invalide après : {operation}")]
    FormeInvalide {
        /// L'opération CAO qui a produit une forme vide.
        operation: String,
    },
    /// Fichier CAO illisible : STEP ou BREP (OF-CAD-0003).
    #[error("fichier {formate} illisible : {raison}")]
    Illisible {
        /// Le format du fichier refusé (`STEP` ou `BREP`).
        formate: String,
        /// La cause du refus.
        raison: String,
    },
    /// Budget de ressources dépassé (OF-CAD-0004).
    #[error("budget de ressources dépassé : {limite}")]
    Budget {
        /// La limite franchie, en clair.
        limite: String,
    },
}

impl CadError {
    /// Code stable de l'erreur, présent au registre `docs/error-codes.md`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Occt { .. } => "OF-CAD-0001",
            Self::FormeInvalide { .. } => "OF-CAD-0002",
            Self::Illisible { .. } => "OF-CAD-0003",
            Self::Budget { .. } => "OF-CAD-0004",
        }
    }
}
