//! Tests de forge-session (ADR-0008, ADR-0011) : commandes, undo/redo
//! par curseur, abandon des événements défaisables, imports de géométrie
//! et purge des blobs orphelins, sauvegarde du journal visible.
//! Croquis contraint (ADR-0018) : entités, contraintes, graines et
//! résolution dérivée par rejeu. Phase E (ADR-0019) : unités déclarées
//! sur les valeurs, cotes de distance fixées en `ValeurCote`.

use forge_doc::{Document, FixedClock, NewEvent};
use forge_gcs::Systeme2D;
use forge_session::{
    AddConstraint, AddEntity, AncrerPoint, ContrainteEsquisse, EntiteEsquisse, MovePoint,
    ValeurCote,
};
use forge_session::{
    Command, CommandPlan, EVENT_INPUT_SET, EVENT_TRACES_IMPORTED, ImportTraces, Projection,
    Session, SessionError, SetExpression, SetInput,
};
use forge_units::{Angle, Length};

/// Session de référence : deux entrées + une expression (diagonale 1000).
fn rectangle() -> Session {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "width_mm".into(),
            value: 800.0,
            unite: None,
        })
        .expect("largeur");
    session
        .execute(&SetInput {
            name: "height_mm".into(),
            value: 600.0,
            unite: None,
        })
        .expect("hauteur");
    session
        .execute(&SetExpression {
            name: "diagonal".into(),
            expression: "sqrt(width_mm * width_mm + height_mm * height_mm)".into(),
            unite: None,
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
            unite: None,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    let erreur = session
        .execute(&SetInput {
            name: "  ".into(),
            value: 1.0,
            unite: None,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    let erreur = session
        .execute(&SetInput {
            name: "x".into(),
            value: f64::NAN,
            unite: None,
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
            unite: None,
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

/// La sauvegarde écrit le journal appliqué sans casser le redo : le
/// fichier reste le journal linéaire visible (ADR-0008) et la pile
/// redo survit à l'enregistrement (ADR-0016 C3).
#[test]
fn sauvegarde_ecrit_le_journal_applique_sans_casser_le_redo() {
    let mut session = rectangle();
    session.undo().expect("undo");
    let chemin = std::env::temp_dir().join("omnifab-tests-forge-session.omnifab");
    session.save_to_path(&chemin).expect("sauvegarde");
    assert!(
        session.can_redo(),
        "enregistrer préserve le redo en attente"
    );
    let document = Document::open_from_path(&chemin).expect("ouverture");
    assert_eq!(
        document.events().len(),
        2,
        "le fichier ne contient pas les événements défaits"
    );
    session.redo().expect("redo");
    let diagonale = session
        .graph_mut()
        .value("diagonal")
        .expect("diagonale rétablie");
    assert!((diagonale - 1000.0).abs() < 1e-9);
    let _ = std::fs::remove_file(&chemin);
}

/// Un conteneur en mémoire fait l'aller-retour complet : sauvegarde →
/// rejeu (verrous 1 et 2).
#[test]
fn aller_retour_en_memoire() {
    let session = rectangle();
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
            unite: None,
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
            unite: None,
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

/// Une commande refusée par une projection après un undo préserve la
/// pile redo : ni le journal ni les blobs ne sont touchés (ADR-0016 A1).
#[test]
fn commande_refusee_apres_undo_preserve_le_redo() {
    let mut session = rectangle();
    session.undo().expect("undo");
    assert!(session.can_redo());
    let erreur = session
        .execute(&SetExpression {
            name: "diagonal".into(),
            expression: "inconnu + 1".into(),
            unite: None,
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0002");
    assert!(session.can_redo(), "le refus préserve le redo");
    assert_eq!(
        session.document().events().len(),
        3,
        "l'événement défait est toujours dans le journal"
    );
    session.redo().expect("redo");
    let diagonale = session
        .graph_mut()
        .value("diagonal")
        .expect("diagonale rétablie");
    assert!((diagonale - 1000.0).abs() < 1e-9);
}

/// Un plan dont un blob est refusé ne laisse aucun blob du plan en
/// place : ceux déjà déposés sont retirés, journal et pile redo
/// intacts (ADR-0016 A1).
#[test]
fn refus_d_un_blob_retire_les_blobs_deposes() {
    struct DeuxBlobs;
    impl Command for DeuxBlobs {
        fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
            let evenement = NewEvent::new("bidon.deux_blobs", 1, "", serde_json::json!({"x": 1}));
            let mut plan = CommandPlan::new(vec![evenement]);
            plan.blobs
                .push(("geometry/premier".into(), b"premier".to_vec()));
            plan.blobs.push(("../invalide".into(), b"second".to_vec()));
            Ok(plan)
        }
    }
    let mut session = rectangle();
    session.undo().expect("undo");
    assert!(session.can_redo());
    let erreur = session.execute(&DeuxBlobs).unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0006");
    assert!(
        session.document().blob("geometry/premier").is_none(),
        "le blob déposé avant le refus est retiré"
    );
    assert!(
        session.document().blob_names().is_empty(),
        "aucun blob du plan ne survit"
    );
    assert!(session.can_redo(), "le refus préserve le redo");
    assert_eq!(
        session.document().events().len(),
        3,
        "le journal est intact"
    );
    session.redo().expect("redo");
    assert!(
        session.document().blob_names().is_empty(),
        "le rejeu ne ressuscite aucun blob"
    );
}

/// Un nom qui n'est pas référençable par le lexeur d'expressions
/// (espace, accent, chiffre initial) est refusé au plan : plus
/// d'entrée créable mais jamais référençable (ADR-0016 A8).
#[test]
fn nom_non_referencable_refuse_au_plan() {
    let mut session = Session::new("tests");
    for name in ["largeur mm", "largéur", "1argeur"] {
        let erreur = session
            .execute(&SetInput {
                name: name.into(),
                value: 1.0,
                unite: None,
            })
            .unwrap_err();
        assert_eq!(erreur.code(), "OF-SES-0001");
        let erreur = session
            .execute(&SetExpression {
                name: name.into(),
                expression: "1".into(),
                unite: None,
            })
            .unwrap_err();
        assert_eq!(erreur.code(), "OF-SES-0001");
    }
    assert_eq!(
        session.document().events().len(),
        0,
        "aucun événement écrit"
    );
    session
        .execute(&SetInput {
            name: "_x1".into(),
            value: 1.0,
            unite: None,
        })
        .expect("tiret bas initial et chiffres : identifiant valide");
    assert_eq!(session.document().events().len(), 1);
}

/// L'identifiant de transaction est horodaté par l'horloge du document,
/// pas par l'horloge système : une session à horloge figée attribue des
/// transactions reproductibles (ADR-0016 A7).
#[test]
fn txn_horodate_par_l_horloge_du_document() {
    let fabrique = || {
        let document = Document::new().with_clock(Box::new(FixedClock { unix: 951782400 }));
        let mut session = Session::from_document("tests", document).expect("rejeu");
        session
            .execute(&SetInput {
                name: "width_mm".into(),
                value: 800.0,
                unite: None,
            })
            .expect("commande");
        session.document().events()[0]
            .txn
            .clone()
            .expect("txn attribué")
    };
    assert_eq!(
        fabrique(),
        fabrique(),
        "une session à horloge figée attribue un txn reproductible"
    );
}
/// L'échelle d'import vit dans l'événement `geom.mesh.imported`
/// schéma v2 (ADR-0017) : elle survit à l'annulation et au
/// rétablissement, le blob garde les octets bruts.
#[test]
fn import_maillage_porte_son_echelle() {
    let mut session = Session::new("tests");
    session
        .execute(&forge_session::ImportMesh {
            format: "stl".into(),
            octets: b"solid".to_vec(),
            triangles: 4,
            echelle: 1000.0,
        })
        .expect("commande d'import");
    let maillage = session.geom().maillage().expect("maillage projeté");
    assert_eq!(maillage.echelle, 1000.0, "échelle portée");
    assert_eq!(
        session.document().events()[0].schema_version,
        2,
        "schéma v2 à l'écriture"
    );
    session.undo().expect("undo");
    assert!(session.geom().maillage().is_none(), "annulé");
    session.redo().expect("redo");
    let maillage = session.geom().maillage().expect("maillage rétabli");
    assert_eq!(maillage.echelle, 1000.0, "échelle restaurée");
}

/// Une échelle non finie ou non positive est refusée au plan :
/// rien n'atteint le journal (ADR-0017).
#[test]
fn import_maillage_echelle_invalide_refuse() {
    let mut session = Session::new("tests");
    for echelle in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let resultat = session.execute(&forge_session::ImportMesh {
            format: "stl".into(),
            octets: b"solid".to_vec(),
            triangles: 1,
            echelle,
        });
        assert!(resultat.is_err(), "l'échelle {echelle} est refusée");
        assert!(session.document().events().is_empty());
    }
}

/// Un journal v1 du maillage importé est migré à l'ouverture par
/// le registre courant : l'échelle est posée à 1,0 et la
/// projection lit le schéma v2 (ADR-0017).
#[test]
fn journal_v1_migre_le_maillage_en_v2() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        forge_session::EVENT_MESH_IMPORTED,
        1,
        "tests",
        serde_json::json!({"format": "stl", "blob": "geometry/maillage", "triangles": 3}),
    ));
    let octets = document.to_bytes().expect("conteneur écrit");
    let registre = forge_session::registre_migrations_courant();
    let document = Document::from_bytes_with(&octets, registre).expect("journal migré");
    assert_eq!(document.events()[0].schema_version, 2, "migré au v2");
    let session = Session::from_document("tests", document).expect("rejeu");
    let maillage = session.geom().maillage().expect("maillage projeté");
    assert_eq!(maillage.echelle, 1.0, "échelle de migration");
}

/// Croquis rectangle de référence (ADR-0018) : quatre points, quatre
/// lignes, coin fixé, deux horizontales, deux verticales, deux
/// distances — quinze événements. Les graines sont volontairement
/// déformées : la résolution doit retomber sur le rectangle exact.
fn croquis_rectangle() -> Session {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (80.0, -3.0), (97.0, 44.0), (-6.0, 51.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("ligne ajoutée");
    }
    let contraintes = [
        ContrainteEsquisse::PointFixe { point: 0 },
        ContrainteEsquisse::Horizontale { ligne: 4 },
        ContrainteEsquisse::Horizontale { ligne: 6 },
        ContrainteEsquisse::Verticale { ligne: 5 },
        ContrainteEsquisse::Verticale { ligne: 7 },
        ContrainteEsquisse::Distance {
            p1: 0,
            p2: 1,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(100.0),
            },
        },
        ContrainteEsquisse::Distance {
            p1: 1,
            p2: 2,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(50.0),
            },
        },
    ];
    for contrainte in contraintes {
        session
            .execute(&AddConstraint { contrainte })
            .expect("contrainte ajoutée");
    }
    session
}

/// Vérifie qu'une position résolue est la position attendue à la
/// tolérance géométrique du projet près (1e-6 mm).
fn proche(position: Option<(Length, Length)>, x: f64, y: f64) -> bool {
    match position {
        Some((px, py)) => {
            px.approx_eq(Length::from_millimeters(x)) && py.approx_eq(Length::from_millimeters(y))
        }
        None => false,
    }
}

/// Position résolue en millimètres, pour les asserts RELATIFS à la
/// forme : l'ancrage fige la position de la passe de base, la
/// translation absolue du croquis n'est pas déterminée (CI #275).
fn mm(position: Option<(Length, Length)>) -> (f64, f64) {
    let (x, y) = position.expect("position résolue");
    (x.millimeters(), y.millimeters())
}

/// La forme du rectangle de référence est exacte — bords
/// horizontaux et verticaux, largeur 100, hauteur 50 — SANS préjuger
/// de la position absolue : seul l'ancrage la détermine (CI #275).
fn forme_exacte(resolu: &forge_session::CroquisResolu) {
    let (x0, y0) = mm(resolu.position(0));
    let (x1, y1) = mm(resolu.position(1));
    let (x2, y2) = mm(resolu.position(2));
    let (x3, y3) = mm(resolu.position(3));
    assert!((y1 - y0).abs() < 1e-6, "bord haut horizontal");
    assert!((x2 - x1).abs() < 1e-6, "flanc droit vertical");
    assert!((y3 - y2).abs() < 1e-6, "bord bas horizontal");
    assert!((x3 - x0).abs() < 1e-6, "flanc gauche vertical");
    assert!(
        ((x1 - x0).hypot(y1 - y0) - 100.0).abs() < 1e-6,
        "largeur exacte de 100 mm"
    );
    assert!(
        ((x2 - x1).hypot(y2 - y1) - 50.0).abs() < 1e-6,
        "hauteur exacte de 50 mm"
    );
}

/// Le croquis rectangle de référence est résolu exactement : degrés
/// de liberté à zéro, reproductibilité garantie, forme exacte — et
/// aucune position résolue au journal (ADR-0018 D3). La position
/// absolue n'est pas assertée : l'ancrage fige la position résolue
/// de la passe de base, pas l'origine (CI #275).
#[test]
fn le_rectangle_est_resolu_exactement() {
    let session = croquis_rectangle();
    assert_eq!(session.document().events().len(), 15);
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 0, "croquis entièrement contraint");
    assert!(resolu.reproductible(), "l'arène garantit le rejeu");
    forme_exacte(&resolu);
    assert_eq!(
        session.document().events().len(),
        15,
        "aucune position résolue au journal"
    );
}

/// Le rejeu complet du croquis (sauvegarde → ouverture → résolution)
/// donne des positions identiques au premier passage (écart ≤ 1e-9,
/// ADR-0018 D8).
#[test]
fn le_rejeu_du_croquis_est_reproductible() {
    let session = croquis_rectangle();
    let mut solveur = Systeme2D::new().expect("solveur");
    let premier = session.esquisse().resoudre(&mut solveur).expect("résolu");

    let octets = session.save_to_bytes().expect("sauvegarde");
    let registre = forge_session::registre_migrations_courant();
    let document = Document::from_bytes_with(&octets, registre).expect("journal migré");
    let session = Session::from_document("tests", document).expect("rejeu");
    let mut solveur = Systeme2D::new().expect("solveur");
    let rejoue = session.esquisse().resoudre(&mut solveur).expect("résolu");

    assert!(rejoue.reproductible(), "reproductibilité signalée");
    for id in 0..4u32 {
        let (x1, y1) = premier.position(id).expect("position au premier passage");
        let (x2, y2) = rejoue.position(id).expect("position au rejeu");
        assert!(
            (x1.millimeters() - x2.millimeters()).abs() <= 1e-9,
            "x divergent au rejeu pour le point {id}"
        );
        assert!(
            (y1.millimeters() - y2.millimeters()).abs() <= 1e-9,
            "y divergent au rejeu pour le point {id}"
        );
    }
}

/// Une contrainte référençant une entité absente est refusée au
/// plan : rien au journal (OF-SES-0001, ADR-0018 D2).
#[test]
fn une_contrainte_sur_entite_absente_est_refusee() {
    let mut session = Session::new("tests");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point ajouté");
    let erreur = session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 9 },
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(
        session.document().events().len(),
        1,
        "seul le point initial est au journal"
    );
    assert_eq!(session.esquisse().entites().len(), 1);
    assert_eq!(session.esquisse().contraintes().len(), 0);
}

