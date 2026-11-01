//! Rejet des résultats non finis (revue point 5) : ni NaN ni un infini
//! ne doit pouvoir entrer dans le journal via une valeur calculée.

// Convention expect v2 (2026-11-01, lot E) : échafaudage de test —
// `allow-expect-in-tests` (clippy.toml) ne couvre que les fonctions
// `#[test]` et les modules `#[cfg(test)]`, pas les helpers de ce
// fichier d'intégration : expect admis dans tout le fichier.
#![allow(clippy::expect_used)]

use forge_values::{ValueGraph, ValuesError};

/// Construit un graphe minimal et évalue l'expression « x ».
fn evalue(a: f64, expression: &str) -> Result<f64, ValuesError> {
    let mut graph = ValueGraph::new();
    graph.set_input("a", a).expect("valeur finie");
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
    graph.set_input("a", 1.0).expect("valeur finie");
    let erreur = graph.set_expression("x", &"9".repeat(400)).unwrap_err();
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

/// Une valeur d'entrée non finie est refusée à l'écriture : ni NaN ni
/// un infini ne peut entrer dans le graphe par set_input — l'invariant
/// tient au niveau du graphe, pas seulement de la commande
/// (ADR-0016 A8).
#[test]
fn entree_non_finie_refusee_a_l_ecriture() {
    let mut graph = ValueGraph::new();
    for valeur in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let erreur = graph.set_input("a", valeur).unwrap_err();
        assert!(
            erreur.to_string().contains("OF-VAL-0006"),
            "un non fini doit être refusé, obtenu {erreur}"
        );
    }
    assert!(
        graph.value("a").is_err(),
        "aucun nœud créé par une écriture refusée"
    );
    graph.set_input("a", 1.0).expect("valeur finie acceptée");
}
