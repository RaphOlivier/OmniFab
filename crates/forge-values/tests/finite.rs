//! Rejet des résultats non finis (revue point 5) : ni NaN ni un infini
//! ne doit pouvoir entrer dans le journal via une valeur calculée.

use forge_values::{ValueGraph, ValuesError};

/// Construit un graphe minimal et évalue l'expression « x ».
fn evalue(a: f64, expression: &str) -> Result<f64, ValuesError> {
    let mut graph = ValueGraph::new();
    graph.set_input("a", a);
    graph.set_expression("x", expression)?;
    graph.value("x")
}

fn est_erreur_evaluation(erreur: &ValuesError) -> bool {
    erreur.to_string().contains("OF-VAL-0004")
}

#[test]
fn debordement_vers_infini_rejete() {
    let erreur = evalue(1.0e308, "a * a").unwrap_err();
    assert!(
        est_erreur_evaluation(&erreur),
        "un débordement vers l'infini doit être rejeté, obtenu {erreur}"
    );
}

#[test]
fn nan_rejete() {
    // infini - infini = NaN : le résultat non fini est rejeté à la lecture.
    let erreur = evalue(1.0e308, "a * a - a * a").unwrap_err();
    assert!(
        est_erreur_evaluation(&erreur),
        "un NaN doit être rejeté, obtenu {erreur}"
    );
}

#[test]
fn litteral_non_fini_rejete_a_la_syntaxe() {
    let mut graph = ValueGraph::new();
    graph.set_input("a", 1.0);
    let erreur = graph
        .set_expression("x", "99999999999999999999999999999999")
        .unwrap_err();
    assert!(
        erreur.to_string().contains("OF-VAL-0003"),
        "un littéral non fini doit être refusé au parse, obtenu {erreur}"
    );
}

#[test]
fn racine_carree_negative_rejetee() {
    let erreur = evalue(-1.0, "sqrt(a)").unwrap_err();
    assert!(
        est_erreur_evaluation(&erreur),
        "la racine carrée d'un négatif doit être rejetée, obtenu {erreur}"
    );
}