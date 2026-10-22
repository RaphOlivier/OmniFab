successfully downloaded text file (SHA: 5c227382f7166eee548474f8f1a34894fc68be5e)
//! Tests du moteur de liaisons en phase E (ADR-0019) : unités
//! déclarées (ADR-0007 phase 1), préfixes réservés et suppression
//! d'alias en trois modes (OF-VAL-0008/0009), cotes d'esquisse
//! pilotées par expressions, migrations de schéma v1 → v2 et rejeu
//! de `sketch.constraint.updated`.

use forge_doc::{Document, NewEvent};
use forge_gcs::Systeme2D;
use forge_session::{
    AddConstraint, AddEntity, ContrainteEsquisse, EVENT_ALIAS_REMOVED, EVENT_EXPRESSION_SET,
    EVENT_INPUT_SET, EVENT_SKETCH_CONSTRAINT_ADDED, EVENT_SKETCH_CONSTRAINT_UPDATED,
    EVENT_SKETCH_ENTITY_ADDED, EntiteEsquisse, ModeSuppression, RemoveAlias, Session,
    SetExpression, SetInput, Unite, ValeurCote, registre_migrations_courant,
};
use forge_units::Length;

/// L'unité déclarée vit dans l'événement `value.input_set` schéma v2
/// (ADR-0019, ADR-0007) : elle survit au journal et la relecture la
/// retrouve sur le nœud du graphe.
#[test]
fn l_unite_declaree_vit_dans_l_evenement() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
    session
        .execute(&SetInput {
            name: "n".into(),
            value: 3.0,
            unite: None,
        })
        .expect("sans unité");
    let events = session.document().events();
    assert_eq!(events[0].event_type, EVENT_INPUT_SET);
    assert_eq!(events[0].schema_version, 2, "schéma v2 dès la phase E");
    assert_eq!(events[0].data["unit"], "mm");
    assert_eq!(events[1].data["unit"], serde_json::Value::Null);
    let noeud = session.graph().noeud("w").expect("nœud w");
    assert_eq!(noeud.unite, Some(Unite::Millimetre));
}

/// Une valeur déclarée en degrés qui lit une référence en
/// millimètres est refusée à la définition (OF-VAL-0007, ADR-0019) :
/// rien n'atteint le journal.
#[test]
fn unite_incoherente_refusee_a_la_definition() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
    let erreur = session
        .execute(&SetExpression {
            name: "angle".into(),
            expression: "w * 2".into(),
            unite: Some(Unite::Degre),
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    assert!(
        erreur.to_string().contains("OF-VAL-0007"),
        "le message porte le code du moteur : {erreur}"
    );
    assert_eq!(session.document().events().len(), 1);
}

/// Le refus d'unité n'est pas définitif (re-validation §7) : la même
/// expression `angle` qui lit `largeur` en millimètres est refusée
/// en degrés (OF-VAL-0007), puis acceptée dès que l'unité déclarée
/// redevient des millimètres — la reprise du dialogue et du
/// panneau reste saine.
#[test]
fn unite_corrigee_apres_refus_redefinit_l_expression() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "largeur".into(),
            value: 100.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
    let erreur = session
        .execute(&SetExpression {
            name: "angle".into(),
            expression: "largeur".into(),
            unite: Some(Unite::Degre),
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    assert!(
        erreur.to_string().contains("OF-VAL-0007"),
        "le message porte le code du moteur : {erreur}"
    );
    session
        .execute(&SetExpression {
            name: "angle".into(),
            expression: "largeur".into(),
            unite: Some(Unite::Millimetre),
        })
        .expect("unité corrigée");
    let noeud = session.graph().noeud("angle").expect("nœud angle");
    assert_eq!(noeud.unite, Some(Unite::Millimetre));
}

/// La création utilisateur sous un préfixe réservé est refusée au
/// plan (OF-VAL-0008, ADR-0019) : les nœuds système ne naissent que
/// par les cotes d'esquisse.
#[test]
fn creation_sous_prefixe_reserve_refusee_au_plan() {
    let mut session = Session::new("tests");
    for name in ["esquisse.demi", "param.acier", "doc.auteur"] {
        let erreur = session
            .execute(&SetInput {
                name: name.into(),
                value: 1.0,
                unite: None,
            })
            .unwrap_err();
        assert_eq!(erreur.code(), "OF-SES-0001");
        assert!(
            erreur.to_string().contains("OF-VAL-0008"),
            "le message porte le code du moteur : {erreur}"
        );
        let erreur = session
            .execute(&SetExpression {
                name: name.into(),
                expression: "1".into(),
                unite: None,
            })
            .unwrap_err();
        assert_eq!(erreur.code(), "OF-SES-0001");
    }
    assert_eq!(session.document().events().len(), 0, "rien au journal");
}

/// La suppression `Simple` d'un alias encore référencé est refusée
/// (OF-VAL-0009, ADR-0019) : les formules qui le lisent ne se
/// cassent jamais en silence.
#[test]
fn suppression_simple_refuse_un_alias_reference() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: None,
        })
        .expect("largeur");
    session
        .execute(&SetExpression {
            name: "d".into(),
            expression: "w * 2".into(),
            unite: None,
        })
        .expect("double");
    let erreur = session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Simple,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("OF-VAL-0009"),
        "le message porte le code du moteur : {erreur}"
    );
    assert!(session.graph().noeud("w").is_some(), "l'alias survit");
}

