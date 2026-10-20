use forge_doc::{DocError, Document, NewEvent};
use forge_session::{
    EVENT_EXPRESSION_SET, EVENT_INPUT_SET, EVENT_TRACES_IMPORTED, GeomProjection, Projection,
    SessionError, ValuesProjection, registre_migrations_courant,
};
use serde_json::json;

/// Les compteurs de la géométrie sont des entiers non négatifs :
/// négatifs et fractionnaires sont refusés — jamais de conversion
/// `as` silencieuse depuis un `f64` de fichier (ADR-0016 A3).
#[test]
fn compteurs_invalides_rejetes() {
    let mut geom = GeomProjection::new();
    for compteur in [-1.0, 2.5] {
        let event = NewEvent::new(
            EVENT_TRACES_IMPORTED,
            1,
            "tests",
            json!({"format": "svg", "blob": "b", "traces": compteur, "sommets": 2}),
        );
        match geom.apply(&event) {
            Err(SessionError::InvalidEventData { .. }) => {}
            other => panic!("compteur {compteur} devait être rejeté, obtenu {other:?}"),
        }
    }
}

/// Un événement v1 valide s'applique : les compteurs entiers sont
/// lus sans perte (ADR-0016 A3).
#[test]
fn compteurs_valides_lus() {
    let mut geom = GeomProjection::new();
    let event = NewEvent::new(
        EVENT_TRACES_IMPORTED,
        1,
        "tests",
        json!({"format": "svg", "blob": "b", "traces": 3, "sommets": 10}),
    );
    geom.apply(&event).expect("événement valide");
    let traces = geom.traces().expect("traces projetées");
    assert_eq!(traces.traces, 3);
    assert_eq!(traces.sommets, 10);
}

/// Un type d'événement connu portant une version de schéma plus
/// récente est refusé explicitement par la projection : jamais lu
/// comme du v1 (ADR-0016 A3).
#[test]
fn schema_recent_refuse_par_la_projection() {
    let mut values = ValuesProjection::new();
    let event = NewEvent::new(
        EVENT_INPUT_SET,
        3,
        "tests",
        json!({"name": "w", "value": 1.0}),
    );
    match values.apply(&event) {
        Err(SessionError::InvalidEventData { .. }) => {}
        other => panic!("version trop récente attendue, obtenu {other:?}"),
    }
    let mut geom = GeomProjection::new();
    let event = NewEvent::new(
        EVENT_TRACES_IMPORTED,
        2,
        "tests",
        json!({"format": "svg", "blob": "b", "traces": 1, "sommets": 2}),
    );
    match geom.apply(&event) {
        Err(SessionError::InvalidEventData { .. }) => {}
        other => panic!("version trop récente attendue, obtenu {other:?}"),
    }
}

/// Un type inconnu reste toléré, quelle que soit sa version
/// (compatibilité avant) : seuls les types connus sont vérifiés.
#[test]
fn type_inconnu_tolele() {
    let mut values = ValuesProjection::new();
    let event = NewEvent::new("domaine.inconnu", 99, "tests", json!({}));
    assert!(values.apply(&event).is_ok());
    assert_eq!(
        values.unknown_types().to_vec(),
        vec!["domaine.inconnu".to_string()],
        "le type inconnu est toléré et signalé"
    );
}

/// À l'ouverture, le registre courant déclare les types connus :
/// un journal écrit par une version plus récente du logiciel est
/// refusé par `SchemaTooNew` (ADR-0001/0016 A3).
#[test]
fn journal_plus_recent_refuse_a_l_ouverture() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        EVENT_EXPRESSION_SET,
        3,
        "tests",
        json!({"name": "w", "expression": "1"}),
    ));
    let octets = document.to_bytes().expect("conteneur écrit");
    match Document::from_bytes_with(&octets, registre_migrations_courant()) {
        Err(DocError::SchemaTooNew { .. }) => {}
        other => panic!("SchemaTooNew attendu, obtenu {other:?}"),
    }
}

/// Un journal v1 des types connus s'ouvre normalement avec le
/// registre courant : déclarer les types ne casse rien.
#[test]
fn journal_v1_s_ouvre_avec_le_registre() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        EVENT_INPUT_SET,
        1,
        "tests",
        json!({"name": "w", "value": 12.0}),
    ));
    let octets = document.to_bytes().expect("conteneur écrit");
    let relu = Document::from_bytes_with(&octets, registre_migrations_courant())
        .expect("journal v1 accepté");
    assert_eq!(relu.events().len(), 1);
}
