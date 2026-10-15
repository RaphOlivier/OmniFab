//! ADR-0003 : chaque code de `SessionError` est stable, unique, et
//! cohérent avec le registre central `docs/error-codes.md` — dans les
//! deux sens (chaque code du registre SES est porté par une variante,
//! chaque variante porte un code du registre).

use forge_session::SessionError;

const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

/// Une instance de chaque variante, pour vérifier `code()`.
fn toutes_les_variantes() -> Vec<SessionError> {
    vec![
        SessionError::CommandRejected {
            reason: "vide".to_string(),
        },
        SessionError::ProjectionFailed {
            projection: "values".to_string(),
            message: "OF-VAL-0002".to_string(),
        },
        SessionError::UndoEmpty,
        SessionError::RedoEmpty,
        SessionError::InvalidEventData {
            event_type: "value.input_set".to_string(),
            reason: "champ absent".to_string(),
        },
        SessionError::BlobRejected {
            reason: "OF-DOC-0013".to_string(),
        },
    ]
}

/// Codes du domaine `SES` présents dans le registre (lignes de table).
fn codes_du_registre() -> Vec<String> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| OF-SES-"))
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
        assert!(porte, "code {code} du registre inconnu de SessionError");
    }
}