/// La suppression `Simple` d'un alias libre écrit un seul
/// `value.alias_removed` (ADR-0019) : le nœud disparaît, l'undo le
/// rétablit.
#[test]
fn suppression_simple_d_un_alias_libre() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: None,
        })
        .expect("largeur");
    session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Simple,
        })
        .expect("suppression");
    assert!(session.graph().noeud("w").is_none(), "nœud retiré");
    let events = session.document().events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, EVENT_ALIAS_REMOVED);
    assert_eq!(events[1].schema_version, 1);
    assert_eq!(events[1].data["name"], "w");
    session.undo().expect("undo");
    assert!(
        session.graph().noeud("w").is_some(),
        "undo rétablit l'alias"
    );
}

/// La suppression `Cascade` descend les dépendants transitifs : tout
/// le groupe s'annule et se rétablit en une commande (ADR-0019) —
/// un undo restaure l'alias, ses formules et leurs valeurs.
#[test]
fn suppression_cascade_descend_et_s_annule_en_groupe() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: None,
        })
        .expect("largeur");
    session
        .execute(&SetExpression {
            name: "d".into(),
            expression: "w * 2".into(),
            unite: None,
        })
        .expect("double");
    session
        .execute(&SetExpression {
            name: "e".into(),
            expression: "d + 1".into(),
            unite: None,
        })
        .expect("successeur");
    session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Cascade,
        })
        .expect("cascade");
    assert!(session.graph().noeud("w").is_none());
    assert!(session.graph().noeud("d").is_none());
    assert!(session.graph().noeud("e").is_none());
    let retraits = session
        .document()
        .events()
        .iter()
        .filter(|event| event.event_type == EVENT_ALIAS_REMOVED)
        .count();
    assert_eq!(retraits, 3, "trois nœuds retirés en une transaction");
    session.undo().expect("undo du groupe");
    assert!(session.graph().noeud("w").is_some(), "w rétabli");
    let e = session.graph_mut().value("e").expect("e rétabli");
    assert!((e - 201.0).abs() < 1e-9, "la chaîne entière est recalculée");
    session.redo().expect("redo du groupe");
    assert!(
        session.graph().noeud("e").is_none(),
        "le redo retire à nouveau"
    );
}

/// La cascade qui atteindrait une cote d'esquisse (nœud système) est
/// refusée : une cote ne se supprime pas indirectement (ADR-0019).
#[test]
fn cascade_refusee_par_une_cote_systeme() {
    let mut session = croquis_pilote();
    let erreur = session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Cascade,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("OF-VAL-0009"),
        "le message porte le code du moteur : {erreur}"
    );
    assert!(session.graph().noeud("w").is_some(), "rien n'est retiré");
    assert!(
        session.graph().noeud("esquisse.demi").is_some(),
        "la cote survit"
    );
}

/// Le mode `Remplacer` réécrit les formules vers l'alias de
/// remplacement : les valeurs recalculent, l'ancien alias disparaît
/// (ADR-0019).
#[test]
fn remplacement_substitue_les_formules() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: None,
        })
        .expect("largeur");
    session
        .execute(&SetInput {
            name: "b".into(),
            value: 3.0,
            unite: None,
        })
        .expect("remplacement");
    session
        .execute(&SetExpression {
            name: "d".into(),
            expression: "w * 2".into(),
            unite: None,
        })
        .expect("double");
    session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Remplacer { par: "b".into() },
        })
        .expect("remplacement");
    let noeud = session.graph().noeud("d").expect("d survit");
    assert_eq!(noeud.expression.as_deref(), Some("b * 2"));
    assert!(session.graph().noeud("w").is_none(), "w disparaît");
    let valeur = session.graph_mut().value("d").expect("d calcule");
    assert!((valeur - 6.0).abs() < 1e-9, "d = b * 2");
}

