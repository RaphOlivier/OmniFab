//! Suppression d'alias et namespaces (ADR-0019 phase E) : préfixes
//! réservés, protection des nœuds système, cohérence des unités à la
//! définition, et les trois modes Simple / Cascade / Remplacer.

use forge_values::{ModeSuppression, Origine, ResultatSuppression, Unite, ValueGraph, ValuesError};

/// Pose une entrée utilisateur et panique si elle est refusée.
fn entre(g: &mut ValueGraph, nom: &str, valeur: f64, unite: Option<Unite>) {
    g.set_input_unite(nom, valeur, unite, Origine::Utilisateur)
        .expect("acceptée");
}

/// Pose une expression utilisateur et panique si elle est refusée.
fn formule(g: &mut ValueGraph, nom: &str, texte: &str) {
    g.set_expression(nom, texte).expect("acceptée");
}

/// Pose une expression système (cote d'esquisse) et panique sinon.
fn cote(g: &mut ValueGraph, nom: &str, texte: &str, unite: Option<Unite>) {
    g.set_expression_systeme(nom, texte, unite)
        .expect("acceptée");
}

/// Affirme qu'une écriture est refusée avec ce code d'erreur.
fn refuse(res: Result<(), ValuesError>, attendu: &str) {
    let err = res.unwrap_err();
    assert_eq!(err.code(), attendu);
}

/// Affirme qu'une suppression est refusée avec ce code d'erreur.
fn refuse_supprim(res: Result<ResultatSuppression, ValuesError>, attendu: &str) {
    let err = res.unwrap_err();
    assert_eq!(err.code(), attendu);
}

/// Affirme qu'une écriture est acceptée.
fn ok(res: Result<(), ValuesError>) {
    res.expect("acceptée");
}

/// Exécute une suppression et panique si elle est refusée.
fn supprime(g: &mut ValueGraph, nom: &str, mode: &ModeSuppression) -> ResultatSuppression {
    g.supprimer(nom, mode).expect("suppression acceptée")
}

#[test]
fn les_prefixes_reserves_sont_refuses() {
    let mut g = ValueGraph::new();
    for nom in ["esquisse.mien", "param.x", "doc.y"] {
        refuse(
            g.set_input_unite(nom, 1.0, None, Origine::Utilisateur),
            "OF-VAL-0008",
        );
    }
    // le système, lui, peut créer un nœud sous préfixe réservé
    ok(g.set_expression_systeme("esquisse.demi", "2 + 3", None));
}

#[test]
fn un_noeud_systeme_est_protege() {
    let mut g = ValueGraph::new();
    ok(g.set_expression_systeme("esquisse.demi", "5", Some(Unite::Millimetre)));
    refuse_supprim(
        g.supprimer("esquisse.demi", &ModeSuppression::Simple),
        "OF-VAL-0008",
    );
}

#[test]
fn le_systeme_ne_prend_pas_un_noeud_utilisateur() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    refuse(g.set_expression_systeme("a", "2", None), "OF-VAL-0008");
    // et réciproquement, l'utilisateur ne touche pas un nœud système
    cote(&mut g, "esquisse.c", "1", None);
    refuse(
        g.set_input_unite("esquisse.c", 2.0, None, Origine::Utilisateur),
        "OF-VAL-0008",
    );
}

#[test]
fn la_coherence_des_unites_verifiee_a_la_definition() {
    let mut g = ValueGraph::new();
    entre(&mut g, "largeur", 100.0, Some(Unite::Millimetre));
    refuse(
        g.set_expression_unite(
            "angle",
            "largeur / 2",
            Some(Unite::Degre),
            Origine::Utilisateur,
        ),
        "OF-VAL-0007",
    );
    ok(g.set_expression_unite("brute", "largeur / 2", None, Origine::Utilisateur));
    // une cote (mm) ne peut pas lire une valeur en degres
    entre(&mut g, "angle2", 30.0, Some(Unite::Degre));
    refuse(
        g.set_expression_systeme("esquisse.c", "angle2", Some(Unite::Millimetre)),
        "OF-VAL-0007",
    );
}