/// L'orientation sur une ligne déjà orientée est refusée, comme
/// dans FreeCAD (ADR-0021 D1 révisée le 2026-10-22 — retour
/// checklist jalon 2) : même genre (redondance) ou genre opposé,
/// rien au journal dans les deux cas ; annuler la contrainte
/// existante puis reposer reste le chemin pour réorienter.
#[test]
fn l_orientation_opposee_est_refusee() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (40.0, 25.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne ajoutée");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 2 },
        })
        .expect("orientation posée");
    let journal_avant = session.document().events().len();
    // Genre opposé : refusé (alignement FreeCAD), rien au journal.
    let erreur = session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Verticale { ligne: 2 },
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(
        session.document().events().len(),
        journal_avant,
        "le refus n'écrit rien"
    );
    // Reposer le même genre : refusé aussi.
    let erreur = session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 2 },
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    // Annuler l'orientation existante puis reposer : le chemin
    // FreeCAD pour réorienter la ligne.
    session.undo().expect("undo");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Verticale { ligne: 2 },
        })
        .expect("repose après annulation");
    let contraintes = session.esquisse().contraintes();
    assert_eq!(contraintes.len(), 1, "une seule orientation");
    assert!(matches!(
        contraintes[0].1,
        ContrainteEsquisse::Verticale { ligne: 2 }
    ));
}

