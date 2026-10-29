//! Erreurs typées de forge-values (ADR-0003 : codes stables `OF-VAL-NNNN`,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

use crate::Unite;

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
    /// Incohérence d'unités à la définition (OF-VAL-0007) : une valeur
    /// déclarée d'une unité lit une référence d'une autre unité.
    #[error("OF-VAL-0007 : « {node} » en {unite} lit « {reference} » en {unite_reference}")]
    UniteIncoherente {
        /// Nom de la valeur portant la définition fautive.
        node: String,
        /// Unité déclarée de la valeur fautive.
        unite: Unite,
        /// Nom de la référence incompatible.
        reference: String,
        /// Unité déclarée de la référence.
        unite_reference: Unite,
    },
    /// Alias réservé (OF-VAL-0008) : création sous un préfixe système
    /// ou écrasement d'un nœud d'une autre origine.
    #[error("OF-VAL-0008 : alias réservé : « {name} »")]
    AliasReserve {
        /// Nom de l'alias refusé.
        name: String,
    },
    /// Alias encore référencé (OF-VAL-0009) : suppression `Simple`
    /// impossible tant que des formules lisent l'alias.
    #[error("OF-VAL-0009 : alias « {name} » référencé par : {references}")]
    AliasReference {
        /// Nom de l'alias supprimé.
        name: String,
        /// Noms des valeurs qui le référencent, joints par « , ».
        references: String,
    },
    /// Chaîne de dépendances trop profonde (OF-VAL-0011) : le coût
    /// d'évaluation — profondeur d'AST plus sauts de dépendances —
    /// dépasse la borne, refus à la définition (audit n°3, A3-03).
    #[error("OF-VAL-0011 : « {node} » dépasse la profondeur de chaîne : {profondeur} > {max}")]
    ProfondeurChaine {
        /// Nom de la valeur refusée.
        node: String,
        /// Coût d'évaluation mesuré.
        profondeur: usize,
        /// Borne maximale acceptée.
        max: usize,
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
            Self::UniteIncoherente { .. } => "OF-VAL-0007",
            Self::AliasReserve { .. } => "OF-VAL-0008",
            Self::AliasReference { .. } => "OF-VAL-0009",
            Self::ProfondeurChaine { .. } => "OF-VAL-0011",
        }
    }
}
