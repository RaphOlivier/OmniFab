//! Événements du journal (ADR-0001).

use serde::{Deserialize, Serialize};

/// Événement persisté dans le journal `events.jl`, un par ligne (JSON
/// Lines). Les gros objets (maillages, images, géométrie) ne vivent jamais
/// dans les événements : ils sont des objets du conteneur, référencés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Numéro de séquence, strictement croissant à partir de 1.
    pub seq: u64,
    /// Nom stable du type d'événement, jamais renommé une fois publié.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Version du schéma de ce type d'événement.
    pub schema_version: u32,
    /// Horodatage ISO 8601 (UTC).
    pub timestamp: String,
    /// Identifiant de l'acteur / de la zone UI à l'origine de l'événement.
    pub actor: String,
    /// Données propres à l'événement (petit enregistrement fermé).
    pub data: serde_json::Value,
    /// Identifiant de transaction, si l'événement appartient à un groupe
    /// à appliquer ensemble (absent des événements hors transaction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub txn: Option<String>,
}

/// Événement neuf, avant attribution du numéro de séquence et de
/// l'horodatage par le document.
#[derive(Debug, Clone)]
pub struct NewEvent {
    /// Nom stable du type d'événement.
    pub event_type: String,
    /// Version du schéma de ce type.
    pub schema_version: u32,
    /// Acteur / zone UI à l'origine de l'événement.
    pub actor: String,
    /// Données propres à l'événement.
    pub data: serde_json::Value,
    /// Identifiant de transaction, `None` hors transaction.
    pub txn: Option<String>,
}

impl NewEvent {
    /// Construit un nouvel événement.
    pub fn new(
        event_type: &str,
        schema_version: u32,
        actor: &str,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_type.to_string(),
            schema_version,
            actor: actor.to_string(),
            data,
            txn: None,
        }
    }

    /// Marque l'événement comme appartenant à la transaction donnée : les
    /// événements d'un même groupe portent le même identifiant.
    pub fn in_txn(mut self, txn: impl Into<String>) -> Self {
        self.txn = Some(txn.into());
        self
    }
}
