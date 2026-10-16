//! Tests de forge-session (ADR-0008, ADR-0011) : commandes, undo/redo
//! par curseur, abandon des événements défaisables, imports de géométrie
//! et purge des blobs orphelins, sauvegarde du journal visible.

use forge_doc::{Document, NewEvent};
use forge_session::{
    EVENT_INPUT_SET, EVENT_TRACES_IMPORTED, ImportTraces, Projection, Session, SessionError,
    SetExpression, SetInput,
};

/// Session de référence : deux entrées + une expression (diagonale 1000).
fn rectangle() -> Session {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "width_mm".into(),
            value: 800.0,
        })
        .expect("largeur");
    session
        .execute(&SetInput {
            name: "height_mm".into(),
            value: 600.0,
        })
        .expect("hauteur");
    session
        .execute(&SetExpression {
            name: "diagonal".into(),
            expression: "sqrt(width_mm * width_mm + height_mm * height_mm)".into(),
        })
        .expect("diagonale");
    session
}

/// Un événement d'entrée brute, comme un journal écrit à la main.
fn entree(name: &str, value: f64) -> NewEvent {
    NewEvent::new(
        EVENT_INPUT_SET,
        1,
        "tests",
        serde_json::json!({"name": name, "value": value}),
    )
}

/// Une commande refusée (au plan ou par une projection) ne laisse
/// aucune trace au journal.
#[test]
fn commande_refusee_n_ecrit_rien_au_journal() {
    let mut session = Session::new("tests");
    let erreur = session
        .execute(&SetExpression {
            name: "diagonal".into(),
            expression: "inconnu + 1".into(),
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    let erreur = session
        .execute(&SetInput {
            name: "  ".into(),
            value: 1.0,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    let erreur = session
        .execute(&SetInput {
            name: "x".into(),
            value: f64::NAN,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(
        session.document().events().len(),
        0,
        "aucun événement écrit"
    );
    assert!(!session.can_undo());
}

/// Undo puis redo restaurent exactement l'état du graphe.
#[test]
fn undo_redo_restaurent_l_etat() {
    let mut session = rectangle();
    let avant = session.graph_mut().value("diagonal").expect("diagonale");
    session.undo().expect("undo");
    assert!(
        session.graph_mut().value("diagonal").is_err(),
        "la diagonale est défaite"
    );
    let largeur = session
        .graph_mut()
        .value("width_mm")
        .expect("largeur intacte");
    assert!((largeur - 800.0).abs() < 1e-9);
    assert!(session.can_redo());
    session.redo().expect("redo");
    let apres = session
        .graph_mut()
        .value("diagonal")
        .expect("diagonale rétablie");
    assert!((apres - avant).abs() < 1e-9);
}

/// L'undo remonte tout le journal ; aux bornes : UndoEmpty / RedoEmpty.
#[test]
fn bornes_undo_redo() {
    let mut session = rectangle();
    for _ in 0..3 {
        session.undo().expect("undo");
    }
    assert!(!session.can_undo());
    assert_eq!(session.undo().unwrap_err().code(), "OF-SES-0003");
    let mut session = rectangle();
    assert!(!session.can_redo());
    assert_eq!(session.redo().unwrap_err().code(), "OF-SES-0004");
}

/// Une nouvelle action après un undo abandonne la pile redo (ADR-0008).
#[test]
fn nouvelle_action_apres_undo_abandonne_le_redo() {
    let mut session = rectangle();
    session.undo().expect("undo");
    assert!(session.can_redo());
    session
        .execute(&SetInput {
            name: "width_mm".into(),
            value: 900.0,
        })
        .expect("largeur");
    assert!(!session.can_redo(), "la nouvelle action vide la pile redo");
    assert_eq!(
        session.document().events().len(),
        3,
        "l'événement défait est retiré du journal"
    );
    let largeur = session.graph_mut().value("width_mm").expect("largeur");
    assert!((largeur - 900.0).abs() < 1e-9);
}

/// La sauvegarde écrit le journal visible, sans les événements défaits.
#[test]
fn sauvegarde_ecrit_le_journal_visible() {
    let mut session = rectangle();
    session.undo().expect("undo");
    let chemin = std::env::temp_dir().join("omnifab-tests-forge-session.omnifab");
    session.save_to_path(&chemin).expect("sauvegarde");
    assert!(
        !session.can_redo(),
        "enregistrer abandonne le redo en attente"
    );
    let document = Document::open_from_path(&chemin).expect("ouverture");
    assert_eq!(
        document.events().len(),
        2,
        "le fichier ne contient pas les événements défaits"
    );
    let mut session = Session::from_document("tests", document).expect("rejeu");
    assert!(!session.can_redo());
    assert!(session.graph_mut().value("width_mm").is_ok());
    let _ = std::fs::remove_file(&chemin);
}

/// Un conteneur en mémoire fait l'aller-retour complet : sauvegarde →
/// rejeu (verrous 1 et 2).
#[test]
fn aller_retour_en_memoire() {
    let mut session = rectangle();
    let octets = session.save_to_bytes().expect("octets");
    let document = Document::from_bytes(&octets).expect("document");
    let mut session = Session::from_document("tests", document).expect("rejeu");
    let diagonale = session
        .graph_mut()
        .value("diagonal")
        .expect("diagonale rejouée");
    assert!((diagonale - 1000.0).abs() < 1e-9);
}

/// Une transaction d'événements contigus s'annule en un seul undo.
#[test]
fn transaction_entiere_annulee_en_un_undo() {
    let mut document = Document::new();
    document.append(entree("a", 1.0).in_txn("t1"));
    document.append(entree("b", 2.0).in_txn("t1"));
    document.append(entree("c", 3.0));
    let mut session = Session::from_document("tests", document).expect("rejeu");
    session.undo().expect("undo");
    assert!(
        session.graph_mut().value("c").is_err(),
        "l'événement isolé est défait"
    );
    assert!(
        session.graph_mut().value("b").is_ok(),
        "la transaction tient encore"
    );
    session.undo().expect("undo");
    assert!(
        session.graph_mut().value("a").is_err(),
        "t1 défaite en un seul undo"
    );
    assert!(
        session.graph_mut().value("b").is_err(),
        "t1 défaite en un seul undo"
    );
}

/// Les types d'événements inconnus sont tolérés (compatibilité avant).
#[test]
fn types_inconnus_toleres() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        "futur.feature",
        1,
        "tests",
        serde_json::json!({"x": 1}),
    ));
    let session = Session::from_document("tests", document).expect("rejeu");
    assert!(
        session
            .unknown_types()
            .contains(&"futur.feature".to_string())
    );
}

/// Un journal altéré (champ absent d'un type connu) est refusé au
/// chargement.
#[test]
fn journal_altere_refuse_au_chargement() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        EVENT_INPUT_SET,
        1,
        "tests",
        serde_json::json!({"name": "a"}),
    ));
    let erreur = Session::from_document("tests", document)
        .err()
        .expect("rejet attendu");
    assert_eq!(erreur.code(), "OF-SES-0005");
}

