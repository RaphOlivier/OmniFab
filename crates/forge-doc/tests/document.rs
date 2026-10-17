use std::io::{Read, Write};

use forge_doc::{DocError, Document, FixedClock, MigrationRegistry, NewEvent, iso8601_from_unix};
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
    doc.set_blob("apercu.png", vec![1, 2, 3]).unwrap();
    doc.set_snapshot(json!({"values": []})).unwrap();

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

#[test]
fn horloge_fixe_deterministe() {
    // Une horloge figée rend le journal déterministe : l'horodatage ne
    // dépend plus de l'heure d'exécution.
    let mut doc = Document::new().with_clock(Box::new(FixedClock { unix: 951782400 }));
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    assert_eq!(doc.events()[0].timestamp, "2000-02-29T00:00:00Z");

    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.events()[0].timestamp, "2000-02-29T00:00:00Z");
}

#[test]
fn nom_de_blob_invalide_rejete() {
    // OF-DOC-0013 : noms capables de sortir du dossier `objects/`.
    let mut doc = Document::new();
    for nom in ["", "../escape", "/absolu", "sous\\windows", "nul\u{0}octet"] {
        match doc.set_blob(nom, vec![1]) {
            Err(DocError::InvalidBlobName { .. }) => {}
            other => panic!("nom « {nom} » devait être rejeté, obtenu {other:?}"),
        }
    }
    // Un nom sain reste accepté.
    doc.set_blob("apercu.png", vec![1, 2, 3]).unwrap();
    assert_eq!(doc.blob("apercu.png"), Some(&[1u8, 2, 3][..]));
}

#[test]
fn trop_d_entrees_rejetees() {
    // Garde-fou « bombe ZIP » (OF-DOC-0012) : au-delà de 10 000 entrées,
    // le conteneur est refusé avant même de lire le journal.
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(MANIFEST_V1.as_bytes()).unwrap();
    writer.start_file("events.jl", options).unwrap();
    let _ = writer.write(b"");
    for i in 0..10_001 {
        writer
            .start_file(format!("objects/blob{i}"), options)
            .unwrap();
        let _ = writer.write(b"x");
    }
    let bytes = writer.finish().unwrap().into_inner();
    match Document::from_bytes(&bytes) {
        Err(DocError::TooManyEntries { found, max }) => {
            assert_eq!(max, 10_000);
            assert!(found > max);
        }
        other => panic!("trop d'entrées attendu, obtenu {other:?}"),
    }
}

#[test]
fn transaction_propagee_et_persistee() {
    // Les événements d'un même groupe portent le même identifiant de
    // transaction, persisté et conservé à la relecture (schéma A validé).
    let txn = "demo-txn-1";
    let mut doc = Document::new();
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})).in_txn(txn));
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    assert_eq!(doc.events()[0].txn.as_deref(), Some(txn));
    assert_eq!(doc.events()[1].txn, None);

    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.events()[0].txn.as_deref(), Some(txn));
    assert_eq!(reloaded.events()[1].txn, None);
}

#[test]
fn identifiant_attribue_et_persiste() {
    // Chaque événement reçoit un identifiant unique (UUIDv7, ordonné dans
    // le temps), attribué par `append` et conservé à la relecture (schéma C
    // validé). Deux événements successifs ont des identifiants distincts.
    let mut doc = Document::new();
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    let premier = doc.events()[0].id.as_deref().expect("id attribué");
    let second = doc.events()[1].id.as_deref().expect("id attribué");
    assert_ne!(premier, second);
    assert_eq!(premier.len(), 36, "format UUID canonique");

    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.events()[0].id.as_deref(), Some(premier));
    assert_eq!(reloaded.events()[1].id.as_deref(), Some(second));
}

#[test]
fn snapshot_lie_au_journal_et_poursuit_le_rejeu() {
    // Schéma B : le snapshot porte le numéro de séquence atteint et un
    // condensat du journal ; le rejeu reprend après `at_seq`.
    let mut doc = Document::new();
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    doc.set_snapshot(json!({"values": []})).unwrap();
    let at_seq = doc.snapshot().unwrap().at_seq;
    assert_eq!(at_seq, 2);
    assert!(
        doc.snapshot()
            .unwrap()
            .journal_digest
            .starts_with("sha256:")
    );

    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert!(reloaded.snapshot().is_some());
    assert!(reloaded.events_after(at_seq).is_empty());

    // Un événement de plus : seule la part postérieure au snapshot est à
    // rejouer.
    doc.append(NewEvent::new("demo.test", 1, "test", json!({})));
    let bytes = doc.to_bytes().unwrap();
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.events_after(at_seq).len(), 1);
    assert!(
        reloaded.snapshot().is_some(),
        "le condensat couvre le journal jusqu'à at_seq, pas au-delà"
    );
}

