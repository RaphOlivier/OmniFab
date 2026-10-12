#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-doc — conteneur de projet et journal d'événements
//!
//! (ADR-0000 verrous nº1 et nº2, ADR-0001.) Un projet OmniFab est un
//! **conteneur ZIP versionné** : un manifeste JSON (`manifest.json`), un
//! journal d'événements **JSON Lines** (`events.jl`), des objets binaires
//! (`objects/`) et un éventuel snapshot (`snapshot.json`).
//!
//! - chaque opération utilisateur est un **événement sérialisé et
//!   rejouable** ([`Document::append`], [`Document::replay`]) ; le modèle
//!   de document se reconstruit par projection (rejeu du journal) ;
//! - chaque type d'événement porte sa **version de schéma** ; les
//!   migrations sont des fonctions pures `v_n → v_n+1` enregistrées dans
//!   un [`MigrationRegistry`], appliquées au chargement ;
//! - les snapshots sont des **accélérateurs non autoritaires** : un
//!   snapshot incompatible est ignoré et le journal rejoué ;
//! - les gros objets ne vivent jamais dans le journal : ils sont des blobs
//!   (`Document::set_blob`) et les événements n'en portent que des
//!   références ;
//! - la sérialisation d'archive est en mémoire (compatible wasm) ;
//!   l'accès disque est isolé derrière la feature `native`
//!   (`Document::save_to_path`, `Document::open_from_path`).

mod container;
mod error;
mod events;
mod migration;
mod snapshot;

pub use error::DocError;
pub use events::{Event, NewEvent};
pub use migration::{MigrationFn, MigrationRegistry};
pub use snapshot::Snapshot;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Format du conteneur de projet (champ `format` du manifeste).
pub const CONTAINER_FORMAT: &str = "omnifab-document";

/// Version du conteneur de projet (champ `version` du manifeste).
pub const CONTAINER_VERSION: u32 = 1;

/// Version du format de snapshot.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Chemin du manifeste dans l'archive.
pub const MANIFEST_PATH: &str = "manifest.json";

/// Chemin du journal d'événements dans l'archive.
pub const JOURNAL_PATH: &str = "events.jl";

/// Chemin du snapshot dans l'archive.
pub const SNAPSHOT_PATH: &str = "snapshot.json";

/// Préfixe des objets binaires dans l'archive.
pub const BLOBS_PREFIX: &str = "objects/";

/// Document OmniFab : conteneur ZIP versionné + journal d'événements.
#[derive(Debug, Default)]
pub struct Document {
    /// Journal complet, dans l'ordre.
    pub(crate) events: Vec<Event>,
    /// Objets binaires (maillages, images…) référencés par les événements.
    pub(crate) blobs: BTreeMap<String, Vec<u8>>,
    /// Snapshot non autoritaire, s'il a été enregistré et est compatible.
    pub(crate) snapshot: Option<Snapshot>,
    /// Registre de migrations appliqué au chargement.
    pub(crate) migrations: MigrationRegistry,
}

impl Document {
    /// Crée un document vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Crée un document vide avec un registre de migrations donné.
    pub fn with_migrations(migrations: MigrationRegistry) -> Self {
        Self {
            migrations,
            ..Self::new()
        }
    }

    /// Ajoute un événement au journal : lui attribue le prochain numéro de
    /// séquence et un horodatage ISO 8601, puis retourne le numéro attribué.
    pub fn append(&mut self, new_event: NewEvent) -> u64 {
        let seq = self.events.len() as u64 + 1;
        self.events.push(Event {
            seq,
            event_type: new_event.event_type,
            schema_version: new_event.schema_version,
            timestamp: now_iso8601(),
            actor: new_event.actor,
            data: new_event.data,
        });
        seq
    }

    /// Journal complet, dans l'ordre.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Rejoue le journal dans l'ordre : chaque événement est passé au
    /// `handler`, qui peut échouer avec son propre type d'erreur `E`
    /// (la projection est du ressort des couches métier, pas de forge-doc).
    pub fn replay<E>(&self, mut handler: impl FnMut(&Event) -> Result<(), E>) -> Result<(), E> {
        for event in &self.events {
            handler(event)?;
        }
        Ok(())
    }

    /// Ré-applique les migrations du registre du document à son journal.
    ///
    /// Utile lorsque le registre a été enrichi (`declare`/`register`) après
    /// le chargement du document, pour amener chaque événement à la version
    /// de schéma la plus récente connue (ADR-0001).
    pub fn migrate_events(&mut self) -> Result<(), DocError> {
        for event in &mut self.events {
            *event = self.migrations.migrate(event)?;
        }
        Ok(())
    }
    /// Enregistre un objet binaire (maillage, image…), sous `objects/`.
    pub fn set_blob(&mut self, name: &str, data: Vec<u8>) {
        self.blobs.insert(name.to_string(), data);
    }

    /// Retourne un objet binaire par son nom.
    pub fn blob(&self, name: &str) -> Option<&[u8]> {
        self.blobs.get(name).map(Vec::as_slice)
    }

    /// Noms des objets binaires, triés.
    pub fn blob_names(&self) -> Vec<String> {
        self.blobs.keys().cloned().collect()
    }

    /// Enregistre un snapshot **non autoritaire** (accélérateur, ADR-0001).
    pub fn set_snapshot(&mut self, data: serde_json::Value) {
        self.snapshot = Some(Snapshot {
            format_version: SNAPSHOT_VERSION,
            data,
        });
    }

    /// Snapshot enregistré, s'il existe et est compatible.
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// Sérialise le document en conteneur ZIP (en mémoire).
    pub fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
        container::write(self)
    }

    /// Charge un document depuis un conteneur ZIP, sans migrations
    /// déclarées (les événements restent à leur version d'origine).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
        Self::from_bytes_with(bytes, MigrationRegistry::new())
    }

    /// Charge un document en appliquant les migrations du registre donné.
    pub fn from_bytes_with(bytes: &[u8], migrations: MigrationRegistry) -> Result<Self, DocError> {
        container::read(bytes, migrations)
    }

    /// Enregistre le document dans un fichier (feature `native`).
    #[cfg(feature = "native")]
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), DocError> {
        std::fs::write(path, self.to_bytes()?)?;
        Ok(())
    }

    /// Charge un document depuis un fichier (feature `native`).
    #[cfg(feature = "native")]
    pub fn open_from_path(path: &std::path::Path) -> Result<Self, DocError> {
        Self::from_bytes(&std::fs::read(path)?)
    }
}

/// Horodatage ISO 8601 UTC courant.
fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    iso8601_from_unix(secs)
}

/// Convertit un horodatage Unix (secondes) en chaîne ISO 8601 UTC.
///
/// Exemple : `0` → `1970-01-01T00:00:00Z`.
pub fn iso8601_from_unix(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Convertit un nombre de jours depuis l'époque Unix en date civile.
/// Algorithme de Howard Hinnant (domaine public).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::iso8601_from_unix;

    #[test]
    fn iso8601() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        // 2000-02-29 : année bissextile.
        assert_eq!(iso8601_from_unix(951782400), "2000-02-29T00:00:00Z");
    }
}