#[test]
fn le_mode_simple_refuse_un_alias_reference() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    formule(&mut g, "b", "a + 1");
    refuse_supprim(g.supprimer("a", &ModeSuppression::Simple), "OF-VAL-0009");
    assert!(g.noeud("a").is_some());
    // sans dépendant, la suppression simple passe
    ok(g.set_input("iso", 2.0));
    let res = supprime(&mut g, "iso", &ModeSuppression::Simple);
    assert_eq!(res.supprimes, vec!["iso".to_string()]);
}

#[test]
fn la_cascade_supprime_les_formules() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    formule(&mut g, "b", "a + 1");
    formule(&mut g, "c", "b + 1");
    formule(&mut g, "d", "c + 1");
    let res = supprime(&mut g, "a", &ModeSuppression::Cascade);
    assert_eq!(res.supprimes.len(), 4);
    assert!(res.supprimes.contains(&"b".to_string()));
    assert!(g.noeud("d").is_none());
    assert!(g.noeud("a").is_none());
}

#[test]
fn la_cascade_epargne_les_cotes_systeme() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    cote(&mut g, "esquisse.cote", "a * 2", Some(Unite::Millimetre));
    refuse_supprim(g.supprimer("a", &ModeSuppression::Cascade), "OF-VAL-0009");
    // en mode Remplacer, la cote est réécrite au lieu d'être refusée
    ok(g.set_input("e", 3.0));
    let mode = ModeSuppression::Remplacer {
        par: "e".to_string(),
    };
    let res = supprime(&mut g, "a", &mode);
    assert_eq!(res.modifies, vec!["esquisse.cote".to_string()]);
    assert!(g.noeud("a").is_none());
    let info = g.noeud("esquisse.cote").expect("cote presente");
    assert_eq!(info.expression, Some("e * 2".to_string()));
}

#[test]
fn le_remplacement_reecrit_les_formules() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 5.0));
    ok(g.set_input("e", 3.0));
    formule(&mut g, "b", "a + 1");
    let mode = ModeSuppression::Remplacer {
        par: "e".to_string(),
    };
    let res = supprime(&mut g, "a", &mode);
    assert_eq!(res.modifies, vec!["b".to_string()]);
    assert_eq!(res.supprimes, vec!["a".to_string()]);
    let info = g.noeud("b").expect("b present");
    assert_eq!(info.expression, Some("e + 1".to_string()));
    assert_eq!(g.value("b").unwrap(), 4.0);
}

#[test]
fn le_remplacement_refuse_cible_invalide() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    formule(&mut g, "b", "a + 1");
    // cible inconnue
    let inconnu = ModeSuppression::Remplacer {
        par: "zz".to_string(),
    };
    refuse_supprim(g.supprimer("a", &inconnu), "OF-VAL-0005");
    // se remplacer soi-même crée un cycle
    let soi_meme = ModeSuppression::Remplacer {
        par: "a".to_string(),
    };
    refuse_supprim(g.supprimer("a", &soi_meme), "OF-VAL-0001");
    // remplacer par un de ses propres dépendants aussi
    let dependent = ModeSuppression::Remplacer {
        par: "b".to_string(),
    };
    refuse_supprim(g.supprimer("a", &dependent), "OF-VAL-0001");
    // rien n'a bougé : a et b sont toujours là
    assert!(g.noeud("a").is_some());
    assert!(g.noeud("b").is_some());
}

#[test]
fn le_remplacement_sans_dependant_ne_modifie_rien() {
    let mut g = ValueGraph::new();
    ok(g.set_input("a", 1.0));
    ok(g.set_input("e", 2.0));
    let mode = ModeSuppression::Remplacer {
        par: "e".to_string(),
    };
    let res = supprime(&mut g, "a", &mode);
    assert!(res.modifies.is_empty());
    assert_eq!(res.supprimes, vec!["a".to_string()]);
    assert!(g.noeud("a").is_none());
    assert!(g.value("e").is_ok());
}
