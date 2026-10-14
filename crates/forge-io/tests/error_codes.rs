//! ADR-0003 : chaque code de `IoError` est stable, unique, et cohérent
//! avec le registre central `docs/error-codes.md` — dans les deux sens.

use forge_geom::GeomError;
use forge_io::IoError;

const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

/// Une instance de chaque variante, pour vérifier `code()`.
fn toutes_les_variantes() -> Vec<IoError> {
    let geometrie = GeomError::NonFinite { axis: "x" };
    vec![
        IoError::TooLarge { found: 1, max: 0 },
        IoError::InvalidBinary {
            reason: "test".to_string(),
        },
        IoError::TooManyTriangles { found: 3, max: 1 },
        IoError::InvalidAscii {
            line: 1,
            reason: "test".to_string(),
        },
        IoError::UnknownFormat,
        IoError::InvalidGeometry(geometrie),
    ]
}

/// Codes du domaine `IO` présents dans le registre (lignes de table).
fn codes_du_registre() -> Vec<String> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| OF-IO-"))
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
        assert!(porte, "code {code} du registre inconnu de IoError");
    }
}
