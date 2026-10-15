//! Test bidirectionnel des codes d'erreur (ADR-0003) : chaque code
//! `OF-GCS-*` émis par forge-gcs figure dans le registre central
//! `docs/error-codes.md`, et le registre ne déclare aucun code que la
//! crate n'émette.

use forge_gcs::{Algorithme, GcsError};

/// Toutes les variantes de [`GcsError`], avec des champs de test.
fn toutes_les_variantes() -> Vec<GcsError> {
    vec![
        GcsError::IdInconnu {
            contexte: "test".to_string(),
        },
        GcsError::ResolutionEchouee {
            algorithme: Algorithme::DogLeg,
        },
        GcsError::NonReproductible {
            contexte: "test".to_string(),
        },
        GcsError::AbiInvalide {
            contexte: "test".to_string(),
        },
    ]
}

#[test]
fn tous_les_codes_sont_au_registre() {
    let registre = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/error-codes.md"
    ))
    .expect("registre docs/error-codes.md lisible");

    let codes: std::collections::BTreeSet<&str> = toutes_les_variantes()
        .iter()
        .map(|erreur| erreur.code())
        .collect();

    // Aucun doublon entre variantes.
    assert_eq!(
        codes.len(),
        toutes_les_variantes().len(),
        "des variantes partagent un code"
    );

    for code in &codes {
        assert!(
            registre.contains(code),
            "code {code} absent du registre docs/error-codes.md"
        );
    }

    // Réciproque : le registre ne déclare que des codes émis.
    let codes_registre: Vec<String> = registre
        .lines()
        .filter_map(|ligne| {
            ligne
                .split('|')
                .nth(1)
                .map(|c| c.trim().to_string())
                .filter(|c| c.starts_with("OF-GCS-"))
        })
        .collect();
    for code in codes_registre {
        assert!(
            codes.contains(code.as_str()),
            "le registre déclare {code} mais forge-gcs ne l'émet pas"
        );
    }
}
