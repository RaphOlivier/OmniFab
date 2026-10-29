//! ADR-0003 : chaque code de `ValuesError` est stable, unique, et
//! cohérent avec le registre central `docs/error-codes.md` — dans les
//! deux sens (chaque code du registre VAL est porté par une variante,
//! chaque variante porte un code du registre).

use forge_values::{Unite, ValuesError};

const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

/// Une instance de chaque variante, pour vérifier `code()`.
fn toutes_les_variantes() -> Vec<ValuesError> {
    vec![
        ValuesError::Cycle {
            path: "a → b → a".to_string(),
        },
        ValuesError::UnknownReference {
            node: "x".to_string(),
            name: "y".to_string(),
        },
        ValuesError::InvalidExpression {
            node: "x".to_string(),
            reason: "syntaxe".to_string(),
        },
        ValuesError::Evaluation {
            node: "x".to_string(),
            reason: "division par zéro".to_string(),
        },
        ValuesError::UnknownValue {
            name: "x".to_string(),
        },
        ValuesError::NonFiniteInput {
            name: "x".to_string(),
        },
        ValuesError::UniteIncoherente {
            node: "x".to_string(),
            unite: Unite::Degre,
            reference: "y".to_string(),
            unite_reference: Unite::Millimetre,
        },
        ValuesError::AliasReserve {
            name: "esquisse.x".to_string(),
        },
        ValuesError::AliasReference {
            name: "x".to_string(),
            references: "y".to_string(),
        },
        ValuesError::ProfondeurChaine {
            node: "x".to_string(),
            profondeur: 1027,
            max: 1024,
        },
    ]
}

/// Codes du domaine `VAL` présents dans le registre (lignes de table).
fn codes_du_registre() -> Vec<String> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| OF-VAL-"))
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
        assert!(porte, "code {code} du registre inconnu de ValuesError");
    }
}