/// L'ancrage épingle la position RÉSOLUE du point, pas sa graine
/// (retour checklist jalon 2, point 8) : un point que les
/// contraintes ont tiré loin de sa graine reste là où
/// l'utilisateur le voit au moment de l'ancrage — la forme ne
/// saute jamais. Ancien comportement : `fixer_point` posé avant
/// la résolution épinglait la graine, et tout le croquis bondissait
/// pour la rejoindre. Croquis sous-déterminé : la position du point
/// le long de la ligne n'est pas déterminée, les asserts sont
/// relatifs à la forme (CI #275).
#[test]
fn l_ancrage_epingle_la_position_resolue() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (100.0, 0.0), (50.0, 30.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne ajoutée");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 3 },
        })
        .expect("orientation posée");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::PointSurLigne { point: 2, ligne: 3 },
        })
        .expect("point sur la ligne");
    let mut solveur = Systeme2D::new().expect("solveur");
    let avant = session.esquisse().resoudre(&mut solveur).expect("résolu");
    // Croquis sous-déterminé : la position du point 2 LE LONG de la
    // ligne n'est pas déterminée — les asserts sont relatifs à la
    // forme (CI #275).
    let (x0, y0) = mm(avant.position(0));
    let (x1, y1) = mm(avant.position(1));
    let (x2, y2) = mm(avant.position(2));
    assert!((y2 - y0).abs() < 1e-6, "le point est sur la ligne");
    assert!((y1 - y0).abs() < 1e-6, "la ligne est horizontale");
    // Ancrage : le point reste à sa position résolue — la forme ne
    // saute pas.
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 2 },
        })
        .expect("ancrage posé");
    let apres = session.esquisse().resoudre(&mut solveur).expect("résolu");
    let (xa, ya) = mm(apres.position(2));
    assert!(
        (xa - x2).abs() < 1e-9 && (ya - y2).abs() < 1e-9,
        "l'ancrage fige la position résolue"
    );
    assert!(proche(apres.position(0), x0, y0), "le point 0 ne bouge pas");
    assert!(proche(apres.position(1), x1, y1), "le point 1 ne bouge pas");
    assert!(
        apres.dof() < avant.dof(),
        "l'ancrage retire des degrés de liberté"
    );
}

