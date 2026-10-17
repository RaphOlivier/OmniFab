//! Lecture/écriture du conteneur ZIP (ADR-0000 verrou nº1, ADR-0001).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};

use crate::error::DocError;
use crate::events::Event;
use crate::migration::MigrationRegistry;
use crate::snapshot::Snapshot;
use crate::{
    BLOBS_PREFIX, CONTAINER_FORMAT, CONTAINER_VERSION, Document, JOURNAL_PATH, MANIFEST_PATH,
    SNAPSHOT_PATH, SNAPSHOT_VERSION,
};

/// Manifeste du conteneur (manifeste JSON versionné).
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

/// Nombre maximal d'entrées dans l'archive (garde-fou « bombe ZIP »).
pub(crate) const MAX_ARCHIVE_ENTRIES: usize = 10_000;
/// Taille maximale du manifeste, en octets.
pub(crate) const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
/// Taille maximale du journal d'événements, en octets.
pub(crate) const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
/// Taille maximale du snapshot, en octets.
pub(crate) const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
/// Taille maximale d'un objet binaire, en octets.
pub(crate) const MAX_BLOB_BYTES: u64 = 1024 * 1024 * 1024;
/// Taille cumulée maximale des objets binaires au chargement, en
/// octets (défaut réglable via `Document::from_bytes_with_budget`,
/// ADR-0016 A4 — garde-fou « bombe ZIP » cumulative).
pub(crate) const MAX_TOTAL_BLOB_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Valide un nom de blob : ni vide, ni traversée de répertoire (`..`),
/// ni chemin absolu, ni séparateur Windows, ni octet nul (OF-DOC-0013).
pub(crate) fn validate_blob_name(name: &str) -> Result<(), DocError> {
    if name.is_empty()
        || name.contains("..")
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains('\u{0}')
    {
        return Err(DocError::InvalidBlobName {
            name: name.to_string(),
        });
    }
    Ok(())
}

/// Lit une entrée d'archive en bornant sa taille : une entrée qui
/// dépasse `limit` est une erreur OF-DOC-0011, sans jamais charger le
/// dépassement en mémoire (garde-fou « bombe ZIP »).
fn read_bounded(
    file: &mut zip::read::ZipFile,
    entry: &str,
    limit: u64,
) -> Result<Vec<u8>, DocError> {
    let mut data = Vec::new();
    file.by_ref().take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(DocError::EntryTooLarge {
            entry: entry.to_string(),
            found: data.len() as u64,
            max: limit,
        });
    }
    Ok(data)
}

/// Condensat SHA-256 du journal jusqu'à `at_seq`, préfixé `sha256:`
/// (schéma B). Couvre les lignes sérialisées des événements de séquence
/// inférieure ou égale à `at_seq`, chacune terminée par un saut de ligne.
pub(crate) fn journal_digest(events: &[Event], at_seq: u64) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for event in events.iter().take_while(|event| event.seq <= at_seq) {
        let line = serde_json::to_string(event).unwrap_or_default();
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let mut hex = String::from("sha256:");
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Comme [`read_bounded`], puis décode en UTF-8.
fn bounded_string(
    file: &mut zip::read::ZipFile,
    entry: &str,
    limit: u64,
) -> Result<String, DocError> {
    let data = read_bounded(file, entry, limit)?;
    String::from_utf8(data)
        .map_err(|error| DocError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error)))
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

