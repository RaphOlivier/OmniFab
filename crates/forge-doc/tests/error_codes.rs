//! ADR-0003 : la CI refuse un code d'erreur non documenté dans le registre
//! central `docs/error-codes.md`.

const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

#[test]
fn tous_les_codes_sont_dans_le_registre() {
    for code in [
        "OF-DOC-0001",
        "OF-DOC-0002",
        "OF-DOC-0003",
        "OF-DOC-0004",
        "OF-DOC-0005",
        "OF-DOC-0006",
        "OF-DOC-0007",
        "OF-DOC-0008",
        "OF-DOC-0009",
        "OF-DOC-0010",
    ] {
        assert!(
            REGISTRY.contains(code),
            "code {code} absent du registre docs/error-codes.md"
        );
    }
}