/// Le déplacement d'un point ancré est refusé (ADR-0021 D3) : la
/// graine d'un `PointFixe` ne bouge plus, rien au journal ; un
/// point libre se déplace toujours.
#[test]
fn le_deplacement_d_un_point_ancre_est_refuse() {
    let mut session = Session::new("tests");
    for x in [0.0, 30.0] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(0.0),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("ancrage posé");
    let erreur = session
        .execute(&MovePoint {
            point: 0,
            x: Length::from_millimeters(50.0),
            y: Length::from_millimeters(20.0),
        })
        .unwrap_err();
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(
        session.document().events().len(),
        3,
        "le refus n'écrit rien : deux points et l'ancrage"
    );
    assert!(proche(session.esquisse().position_graine(0), 0.0, 0.0));
    // Un point libre se déplace toujours.
    session
        .execute(&MovePoint {
            point: 1,
            x: Length::from_millimeters(60.0),
            y: Length::from_millimeters(10.0),
        })
        .expect("déplacement");
    assert_eq!(session.document().events().len(), 4, "le déplacement écrit");
    assert!(proche(session.esquisse().position_graine(1), 60.0, 10.0));
}

/// L'ancrage posé après un geste fige la position affichée
/// (ADR-0021 D2, révisée le 2026-10-23 — re-test jalon 2, point 8) :
/// la commande `AncrerPoint` capture la position de la passe épinglée
/// — celle que l'utilisateur voit — et la passe de base fige le
/// point capturé avant la première résolution : la forme ne saute
/// pas, le reste absorbe. Le journal rejoué reconstruit la capture.
#[test]
fn l_ancrage_fige_la_position_affichee_apres_un_geste() {
    let mut session = Session::new("tests");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(80.0),
                y: Length::from_millimeters(-3.0),
            },
        })
        .expect("point posé");
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("distance posée");
    // Le geste : le point 0 part à (10, 20) — la passe épinglée
    // le maintient exactement là.
    session
        .execute(&MovePoint {
            point: 0,
            x: Length::from_millimeters(10.0),
            y: Length::from_millimeters(20.0),
        })
        .expect("déplacement");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    let (x0, y0) = mm(resolu.position(0));
    assert!((x0 - 10.0).abs() < 1e-9, "l'affichage épingle (10, 20)");
    assert!((y0 - 20.0).abs() < 1e-9, "l'affichage épingle (10, 20)");
    // L'ancrage capture la position affichée…
    session
        .execute(&AncrerPoint {
            point: 0,
            x: Length::from_millimeters(x0),
            y: Length::from_millimeters(y0),
        })
        .expect("ancrage posé");
    // …et la résolution la tient exactement : la forme ne saute pas.
    let ancre = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert!(
        proche(ancre.position(0), 10.0, 20.0),
        "l'ancre fige l'affiché"
    );
    let (x1, y1) = mm(ancre.position(1));
    let dx = x1 - 10.0;
    let dy = y1 - 20.0;
    assert!(
        (dx * dx + dy * dy - 1600.0).abs() < 1e-6,
        "la distance de 40 mm reste tenue : le reste absorbe"
    );
    // Le journal rejoué reconstruit la capture : mêmes événements,
    // même résolution (ADR-0021 D2 révisée).
    let octets = session.save_to_bytes().expect("octets");
    let document = Document::from_bytes(&octets).expect("document");
    let rejouee = Session::from_document("tests", document).expect("rejeu");
    let mut solveur_rejeu = Systeme2D::new().expect("solveur");
    let rejoue = rejouee
        .esquisse()
        .resoudre(&mut solveur_rejeu)
        .expect("résolu");
    assert!(proche(rejoue.position(0), 10.0, 20.0), "le rejeu tient");
}

