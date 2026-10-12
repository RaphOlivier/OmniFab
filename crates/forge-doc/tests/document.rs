use std::io::Write;

use forge_doc::{iso8601_from_unix, DocError, Document, MigrationRegistry, NewEvent};
use serde_json::json;

const MANIFEST_V1: &str = r#"{"format":"omnifab-document","version":1}"#;

fn archive(files: &[(&str, String)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn evenement(seq: u64, version: u32) -> String {
    format!(
        r#"{{"seq":{seq},"type":"demo.test","schema_version":{version},"timestamp":"2026-10-12T10:00:00Z","actor":"test","data":{{}}}}"#
    )
}

#[test]
fn aller_retour_complet() {
    let mut doc = Document::new();
    doc.append(NewEvent::new(
        "demo.value.input_set",
        1,
        "test",
        json!({"name": "width_mm", "value": 800.0}),
    ));
    doc.append(NewEvent::new(
        "demo.value.expression_set",
        1,
        "test",
        json!({"name": "diagonal", "expression": "sqrt(width_mm * width_mm)"}),
    ));
    doc.set_blob("apercu.png", vec![1, 2, 3]);
    doc.set_snapshot(json!({"values": []}));

    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();

    assert_eq!(reloaded.events().len(), 2);
    assert_eq!(reloaded.events()[0], doc.events()[0]);
    assert_eq!(reloaded.events()[1], doc.events()[1]);
    assert_eq!(reloaded.blob("apercu.png"), Some(&[1u8, 2, 3][..]));
    assert!(reloaded.snapshot().is_some());
    // Horodatage ISO 8601 UTC, tronqué à la seconde.
    assert_eq!(reloaded.events()[0].timestamp.len(), 20);
}

#[test]
fn rejeu_dans_l_ordre() {
    let mut doc = Document::new();
    for i in 0..5 {
        doc.append(NewEvent::new(
            "demo.evenement",
            1,
            "test",
            json!({ "i": i }),
        ));
    }
    let mut seqs = Vec::new();
    doc.replay(|event| {
        seqs.push(event.seq);
        Ok::<(), String>(())
    })
    .unwrap();
    assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
}

#[test]
fn version_conteneur_non_supportee() {
    let bytes = archive(&[
        (
            "manifest.json",
            r#"{"format":"omnifab-document","version":99}"#.to_string(),
        ),
        ("events.jl", String::new()),
    ]);
    match Document::from_bytes(&bytes) {
        Err(DocError::UnsupportedContainerVersion {
            found: 99,
            supported: 1,
        }) => {}
        other => panic!("version non supportée attendue, obtenu {other:?}"),
    }
}

#[test]
fn format_conteneur_inconnu() {
    let bytes = archive(&[
        (
            "manifest.json",
            r#"{"format":"autre-chose","version":1}"#.to_string(),
        ),
        ("events.jl", String::new()),
    ]);
    match Document::from_bytes(&bytes) {
        Err(DocError::UnknownContainerFormat { .. }) => {}
        other => panic!("format inconnu attendu, obtenu {other:?}"),
    }
}

#[test]
fn manifeste_absent() {
    let bytes = archive(&[("events.jl", String::new())]);
    match Document::from_bytes(&bytes) {
        Err(DocError::MissingEntry { path }) => assert_eq!(path, "manifest.json"),
        other => panic!("entrée manquante attendue, obtenu {other:?}"),
    }
}

#[test]
fn journal_corrompu() {
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", "ceci n'est pas du json\n".to_string()),
    ]);
    match Document::from_bytes(&bytes) {
        Err(DocError::CorruptJournal { line: 1, .. }) => {}
        other => panic!("journal corrompu attendu, obtenu {other:?}"),
    }
}

#[test]
fn sequence_incoherente() {
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        (
            "events.jl",
            format!("{}\n{}\n", evenement(1, 1), evenement(5, 1)),
        ),
    ]);
    match Document::from_bytes(&bytes) {
        Err(DocError::BadSequence {
            expected: 2,
            found: 5,
        }) => {}
        other => panic!("séquence incohérente attendue, obtenu {other:?}"),
    }
}

#[test]
fn migration_appliquee_au_chargement() {
    fn v1_vers_v2(data: &serde_json::Value) -> Result<serde_json::Value, DocError> {
        match data {
            serde_json::Value::Object(map) => {
                let mut migrated = map.clone();
                migrated.insert("facteur".to_string(), json!(2.0));
                Ok(serde_json::Value::Object(migrated))
            }
            _ => Err(DocError::CorruptJournal {
                line: 0,
                reason: "données non objet".to_string(),
            }),
        }
    }
    let mut registry = MigrationRegistry::new();
    registry.declare("demo.test", 2);
    registry.register("demo.test", 1, v1_vers_v2);
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("{}\n", evenement(1, 1))),
    ]);
    let doc = Document::from_bytes_with(&bytes, registry).unwrap();
    assert_eq!(doc.events()[0].schema_version, 2);
    assert_eq!(doc.events()[0].data["facteur"], json!(2.0));
}

#[test]
fn migration_manquante() {
    let mut registry = MigrationRegistry::new();
    registry.declare("demo.test", 2);
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("{}\n", evenement(1, 1))),
    ]);
    match Document::from_bytes_with(&bytes, registry) {
        Err(DocError::MissingMigration { from: 1, to: 2, .. }) => {}
        other => panic!("migration manquante attendue, obtenu {other:?}"),
    }
}

#[test]
fn schema_trop_recent() {
    let mut registry = MigrationRegistry::new();
    registry.declare("demo.test", 1);
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("{}\n", evenement(1, 2))),
    ]);
    match Document::from_bytes_with(&bytes, registry) {
        Err(DocError::SchemaTooNew { version: 2, .. }) => {}
        other => panic!("schéma trop récent attendu, obtenu {other:?}"),
    }
}

#[test]
fn snapshot_incompatible_ignore() {
    // ADR-0001 : un snapshot non autoritaire, de version inconnue, est
    // ignoré sans erreur — le journal fait foi.
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("{line}\n", line = evenement(1, 1))),
        (
            "snapshot.json",
            r#"{"format_version":99,"data":{"obsolete":true}}"#.to_string(),
        ),
    ]);
    let doc = Document::from_bytes(&bytes).unwrap();
    assert!(doc.snapshot().is_none());
    assert_eq!(doc.events().len(), 1);
}

#[test]
fn blob_absent() {
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", String::new()),
    ]);
    let doc = Document::from_bytes(&bytes).unwrap();
    assert!(doc.blob("nimporte").is_none());
    assert!(doc.blob_names().is_empty());
}

#[test]
fn horodatage_iso8601() {
    assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
    assert_eq!(iso8601_from_unix(951782400), "2000-02-29T00:00:00Z");
}
