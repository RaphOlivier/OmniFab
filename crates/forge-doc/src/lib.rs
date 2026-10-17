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
//!   ([`Document::set_blob`]) et les événements n'en portent que des
//!   références ;
//! - la sérialisation d'archive est en mémoire (compatible wasm) ;
//!   l'accès disque est isolé derrière la feature `native`
//!   ([`Document::save_to_path`], [`Document::open_from_path`]).

mod clock;
mod container;
mod error;
mod events;
mod migration;
mod snapshot;

pub use clock::{Clock, FixedClock, SystemClock};
pub use error::DocError;
pub use events::{Event, NewEvent};
pub use migration::{MigrationFn, MigrationRegistry};
pub use snapshot::Snapshot;

use std::collections::BTreeMap;

/// Format du conteneur de projet (champ `format` du manifeste).
pub const CONTAINER_FORMAT: &str = "omnifab-document";

/// Version du conteneur de projet (champ `version` du manifeste).
pub const CONTAINER_VERSION: u32 = 1;

/// Version du format de snapshot.
pub const SNAPSHOT_VERSION: u32 = 2;

/// Chemin du manifeste dans l'archive.
pub const MANIFEST_PATH: &str = "manifest.json";

/// Chemin du journal d'événements dans l'archive.
pub const JOURNAL_PATH: &str = "events.jl";

/// Chemin du snapshot dans l'archive.
pub const SNAPSHOT_PATH: &str = "snapshot.json";

/// Préfixe des objets binaires dans l'archive.
pub const BLOBS_PREFIX: &str = "objects/";