#[test]
fn snapshot_invalide_si_journal_diverge() {
    // Un snapshot réinséré dans un conteneur au journal différent est
    // ignoré : le condensat ne correspond plus, le journal fait foi.
    let mut doc = Document::new();
    doc.append(NewEvent::new("demo.test", 1, "test", json!({"i": 1})));
    doc.set_snapshot(json!({"values": []})).unwrap();
    let bytes = doc.to_bytes().unwrap();
    let mut zip_source = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut snapshot_json = String::new();
    zip_source
        .by_name("snapshot.json")
        .unwrap()
        .read_to_string(&mut snapshot_json)
        .unwrap();

    let mut autre = Document::new();
    autre.append(NewEvent::new("demo.test", 1, "test", json!({"i": 999})));
    let autre_bytes = autre.to_bytes().unwrap();
    let mut zip_autre = zip::ZipArchive::new(std::io::Cursor::new(autre_bytes)).unwrap();
    let mut journal = String::new();
    zip_autre
        .by_name("events.jl")
        .unwrap()
        .read_to_string(&mut journal)
        .unwrap();

    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", journal),
        ("snapshot.json", snapshot_json),
    ]);
    let reloaded = Document::from_bytes(&bytes).unwrap();
    assert!(
        reloaded.snapshot().is_none(),
        "un snapshot au condensat divergent doit être ignoré"
    );
    assert_eq!(reloaded.events().len(), 1);
}

#[test]
fn derniere_ligne_tronquee_toleree() {
    // Une coupure pendant l'écriture peut tronquer la dernière ligne : elle
    // est tolérée (et signalée) ; les événements précédents sont conservés.
    let ligne = evenement(1, 1);
    let tronquee = &ligne[..ligne.len() - 10];
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("{ligne}\n{tronquee}")),
    ]);
    let doc = Document::from_bytes(&bytes).unwrap();
    assert_eq!(doc.events().len(), 1);

    // Une ligne corrompue qui n'est PAS la dernière reste une erreur.
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", format!("ceci n'est pas du json\n{ligne}\n")),
    ]);
    match Document::from_bytes(&bytes) {
        Err(DocError::CorruptJournal { line: 1, .. }) => {}
        other => panic!("journal corrompu attendu, obtenu {other:?}"),
    }
}

#[test]
fn blob_invalide_au_chargement_rejete() {
    // ADR-0016 A4 : une archive forgée ne contourne pas la validation
    // des noms de blobs (OF-DOC-0013) — le nom relu passe par la même
    // validation que `set_blob` (vide, traversée, séparateur Windows).
    for nom in ["objects/", "objects/../x", "objects/a\\b"] {
        let bytes = archive(&[
            ("manifest.json", MANIFEST_V1.to_string()),
            ("events.jl", String::new()),
            (nom, "x".to_string()),
        ]);
        let attendu = nom.strip_prefix("objects/").unwrap_or(nom);
        match Document::from_bytes(&bytes) {
            Err(DocError::InvalidBlobName { name }) => assert_eq!(name, attendu),
            other => panic!("blob « {nom} » devait être rejeté, obtenu {other:?}"),
        }
    }
}

#[test]
fn doublon_d_entree_rejete_au_chargement() {
    // ADR-0016 A4 : deux entrées de même nom seraient lues avec
    // écrasement silencieux — elles sont refusées (OF-DOC-0015).
    // `ZipWriter` refuse les doublons à l'écriture : l'archive
    // forgée est assemblée octet par octet (entrées stockées).
    let entrees: [(&str, &[u8]); 4] = [
        ("manifest.json", MANIFEST_V1.as_bytes()),
        ("events.jl", b""),
        ("objects/apercu", b"x"),
        ("objects/apercu", b"x"),
    ];
    let bytes = archive_forgee(&entrees);
    match Document::from_bytes(&bytes) {
        Err(DocError::DuplicateEntry { entry }) => assert_eq!(entry, "objects/apercu"),
        other => panic!("doublon attendu, obtenu {other:?}"),
    }
}

/// CRC-32 IEEE 802.3 (polygone réfléchi 0xEDB88320), la somme
/// de contrôle attendue par le lecteur ZIP pour chaque entrée.
fn crc32(donnees: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for octet in donnees {
        crc ^= u32::from(*octet);
        for _ in 0..8 {
            let masque = (crc & 1).wrapping_mul(0xEDB8_8320);
            crc = (crc >> 1) ^ masque;
        }
    }
    !crc
}