pub(crate) fn read(
    bytes: &[u8],
    migrations: MigrationRegistry,
    max_total_blob_bytes: u64,
) -> Result<Document, DocError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(DocError::TooManyEntries {
            found: archive.len(),
            max: MAX_ARCHIVE_ENTRIES,
        });
    }

    // La crate zip indexe les entrées du répertoire central par nom :
    // deux enregistrements de même nom sont fusionnés en silence avant
    // toute lecture. Le répertoire central est parcouru à la main pour
    // refuser l'archive dupliquée (OF-DOC-0015).
    verifie_doublons_centrale(bytes)?;

    // Manifeste : format et version du conteneur.
    let manifest: Manifest = {
        let mut file = archive
            .by_name(MANIFEST_PATH)
            .map_err(|_| DocError::MissingEntry {
                path: MANIFEST_PATH.to_string(),
            })?;
        let content = bounded_string(&mut file, MANIFEST_PATH, MAX_MANIFEST_BYTES)?;
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
        bounded_string(&mut file, JOURNAL_PATH, MAX_JOURNAL_BYTES)?
    };
    let mut events: Vec<Event> = Vec::new();
    // `split('\n')` garde une dernière entrée vide après le saut de
    // ligne final : une ligne corrompue qui n'est pas la toute dernière
    // entrée reste une erreur, seule une fin de fichier tronquée est
    // tolérée.
    let total_lines = journal.split('\n').count();
    for (index, line) in journal.split('\n').enumerate() {
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

    // Objets binaires (blobs) : chaque nom relu passe par la validation
    // de `set_blob` (une archive forgée ne contourne plus l'invariant,
    // ADR-0016 A4) et le cumul des blobs est borné par un plafond
    // global réglable (OF-DOC-0014) — le dépassement est détecté après
    // lecture de l'entrée fautive, au plus `MAX_BLOB_BYTES` au-delà du
    // plafond.
    let mut names: Vec<String> = Vec::new();
    for index in 0..archive.len() {
        names.push(archive.by_index(index)?.name().to_string());
    }
    let mut blobs = BTreeMap::new();
    let mut total_blob_bytes: u64 = 0;
    for name in &names {
        if let Some(blob_name) = name.strip_prefix(BLOBS_PREFIX) {
            validate_blob_name(blob_name)?;
            let mut file = archive.by_name(name)?;
            let data = read_bounded(&mut file, name, MAX_BLOB_BYTES)?;
            total_blob_bytes = total_blob_bytes.saturating_add(data.len() as u64);
            if total_blob_bytes > max_total_blob_bytes {
                return Err(DocError::TotalBlobsTooLarge {
                    found: total_blob_bytes,
                    max: max_total_blob_bytes,
                });
            }
            blobs.insert(blob_name.to_string(), data);
        }
    }

    // Snapshot : non autoritaire — ignoré silencieusement si incompatible.
    let snapshot = if names.iter().any(|n| n == SNAPSHOT_PATH) {
        let mut file = archive.by_name(SNAPSHOT_PATH)?;
        let content = bounded_string(&mut file, SNAPSHOT_PATH, MAX_SNAPSHOT_BYTES)?;
        match serde_json::from_str::<Snapshot>(&content) {
            // Schéma B : le snapshot n'est accepté que si son condensat
            // correspond au journal chargé — sinon ignoré, journal fait foi.
            Ok(snapshot) if snapshot.format_version == SNAPSHOT_VERSION => {
                let digest = journal_digest(&events, snapshot.at_seq);
                (digest == snapshot.journal_digest).then_some(snapshot)
            }
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
        clock: Box::new(crate::SystemClock),
    })
}
/// Cherche la fin du répertoire central (`PK\x05\x06`) en partant de la
/// fin de l'archive : la longueur de commentaire annoncée doit joindre
/// exactement le bout du fichier.
fn cherche_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let borne = bytes.len().saturating_sub(22 + 65_535);
    for position in (borne..=bytes.len() - 22).rev() {
        if &bytes[position..position + 4] == b"PK\x05\x06"
            && usize::from(u16_at(bytes, position + 20)) == bytes.len() - position - 22
        {
            return Some(position);
        }
    }
    None
}

/// Lit un `u16` petit-boutiste à `position` (validité garantie par
/// l'appelant).
fn u16_at(bytes: &[u8], position: usize) -> u16 {
    u16::from_le_bytes([bytes[position], bytes[position + 1]])
}

/// Lit un `u32` petit-boutiste à `position` (validité garantie par
/// l'appelant).
fn u32_at(bytes: &[u8], position: usize) -> u32 {
    u32::from_le_bytes([
        bytes[position],
        bytes[position + 1],
        bytes[position + 2],
        bytes[position + 3],
    ])
}