/// Document OmniFab : conteneur ZIP versionné + journal d'événements.
#[derive(Debug)]
pub struct Document {
    /// Journal complet, dans l'ordre.
    pub(crate) events: Vec<Event>,
    /// Objets binaires (maillages, images…) référencés par les événements.
    pub(crate) blobs: BTreeMap<String, Vec<u8>>,
    /// Snapshot non autoritaire, s'il a été enregistré et est compatible.
    pub(crate) snapshot: Option<Snapshot>,
    /// Registre de migrations appliqué au chargement.
    pub(crate) migrations: MigrationRegistry,
    /// Horloge injectable : horodatage des événements ajoutés
    /// (d'autres horloges que l'horloge système rendent le journal
    /// déterministe en test).
    pub(crate) clock: Box<dyn Clock>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            blobs: BTreeMap::new(),
            snapshot: None,
            migrations: MigrationRegistry::new(),
            clock: Box::new(SystemClock),
        }
    }
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

    /// Remplace l'horloge du document (horloge figée en test).
    pub fn with_clock(mut self, clock: Box<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Ajoute un événement au journal : lui attribue le prochain numéro de
    /// séquence et un horodatage ISO 8601 fourni par l'horloge du
    /// document, puis retourne le numéro attribué.
    pub fn append(&mut self, new_event: NewEvent) -> u64 {
        let seq = self.events.len() as u64 + 1;
        let timestamp = iso8601_from_unix(self.clock.now_unix());
        let id = uuid::Uuid::now_v7().to_string();
        self.events.push(Event {
            seq,
            id: Some(id),
            event_type: new_event.event_type,
            schema_version: new_event.schema_version,
            timestamp,
            actor: new_event.actor,
            data: new_event.data,
            txn: new_event.txn,
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
    ///
    /// Le nom est validé (OF-DOC-0013) et la taille bornée (OF-DOC-0011).
    pub fn set_blob(&mut self, name: &str, data: Vec<u8>) -> Result<(), DocError> {
        container::validate_blob_name(name)?;
        if data.len() as u64 > container::MAX_BLOB_BYTES {
            return Err(DocError::EntryTooLarge {
                entry: format!("{BLOBS_PREFIX}{name}"),
                found: data.len() as u64,
                max: container::MAX_BLOB_BYTES,
            });
        }
        self.blobs.insert(name.to_string(), data);
        Ok(())
    }

    /// Retourne un objet binaire par son nom.
    pub fn blob(&self, name: &str) -> Option<&[u8]> {
        self.blobs.get(name).map(Vec::as_slice)
    }

    /// Noms des objets binaires, triés.
    pub fn blob_names(&self) -> Vec<String> {
        self.blobs.keys().cloned().collect()
    }

    /// Retire un objet binaire du document, et le retourne s'il
    /// existait. Utilisé par forge-session pour purger les blobs
    /// orphelins (ADR-0008/0011).
    pub fn remove_blob(&mut self, name: &str) -> Option<Vec<u8>> {
        self.blobs.remove(name)
    }

    /// Enregistre un snapshot **non autoritaire** (accélérateur, ADR-0001),
    /// lié à l'état du journal au moment de l'enregistrement (schéma B
    /// validé) : il porte le numéro de séquence atteint et un condensat du
    /// journal jusqu'à ce point — à la relecture, toute divergence fait
    /// ignorer le snapshot et rejouer le journal.
    pub fn set_snapshot(&mut self, data: serde_json::Value) -> Result<(), DocError> {
        let at_seq = self.events.last().map(|event| event.seq).unwrap_or(0);
        self.snapshot = Some(Snapshot {
            format_version: SNAPSHOT_VERSION,
            at_seq,
            journal_digest: container::journal_digest(&self.events, at_seq),
            data,
        });
        Ok(())
    }

    /// Événements strictement postérieurs à `at_seq` : la part du journal
    /// à rejouer après un snapshot enregistré à ce numéro.
    pub fn events_after(&self, at_seq: u64) -> &[Event] {
        let index = self.events.partition_point(|event| event.seq <= at_seq);
        &self.events[index..]
    }

    /// Retire du journal, en mémoire, les événements de numéro de séquence
    /// strictement supérieur à `at_seq` et les retourne dans l'ordre. Le
    /// snapshot, s'il porte sur des événements retirés, est abandonné
    /// (ADR-0001 : jamais autoritaire). Utilisé par forge-session pour
    /// abandonner les événements défaisables (ADR-0008).
    pub fn pop_after(&mut self, at_seq: u64) -> Vec<Event> {
        let index = self.events.partition_point(|event| event.seq <= at_seq);
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.at_seq > at_seq)
        {
            self.snapshot = None;
        }
        self.events.split_off(index)
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

    /// Enregistre le document dans un fichier, de manière atomique
    /// (feature `native`) : l'archive est d'abord écrite et synchronisée
    /// dans un fichier temporaire, puis le projet existant est copié
    /// en `{projet}.bak` sans quitter son chemin, et enfin le
    /// temporaire est renommé à sa place. Le fichier courant reste
    /// donc en place tant que le nouveau n'est pas entièrement
    /// écrit : un échec d'écriture (disque plein, erreur E/S) ne peut
    /// ni retirer le projet de son chemin, ni laisser un fichier à
    /// moitié écrit (ADR-0016 A2).
    #[cfg(feature = "native")]
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), DocError> {
        let bytes = self.to_bytes()?;
        let mut tmp = std::ffi::OsString::from(path.as_os_str());
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        let mut backup = std::ffi::OsString::from(path.as_os_str());
        backup.push(".bak");
        let backup = std::path::PathBuf::from(backup);
        if let Err(erreur) = remplacer_sur_disque(path, &tmp, &backup, &bytes) {
            // Aucune étape n'a déplacé le fichier courant : seul le
            // temporaire incomplet peut rester, il est nettoyé.
            let _ = std::fs::remove_file(&tmp);
            return Err(erreur.into());
        }
        Ok(())
    }

    /// Charge un document depuis un fichier (feature `native`).
    #[cfg(feature = "native")]
    pub fn open_from_path(path: &std::path::Path) -> Result<Self, DocError> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// Charge un document depuis un fichier en appliquant les
    /// migrations du registre donné (feature `native`) : l'appelant
    /// déclare les types d'événement qu'il connaît — un journal
    /// écrit par une version plus récente est refusé par `SchemaTooNew`
    /// (ADR-0001/0016 A3) au lieu d'être lu comme du v1.
    #[cfg(feature = "native")]
    pub fn open_from_path_with(
        path: &std::path::Path,
        migrations: MigrationRegistry,
    ) -> Result<Self, DocError> {
        Self::from_bytes_with(&std::fs::read(path)?, migrations)
    }
}

/// Écrit le conteneur dans le fichier temporaire, copie le projet
/// courant en sauvegarde, puis remplace le projet par le temporaire
/// (ADR-0016 A2 : l'original n'est jamais déplacé avant que le
/// remplaçant ne soit entièrement sur le disque).
#[cfg(feature = "native")]
fn remplacer_sur_disque(
    path: &std::path::Path,
    tmp: &std::path::Path,
    backup: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    {
        use std::io::Write;
        let mut file = std::fs::File::create(tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if path.exists() {
        std::fs::copy(path, backup)?;
    }
    std::fs::rename(tmp, path)
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
    use super::{Document, NewEvent, iso8601_from_unix};

    #[test]
    fn iso8601() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        // 2000-02-29 : année bissextile.
        assert_eq!(iso8601_from_unix(951782400), "2000-02-29T00:00:00Z");
    }

    /// pop_after retire et retourne les événements au-delà du numéro.
    #[test]
    fn pop_after_tronque_et_retourne() {
        let mut document = Document::new();
        document.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        document.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        let retires = document.pop_after(1);
        assert_eq!(retires.len(), 1, "un seul événement retiré");
        assert_eq!(retires[0].seq, 2);
        assert_eq!(document.events().len(), 1);
        assert_eq!(document.events()[0].seq, 1);
    }

    /// pop_after abandonne un snapshot portant sur des événements
    /// retirés (jamais autoritaire, ADR-0001).
    #[test]
    fn pop_after_abandonne_un_snapshot_perime() {
        let mut document = Document::new();
        document.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        document
            .set_snapshot(serde_json::json!({}))
            .expect("snapshot enregistré");
        assert!(document.snapshot().is_some());
        document.pop_after(0);
        assert!(
            document.snapshot().is_none(),
            "le snapshot portait sur des événements retirés"
        );
    }

    /// remove_blob retire un objet binaire et le retourne, puis ne
    /// trouve plus rien (purge des blobs orphelins, ADR-0011).
    #[test]
    fn remove_blob_retire_puis_retourne_none() {
        let mut document = Document::new();
        document
            .set_blob("geometry/traces", vec![1, 2, 3])
            .expect("blob valide");
        let retires = document.remove_blob("geometry/traces");
        assert_eq!(retires, Some(vec![1, 2, 3]));
        assert!(document.blob("geometry/traces").is_none());
    }
}