/// Le mode `Remplacer` substitue aussi l'expression des cotes
/// d'esquisse : la contrainte garde son identifiant, un seul
/// `sketch.constraint.updated` la réécrit (ADR-0019).
#[test]
fn remplacement_substitue_les_cotes_d_esquisse() {
    let mut session = croquis_pilote();
    session
        .execute(&SetInput {
            name: "b".into(),
            value: 40.0,
            unite: None,
        })
        .expect("remplacement");
    session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Remplacer { par: "b".into() },
        })
        .expect("remplacement");
    let (id, contrainte) = session
        .esquisse()
        .contrainte_de_cote("esquisse.demi")
        .expect("cote survit");
    assert_eq!(id, 1, "identifiant de contrainte conservé");
    match contrainte {
        ContrainteEsquisse::Distance { valeur, .. } => match valeur {
            ValeurCote::Expression { texte, alias } => {
                assert_eq!(texte, "b / 2");
                assert_eq!(alias, "esquisse.demi");
            }
            autre => panic!("cote toujours pilotée, obtenu {autre:?}"),
        },
        autre => panic!("distance attendue, obtenue {autre:?}"),
    }
    let mises_a_jour = session
        .document()
        .events()
        .iter()
        .filter(|event| event.event_type == EVENT_SKETCH_CONSTRAINT_UPDATED)
        .count();
    assert_eq!(mises_a_jour, 1, "une seule mise à jour de contrainte");
    let demi = session.graph_mut().value("esquisse.demi").expect("cote");
    assert!((demi - 20.0).abs() < 1e-9, "esquisse.demi = b / 2");
    assert!(session.graph().noeud("w").is_none(), "w disparaît");
}

/// Une cote pilotée par expression crée le nœud système et se fige
/// pour la résolution : le solveur ne voit jamais l'expression
/// (ADR-0019), et l'annulation retire contrainte et nœud ensemble.
#[test]
fn cote_pilotee_se_fige_pour_la_resolution() {
    let mut session = croquis_pilote();
    let noeud = session
        .graph()
        .noeud("esquisse.demi")
        .expect("nœud système");
    assert!(noeud.systeme, "la cote crée un nœud système");
    assert_eq!(noeud.unite, Some(Unite::Millimetre));
    assert_eq!(
        session.esquisse().contraintes().len(),
        2,
        "ancrage et cote pilotée avant annulation"
    );
    session.undo().expect("annulation de la cote");
    assert!(
        session
            .esquisse()
            .contrainte_de_cote("esquisse.demi")
            .is_none(),
        "la contrainte est défaite en même temps que le nœud"
    );
    assert!(
        session.graph().noeud("esquisse.demi").is_none(),
        "nœud défait"
    );
    assert_eq!(
        session.esquisse().contraintes().len(),
        1,
        "l'ancrage survit"
    );
    session.redo().expect("rétablissement");
    assert!(
        session.graph().noeud("esquisse.demi").is_some(),
        "nœud rétabli"
    );
    let croquis = session.croquis_pour_resolution().expect("cotes figées");
    assert!(croquis.alias_des_cotes().is_empty(), "toutes figées");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = croquis.resoudre(&mut solveur).expect("résolu");
    // L'ancrage épingle la position résolue du point (ADR-0021,
    // révision du 2026-10-22) : la cote figée se lit autour de
    // l'ancré, quelle que soit sa position résolue.
    let (x0, y0) = resolu.position(0).expect("point ancré résolu");
    let (x1, y1) = resolu.position(1).expect("point mobile résolu");
    let dx = x1.millimeters() - x0.millimeters();
    let dy = y1.millimeters() - y0.millimeters();
    assert!(
        (dx * dx + dy * dy - 2500.0).abs() < 1e-6,
        "la cote figée w / 2 = 50 tient autour de l'ancrage"
    );
}

/// Une cote pilotée qui n'évalue pas en longueur finie et positive
/// refuse la résolution entière : le message désigne la cote
/// (ADR-0019).
#[test]
fn cote_pilotee_negative_refuse_la_resolution() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("origine");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(10.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("mobile");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Expression {
                    texte: "0 - w".into(),
                    alias: "esquisse.neg".into(),
                },
            },
        })
        .expect("cote pilotée");
    let erreur = session.croquis_pour_resolution().unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("esquisse.neg"),
        "le message désigne la cote : {erreur}"
    );
}

