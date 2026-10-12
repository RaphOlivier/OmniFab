//! ADR-0003 : la CI refuse un code d'erreur non documenté dans le registre
//! central `docs/error-codes.md`.

const REGISTRY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/error-codes.md"));

#[test]
fn tous_les_codes_sont_dans_le_registre() {
    for code in [
        "OF-VAL-0001",
        "OF-VAL-0002",
        "OF-VAL-0003",
        "OF-VAL-0004",
        "OF-VAL-0005",
    ] {
        assert!(
            REGISTRY.contains(code),
            "code {code} absent du registre docs/error-codes.md"
        );
    }
}
