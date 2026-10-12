//! Registre des migrations de schémas d'événements (ADR-0001 :
//! fonctions pures `v_n → v_n+1` par type, testées en CI).

use std::collections::BTreeMap;
use std::fmt;

use crate::error::DocError;
use crate::events::Event;

/// Fonction de migration pure d'un schéma `v_n` vers `v_n + 1`.
pub type MigrationFn = fn(&serde_json::Value) -> Result<serde_json::Value, DocError>;

/// Registre des migrations, par type d'événement. Chaque type y déclare sa
/// version la plus récente (`declare`) et les étapes pour y parvenir
/// (`register`).
#[derive(Default)]
pub struct MigrationRegistry {
    steps: BTreeMap<(String, u32), MigrationFn>,
    latest: BTreeMap<String, u32>,
}

impl fmt::Debug for MigrationRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MigrationRegistry")
            .field("types", &self.latest)
            .finish_non_exhaustive()
    }
}

impl MigrationRegistry {
    /// Nouveau registre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Déclare la version de schéma la plus récente connue pour un type.
    pub fn declare(&mut self, event_type: &str, latest_version: u32) {
        self.latest
            .insert(event_type.to_string(), latest_version);
    }

    /// Enregistre l'étape de migration `from_version → from_version + 1`.
    pub fn register(&mut self, event_type: &str, from_version: u32, migration: MigrationFn) {
        self.steps
            .insert((event_type.to_string(), from_version), migration);
    }

    /// Migre un événement vers la version la plus récente connue de son
    /// type. Un type non déclaré est laissé tel quel (compatibilité
    /// ascendante) ; un événement plus récent que la version connue est une
    /// erreur OF-DOC-0005.
    pub fn migrate(&self, event: &Event) -> Result<Event, DocError> {
        let latest = self
            .latest
            .get(&event.event_type)
            .copied()
            .unwrap_or(event.schema_version);
        if event.schema_version > latest {
            return Err(DocError::SchemaTooNew {
                event_type: event.event_type.clone(),
                version: event.schema_version,
            });
        }
        let mut migrated = event.clone();
        while migrated.schema_version < latest {
            let step = self
                .steps
                .get(&(migrated.event_type.clone(), migrated.schema_version))
                .ok_or_else(|| DocError::MissingMigration {
                    event_type: migrated.event_type.clone(),
                    from: migrated.schema_version,
                    to: migrated.schema_version + 1,
                })?;
            migrated.data = (*step)(&migrated.data)?;
            migrated.schema_version += 1;
        }
        Ok(migrated)
    }
}