/// Les journaux v1 des liaisons migrent à l'ouverture (ADR-0019 §1) :
/// `unit` posé à `null`, et la cote de distance nue devient l'objet
/// typé fixe `{"type": "fixe", "mm": …}`.
#[test]
fn journal_v1_des_liaisons_migre_en_v2() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        EVENT_INPUT_SET,
        1,
        "tests",
        serde_json::json!({"name": "w", "value": 100.0}),
    ));
    document.append(NewEvent::new(
        EVENT_EXPRESSION_SET,
        1,
        "tests",
        serde_json::json!({"name": "d", "expression": "w * 2"}),
    ));
    document.append(NewEvent::new(
        EVENT_SKETCH_ENTITY_ADDED,
        1,
        "tests",
        serde_json::json!({"genre": "point", "id": 0, "x": 0.0, "y": 0.0}),
    ));
    document.append(NewEvent::new(
        EVENT_SKETCH_ENTITY_ADDED,
        1,
        "tests",
        serde_json::json!({"genre": "point", "id": 1, "x": 10.0, "y": 0.0}),
    ));
    document.append(NewEvent::new(
        EVENT_SKETCH_CONSTRAINT_ADDED,
        1,
        "tests",
        serde_json::json!({
            "genre": "distance",
            "id": 0,
            "p1": 0,
            "p2": 1,
            "valeur": 25.0,
        }),
    ));
    let octets = document.to_bytes().expect("conteneur écrit");
    let relu = Document::from_bytes_with(&octets, registre_migrations_courant())
        .expect("journal v1 migré");
    let events = relu.events();
    assert_eq!(events[0].schema_version, 2, "input migré");
    assert_eq!(events[0].data["unit"], serde_json::Value::Null);
    assert_eq!(events[1].schema_version, 2, "expression migrée");
    assert_eq!(events[4].schema_version, 2, "contrainte migrée");
    assert_eq!(events[4].data["valeur"]["type"], "fixe");
    assert_eq!(events[4].data["valeur"]["mm"], 25.0);
    let mut session = Session::from_document("tests", relu).expect("rejeu");
    let d = session.graph_mut().value("d").expect("double");
    assert!((d - 200.0).abs() < 1e-9);
    let millimetres = session
        .esquisse()
        .contraintes()
        .iter()
        .find_map(|(_, contrainte)| match contrainte {
            ContrainteEsquisse::Distance { valeur, .. } => valeur.mm(),
            _ => None,
        })
        .expect("cote de distance relue");
    assert!(
        millimetres.approx_eq(Length::from_millimeters(25.0)),
        "la cote migrée est relue comme fixe : 25 mm"
    );
}

/// L'événement `sketch.constraint.updated` rejoue : après une
/// sauvegarde-réouverture, la contrainte porte l'expression
/// substituée et résout à l'identique (ADR-0019 §1).
#[test]
fn mise_a_jour_de_contrainte_survit_au_journal() {
    let mut session = croquis_pilote();
    session
        .execute(&SetInput {
            name: "b".into(),
            value: 40.0,
            unite: None,
        })
        .expect("remplacement");
    session
        .execute(&RemoveAlias {
            name: "w".into(),
            mode: ModeSuppression::Remplacer { par: "b".into() },
        })
        .expect("remplacement");
    let octets = session.save_to_bytes().expect("journal visible");
    let document =
        Document::from_bytes_with(&octets, registre_migrations_courant()).expect("conteneur relu");
    let mut session = Session::from_document("tests", document).expect("rejeu");
    let (_, contrainte) = session
        .esquisse()
        .contrainte_de_cote("esquisse.demi")
        .expect("cote projetée");
    match contrainte {
        ContrainteEsquisse::Distance { valeur, .. } => match valeur {
            ValeurCote::Expression { texte, .. } => assert_eq!(texte, "b / 2"),
            autre => panic!("cote pilotée attendue, obtenue {autre:?}"),
        },
        autre => panic!("distance attendue, obtenue {autre:?}"),
    }
    let croquis = session.croquis_pour_resolution().expect("cotes figées");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = croquis.resoudre(&mut solveur).expect("résolu");
    let (x0, y0) = resolu.position(0).expect("point ancré");
    let (x1, y1) = resolu.position(1).expect("point mobile");
    let dx = x1.millimeters() - x0.millimeters();
    let dy = y1.millimeters() - y0.millimeters();
    assert!(
        (dx * dx + dy * dy - 400.0).abs() < 1e-6,
        "la cote substituée b / 2 = 20 tient après le rejeu"
    );
}

/// Croquis piloté : point fixe, point mobile, cote `esquisse.demi`
/// = `w / 2` (ADR-0019) — le nœud système naît avec la contrainte,
/// dans la même transaction.
fn croquis_pilote() -> Session {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "w".into(),
            value: 100.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("origine");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(10.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("mobile");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("ancrage");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Expression {
                    texte: "w / 2".into(),
                    alias: "esquisse.demi".into(),
                },
            },
        })
        .expect("cote pilotée");
    session
}