/// Une projection supplémentaire qui refuse bloque la commande : rien
/// n'est écrit au journal.
#[test]
fn refus_d_une_projection_supplementaire() {
    struct Refuseur;
    impl Projection for Refuseur {
        fn name(&self) -> &'static str {
            "refuseur"
        }
        fn reset(&mut self) {}
        fn apply(&mut self, _evenement: &NewEvent) -> Result<(), SessionError> {
            Err(SessionError::ProjectionFailed {
                projection: "refuseur".to_string(),
                message: "domaine de test".to_string(),
            })
        }
    }
    let mut session = Session::new("tests");
    session
        .register_projection(Box::new(Refuseur))
        .expect("projection enregistrée");
    let erreur = session
        .execute(&SetInput {
            name: "a".into(),
            value: 1.0,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    assert_eq!(
        session.document().events().len(),
        0,
        "le refus d'une projection laisse le journal intact"
    );
}

/// La session attribue l'acteur et un identifiant de transaction à
/// chaque événement écrit.
#[test]
fn acteur_et_transaction_attribues() {
    let mut session = Session::new("desktop.demo");
    session
        .execute(&SetInput {
            name: "a".into(),
            value: 1.0,
        })
        .expect("commande");
    let dernier = session.document().events().last().expect("un événement");
    assert_eq!(dernier.actor, "desktop.demo");
    assert!(dernier.txn.is_some(), "une commande = une transaction");
}

/// Un import de tracés est annulable : undo vide la géométrie, redo la
/// restaure (ADR-0011).
#[test]
fn import_traces_puis_undo_vide_et_redo_restaure() {
    let mut session = Session::new("tests");
    session
        .execute(&ImportTraces {
            format: "svg".into(),
            octets: b"<svg/>".to_vec(),
            traces: 2,
            sommets: 8,
        })
        .expect("import");
    assert!(session.geom().traces().is_some(), "géométrie importée");
    session.undo().expect("undo");
    assert!(session.geom().traces().is_none(), "l'import est défait");
    session.redo().expect("redo");
    let traces = session.geom().traces().expect("géométrie rétablie");
    assert_eq!(traces.traces, 2);
    assert_eq!(traces.sommets, 8);
}

/// Une nouvelle commande après un undo purge les blobs orphelins :
/// seul le blob de l'état courant survit (ADR-0011).
#[test]
fn nouvelle_commande_apres_undo_purge_les_blobs_orphelins() {
    let mut session = Session::new("tests");
    session
        .execute(&ImportTraces {
            format: "svg".into(),
            octets: b"<svg/>".to_vec(),
            traces: 1,
            sommets: 3,
        })
        .expect("premier import");
    let ancien = session.geom().traces().expect("géométrie").blob.clone();
    session.undo().expect("undo");
    session
        .execute(&ImportTraces {
            format: "svg".into(),
            octets: b"<svg/>".to_vec(),
            traces: 2,
            sommets: 6,
        })
        .expect("nouvel import");
    let blobs = session.document().blob_names();
    assert!(!blobs.contains(&ancien), "le blob orphelin est purgé");
    assert_eq!(blobs.len(), 1, "seul le blob courant est conservé");
}

/// La sauvegarde abandonne les blobs orphelins : l'import annulé
/// n'emporte pas son blob dans le fichier (ADR-0011).
#[test]
fn sauvegarde_abandonne_les_blobs_orphelins() {
    let mut session = Session::new("tests");
    session
        .execute(&ImportTraces {
            format: "svg".into(),
            octets: b"<svg/>".to_vec(),
            traces: 1,
            sommets: 3,
        })
        .expect("import");
    session.undo().expect("undo");
    let octets = session.save_to_bytes().expect("sauvegarde");
    let document = Document::from_bytes(&octets).expect("conteneur");
    assert!(
        document.blob_names().is_empty(),
        "aucun blob orphelin dans le fichier"
    );
}

/// Le rejeu d'un document sauvegardé restaure la géométrie importée.
#[test]
fn rejeu_de_document_restaure_la_geometrie() {
    let mut session = Session::new("tests");
    session
        .execute(&ImportTraces {
            format: "dxf".into(),
            octets: b"0\nSECTION\n".to_vec(),
            traces: 4,
            sommets: 12,
        })
        .expect("import");
    let octets = session.save_to_bytes().expect("sauvegarde");
    let document = Document::from_bytes(&octets).expect("conteneur");
    let session = Session::from_document("tests", document).expect("rejeu");
    let traces = session.geom().traces().expect("géométrie rejouée");
    assert_eq!(traces.format, "dxf");
    assert!(traces.blob.starts_with("geometry/traces"), "nom généré");
    assert_eq!(traces.traces, 4);
    assert_eq!(traces.sommets, 12);
}

/// Un type n'est « ignoré » par l'interface que s'il est inconnu de
/// toutes les projections : les événements geom sont connus de
/// GeomProjection, ils n'apparaissent pas comme ignorés (ADR-0011).
#[test]
fn types_inconnus_seulement_si_inconnus_de_toutes_les_projections() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        "bidon.truc",
        1,
        "tests",
        serde_json::json!({"x": 1}),
    ));
    document.append(NewEvent::new(
        EVENT_TRACES_IMPORTED,
        1,
        "tests",
        serde_json::json!({"format": "svg", "blob": "geometry/traces", "traces": 1, "sommets": 3}),
    ));
    let session = Session::from_document("tests", document).expect("rejeu");
    let inconnus = session.unknown_types();
    assert!(inconnus.contains(&"bidon.truc".to_string()));
    assert!(!inconnus.contains(&EVENT_TRACES_IMPORTED.to_string()));
}

