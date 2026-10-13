//! Lecture/écriture du conteneur ZIP (ADR-0000 verrou nº1, ADR-0001).

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};

use crate::error::DocError;
use crate::events::Event;
use crate::migration::MigrationRegistry;
use crate::snapshot::Snapshot;
use crate::{
    Document, BLOBS_PREFIX, CONTAINER_FORMAT, CONTAINER_VERSION, JOURNAL_PATH, MANIFEST_PATH,
    SNAPSHOT_PATH, SNAPSHOT_VERSION,
};

/// Manifeste du conteneur (manifeste JSON versionné).
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

pub(crate) fn write(doc: &Document) -> Result<Vec<u8>, DocError> {
    let manifest = Manifest {
        format: CONTAINER_FORMAT.to_string(),
        version: CONTAINER_VERSION,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;

    let mut journal = String::new();
    for event in &doc.events {
        journal.push_str(&serde_json::to_string(event)?);
        journal.push('\n');
    }

    let snapshot_json = doc
        .snapshot
        .as_ref()
        .map(serde_json::to_string_pretty)
        .transpose()?;

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file(MANIFEST_PATH, options)?;
    writer.write_all(manifest_json.as_bytes())?;
    writer.start_file(JOURNAL_PATH, options)?;
    writer.write_all(journal.as_bytes())?;
    if let Some(snapshot) = snapshot_json {
        writer.start_file(SNAPSHOT_PATH, options)?;
        writer.write_all(snapshot.as_bytes())?;
    }
    for (name, data) in &doc.blobs {
        writer.start_file(format!("{BLOBS_PREFIX}{name}"), options)?;
        writer.write_all(data)?;
    }
    let cursor = writer.finish()?;
    Ok(cursor.into_inner())
}

pub(crate) fn read(bytes: &[u8], migrations: MigrationRegistry) -> Result<Document, DocError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))?;

    // Manifeste : format et version du conteneur.
    let manifest: Manifest = {
        let mut file = archive
            .by_name(MANIFEST_PATH)
            .map_err(|_| DocError::MissingEntry {
                path: MANIFEST_PATH.to_string(),
            })?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };
    if manifest.format != CONTAINER_FORMAT {
        return Err(DocError::UnknownContainerFormat {
            format: manifest.format,
        });
    }
    if manifest.version > CONTAINER_VERSION {
        return Err(DocError::UnsupportedContainerVersion {
            found: manifest.version,
            supported: CONTAINER_VERSION,
        });
    }

    // Journal : JSON Lines, séquence strictement croissante, migrations.
    let journal = {
        let mut file = archive
            .by_name(JOURNAL_PATH)
            .map_err(|_| DocError::MissingEntry {
                path: JOURNAL_PATH.to_string(),
            })?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        content
    };
    let mut events: Vec<Event> = Vec::new();
    let total_lines = journal.lines().count();
    for (index, line) in journal.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut event: Event = match serde_json::from_str(line) {
            Ok(event) => event,
            Err(error) => {
                // Une coupure pendant l'écriture peut tronquer la toute
                // dernière ligne du journal : elle est tolérée (et
                // signalée) car le fichier `.bak` garde l'archive intacte.
                if index + 1 == total_lines {
                    log::warn!("derniere ligne du journal tronquee, ignoree : {error}");
                    break;
                }
                return Err(DocError::CorruptJournal {
                    line: index + 1,
                    reason: error.to_string(),
                });
            }
        };
        let expected = events.len() as u64 + 1;
        if event.seq != expected {
            return Err(DocError::BadSequence {
                expected,
                found: event.seq,
            });
        }
        event = migrations.migrate(&event)?;
        events.push(event);
    }

    // Objets binaires (blobs).
    let names: Vec<String> = archive.file_names().map(str::to_string).collect();
    let mut blobs = BTreeMap::new();
    for name in &names {
        if let Some(blob_name) = name.strip_prefix(BLOBS_PREFIX) {
            let mut file = archive.by_name(name)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            blobs.insert(blob_name.to_string(), data);
        }
    }

    // Snapshot : non autoritaire — ignoré silencieusement si incompatible.
    let snapshot = if names.iter().any(|n| n == SNAPSHOT_PATH) {
        let mut file = archive.by_name(SNAPSHOT_PATH)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        match serde_json::from_str::<Snapshot>(&content) {
            Ok(snapshot) if snapshot.format_version == SNAPSHOT_VERSION => Some(snapshot),
            _ => None,
        }
    } else {
        None
    };

    Ok(Document {
        events,
        blobs,
        snapshot,
        migrations,
    })
}
