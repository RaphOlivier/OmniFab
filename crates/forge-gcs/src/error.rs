//! Erreurs typées de forge-gcs (ADR-0003 : codes stables `OF-GCS-NNNN`,
//! registre central `docs/error-codes.md`).

use crate::Algorithme;
use thiserror::Error;

/// Erreur du solveur de contraintes 2D. Chaque variante porte un code
/// stable documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum GcsError {
    /// Identifiant hors des identifiants émis par le solveur
    /// (OF-GCS-0001).
    #[error("OF-GCS-0001 : identifiant inconnu : {contexte}")]
    IdInconnu {
        /// Contexte de l'appel fautif.
        contexte: String,
    },
    /// Résolution échouée : croquis conflictuel ou sans solution
    /// (OF-GCS-0002).
    #[error(
        "OF-GCS-0002 : résolution échouée avec {algorithme:?} : croquis \
         conflictuel ou sans solution"
    )]
    ResolutionEchouee {
        /// Algorithme employé.
        algorithme: Algorithme,
    },
    /// Reproductibilité non garantie : l'arène de paramètres n'a pas pu
    /// maintenir des adresses croissantes (OF-GCS-0003, FreeCAD #33399).
    #[error("OF-GCS-0003 : reproductibilité non garantie : {contexte}")]
    NonReproductible {
        /// Explication.
        contexte: String,
    },
    /// Appel invalide à l'ABI C : handle nul, valeur non finie ou
    /// exception C++ rattrapée dans le shim (OF-GCS-0004).
    #[error("OF-GCS-0004 : appel invalide à l'ABI C : {contexte}")]
    AbiInvalide {
        /// Explication.
        contexte: String,
    },
    /// Solution trouvée mais géométrie résultante invalide (statut
    /// SuccessfulSolutionInvalid de planegcs, OF-GCS-0005).
    #[error(
        "OF-GCS-0005 : solution invalide avec {algorithme:?} : résolution \
         réussie mais géométrie invalide (SuccessfulSolutionInvalid)"
    )]
    SolutionInvalide {
        /// Algorithme employé.
        algorithme: Algorithme,
    },
}

impl GcsError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::IdInconnu { .. } => "OF-GCS-0001",
            Self::ResolutionEchouee { .. } => "OF-GCS-0002",
            Self::NonReproductible { .. } => "OF-GCS-0003",
            Self::AbiInvalide { .. } => "OF-GCS-0004",
            Self::SolutionInvalide { .. } => "OF-GCS-0005",
        }
    }
}