/// Un import à un format non supporté est refusé au plan : ni événement
/// ni blob (ADR-0011).
#[test]
fn import_au_format_inconnu_est_refuse() {
    let mut session = Session::new("tests");
    let erreur = session
        .execute(&ImportTraces {
            format: "step".into(),
            octets: vec![],
            traces: 1,
            sommets: 3,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(session.document().events().len(), 0);
    assert!(session.document().blob_names().is_empty());
}

/// Deux imports ne s'écrasent jamais : après l'undo du second, le
/// premier relit ses propres octets d'origine, intacts (bug réel du
/// démonstrateur : blob à nom fixe, écrasé par le second import,
/// corrompant l'undo du premier — signalé à la validation phase B).
#[test]
fn deux_imports_ne_s_ecrasent_pas() {
    let mut session = Session::new("tests");
    session
        .execute(&ImportTraces {
            format: "svg".into(),
            octets: b"<svg a/>".to_vec(),
            traces: 1,
            sommets: 3,
        })
        .expect("premier import");
    let premier = session.geom().traces().expect("géométrie").blob.clone();
    session
        .execute(&ImportTraces {
            format: "dxf".into(),
            octets: b"0\nSECTION\n".to_vec(),
            traces: 2,
            sommets: 6,
        })
        .expect("second import");
    session.undo().expect("undo du second import");
    let reference = session.geom().traces().expect("premier restauré");
    assert_eq!(reference.blob, premier, "référence vers son propre blob");
    let octets = session.document().blob(&premier).expect("blob intact");
    assert_eq!(octets, &b"<svg a/>"[..], "octets d'origine intacts");
}
