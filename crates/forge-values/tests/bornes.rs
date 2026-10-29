//! Bornes de robustesse du moteur de liaisons (audit n°3, A3-02) : un
//! texte d'expression hostile ne doit jamais faire déborder la pile —
//! le refus est une erreur typée, jamais un abort. Les tests hostiles
//! tournent dans un fil de 2 Mio : le pire environnement d'exécution
//! d'OmniFab (fil de rendu Android, audit n°2).

use forge_values::{ValueGraph, ValuesError};

/// Exécute `corps` dans un fil de pile de 2 Mio : un débordement de
/// pile y abort le processus entier (ni `Err`, ni panique rattrapable)
/// — c'est exactement ce que ces tests interdisent.
fn sur_fil_de_2_mio(corps: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(corps)
        .expect("fil de test de 2 Mio")
        .join()
        .expect("aucune panique attendue dans le fil de test");
}

/// A3-02 : une chaîne d'opérateurs hostile (200 000 termes) est
/// refusée par une erreur typée, sans abort. Le peigne gauche ne
/// déclenche pas la garde de profondeur du parseur — sa boucle est
/// itérative — mais l'arbre construit fait aujourd'hui déborder
/// `collect_references`, `evaluate` et le `Drop`.
#[test]
fn parse_rejette_une_chaine_d_operateurs_hostile() {
    sur_fil_de_2_mio(|| {
        let mut graph = ValueGraph::new();
        for operateur in ["+", "*", "-"] {
            let hostile = format!("{}1", format!("1{operateur}").repeat(200_000));
            let erreur = graph.set_expression("hostile", &hostile).unwrap_err();
            assert!(
                matches!(&erreur, ValuesError::InvalidExpression { .. }),
                "opérateur {operateur} : {erreur}"
            );
        }
    });
}

/// La borne de longueur vit dans la crate : le texte est compté en
/// caractères, la limite exacte (4 096 au correctif) reste acceptée,
/// un caractère de plus est refusé (A3-02).
#[test]
fn texte_d_expression_borne_au_caractere_pres() {
    let mut graph = ValueGraph::new();
    // « 1+ » répété 2 047 fois, plus le « 1 » final : 4 095 caractères.
    let base = format!("{}1", "1+".repeat(2047));
    let exacte = format!(" {base}");
    assert_eq!(exacte.chars().count(), 4096);
    graph
        .set_expression("borne", &exacte)
        .expect("la limite exacte est acceptée");
    let erreur = graph
        .set_expression("borne", &format!(" {exacte}"))
        .unwrap_err();
    assert!(matches!(&erreur, ValuesError::InvalidExpression { .. }));
}

/// Un peigne gauche légitime — sous la borne de longueur — reste
/// parsable et évaluable : les bornes ne doivent pas restreindre
/// les cas honnêtes (A3-02).
#[test]
fn peigne_gauche_legitime_evalue() {
    sur_fil_de_2_mio(|| {
        let mut graph = ValueGraph::new();
        let longue = format!("{}1", "1+".repeat(1000));
        graph.set_expression("somme", &longue).expect("sous la borne");
        let total = graph.value("somme").expect("le peigne s'évalue");
        assert!((total - 1001.0).abs() < 1e-9);
    });
}