/// Un relâchement accroché sur le point ancré ne fait pas échouer
/// la résolution (re-test du 2026-10-23) : la graine du point
/// relâché tombe exactement sur la position capturée du point
/// fixé — deux graines confondues rendent le gradient de la cote
/// de distance indéfini, DogLeg échoue sur la passe de base. Le
/// repli la rejoue depuis la graine d'avant geste : la forme
/// reste exacte, le point relâché ne s'écrase pas sur son ancre.
#[test]
fn un_relachement_sur_le_point_fixe_ne_fait_pas_echouer_la_resolution() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("ligne ajoutée");
    }
    let contraintes = [
        ContrainteEsquisse::Horizontale { ligne: 4 },
        ContrainteEsquisse::Horizontale { ligne: 6 },
        ContrainteEsquisse::Verticale { ligne: 5 },
        ContrainteEsquisse::Verticale { ligne: 7 },
        ContrainteEsquisse::Distance {
            p1: 0,
            p2: 1,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(100.0),
            },
        },
        ContrainteEsquisse::Distance {
            p1: 1,
            p2: 2,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(50.0),
            },
        },
    ];
    for contrainte in contraintes {
        session
            .execute(&AddConstraint { contrainte })
            .expect("contrainte ajoutée");
    }
    let mut solveur = Systeme2D::new().expect("solveur");
    let affiche = session.esquisse().resoudre(&mut solveur).expect("résolu");
    let (x0, y0) = mm(affiche.position(0));
    // L'ancrage capture la position affichée du coin 0…
    session
        .execute(&AncrerPoint {
            point: 0,
            x: Length::from_millimeters(x0),
            y: Length::from_millimeters(y0),
        })
        .expect("ancrage posé");
    // …puis le relâchement accroche le point 1 exactement dessus :
    // graine confondue avec le point fixé.
    session
        .execute(&MovePoint {
            point: 1,
            x: Length::from_millimeters(x0),
            y: Length::from_millimeters(y0),
        })
        .expect("déplacement");
    let resolu = session
        .esquisse()
        .resoudre(&mut solveur)
        .expect("résolu : le repli rejoue la base");
    assert!(resolu.conflictuelles().is_empty(), "aucun conflit");
    forme_exacte(&resolu);
    assert!(proche(resolu.position(0), x0, y0), "l'ancre tient");
    assert!(
        !proche(resolu.position(1), x0, y0),
        "le point relâché ne s'écrase pas sur son ancre"
    );
}