/// Lit un `u64` petit-boutiste à `position` (validité garantie par
/// l'appelant).
fn u64_at(bytes: &[u8], position: usize) -> u64 {
    u64::from_le_bytes([
        bytes[position],
        bytes[position + 1],
        bytes[position + 2],
        bytes[position + 3],
        bytes[position + 4],
        bytes[position + 5],
        bytes[position + 6],
        bytes[position + 7],
    ])
}

/// Lit le nombre d'entrées et la position du répertoire central dans
/// l'EOCD ZIP64, via le localisateur placé immédiatement avant l'EOCD
/// ordinaire. `None` si la structure est absente ou incohérente : le
/// contrôle des doublons est alors ignoré.
fn lis_zip64(bytes: &[u8], eocd: usize) -> Option<(u64, u64)> {
    let localisateur = eocd.checked_sub(20)?;
    if &bytes[localisateur..localisateur + 4] != b"PK\x06\x07" {
        return None;
    }
    let position = usize::try_from(u64_at(bytes, localisateur + 8)).ok()?;
    if position.checked_add(56)? > bytes.len() || &bytes[position..position + 4] != b"PK\x06\x06" {
        return None;
    }
    Some((u64_at(bytes, position + 32), u64_at(bytes, position + 48)))
}

/// Refuse les entrées en doublon (OF-DOC-0015) : la crate zip indexe
/// les fichiers par nom — deux enregistrements de même nom seraient
/// fusionnés en silence avant toute lecture. Le répertoire central est
/// donc parcouru à la main, de façon bornée. Parcours défensif : une
/// structure non reconnue (archive préfixée, EOCD ZIP64 absente,
/// longueurs incohérentes) n'invalide pas l'archive, la lecture reste
/// gouvernée par la crate zip.
fn verifie_doublons_centrale(bytes: &[u8]) -> Result<(), DocError> {
    let Some(eocd) = cherche_eocd(bytes) else {
        return Ok(());
    };
    let entrees_eocd = u16_at(bytes, eocd + 10);
    let debut_eocd = u32_at(bytes, eocd + 16);
    let (entrees, debut_centrale) = if entrees_eocd == 0xFFFF || debut_eocd == 0xFFFF_FFFF {
        let Some(valeurs) = lis_zip64(bytes, eocd) else {
            return Ok(());
        };
        valeurs
    } else {
        (u64::from(entrees_eocd), u64::from(debut_eocd))
    };
    let Ok(mut position) = usize::try_from(debut_centrale) else {
        return Ok(());
    };
    let mut vus = BTreeSet::new();
    for _ in 0..entrees {
        // Enregistrement du répertoire central : signature, 42 octets
        // d'en-tête fixe, nom, puis champs « extra » et commentaire de
        // longueurs annoncées.
        let Some(fin_entete) = position.checked_add(46) else {
            return Ok(());
        };
        if fin_entete > bytes.len() || &bytes[position..position + 4] != b"PK\x01\x02" {
            return Ok(());
        }
        let longueur_nom = usize::from(u16_at(bytes, position + 28));
        let longueur_extra = usize::from(u16_at(bytes, position + 30));
        let longueur_commentaire = usize::from(u16_at(bytes, position + 32));
        let Some(fin_nom) = fin_entete.checked_add(longueur_nom) else {
            return Ok(());
        };
        if fin_nom > bytes.len() {
            return Ok(());
        }
        let nom = bytes[fin_entete..fin_nom].to_vec();
        if vus.contains(&nom) {
            return Err(DocError::DuplicateEntry {
                entry: String::from_utf8_lossy(&nom).into_owned(),
            });
        }
        vus.insert(nom);
        let Some(apres_extra) = fin_nom.checked_add(longueur_extra) else {
            return Ok(());
        };
        let Some(prochaine) = apres_extra.checked_add(longueur_commentaire) else {
            return Ok(());
        };
        position = prochaine;
    }
    Ok(())
}
