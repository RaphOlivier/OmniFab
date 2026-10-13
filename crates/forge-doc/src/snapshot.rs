//! Snapshot accélérateur (ADR-0001).

use serde::{Deserialize, Serialize};

/// Snapshot d'état projeté, **non autoritaire** : il accélère l'ouverture
/// des projets longs mais ne fait jamais foi — un snapshot incompatible est
/// ignoré et le journal rejoué intégralement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Version du format de snapshot ; toute autre valeur que
    /// [`crate::SNAPSHOT_VERSION`] entraîne l'ignorance du snapshot.
    pub format_version: u32,
    /// Numéro de séquence du dernier événement inclus dans l'état
    /// projeté : le rejeu du journal reprend après lui.
    #[serde(default)]
    pub at_seq: u64,
    /// Condensat SHA-256 du journal jusqu'à `at_seq`, préfixé `sha256:`
    /// (schéma B) : un snapshot dont le condensat ne correspond pas au
    /// journal est ignoré, le journal fait foi.
    #[serde(default)]
    pub journal_digest: String,
    /// État projeté, opaque pour forge-doc (la sémantique appartient aux
    /// couches métier).
    pub data: serde_json::Value,
}
