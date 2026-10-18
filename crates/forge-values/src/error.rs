//! Erreurs typées de forge-values (ADR-0003 : codes stables `OF-VAL-NNNN`,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreur du moteur de liaisons. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum ValuesError {
    /// Cycle détecté dans le graphe de liaisons (OF-VAL-0001).
    #[error("OF-VAL-0001 : cycle dans le graphe de liaisons : {path}")]
    Cycle {
        /// Chemin du cycle, ex. « a → b → a ».
        path: String,
    },
    /// Référence à une valeur inconnue dans une expression (OF-VAL-0002).
    #[error("OF-VAL-0002 : « {node} » référence la valeur inconnue « {name} »")]
    UnknownReference {
        /// Nom de la valeur portant l'expression fautive.
        node: String,
        /// Nom de la référence introuvable.
        name: String,
    },
    /// Expression hors du sous-ensemble déterministe accepté (OF-VAL-0003).
    #[error("OF-VAL-0003 : expression invalide pour « {node} » : {reason}")]
    InvalidExpression {
        /// Nom de la valeur portant l'expression fautive.
        node: String,
        /// Explication de l'erreur de syntaxe.
        reason: String,
    },
    /// Erreur à l'évaluation, ex. division par zéro, racine carrée d'un
    /// nombre négatif, résultat non fini (OF-VAL-0004).
    #[error("OF-VAL-0004 : erreur d'évaluation de « {node} » : {reason}")]
    Evaluation {
        /// Nom de la valeur fautive.
        node: String,
        /// Explication de l'erreur d'exécution.
        reason: String,
    },
    /// Lecture d'une valeur inexistante (OF-VAL-0005).
    #[error("OF-VAL-0005 : valeur inconnue « {name} »")]
    UnknownValue {
        /// Nom de la valeur introuvable.
        name: String,
    },
    /// Écriture d'une valeur d'entrée non finie (OF-VAL-0006).
    #[error("OF-VAL-0006 : la valeur d'entrée « {name} » doit être un nombre fini")]
    NonFiniteInput {
        /// Nom de la valeur fautive.
        name: String,
    },
}

impl ValuesError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cycle { .. } => "OF-VAL-0001",
            Self::UnknownReference { .. } => "OF-VAL-0002",
            Self::InvalidExpression { .. } => "OF-VAL-0003",
            Self::Evaluation { .. } => "OF-VAL-0004",
            Self::UnknownValue { .. } => "OF-VAL-0005",
            Self::NonFiniteInput { .. } => "OF-VAL-0006",
        }
    }
}
