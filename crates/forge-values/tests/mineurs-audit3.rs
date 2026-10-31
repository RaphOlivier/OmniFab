//! Mineurs de l'audit n°3 côté liaisons : A3-09 (aucun nœud
//! fantôme après une expression refusée sur un nœud neuf) et A3-10
//! (changement d'unité d'une entrée revalidé contre ses dépendants
//! directs — OF-VAL-0007 non contournable).

use forge_values::{Origine, Unite, ValueGraph, ValuesError};

/// A3-09 : une expression invalide posée sur un nœud NOUVEAU ne doit
/// pas laisser de nœud fantôme — après le refus, « a » n'existe pas :
/// `value("a")` est une erreur inconnue, jamais `Ok(0.0)`.
#[test]
fn expression_refusee_ne_laisse_pas_de_noeud_fantome() {
    let mut graphe = ValueGraph::new();
    assert!(graphe.set_expression("a", "1/").is_err());
    let lecture = graphe.value("a");
    assert!(
        matches!(lecture, Err(ValuesError::UnknownValue { name }) if name == "a"),
        "nœud fantôme après un refus : {lecture:?}"
    );
    assert!(
        !graphe.names().iter().any(|n| n == "a"),
        "« a » ne doit exister dans aucun référentiel du graphe"
    );
}

/// A3-10 : changer l'unité d'une entrée lue par un dépendant qui
/// déclare une unité doit être refusé (OF-VAL-0007 ne doit pas être
/// contournable en réécrivant l'unité de la source) ; l'état
/// antérieur tient.
#[test]
fn changer_unite_d_entree_revalide_les_dependants_directs() {
    let mut graphe = ValueGraph::new();
    graphe
        .set_input_unite("a", 10.0, Some(Unite::Millimetre), Origine::Utilisateur)
        .unwrap();
    graphe
        .set_expression_unite("b", "a * 2", Some(Unite::Millimetre), Origine::Utilisateur)
        .unwrap();
    let refus = graphe.set_input_unite("a", 90.0, Some(Unite::Degre), Origine::Utilisateur);
    assert!(
        matches!(
            &refus,
            Err(ValuesError::UniteIncoherente { node, reference, .. })
                if node == "b" && reference == "a"
        ),
        "le changement d'unité devait être refusé : {refus:?}"
    );
    assert_eq!(graphe.value("b").unwrap(), 20.0);
}
