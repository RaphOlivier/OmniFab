//! par curseur, abandon des événements défaisables, imports de géométrie
//! et purge des blobs orphelins, sauvegarde du journal visible.
//! Croquis contraint (ADR-0018) : entités, contraintes, graines et
//! résolution dérivée par rejeu.

use forge_doc::{Document, FixedClock, NewEvent};
use forge_gcs::Systeme2D;
use forge_session::{AddConstraint, AddEntity, ContrainteEsquisse, EntiteEsquisse, MovePoint};
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
            })
            .unwrap_err();
        assert_eq!(erreur.code(), "OF-SES-0001");
        let erreur = session
            .execute(&SetExpression {
                name: name.into(),
                expression: "1".into(),
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
            valeur: Length::from_millimeters(100.0),
        },
        ContrainteEsquisse::Distance {
            p1: 1,
            p2: 2,
            valeur: Length::from_millimeters(50.0),
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

/// Le croquis rectangle de référence est résolu exactement : degrés
/// de liberté à zéro, reproductibilité garantie, positions exactes —
/// et aucune position résolue au journal (ADR-0018 D3).
#[test]
fn le_rectangle_est_resolu_exactement() {
    let session = croquis_rectangle();
    assert_eq!(session.document().events().len(), 15);
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    assert_eq!(resolu.dof(), 0, "croquis entièrement contraint");
    assert!(resolu.reproductible(), "l'arène garantit le rejeu");
    assert!(proche(resolu.position(0), 0.0, 0.0));
    assert!(proche(resolu.position(1), 100.0, 0.0));
    assert!(proche(resolu.position(2), 100.0, 50.0));
    assert!(proche(resolu.position(3), 0.0, 50.0));
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
    assert!(proche(resolu.position(2), 100.0, 50.0));
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
            valeur: Length::from_millimeters(10.0),
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
/// résolution du clone sur un solveur frais retombe sur les
/// positions exactes (pattern C1 du lot 3 : la résolution part sur
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
    assert!(proche(resolu.position(2), 100.0, 50.0));
}
