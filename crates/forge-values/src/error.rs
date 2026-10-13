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
}