/// Le déplacement pose la graine du point, l'annulation la retire :
/// la résolution suit la dernière graine au journal (ADR-0018 D1).
#[test]
fn le_deplacement_pose_la_graine_et_l_annulation_la_retire() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (30.0, 0.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne ajoutée");

    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 4, "deux points libres : quatre degrés");
    assert!(proche(resolu.position(1), 30.0, 0.0));

    session
        .execute(&MovePoint {
            point: 1,
            x: Length::from_millimeters(60.0),
            y: Length::from_millimeters(10.0),
        })
        .expect("déplacement");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert!(proche(resolu.position(1), 60.0, 10.0));

    session.undo().expect("undo");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert!(proche(resolu.position(1), 30.0, 0.0));
}

/// L'annulation remonte tout le journal : le croquis se vide ; le
/// rétablissement le reconstruit et la résolution retombe sur les
/// mêmes positions (ADR-0018).
#[test]
fn l_annulation_vide_le_croquis_le_retablissement_le_restaure() {
    let mut session = croquis_rectangle();
    while session.can_undo() {
        session.undo().expect("undo");
    }
    assert_eq!(session.esquisse().entites().len(), 0, "croquis vide");
    assert_eq!(session.esquisse().contraintes().len(), 0, "croquis vide");
    while session.can_redo() {
        session.redo().expect("redo");
    }
    assert_eq!(session.esquisse().entites().len(), 8);
    assert_eq!(session.esquisse().contraintes().len(), 7);
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 0, "croquis entièrement contraint");
    forme_exacte(&resolu);
}

/// Les entités et contraintes invalides (références absentes, valeurs
/// non finies, cotes non positives) sont refusées au plan : rien au
/// journal (ADR-0018 D2).
#[test]
fn les_entites_invalides_sont_refusees() {
    let mut session = Session::new("tests");
    session
        .execute(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point de référence");
    let entites_invalides = vec![
        EntiteEsquisse::Ligne { p1: 0, p2: 9 },
        EntiteEsquisse::Ligne { p1: 0, p2: 0 },
        EntiteEsquisse::Cercle {
            centre: 9,
            rayon: Length::from_millimeters(10.0),
        },
        EntiteEsquisse::Cercle {
            centre: 0,
            rayon: Length::from_millimeters(0.0),
        },
        EntiteEsquisse::Arc {
            centre: 0,
            debut: 0,
            fin: 9,
            rayon: Length::from_millimeters(10.0),
            angle_debut: Angle::from_radians(0.0),
            angle_fin: Angle::from_radians(1.0),
        },
        EntiteEsquisse::Point {
            x: Length::from_millimeters(f64::NAN),
            y: Length::from_millimeters(0.0),
        },
    ];
    for entite in entites_invalides {
        let resultat = session.execute(&AddEntity { entite });
        assert!(resultat.is_err(), "l'entité invalide doit être refusée");
        assert_eq!(session.document().events().len(), 1);
    }
    let contraintes_invalides = vec![
        ContrainteEsquisse::Coincidence { p1: 0, p2: 0 },
        ContrainteEsquisse::Distance {
            p1: 0,
            p2: 9,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(10.0),
            },
        },
        ContrainteEsquisse::RayonCercle {
            cercle: 0,
            valeur: Length::from_millimeters(10.0),
        },
    ];
    for contrainte in contraintes_invalides {
        let resultat = session.execute(&AddConstraint { contrainte });
        assert!(resultat.is_err(), "contrainte invalide refusée");
        assert_eq!(session.document().events().len(), 1);
    }
}

