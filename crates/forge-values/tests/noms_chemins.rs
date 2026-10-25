//! Noms en chemin (ADR-0006) : identifiants à segments séparés
//! par des points — audit n°2, point N1. Un nom accepté à la
//! création (`piece.largeur`, cote système `esquisse.<alias>`) doit
//! être référençable dans une expression : le lexeur lit le point
//! comme séparateur de segments, jamais comme un début de nombre.

use forge_values::{Unite, ValueGraph};

#[test]
fn un_nom_en_chemin_est_referencable_dans_une_formule() {
    let mut g = ValueGraph::new();
    g.set_input("piece.largeur", 50.0).expect("entrée posée");
    g.set_expression("piece.perimetre", "piece.largeur * 2")
        .expect("formule posée");
    assert_eq!(g.value("piece.perimetre").expect("valeur"), 100.0);
}

#[test]
fn une_cote_systeme_est_lisible_par_l_utilisateur() {
    let mut g = ValueGraph::new();
    g.set_expression_systeme("esquisse.largeur", "80", Some(Unite::Millimetre))
        .expect("cote système posée");
    g.set_expression("caisse", "esquisse.largeur * 2")
        .expect("formule utilisateur");
    assert_eq!(g.value("caisse").expect("valeur"), 160.0);
}

#[test]
fn un_segment_fautif_apres_un_point_est_refuse() {
    let mut g = ValueGraph::new();
    g.set_input("a", 1.0).expect("entrée posée");
    // « a.5 » : le point ouvre un segment, pas un nombre.
    let erreur = g.set_expression("x", "a.5").unwrap_err();
    assert_eq!(erreur.code(), "OF-VAL-0003");
    // « 1.5 » reste un nombre décimal.
    g.set_expression("y", "1.5 * a").expect("nombre décimal");
    assert_eq!(g.value("y").unwrap(), 1.5);
}

#[test]
fn la_substitution_respecte_les_noms_en_chemin() {
    use forge_values::remplacer_identifiants;
    // un segment à l'intérieur d'un nom en chemin n'est pas une
    // référence à l'alias court (audit n°2, N2)
    assert_eq!(
        remplacer_identifiants("piece.a + 1", "a", "b"),
        "piece.a + 1"
    );
    // les décimales d'un nombre ne sont pas un identifiant
    assert_eq!(remplacer_identifiants("a + 1.5", "a", "b"), "b + 1.5");
    assert_eq!(remplacer_identifiants("1.5 * a", "5", "b"), "1.5 * a");
    // un appel de fonction du même nom n'est pas une référence
    assert_eq!(
        remplacer_identifiants("min(a, 1)", "min", "b"),
        "min(a, 1)"
    );
    assert_eq!(remplacer_identifiants("min + 1", "min", "b"), "b + 1");
    // remplacer un nom en chemin entier, lui, réécrit
    assert_eq!(
        remplacer_identifiants("piece.a + 1", "piece.a", "piece.b"),
        "piece.b + 1"
    );
}

#[test]
fn une_expression_trop_imbriquee_est_refusee_sans_deborder() {
    let mut g = ValueGraph::new();
    g.set_input("a", 1.0).expect("entrée posée");
    // 100 000 parenthèses ouvrantes : le parseur refuse bien avant
    // de déborder la pile (audit n°2 — récursion bornée).
    let profonde = format!("{}a{}", "(".repeat(100_000), ")".repeat(100_000));
    let erreur = g.set_expression("x", &profonde).unwrap_err();
    assert_eq!(erreur.code(), "OF-VAL-0003");
    // une chaîne de négations déborde tout autant : même garde.
    let moine = format!("{}a", "-".repeat(100_000));
    let erreur = g.set_expression("y", &moine).unwrap_err();
    assert_eq!(erreur.code(), "OF-VAL-0003");
    // une imbrication raisonnable passe toujours.
    g.set_expression("z", "((a + 1) * 2) - -a").expect("expression bornée");
    assert_eq!(g.value("z").unwrap(), 5.0);
}
