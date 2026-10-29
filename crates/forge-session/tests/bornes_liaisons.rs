//! Bornes des liaisons côté session (audit n°3, A3-02) : le plan de
//! la commande refuse un texte d'expression hors budget — rien
//! n'atteint le journal. La borne vit aussi dans forge-values : un
//! journal forgé ne passe jamais le parseur.

use forge_session::{Session, SetExpression, SetInput};

/// `SetExpression` refuse au plan un texte d'expression plus long que
/// la borne de forge-values (4 096 caractères au correctif) : aucun
/// événement n'atteint le journal (OF-SES-0001, A3-02).
#[test]
fn set_expression_rejette_un_texte_trop_long() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "a".into(),
            value: 1.0,
            unite: None,
        })
        .expect("entrée");
    // « a+ » répété 4 096 fois, plus le « 1 » final : 8 193 caractères.
    let trop_long = format!("{}1", "a+".repeat(4096));
    let erreur = session
        .execute(&SetExpression {
            name: "hostile".into(),
            expression: trop_long,
            unite: None,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("caractères"),
        "le message porte la borne : {erreur}"
    );
    assert_eq!(
        session.document().events().len(),
        1,
        "rien de nouveau au journal"
    );
}