/// Les tags solveur sont exposés par contrainte du croquis ; ils
/// sont distincts deux à deux, et l’ancrage `PointFixe` n’en a pas —
/// ce n’est pas une contrainte du solveur, le diagnostic ne peut
/// jamais le référencer (ADR-0018 D3, lot 3 : affichage des
/// redondances par l’application).
#[test]
fn les_tags_des_contraintes_sont_exposes() {
    let session = croquis_rectangle();
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert!(
        resolu.tag(0).is_none(),
        "l’ancrage PointFixe n’a pas de tag solveur"
    );
    let mut tags = Vec::new();
    for id in 1..7u32 {
        tags.push(resolu.tag(id).expect("contrainte du solveur"));
    }
    assert_eq!(tags.len(), 6, "six contraintes posées au solveur");
    for i in 0..tags.len() {
        for j in i + 1..tags.len() {
            assert_ne!(tags[i], tags[j], "les tags sont distincts");
        }
    }
}

/// Le croquis est clonable en un instantané détaché de la session :
/// l’annulation dans la session ne touche pas l’instantané, et la
/// résolution du clone sur un solveur frais retombe sur la
/// forme exacte (pattern C1 du lot 3 : la résolution part sur
/// le fil de travail avec une copie du croquis).
#[test]
fn le_clone_de_l_esquisse_resout_identiquement() {
    let mut session = croquis_rectangle();
    let instantane = session.esquisse().clone();
    session.undo().expect("undo de la dernière contrainte");
    assert_eq!(instantane.contraintes().len(), 7, "instantané détaché");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = instantane.resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 0, "croquis entièrement contraint");
    forme_exacte(&resolu);
}

/// Le point relâché reste où on le lâche (re-validation §7) : une
/// contrainte de distance relie deux points ; déplacer l'un d'eux
/// épingle sa position germe, l'autre absorbe l'ajustement — plus
/// aucune « moyenne » des deux déplacements.
#[test]
fn le_point_relache_reste_ou_on_le_lache() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (40.0, 0.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("distance posée");
    session
        .execute(&MovePoint {
            point: 1,
            x: Length::from_millimeters(90.0),
            y: Length::from_millimeters(30.0),
        })
        .expect("déplacement");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    // Le point relâché garde exactement sa position germe.
    assert!(
        proche(resolu.position(1), 90.0, 30.0),
        "le point relâché reste où on le lâche"
    );
    // L'autre point absorbe : la distance reste tenue autour de la
    // nouvelle position.
    let (x1, y1) = resolu.position(0).expect("p1 résolu");
    let dx = x1.millimeters() - 90.0;
    let dy = y1.millimeters() - 30.0;
    assert!(
        (dx * dx + dy * dy - 1600.0).abs() < 1e-6,
        "la distance de 40 mm reste tenue"
    );
}

/// Croquis entièrement contraint : le repli tient — la passe
/// épinglée du point déplacé entre en conflit avec les contraintes
/// ET l'ancrage figé à sa position de base (CI #275) ; la
/// résolution de base reste la réponse : aucun conflit, forme
/// exacte du rectangle, le coin tiré ne ré-ancre pas la forme.
#[test]
fn un_croquis_totalement_contraint_replie_sur_la_passe_de_base() {
    let mut session = croquis_rectangle();
    session
        .execute(&MovePoint {
            point: 2,
            x: Length::from_millimeters(130.0),
            y: Length::from_millimeters(20.0),
        })
        .expect("déplacement");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 0, "croquis entièrement contraint");
    assert!(resolu.conflictuelles().is_empty(), "aucun conflit");
    assert!(
        !proche(resolu.position(2), 130.0, 20.0),
        "le coin tiré ne ré-ancre pas le rectangle sur sa graine"
    );
    forme_exacte(&resolu);
}

