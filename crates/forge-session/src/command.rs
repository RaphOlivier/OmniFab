//! Commandes de la session (ADR-0008) : une intention utilisateur = une
//! validation contre l'état courant, puis 1..n événements.

use crate::{Session, SessionError};
use forge_doc::NewEvent;

/// Type d'événement : valeur d'entrée fixée (schéma v1 : `name`, `value`).
pub const EVENT_INPUT_SET: &str = "value.input_set";
/// Type d'événement : expression de liaison fixée (schéma v1 : `name`,
/// `expression`).
pub const EVENT_EXPRESSION_SET: &str = "value.expression_set";

/// Une intention utilisateur. La commande se **planifie** contre l'état
/// courant : elle valide ses données, puis produit 1..n événements. La
/// session attribue ensuite à chaque événement l'acteur et un identifiant
/// de transaction (une commande = un groupe d'undo) — la commande ne
/// s'en occupe pas.
pub trait Command {
    /// Valide la commande contre l'état courant et produit ses
    /// événements.
    fn plan(&self, session: &Session) -> Result<Vec<NewEvent>, SessionError>;
}

/// Fixe la valeur d'entrée nommée du graphe de liaisons.
///
/// Le nombre est sans unité (ADR-0007 phase 1 : l'unité sera déclarée par
/// le nœud au jalon 2) ; les newtypes de forge-units restent la frontière
/// programmatique des grandeurs dimensionnées.
#[derive(Debug, Clone)]
pub struct SetInput {
    /// Nom de la valeur.
    pub name: String,
    /// Valeur d'entrée ; un nombre non fini est refusé au plan.
    pub value: f64,
}

impl Command for SetInput {
    fn plan(&self, _session: &Session) -> Result<Vec<NewEvent>, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        if !self.value.is_finite() {
            return Err(SessionError::CommandRejected {
                reason: "la valeur doit être un nombre fini".to_string(),
            });
        }
        Ok(vec![NewEvent::new(
            EVENT_INPUT_SET,
            1,
            "",
            serde_json::json!({"name": self.name, "value": self.value}),
        )])
    }
}

/// Définit l'expression de la valeur nommée du graphe de liaisons.
///
/// La validation syntaxique et la résolution des références ont lieu à
/// l'application (projection forge-values) : une expression fautive est
/// rejetée par [`SessionError::ProjectionFailed`] sans toucher au
/// journal.
#[derive(Debug, Clone)]
pub struct SetExpression {
    /// Nom de la valeur.
    pub name: String,
    /// Expression dans le sous-ensemble déterministe de forge-values.
    pub expression: String,
}

impl Command for SetExpression {
    fn plan(&self, _session: &Session) -> Result<Vec<NewEvent>, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        if self.expression.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "l'expression ne peut pas être vide".to_string(),
            });
        }
        Ok(vec![NewEvent::new(
            EVENT_EXPRESSION_SET,
            1,
            "",
            serde_json::json!({"name": self.name, "expression": self.expression}),
        )])
    }
}
