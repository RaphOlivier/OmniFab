successfully downloaded text file (SHA: 40e7cf65a0f2411be0d2fc29a2cc46549d5758bf)//! ADR-0003 : chaque code de `DocError` est stable, unique, et cohérent
//! avec le registre central `docs/error-codes.md` — dans les deux sens.

use forge_doc::DocError;

const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

/// Une instance de chaque variante, pour vérifier `code()`.
fn toutes_les_variantes() -> Vec<DocError> {
    vec![
        DocError::UnsupportedContainerVersion {
            found: 99,
            supported: 1,
        },
        DocError::UnknownContainerFormat {
            format: "autre".to_string(),
        },
        DocError::CorruptJournal {
            line: 1,
            reason: "json invalide".to_string(),
        },
        DocError::BadSequence {
            expected: 1,
            found: 2,
        },
        DocError::SchemaTooNew {
            event_type: "demo.test".to_string(),
            version: 2,
        },
        DocError::MissingMigration {
            event_type: "demo.test".to_string(),
            from: 1,
            to: 2,
        },
        DocError::MissingEntry {
            path: "manifest.json".to_string(),
        },
        DocError::Archive(zip::result::ZipError::InvalidArchive("archive")),
        DocError::Serialization(serde_json::from_str::<i32>("invalide").unwrap_err()),
        DocError::Io(std::io::Error::other("test")),
        DocError::EntryTooLarge {
            entry: "objects/x".to_string(),
            found: 1,
            max: 0,
        },
        DocError::TooManyEntries { found: 2, max: 1 },
        DocError::InvalidBlobName {
            name: "..".to_string(),
        },
        DocError::TotalBlobsTooLarge { found: 1, max: 0 },
        DocError::DuplicateEntry {
            entry: "objects/x".to_string(),
        },
    ]
}

/// Codes du domaine `DOC` présents dans le registre (lignes de table).
fn codes_du_registre() -> Vec<String> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| OF-DOC-"))
        .map(|line| {
            line.trim_start_matches('|')
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

#[test]
fn codes_uniques() {
    let codes: Vec<&str> = toutes_les_variantes()
        .iter()
        .map(|erreur| erreur.code())
        .collect();
    let mut uniques = codes.clone();
    uniques.sort_unstable();
    uniques.dedup();
    assert_eq!(
        codes.len(),
        uniques.len(),
        "chaque variante doit porter un code distinct"
    );
}

#[test]
fn registre_coherent_dans_les_deux_sens() {
    let registre = codes_du_registre();
    let variantes = toutes_les_variantes();
    for erreur in &variantes {
        assert!(
            registre.contains(&erreur.code().to_string()),
            "code {} absent du registre docs/error-codes.md",
            erreur.code()
        );
    }
    for code in &registre {
        let porte = variantes.iter().any(|erreur| erreur.code() == code);
        assert!(porte, "code {code} du registre inconnu de DocError");
    }
}