/// Croquis rigide en translation (DoF 2, aucun ancrage) : le geste
/// du point relâché se transmet au croquis entier — le rectangle
/// suit le coin tiré, il ne s'arrête plus « à mi-chemin ».
#[test]
fn le_croquis_rigide_suit_le_geste() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("ligne ajoutée");
    }
    let contraintes = [
        ContrainteEsquisse::Horizontale { ligne: 4 },
        ContrainteEsquisse::Horizontale { ligne: 6 },
        ContrainteEsquisse::Verticale { ligne: 5 },
        ContrainteEsquisse::Verticale { ligne: 7 },
        ContrainteEsquisse::Distance {
            p1: 0,
            p2: 1,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(100.0),
            },
        },
        ContrainteEsquisse::Distance {
            p1: 1,
            p2: 2,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(50.0),
            },
        },
    ];
    for contrainte in contraintes {
        session
            .execute(&AddConstraint { contrainte })
            .expect("contrainte ajoutée");
    }
    session
        .execute(&MovePoint {
            point: 2,
            x: Length::from_millimeters(130.0),
            y: Length::from_millimeters(70.0),
        })
        .expect("déplacement");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 2, "translation libre, forme rigide");
    // Le point relâché reste où on le lâche…
    assert!(
        proche(resolu.position(2), 130.0, 70.0),
        "le coin relâché tient sa graine"
    );
    // …et le rectangle entier suit le geste : translation (30, 20).
    assert!(proche(resolu.position(0), 30.0, 20.0), "coin 0 suit");
    assert!(proche(resolu.position(1), 130.0, 20.0), "coin 1 suit");
    assert!(proche(resolu.position(3), 30.0, 70.0), "coin 3 suit");
}

/// Des graines confondues ne font pas échouer la résolution (re-test
/// du 2026-10-23) : un carré affaissé par deux orientations posées
/// sur des côtés opposés laisse des points exactement superposés —
/// le gradient d'une cote de distance est indéfini aux graines,
/// DogLeg échouait alors que le croquis a une solution. La passe
/// répare en écartant les graines confondues d'un epsilon
/// déterministe : la cote tient, sans conflit.
#[test]
fn des_graines_confondues_ne_font_pas_echouer_la_resolution() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (0.0, 0.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    session
        .execute(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(50.0),
                },
            },
        })
        .expect("contrainte ajoutée");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session
        .esquisse()
        .resoudre(&mut solveur)
        .expect("résolu : les graines confondues sont réparées");
    assert!(resolu.conflictuelles().is_empty(), "aucun conflit");
    let (x0, y0) = mm(resolu.position(0));
    let (x1, y1) = mm(resolu.position(1));
    assert!(
        ((x1 - x0).hypot(y1 - y0) - 50.0).abs() < 1e-6,
        "la cote de distance tient"
    );
}

/// L'aperçu de glissement suit le geste sans écrire au journal
/// (re-test du 2026-10-23) : la projection d'aperçu reçoit la
/// position visée comme relâchement provisoire — un croquis rigide
/// suit en entier, exactement comme au relâchement — et le journal
/// de la session reste inchangé.
#[test]
fn l_apercu_de_glissement_suite_le_geste_sans_ecrire_au_journal() {
    let mut session = Session::new("tests");
    for (x, y) in [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point ajouté");
    }
    for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        session
            .execute(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("ligne ajoutée");
    }
    let contraintes = [
        ContrainteEsquisse::Horizontale { ligne: 4 },
        ContrainteEsquisse::Horizontale { ligne: 6 },
        ContrainteEsquisse::Verticale { ligne: 5 },
        ContrainteEsquisse::Verticale { ligne: 7 },
        ContrainteEsquisse::Distance {
            p1: 0,
            p2: 1,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(100.0),
            },
        },
        ContrainteEsquisse::Distance {
            p1: 1,
            p2: 2,
            valeur: ValeurCote::Fixe {
                mm: Length::from_millimeters(50.0),
            },
        },
    ];
    for contrainte in contraintes {
        session
            .execute(&AddConstraint { contrainte })
            .expect("contrainte ajoutée");
    }
    let mut solveur = Systeme2D::new().expect("solveur");
    let _ = session
        .esquisse()
        .resoudre(&mut solveur)
        .expect("résolu : le rectangle de départ tient");
    let edition = session.esquisse().edition();
    // Aperçu du point 1 visé à (130, 20) : rien au journal, la
    // forme suit en entier, comme au relâchement.
    let apercu = session
        .esquisse()
        .apercu_de_glissement(
            1,
            Length::from_millimeters(130.0),
            Length::from_millimeters(20.0),
        )
        .expect("point saisi");
    let resolu = apercu.resoudre(&mut solveur).expect("aperçu résolu");
    assert_eq!(session.esquisse().edition(), edition, "rien au journal");
    assert!(
        proche(resolu.position(1), 130.0, 20.0),
        "le point saisi tient la visée"
    );
    assert!(proche(resolu.position(0), 30.0, 20.0), "coin 0 suit");
    assert!(proche(resolu.position(2), 130.0, 70.0), "coin 2 suit");
    assert!(proche(resolu.position(3), 30.0, 70.0), "coin 3 suit");
}