/// Assemble une archive ZIP « stockée » (sans compression) à
/// partir d'entrées nommées, y compris en doublon — ce que le
/// `ZipWriter` de la fonction `archive` refuse de produire.
fn archive_forgee(entrees: &[(&str, &[u8])]) -> Vec<u8> {
    let mut locales = Vec::new();
    let mut centrales = Vec::new();
    for (nom, contenu) in entrees {
        let debut = locales.len() as u32;
        let somme = crc32(contenu);
        let taille = contenu.len() as u32;
        // En-tête local : signature, version, drapeaux, méthode
        // « stockée », date 1980-01-01, somme de contrôle, tailles,
        // longueur du nom, puis les données.
        locales.extend_from_slice(b"PK\x03\x04");
        locales.extend_from_slice(&20u16.to_le_bytes());
        locales.extend_from_slice(&0u16.to_le_bytes());
        locales.extend_from_slice(&0u16.to_le_bytes());
        locales.extend_from_slice(&0u16.to_le_bytes());
        locales.extend_from_slice(&0x21u16.to_le_bytes());
        locales.extend_from_slice(&somme.to_le_bytes());
        locales.extend_from_slice(&taille.to_le_bytes());
        locales.extend_from_slice(&taille.to_le_bytes());
        locales.extend_from_slice(&(nom.len() as u16).to_le_bytes());
        locales.extend_from_slice(&0u16.to_le_bytes());
        locales.extend_from_slice(nom.as_bytes());
        locales.extend_from_slice(contenu);
        // Enregistrement du répertoire central : mêmes champs que
        // l'en-tête local, plus l'offset de l'entrée.
        centrales.extend_from_slice(b"PK\x01\x02");
        centrales.extend_from_slice(&20u16.to_le_bytes());
        centrales.extend_from_slice(&20u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0x21u16.to_le_bytes());
        centrales.extend_from_slice(&somme.to_le_bytes());
        centrales.extend_from_slice(&taille.to_le_bytes());
        centrales.extend_from_slice(&taille.to_le_bytes());
        centrales.extend_from_slice(&(nom.len() as u16).to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u16.to_le_bytes());
        centrales.extend_from_slice(&0u32.to_le_bytes());
        centrales.extend_from_slice(&debut.to_le_bytes());
        centrales.extend_from_slice(nom.as_bytes());
    }
    let mut sortie = locales;
    let debut_centrales = sortie.len() as u32;
    sortie.extend_from_slice(&centrales);
    // Fin du répertoire central : nombres d'entrées, taille et
    // position du répertoire central, longueur du commentaire.
    sortie.extend_from_slice(b"PK\x05\x06");
    sortie.extend_from_slice(&0u16.to_le_bytes());
    sortie.extend_from_slice(&0u16.to_le_bytes());
    sortie.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    sortie.extend_from_slice(&(entrees.len() as u16).to_le_bytes());
    sortie.extend_from_slice(&(centrales.len() as u32).to_le_bytes());
    sortie.extend_from_slice(&debut_centrales.to_le_bytes());
    sortie.extend_from_slice(&0u16.to_le_bytes());
    sortie
}

#[test]
fn budget_cumule_de_blobs_borne_au_chargement() {
    // ADR-0016 A4 : le plafond par blob ne suffit pas (bombe ZIP
    // cumulative) — le cumul des blobs est borné par un plafond global,
    // par défaut 2 Gio (OF-DOC-0014), réglable par l'appelant.
    let blob = "x".repeat(10);
    let bytes = archive(&[
        ("manifest.json", MANIFEST_V1.to_string()),
        ("events.jl", String::new()),
        ("objects/a", blob.clone()),
        ("objects/b", blob.clone()),
        ("objects/c", blob.clone()),
    ]);

    // Trois blobs de 10 octets : le plafond réglable à 20 octets est
    // dépassé au troisième (cumul 30).
    match Document::from_bytes_with_budget(&bytes, MigrationRegistry::new(), 20) {
        Err(DocError::TotalBlobsTooLarge { found, max }) => {
            assert_eq!(found, 30);
            assert_eq!(max, 20);
        }
        other => panic!("budget cumulé dépassé attendu, obtenu {other:?}"),
    }

    // Le plafond par défaut accepte le même conteneur.
    let doc = Document::from_bytes(&bytes).unwrap();
    assert_eq!(doc.blob_names().len(), 3);
}
