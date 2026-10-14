//! Projections par domaine (ADR-0008) : état métier reconstruit par rejeu
//! du journal d'événements.

use crate::command::{EVENT_EXPRESSION_SET, EVENT_INPUT_SET};
use crate::SessionError;
use forge_doc::NewEvent;
use forge_values::ValueGraph;

/// Projection d'un domaine métier : l'état du domaine reconstruit par
/// rejeu du journal. Les domaines futurs (esquisse, CAO, CAM…)
/// enregistrent leurs projections auprès de la session sans modifier
/// cette crate.
pub trait Projection {
    /// Nom du domaine, pour les messages d'erreur.
    fn name(&self) -> &'static str;
    /// Remet la projection à vide, avant un rejeu.
    fn reset(&mut self);
    /// Applique un événement. Les types inconnus sont tolérés
    /// (compatibilité avant : journal écrit par une version plus
    /// récente) ; un refus est une erreur typée.
    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError>;
}

/// Projection du domaine des liaisons (forge-values) — la projection
/// enregistrée au jalon 1.
#[derive(Debug, Default)]
pub struct ValuesProjection {
    /// Graphe de liaisons reconstruit par rejeu.
    graph: ValueGraph,
    /// Types d'événements ignorés, uniques, dans l'ordre de rencontre.
    unknown: Vec<String>,
}

impl ValuesProjection {
    /// Nouvelle projection vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Graphe de liaisons projeté.
    pub fn graph(&self) -> &ValueGraph {
        &self.graph
    }

    /// Graphe de liaisons projeté, mutable (lecture paresseuse et
    /// recalculs, cf. ADR-0002).
    pub fn graph_mut(&mut self) -> &mut ValueGraph {
        &mut self.graph
    }

    /// Types d'événements ignorés par cette projection.
    pub fn unknown_types(&self) -> &[String] {
        &self.unknown
    }
}

impl Projection for ValuesProjection {
    fn name(&self) -> &'static str {
        "values"
    }

    fn reset(&mut self) {
        self.graph = ValueGraph::new();
        self.unknown.clear();
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        match event.event_type.as_str() {
            EVENT_INPUT_SET => {
                let name = champ_str(event, "name")?;
                let value = champ_nombre(event, "value")?;
                self.graph.set_input(name, value);
                Ok(())
            }
            EVENT_EXPRESSION_SET => {
                let name = champ_str(event, "name")?;
                let expression = champ_str(event, "expression")?;
                self.graph.set_expression(name, expression).map_err(|source| {
                    SessionError::ProjectionFailed {
                        projection: "values".to_string(),
                        source: source.to_string(),
                    }
                })
            }
            _ => {
                if !self.unknown.contains(&event.event_type) {
                    self.unknown.push(event.event_type.clone());
                }
                Ok(())
            }
        }
    }
}

/// Lit un champ chaîne du schéma v1 du type d'événement.
fn champ_str<'a>(event: &'a NewEvent, champ: &str) -> Result<&'a str, SessionError> {
    event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (chaîne) absent"),
        })
}

/// Lit un champ nombre du schéma v1 du type d'événement.
fn champ_nombre(event: &NewEvent, champ: &str) -> Result<f64, SessionError> {
    event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (nombre) absent"),
        })
}
