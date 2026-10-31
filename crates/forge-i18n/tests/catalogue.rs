//! Tests d'intégration du catalogue : arguments de mise en forme.

use forge_i18n::Argument;

/// A4-13c (audit n°4) : `usize::MAX` sature à `i64::MAX` — un `as i64`
/// silencieux wrapperait en négatif l'affichage d'un compteur.
#[test]
fn usize_extreme_sature_a_i64_max() {
    let Argument::Entier(valeur) = Argument::from(usize::MAX) else {
        panic!("usize::MAX doit être mis en forme comme un Entier");
    };
    assert_eq!(
        valeur,
        i64::MAX,
        "usize::MAX doit saturer à i64::MAX, pas wrapper en négatif"
    );
}
