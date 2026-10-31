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
/// Taille cumulée maximale des objets binaires acceptée au chargement
/// d'un conteneur (défaut de [`Document::from_bytes_with_budget`],
/// ADR-0016 A4 — garde-fou « bombe ZIP » cumulative).
pub const MAX_TOTAL_BLOB_BYTES: u64 = container::MAX_TOTAL_BLOB_BYTES;

/// Budget d'octets d'un fichier projet ouvert depuis le disque
/// (audit n°3 A3-07, ADR-0009) : le double du plafond des blobs
/// cumulés — un fichier légitime peut embarquer 2 Gio de blobs, le
/// refus ne touche que les fichiers démesurés.
pub const MAX_FICHIER_OCTETS: u64 = 2 * MAX_TOTAL_BLOB_BYTES;

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
    /// Dernière milliseconde d'un UUIDv7 attribué : le compteur de
    /// monotonie repart à zéro à chaque nouvelle milliseconde.
    pub(crate) uuid_derniere_ms: u64,
    /// Compteur de monotonie des UUIDv7 attribués dans la même
    /// milliseconde (ADR-0016 A7).
    pub(crate) uuid_counter: u16,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            blobs: BTreeMap::new(),
            snapshot: None,
            migrations: MigrationRegistry::new(),
            clock: Box::new(SystemClock),
            uuid_derniere_ms: 0,
            uuid_counter: 0,
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
    /// séquence, un horodatage ISO 8601 et un identifiant UUIDv7, tous
    /// tirés de l'horloge du document (ADR-0016 A7), puis retourne le
    /// numéro attribué.
    pub fn append(&mut self, new_event: NewEvent) -> u64 {
        let seq = self.events.len() as u64 + 1;
        let timestamp = iso8601_from_unix(self.clock.now_unix());
        let id = self.nouvel_uuid_v7();
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

    /// Attribue un nouvel identifiant UUIDv7 : l'horodatage (précision
    /// ms) vient de l'horloge injectable du document, et un compteur de
    /// monotonie sur 12 bits distingue les identifiants attribués dans
    /// la même milliseconde — le journal reste déterministe sous
    /// [`FixedClock`] et ne dépend plus de l'horloge système, dont le
    /// comportement est incertain en wasm32 (ADR-0016 A7). Le compteur
    /// n'est remis à zéro que si l'horloge avance ; à saturation des
    /// 12 bits, la milliseconde de référence avance d'un cran, et au
    /// chargement d'un conteneur `reprendre_compteur_uuid` repart
    /// au-delà du plus grand identifiant du journal (audit n°3 A3-11).
    pub fn nouvel_uuid_v7(&mut self) -> String {
        let millis = u64::try_from(self.clock.now_unix()).unwrap_or(0) * 1000;
        // Remise à zéro seulement si l'horloge AVANCE : sous horloge
        // figée (tests, reprise après rechargement), le compteur
        // continue de croître (audit n°3 A3-11).
        if millis > self.uuid_derniere_ms {
            self.uuid_derniere_ms = millis;
            self.uuid_counter = 0;
        }
        // Saturation des 12 bits du compteur : la milliseconde de
        // référence avance d'un cran pour rester monotone (l'horloge
        // peut être figée — le déterminisme est préservé).
        if self.uuid_counter > 0x0FFF {
            self.uuid_derniere_ms = self.uuid_derniere_ms.saturating_add(1);
            self.uuid_counter = 0;
        }
        let millis = self.uuid_derniere_ms;
        let compteur = self.uuid_counter.min(0x0FFF);
        self.uuid_counter = self.uuid_counter.saturating_add(1);
        let mut counter_random_bytes = [0u8; 10];
        counter_random_bytes[0] = (compteur >> 8) as u8;
        counter_random_bytes[1] = (compteur & 0xFF) as u8;
        uuid::Builder::from_unix_timestamp_millis(millis, &counter_random_bytes)
            .into_uuid()
            .to_string()
    }

    /// Reprise du compteur de monotonie au chargement : parcourt le
    /// journal rechargé à la recherche du plus grand UUIDv7 (date,
    /// compteur) et repart au-delà — un événement ajouté après
    /// rechargement ne peut plus recycler l'identifiant d'un événement
    /// chargé, même sous une horloge figée, et les sessions successives
    /// ne partagent plus de transaction (audit n°3 A3-11).
    pub(crate) fn reprendre_compteur_uuid(&mut self) {
        let mut max_ms = 0u64;
        let mut max_compteur = 0u16;
        for id in self.events.iter().filter_map(|event| event.id.as_deref()) {
            let Ok(uuid) = uuid::Uuid::parse_str(id) else {
                continue;
            };
            if uuid.get_version_num() != 7 {
                continue;
            }
            let octets = uuid.as_bytes();
            let mut date = [0u8; 8];
            date[2..].copy_from_slice(&octets[0..6]);
            let ms = u64::from_be_bytes(date);
            let compteur = u16::from_be_bytes([octets[6] & 0x0F, octets[7]]);
            if (ms, compteur) > (max_ms, max_compteur) {
                max_ms = ms;
                max_compteur = compteur;
            }
        }
        self.uuid_derniere_ms = max_ms;
        self.uuid_counter = max_compteur.saturating_add(1);
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
    ///
    /// La migration est tout-ou-rien : si une étape échoue, le journal
    /// reste inchangé (ADR-0016 A6).
    pub fn migrate_events(&mut self) -> Result<(), DocError> {
        let mut migrated = Vec::with_capacity(self.events.len());
        for event in &self.events {
            migrated.push(self.migrations.migrate(event)?);
        }
        self.events = migrated;
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

    /// Sérialise le préfixe appliqué du journal en conteneur ZIP (en
    /// mémoire), **sans muter le document** : les événements défaits
    /// ne figurent pas dans le fichier — qui reste le journal linéaire
    /// visible (ADR-0008) — et la pile redo survit à l'enregistrement
    /// (ADR-0016 C3). Seuls les blobs référencés par le préfixe
    /// sérialisé accompagnent le fichier.
    pub fn to_bytes_prefix(&self, applied: usize) -> Result<Vec<u8>, DocError> {
        let applique = applied.min(self.events.len());
        let mut document = Document::new();
        document
            .events
            .extend(self.events[..applique].iter().cloned());
        let references: Vec<String> = document
            .events
            .iter()
            .filter_map(|event| event.data.get("blob").and_then(|donnee| donnee.as_str()))
            .map(|nom| nom.to_string())
            .collect();
        for nom in references {
            if let Some(octets) = self.blobs.get(&nom) {
                document.blobs.insert(nom, octets.clone());
            }
        }
        if let Some(snapshot) = &self.snapshot {
            let dernier_seq = document.events.last().map(|event| event.seq).unwrap_or(0);
            if snapshot.at_seq <= dernier_seq {
                document.snapshot = Some(snapshot.clone());
            }
        }
        container::write(&document)
    }

    /// Charge un document depuis un conteneur ZIP, sans migrations
    /// déclarées (les événements restent à leur version d'origine).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
        Self::from_bytes_with(bytes, MigrationRegistry::new())
    }

    /// Charge un document en appliquant les migrations du registre donné,
    /// avec le plafond global par défaut pour la taille cumulée des objets
    /// binaires ([`MAX_TOTAL_BLOB_BYTES`]).
    pub fn from_bytes_with(bytes: &[u8], migrations: MigrationRegistry) -> Result<Self, DocError> {
        Self::from_bytes_with_budget(bytes, migrations, MAX_TOTAL_BLOB_BYTES)
    }

    /// Charge un document en appliquant les migrations du registre donné,
    /// avec un plafond global réglable pour la taille cumulée des objets
    /// binaires (ADR-0016 A4) : au-delà, le conteneur est refusé par
    /// `OF-DOC-0014` au lieu d'épuiser la mémoire (bombe ZIP cumulative).
    pub fn from_bytes_with_budget(
        bytes: &[u8],
        migrations: MigrationRegistry,
        max_total_blob_bytes: u64,
    ) -> Result<Self, DocError> {
        container::read(bytes, migrations, max_total_blob_bytes)
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
        ecrire_atomiquement(path, &self.to_bytes()?)
    }

    /// Enregistre le préfixe appliqué du journal dans un fichier, de
    /// manière atomique (feature `native`), **sans muter le
    /// document** : les événements défaits ne figurent pas dans le
    /// fichier — qui reste le journal linéaire visible (ADR-0008) —
    /// et la pile redo survit à l'enregistrement (ADR-0016 C3).
    #[cfg(feature = "native")]
    pub fn save_to_path_prefix(
        &self,
        path: &std::path::Path,
        applied: usize,
    ) -> Result<(), DocError> {
        ecrire_atomiquement(path, &self.to_bytes_prefix(applied)?)
    }

    /// Charge un document depuis un fichier (feature `native`),
    /// sous le budget d'octets [`MAX_FICHIER_OCTETS`] : un fichier
    /// démesuré est refusé avant lecture (audit n°3 A3-07).
    #[cfg(feature = "native")]
    pub fn open_from_path(path: &std::path::Path) -> Result<Self, DocError> {
        Self::open_from_path_with_budget(path, MigrationRegistry::new(), MAX_FICHIER_OCTETS)
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
        Self::open_from_path_with_budget(path, migrations, MAX_FICHIER_OCTETS)
    }

    /// Charge un document depuis un fichier sous un budget d'octets
    /// (feature `native`), refusé AVANT lecture si le fichier le
    /// dépasse (audit n°3 A3-07, ADR-0009) : la taille mesurée sur
    /// les métadonnées suffit — rien n'est lu ni alloué, le refus
    /// porte le code OF-DOC-0016 du registre central.
    #[cfg(feature = "native")]
    pub fn open_from_path_with_budget(
        path: &std::path::Path,
        migrations: MigrationRegistry,
        max: u64,
    ) -> Result<Self, DocError> {
        let annonce = std::fs::metadata(path)?.len();
        if annonce > max {
            return Err(DocError::FileTooLarge {
                found: annonce,
                max,
            });
        }
        Self::from_bytes_with(&std::fs::read(path)?, migrations)
    }
}

/// Écrit les octets au chemin donné, de manière atomique : temporaire
/// synchronisé, sauvegarde `.bak`, puis remplacement (ADR-0016 A2) ;
/// au moindre échec, seul le temporaire incomplet est nettoyé.
#[cfg(feature = "native")]
fn ecrire_atomiquement(path: &std::path::Path, bytes: &[u8]) -> Result<(), DocError> {
    let mut tmp = std::ffi::OsString::from(path.as_os_str());
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    let mut backup = std::ffi::OsString::from(path.as_os_str());
    backup.push(".bak");
    let backup = std::path::PathBuf::from(backup);
    if let Err(erreur) = remplacer_sur_disque(path, &tmp, &backup, bytes) {
        // Aucune étape n'a déplacé le fichier courant : seul le
        // temporaire incomplet peut rester, il est nettoyé.
        let _ = std::fs::remove_file(&tmp);
        return Err(erreur.into());
    }
    Ok(())
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
    use super::{Document, FixedClock, NewEvent, iso8601_from_unix};

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

    /// to_bytes_prefix sérialise le journal appliqué sans muter le
    /// document : les événements défaits et leurs blobs ne figurent
    /// pas dans le fichier, et le document en mémoire garde sa pile
    /// redo (ADR-0016 C3).
    #[test]
    fn to_bytes_prefix_relit_le_journal_applique() {
        let mut document = Document::new();
        document.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        document.append(NewEvent::new(
            "t",
            1,
            "tests",
            serde_json::json!({"blob": "geometry/traces"}),
        ));
        document
            .set_blob("geometry/traces", vec![1, 2, 3])
            .expect("blob valide");
        let octets = document.to_bytes_prefix(1).expect("conteneur du préfixe");
        let relu = Document::from_bytes(&octets).expect("relecture");
        assert_eq!(relu.events().len(), 1, "seul le journal appliqué est écrit");
        assert!(
            relu.blob_names().is_empty(),
            "le blob de l'événement défait n'embarque pas"
        );
        assert_eq!(document.events().len(), 2, "le document n'est pas muté");
        assert_eq!(
            document.blob_names().len(),
            1,
            "le blob survit en mémoire pour le redo"
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

    /// Le chargement depuis un fichier refuse AVANT lecture un
    /// fichier plus gros que le budget d'octets (audit n°3 A3-07,
    /// ADR-0009) : rien n'est lu en mémoire, le refus porte le code
    /// OF-DOC-0016 du registre central.
    #[cfg(feature = "native")]
    #[test]
    fn open_from_path_refuse_fichier_hors_budget() {
        let chemin = std::env::temp_dir().join("omnifab-audit3-budget.ofdoc");
        std::fs::write(&chemin, [0u8; 32]).expect("écriture du fichier test");
        let erreur = match Document::open_from_path_with_budget(
            &chemin,
            super::MigrationRegistry::new(),
            16,
        ) {
            Err(erreur) => erreur,
            Ok(_) => panic!("un fichier de 32 octets doit être refusé au budget de 16"),
        };
        assert_eq!(
            erreur.code(),
            "OF-DOC-0016",
            "le refus doit porter le code du registre"
        );
        let _ = std::fs::remove_file(&chemin);
    }

    /// A3-11 : après rechargement sous la même horloge figée, un
    /// événement ajouté ne recycle pas l'identifiant d'un événement
    /// chargé — le compteur de monotonie reprend au-delà du plus
    /// grand UUIDv7 du journal.
    #[test]
    fn uuidv7_ne_recycle_pas_un_id_au_rechargement() {
        let mut document = Document::new().with_clock(Box::new(FixedClock { unix: 1000 }));
        document.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        let ancien = document.events()[0].id.clone().expect("id attribué");
        let octets = document.to_bytes().expect("conteneur écrit");
        let mut relu = Document::from_bytes(&octets)
            .expect("relecture")
            .with_clock(Box::new(FixedClock { unix: 1000 }));
        relu.append(NewEvent::new("t", 1, "tests", serde_json::json!({})));
        let nouveau = relu.events()[1].id.clone().expect("id attribué");
        assert_ne!(
            nouveau, ancien,
            "un événement ajouté après rechargement sous la même horloge ne doit pas recycler un identifiant"
        );
    }

    /// A3-12 : une dernière ligne de journal non analysable n'est plus
    /// tolérée en silence — le journal fait foi, le refus est typé
    /// OF-DOC-0003 (l'écriture atomique et le CRC32 rendaient la
    /// tolérance inatteignable pour une vraie troncature).
    #[test]
    fn journal_corrompu_derniere_ligne_refuse() {
        let journal = "{\"seq\":1,\"type\":\"t\",\"schema_version\":1,\"timestamp\":\"1970-01-01T00:00:00Z\",\"actor\":\"tests\",\"data\":{}}\n{\"seq\": 2, ";
        let octets = conteneur_minimal(journal);
        let erreur = match Document::from_bytes(&octets) {
            Err(erreur) => erreur,
            Ok(_) => panic!(
                "un journal dont la dernière ligne est corrompue doit être refusé, pas toléré en silence"
            ),
        };
        assert_eq!(erreur.code(), "OF-DOC-0003", "le refus doit être typé");
    }

    /// A3-13 : une entrée répertoire `objects/` (ajoutée par zip -r ou
    /// 7-Zip) est ignorée au chargement — un projet recompressé à la
    /// main n'est plus refusé par OF-DOC-0013.
    #[test]
    fn entree_repertoire_objects_ignoree() {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file(super::MANIFEST_PATH, options)
            .expect("manifeste ouvert");
        writer
            .write_all(b"{\"format\":\"omnifab-document\",\"version\":1}")
            .expect("manifeste écrit");
        writer
            .start_file(super::JOURNAL_PATH, options)
            .expect("journal ouvert");
        writer.write_all(b"").expect("journal écrit");
        writer
            .add_directory("objects", options)
            .expect("répertoire enregistré");
        let octets = writer.finish().expect("archive").into_inner();
        let document = match Document::from_bytes(&octets) {
            Ok(document) => document,
            Err(erreur) => {
                panic!("une entrée répertoire objects/ doit être ignorée, pas refusée : {erreur}")
            }
        };
        assert!(
            document.blob_names().is_empty(),
            "aucun blob n'est chargé depuis une entrée répertoire"
        );
    }

    /// A3-18 : la traversée de répertoire se teste par segment —
    /// `a..b` est un nom légitime, `a/../b` une traversée refusée
    /// (OF-DOC-0013).
    #[test]
    fn nom_de_blob_traversee_par_segment() {
        let mut document = Document::new();
        document
            .set_blob("a..b", vec![1])
            .expect("a..b ne contient pas de segment .. : nom légitime");
        let erreur = match document.set_blob("a/../b", vec![1]) {
            Err(erreur) => erreur,
            Ok(()) => panic!("une traversée réelle (segment ..) doit être refusée"),
        };
        assert_eq!(erreur.code(), "OF-DOC-0013");
    }

    /// Construit un conteneur minimal (manifeste + journal donné) pour
    /// les tests de lecture du conteneur.
    fn conteneur_minimal(journal: &str) -> Vec<u8> {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file(super::MANIFEST_PATH, options)
            .expect("manifeste ouvert");
        writer
            .write_all(b"{\"format\":\"omnifab-document\",\"version\":1}")
            .expect("manifeste écrit");
        writer
            .start_file(super::JOURNAL_PATH, options)
            .expect("journal ouvert");
        writer.write_all(journal.as_bytes()).expect("journal écrit");
        writer.finish().expect("archive").into_inner()
    }
}
