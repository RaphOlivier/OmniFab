//! Tests A3-01 (audit n°3) : le ramasse-miettes de blobs ne doit purger
//! que les blobs des événements tronqués (pile redo abandonnée) —
//! jamais un blob référencé par un événement appliqué du journal.
//! Fichier séparé de `session.rs` (même crate de tests) pour éviter le
//! transit de gros fichiers par l'assistant (leçon v39).

use forge_doc::{Document, NewEvent};
use forge_session::{
    Command, CommandPlan, EVENT_TRACES_IMPORTED, ImportTraces, Session, SessionError, SetInput,
};

/// Import SVG minimal (octets factices : la validation des octets
/// appartient à l'application, pas à la session).
fn import(octets: &[u8], traces: usize, sommets: usize) -> ImportTraces {
    ImportTraces {
        format: "svg".into(),
        octets: octets.to_vec(),
        traces,
        sommets,
    }
}

/// A3-01 : un undo profond ne doit pas perdre les blobs des imports
/// antérieurs. Le gc ne peut purger que les blobs des événements
/// tronqués (redo abandonné), jamais ceux référencés par le journal
/// appliqué (clé `data.blob`, comme `to_bytes_prefix`).
#[test]
fn undo_profond_conserve_les_blobs_des_imports_anterieurs() {
    let mut session = Session::new("tests");
    session
        .execute(&import(b"<svg a/>", 1, 3))
        .expect("import A");
    let nom_a = session.geom().traces().expect("géométrie A").blob.clone();
    session
        .execute(&import(b"<svg b/>", 2, 6))
        .expect("import B");
    session
        .execute(&SetInput {
            name: "x".into(),
            value: 1.0,
            unite: None,
        })
        .expect("entrée x");
    for _ in 0..3 {
        session.undo().expect("undo");
    }
    session.redo().expect("rejeu de l'import A");
    assert!(
        session.document().blob(&nom_a).is_some(),
        "le rejeu de l'import A retrouve son blob"
    );
    // Sauvegarde du préfixe appliqué puis relecture : le fichier
    // embarque le blob référencé par le journal, aucune référence
    // orpheline, le rejeu ne peut pas échouer.
    let octets = session.save_to_bytes().expect("conteneur du préfixe");
    let document = Document::from_bytes(&octets).expect("relecture");
    assert!(
        document.blob(&nom_a).is_some(),
        "aucune référence orpheline : le blob du journal est dans le fichier"
    );
    let relue = Session::from_document("tests", document).expect("rejeu sans erreur");
    assert!(relue.geom().traces().is_some(), "la géométrie rejouée vit");
}

/// A3-01, cas limite : la purge reste effective — un import défait dont
/// le redo est abandonné par une nouvelle commande perd son blob.
#[test]
fn import_defait_puis_nouvelle_action_purge_son_blob() {
    let mut session = Session::new("tests");
    session
        .execute(&import(b"<svg a/>", 1, 3))
        .expect("import A");
    let nom_a = session.geom().traces().expect("géométrie A").blob.clone();
    session.undo().expect("undo");
    session
        .execute(&SetInput {
            name: "x".into(),
            value: 1.0,
            unite: None,
        })
        .expect("nouvelle action");
    assert!(!session.can_redo(), "le redo est abandonné");
    assert!(
        session.document().blob(&nom_a).is_none(),
        "le blob de l'import défait est purgé"
    );
}

/// Commande de test : événement de tracés référençant un blob déjà
/// déposé, sans en déposer (un même blob partagé par deux événements
/// du journal).
struct ReimportTraces {
    blob: String,
}

impl Command for ReimportTraces {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_TRACES_IMPORTED,
            1,
            "tests",
            serde_json::json!({
                "format": "svg",
                "blob": self.blob,
                "traces": 1,
                "sommets": 3,
            }),
        )]))
    }
}

/// A3-01, cas limite : un blob référencé à la fois par un événement
/// appliqué et par un événement tronqué survit à la purge.
#[test]
fn blob_partage_applique_tronque_est_conserve() {
    let mut session = Session::new("tests");
    session
        .execute(&import(b"<svg a/>", 1, 3))
        .expect("import A");
    let nom_a = session.geom().traces().expect("géométrie A").blob.clone();
    session
        .execute(&ReimportTraces {
            blob: nom_a.clone(),
        })
        .expect("second événement sur le même blob");
    session.undo().expect("undo du second événement");
    // Le redo du second événement est abandonné : nom_a reste pourtant
    // référencé par le premier événement (appliqué) — il doit survivre.
    session
        .execute(&SetInput {
            name: "x".into(),
            value: 1.0,
            unite: None,
        })
        .expect("nouvelle action");
    assert!(
        session.document().blob(&nom_a).is_some(),
        "le blob partagé avec un événement appliqué survit à la purge"
    );
}
