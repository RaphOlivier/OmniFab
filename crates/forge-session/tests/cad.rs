//! Lot 2 de la phase B (ADR-0024, designs D3/D4) : croquis nommés
//! (`sketch.created`), champ `croquis` des événements d'esquisse,
//! features d'extrusion (`cad.feature.added` / `cad.feature.updated`)
//! et alias `cad.*` auto-nommés — la reconstruction du corps passe
//! par un moteur CAO **injecté** ([`MoteurCad`]) : moteur de test
//! trivial, volumes analytiques du prisme (aire nette du lacet de
//! la discrétisation — extérieur moins trous — × profondeur).

// Convention expect v2 (2026-11-01, lot E) : échafaudage de test —
// `allow-expect-in-tests` (clippy.toml) ne couvre que les fonctions
// `#[test]` et les modules `#[cfg(test)]`, pas les helpers de ce
// fichier d'intégration : expect admis dans tout le fichier.
#![allow(clippy::expect_used)]

use forge_doc::{Document, NewEvent};
use forge_gcs::Systeme2D;
use forge_geom::{Mesh, Point2, Point3, Region, TOLERANCE_DISCRETISATION_MM};
use forge_session::{
    AddEntity, AddFeature, BoiteEnglobante, CouleurFeature, CreateSketch, EVENT_FEATURE_ADDED,
    EVENT_FEATURE_UPDATED, EVENT_SKETCH_CREATED, EVENT_SKETCH_ENTITY_ADDED,
    EVENT_SKETCH_POINT_MOVED, EntiteEsquisse, ErreurMoteurCad, EtatFeature, FeatureCad,
    ModeExtrusion, ModeSuppression, MoteurCad, MovePoint, ProfondeurCad, ProfondeurFeature,
    RemoveAlias, Session, SetExpression, SetInput, UpdateFeature, ValeurCote,
    registre_migrations_courant,
};
use forge_units::Length;

/// Registre des codes d'erreur (garde bidirectionnelle, gabarit
/// groupe B) : aucun code ne sort sans sa ligne dans le registre
/// public `docs/error-codes.md`.
const REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/error-codes.md"
));

/// Millimètres — jamais de f64 nu (verrou n°5).
fn mm(valeur: f64) -> Length {
    Length::from_millimeters(valeur)
}

/// Pose un point sur le croquis actif.
fn pose_point(session: &mut Session, x: f64, y: f64) {
    let entite = EntiteEsquisse::Point { x: mm(x), y: mm(y) };
    session.execute(&AddEntity { entite }).expect("point posé");
}

/// Pose une ligne entre deux points du croquis actif.
fn pose_ligne(session: &mut Session, p1: u32, p2: u32) {
    let entite = EntiteEsquisse::Ligne { p1, p2 };
    session.execute(&AddEntity { entite }).expect("ligne posée");
}

/// Pose un rectangle (4 points + 4 lignes chaînées) sur le croquis
/// actif, coin inférieur en (x, y) — identifiants séquentiels
/// depuis un croquis vierge.
fn pose_rectangle(session: &mut Session, x: f64, y: f64, largeur: f64, hauteur: f64) {
    let coins = [
        (x, y),
        (x + largeur, y),
        (x + largeur, y + hauteur),
        (x, y + hauteur),
    ];
    for (px, py) in coins {
        pose_point(session, px, py);
    }
    for p1 in 0..4u32 {
        pose_ligne(session, p1, (p1 + 1) % 4);
    }
}

/// Pose un cercle (centre + cercle de rayon) sur le croquis actif,
/// identifiants séquentiels depuis un croquis vierge.
fn pose_cercle(session: &mut Session, x: f64, y: f64, rayon: f64) {
    pose_point(session, x, y);
    let entite = EntiteEsquisse::Cercle {
        centre: 0,
        rayon: mm(rayon),
    };
    session.execute(&AddEntity { entite }).expect("cercle posé");
}

/// Aire du lacet (valeur absolue) d'une polyligne discrétisée.
fn aire_du_lacet(points: &[Point2]) -> f64 {
    let mut aire = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let croix_x = a.x().millimeters() * b.y().millimeters();
        let croix_y = b.x().millimeters() * a.y().millimeters();
        aire += croix_x - croix_y;
    }
    aire.abs() / 2.0
}

/// Solide analytique du moteur trivial : volume et boîte
/// englobante — assez pour les volumes attendus des tests, jamais
/// pour la géométrie réelle (lot 3, noyau OCCT).
#[derive(Debug, Clone)]
struct SolideAnalytique {
    /// Volume en millimètres cubes.
    volume_mm3: f64,
    /// Boîte englobante [min ; max], en millimètres.
    boite: [(f64, f64, f64); 2],
}

/// Échec de reconstruction du moteur trivial.
fn echec(raison: &str) -> ErreurMoteurCad {
    ErreurMoteurCad::Echec {
        raison: raison.to_string(),
    }
}

/// Boîte englobante des points du lacet, extrudée de zéro à la
/// profondeur (le plan du croquis est xy, ADR-0024 D3).
fn boite_des_points(points: &[Point2], profondeur: Length) -> [(f64, f64, f64); 2] {
    let mut min = (f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for point in points {
        min.0 = min.0.min(point.x().millimeters());
        min.1 = min.1.min(point.y().millimeters());
        max.0 = max.0.max(point.x().millimeters());
        max.1 = max.1.max(point.y().millimeters());
    }
    [
        (min.0, min.1, 0.0),
        (max.0, max.1, profondeur.millimeters()),
    ]
}

/// Moteur de test trivial (ADR-0024 §7, lot 2) : l'extrusion
/// approche le contour par son lacet discrétisé — volume = aire ×
/// profondeur ; l'union additionne les volumes et unit les boîtes,
/// la soustraction est naïve (jamais négative). Il suffit aux
/// volumes analytiques attendus (rectangle, cercle) — le lot 3
/// branche le vrai noyau OCCT.
struct MoteurPrismesAnalytiques;

impl MoteurCad for MoteurPrismesAnalytiques {
    type Solide = SolideAnalytique;

    fn extruder(
        &self,
        region: &Region,
        _mode: ModeExtrusion,
        profondeur: Length,
    ) -> Result<SolideAnalytique, ErreurMoteurCad> {
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = region
            .exterieur()
            .discretiser(tolerance)
            .map_err(|erreur| echec(&erreur.to_string()))?;
        if points.len() < 3 {
            return Err(echec("moins de trois sommets au contour extérieur"));
        }
        // ADR-0037 D3 : aire nette — l'extérieur moins les trous.
        let mut aire = aire_du_lacet(&points);
        for trou in region.trous() {
            let points_trou = trou
                .discretiser(tolerance)
                .map_err(|erreur| echec(&erreur.to_string()))?;
            aire -= aire_du_lacet(&points_trou);
        }
        if aire <= 0.0 {
            return Err(echec("région d'aire nette vide ou négative"));
        }
        Ok(SolideAnalytique {
            volume_mm3: aire * profondeur.millimeters(),
            boite: boite_des_points(&points, profondeur),
        })
    }

    fn fusionner(
        &self,
        base: &SolideAnalytique,
        autre: &SolideAnalytique,
    ) -> Result<SolideAnalytique, ErreurMoteurCad> {
        let min = (
            base.boite[0].0.min(autre.boite[0].0),
            base.boite[0].1.min(autre.boite[0].1),
            base.boite[0].2.min(autre.boite[0].2),
        );
        let max = (
            base.boite[1].0.max(autre.boite[1].0),
            base.boite[1].1.max(autre.boite[1].1),
            base.boite[1].2.max(autre.boite[1].2),
        );
        Ok(SolideAnalytique {
            volume_mm3: base.volume_mm3 + autre.volume_mm3,
            boite: [min, max],
        })
    }

    fn soustraire(
        &self,
        base: &SolideAnalytique,
        outil: &SolideAnalytique,
    ) -> Result<SolideAnalytique, ErreurMoteurCad> {
        let volume = base.volume_mm3 - outil.volume_mm3;
        if volume <= 0.0 {
            return Err(echec("soustraction au résultat vide ou négatif"));
        }
        Ok(SolideAnalytique {
            volume_mm3: volume,
            boite: base.boite,
        })
    }

    fn tesseller(
        &self,
        _solide: &SolideAnalytique,
        _tolerance: Length,
    ) -> Result<Mesh, ErreurMoteurCad> {
        // Moteur trivial : maillage vide — la tessellation réelle
        // arrive avec le noyau OCCT (lot 3).
        Ok(Mesh::new())
    }

    fn boite_englobante(
        &self,
        solide: &SolideAnalytique,
    ) -> Result<BoiteEnglobante, ErreurMoteurCad> {
        let min = Point3::new(
            mm(solide.boite[0].0),
            mm(solide.boite[0].1),
            mm(solide.boite[0].2),
        )
        .expect("coordonnées finies");
        let max = Point3::new(
            mm(solide.boite[1].0),
            mm(solide.boite[1].1),
            mm(solide.boite[1].2),
        )
        .expect("coordonnées finies");
        Ok(BoiteEnglobante { min, max })
    }
}

#[test]
fn create_sketch_journalise_et_devient_actif() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis créé");
    let events = session.document().events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EVENT_SKETCH_CREATED);
    assert_eq!(events[0].schema_version, 1);
    assert_eq!(events[0].data["nom"], "croquis-1");
    assert_eq!(events[0].data["plan"]["type"], "xy");
    assert_eq!(session.esquisse().croquis_actif(), 1);
    assert_eq!(session.esquisse().croquis_de_nom("croquis-1"), Some(1));
    // le croquis implicite historique reste, sans nom
    assert!(session.esquisse().croquis_existe(0));
    assert_eq!(session.esquisse().croquis_de_nom(""), None);
}

#[test]
fn create_sketch_nom_duplique_refuse() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "bois".to_string(),
        })
        .expect("croquis créé");
    let erreur = session
        .execute(&CreateSketch {
            nom: "bois".to_string(),
        })
        .expect_err("nom dupliqué refusé");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert_eq!(session.document().events().len(), 1, "rien au journal");
}

#[test]
fn evenements_de_croquis_portent_le_champ_croquis() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    session
        .execute(&MovePoint {
            point: 0,
            x: mm(1.0),
            y: mm(1.0),
        })
        .expect("déplacement");
    let events = session.document().events();
    // entités et déplacements portent le croquis actif, aux
    // schémas du lot 2 (ADR-0024 §7)
    let entite = events
        .iter()
        .find(|evenement| evenement.event_type == EVENT_SKETCH_ENTITY_ADDED)
        .expect("entité journalisée");
    assert_eq!(entite.schema_version, 3);
    assert_eq!(entite.data["croquis"], 1);
    let deplace = events
        .iter()
        .find(|evenement| evenement.event_type == EVENT_SKETCH_POINT_MOVED)
        .expect("déplacement journalisé");
    assert_eq!(deplace.schema_version, 2);
    assert_eq!(deplace.data["croquis"], 1);
    // les entités sont portées par le croquis 1, le croquis 0
    // historique reste vide
    let entites = session.esquisse().entites_du_croquis(1).expect("croquis 1");
    assert_eq!(entites.len(), 8);
    assert!(
        session
            .esquisse()
            .entites_du_croquis(0)
            .expect("croquis 0")
            .is_empty()
    );
}

#[test]
fn croquis_implicite_v0_rejoue_les_evenements_anciens() {
    // journaux d'avant le champ « croquis » : entité v2,
    // déplacement v1 — le rejeu les porte au croquis implicite 0,
    // sans refus.
    let mut document = Document::new();
    document.append(NewEvent::new(
        "sketch.entity.added",
        2,
        "tests",
        serde_json::json!({"genre": "point", "id": 0, "x": 10.0, "y": 20.0}),
    ));
    document.append(NewEvent::new(
        "sketch.point.moved",
        1,
        "tests",
        serde_json::json!({"point": 0, "x": 30.0, "y": 40.0}),
    ));
    let session = Session::from_document("tests", document).expect("rejeu");
    assert_eq!(session.esquisse().croquis_actif(), 0, "croquis implicite");
    let entites = session.esquisse().entites_du_croquis(0).expect("croquis 0");
    assert_eq!(entites.len(), 1);
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session
        .esquisse()
        .resoudre_croquis(0, &mut solveur)
        .expect("résolu");
    let (x, y) = resolu.position(0).expect("graine posée");
    assert!(
        (x.millimeters() - 30.0).abs() < 1e-9 && (y.millimeters() - 40.0).abs() < 1e-9,
        "le rejeu tient la dernière graine du croquis implicite"
    );
}

#[test]
fn migrations_croquis_implicite_alignent_les_anciens_journaux() {
    // les journaux d'avant le champ « croquis » migrent au schéma
    // courant : entité v2→v3, déplacement v1→v2, champ « croquis »
    // posé à 0 (croquis implicite)
    let mut document = Document::new();
    document.append(NewEvent::new(
        "sketch.entity.added",
        2,
        "tests",
        serde_json::json!({"genre": "point", "id": 0, "x": 1.0, "y": 2.0}),
    ));
    document.append(NewEvent::new(
        "sketch.point.moved",
        1,
        "tests",
        serde_json::json!({"point": 0, "x": 3.0, "y": 4.0}),
    ));
    let octets = document.to_bytes().expect("conteneur sérialisé");
    let document =
        Document::from_bytes_with(&octets, registre_migrations_courant()).expect("journal migré");
    let events = document.events();
    assert_eq!(events[0].schema_version, 3, "entité au schéma du croquis");
    assert_eq!(events[0].data["croquis"], 0);
    assert_eq!(
        events[1].schema_version, 2,
        "déplacement au schéma du croquis"
    );
    assert_eq!(events[1].data["croquis"], 0);
}

#[test]
fn add_feature_extrusion_rectangle_volume_analytique() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    // journal : un événement de feature au schéma du lot 2
    let events = session.document().events();
    let dernier = events.last().expect("journal non vide");
    assert_eq!(dernier.event_type, EVENT_FEATURE_ADDED);
    assert_eq!(dernier.schema_version, 1);
    assert_eq!(dernier.data["id"], 0);
    assert_eq!(dernier.data["feature"]["type"], "extrusion");
    assert_eq!(dernier.data["feature"]["croquis"], 1);
    assert_eq!(dernier.data["feature"]["mode"], "addition");
    assert_eq!(dernier.data["feature"]["profondeur"]["type"], "cote");
    assert_eq!(
        dernier.data["feature"]["profondeur"]["valeur"]["type"],
        "fixe"
    );
    assert_eq!(dernier.data["feature"]["profondeur"]["valeur"]["mm"], 4.0);
    // projection : une feature extrusion, profondeur fixe
    let features = session.cad().features();
    assert_eq!(features.len(), 1);
    match &features[0].1 {
        FeatureCad::Extrusion {
            profondeur: ProfondeurCad::Cote(ValeurCote::Fixe { mm: profondeur }),
            ..
        } => assert!((profondeur.millimeters() - 4.0).abs() < 1e-9),
        autre => panic!("feature extrusion attendue : {autre:?}"),
    }
    // reconstruction : volume analytique du prisme 40×25×4
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    let corps = session
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    let solide = corps.corps.expect("corps reconstruit");
    assert!(
        (solide.volume_mm3 - 4000.0).abs() < 1e-9,
        "volume attendu 4 000 mm³, obtenu {}",
        solide.volume_mm3
    );
    assert_eq!(corps.etats.len(), 1);
    assert!(matches!(corps.etats[0].1, EtatFeature::Ok));
}

#[test]
fn add_feature_refuse_croquis_inconnu() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let commande = AddFeature {
        croquis: 9,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    let erreur = session
        .execute(&commande)
        .expect_err("croquis inconnu refusé");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(session.cad().features().is_empty(), "rien projeté");
}

#[test]
fn add_feature_refuse_contour_ouvert() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    // deux points et une ligne : chaîne ouverte, jamais un contour
    pose_point(&mut session, 0.0, 0.0);
    pose_point(&mut session, 10.0, 0.0);
    pose_ligne(&mut session, 0, 1);
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    let erreur = session
        .execute(&commande)
        .expect_err("contour ouvert refusé");
    assert_eq!(erreur.code(), "OF-SES-0001");
}

#[test]
fn add_feature_refuse_profondeur_invalide() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    // A5-04 (audit n°5, ADR-0036) : bornes de la profondeur fixe —
    // finie, strictement positive, bornée haut : 1e300 mm passait
    // à l'écriture (rouge D1c)
    for profondeur in [0.0, -4.0, 1e300, f64::NAN] {
        let avant = session.document().events().len();
        let commande = AddFeature {
            croquis: 1,
            mode: ModeExtrusion::Addition,
            profondeur: ProfondeurFeature::Fixe { mm: mm(profondeur) },
            couleur: None,
        };
        let erreur = session.execute(&commande).expect_err("profondeur invalide");
        assert_eq!(erreur.code(), "OF-SES-0001", "profondeur {profondeur}");
        assert_eq!(
            session.document().events().len(),
            avant,
            "rien au journal : profondeur {profondeur}"
        );
    }
    // « à travers », lui, n'a pas de profondeur à valider — accepté
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::ATravers,
        couleur: None,
    };
    session.execute(&commande).expect("à travers accepté");
    assert_eq!(session.cad().features().len(), 1);
}

#[test]
fn add_feature_refuse_couleur_hors_bornes() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    // A5-04 (audit n°5, ADR-0036 D2) : composantes RGBA finies ET
    // dans 0..=1 — rouge 2.0 ou alpha négatif passaient au plan
    // (rouge D2b)
    for rgba in [
        [2.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0, -0.5],
        [0.2, 0.4, 0.6, 1.5],
    ] {
        let avant = session.document().events().len();
        let commande = AddFeature {
            croquis: 1,
            mode: ModeExtrusion::Addition,
            profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
            couleur: Some(CouleurFeature {
                rgba,
                speculaire: 0.5,
            }),
        };
        let erreur = session.execute(&commande).expect_err("couleur hors bornes");
        assert_eq!(erreur.code(), "OF-SES-0001", "rgba {rgba:?}");
        assert_eq!(
            session.document().events().len(),
            avant,
            "rien au journal : rgba {rgba:?}"
        );
    }
}

#[test]
fn add_feature_expression_pose_alias_auto_et_undo_groupe() {
    let mut session = Session::new("tests");
    session
        .execute(&SetInput {
            name: "epaisseur".to_string(),
            value: 2.0,
            unite: None,
        })
        .expect("entrée");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let avant = session.document().events().len();
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Expression {
            texte: "epaisseur * 2".to_string(),
        },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    // une commande, deux événements : l'alias cad.* et la feature,
    // un seul groupe d'undo (ADR-0011)
    let apres = session.document().events().len();
    assert_eq!(apres, avant + 2);
    let noeud = session
        .graph()
        .noeud("cad.extrusion0.profondeur")
        .expect("alias posé");
    assert!(noeud.systeme, "le nœud cad.* est système");
    session.undo().expect("undo du groupe entier");
    assert!(session.graph().noeud("cad.extrusion0.profondeur").is_none());
    assert!(session.cad().features().is_empty());
    session.redo().expect("redo du groupe entier");
    assert!(session.graph().noeud("cad.extrusion0.profondeur").is_some());
    assert_eq!(session.cad().features().len(), 1);
    // profondeur évaluée : 2 × epaisseur = 4 mm — volume analytique
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    let corps = session
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    let solide = corps.corps.expect("corps reconstruit");
    assert!(
        (solide.volume_mm3 - 4000.0).abs() < 1e-9,
        "volume attendu 4 000 mm³, obtenu {}",
        solide.volume_mm3
    );
}

#[test]
fn alias_cad_reserve_a_la_creation_utilisateur() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    // le préfixe cad. est réservé : ni expression, ni entrée
    let erreur = session
        .execute(&SetExpression {
            name: "cad.extrusion0.profondeur".to_string(),
            expression: "2".to_string(),
            unite: None,
        })
        .expect_err("préfixe cad. réservé");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(erreur.to_string().contains("OF-VAL-0008"));
    // un nœud cad.* posé par le système n'est pas supprimable
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Expression {
            texte: "2".to_string(),
        },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    let suppression = RemoveAlias {
        name: "cad.extrusion0.profondeur".to_string(),
        mode: ModeSuppression::Simple,
    };
    let erreur = session.execute(&suppression).expect_err("nœud système");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(erreur.to_string().contains("OF-VAL-0008"));
}

#[test]
fn update_feature_remplace_a_identifiant_conserve() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    // second croquis : le cercle de la future poche
    session
        .execute(&CreateSketch {
            nom: "poche".to_string(),
        })
        .expect("croquis 2");
    pose_cercle(&mut session, 20.0, 12.5, 5.0);
    let maj = UpdateFeature {
        id: 0,
        croquis: 2,
        mode: ModeExtrusion::Soustraction,
        profondeur: ProfondeurFeature::Fixe { mm: mm(12.0) },
        couleur: None,
    };
    session.execute(&maj).expect("feature mise à jour");
    // journal : cad.feature.updated, identifiant conservé
    let events = session.document().events();
    let dernier = events.last().expect("journal non vide");
    assert_eq!(dernier.event_type, EVENT_FEATURE_UPDATED);
    assert_eq!(dernier.data["id"], 0);
    // la projection a remplacé, pas ajouté
    let features = session.cad().features();
    assert_eq!(features.len(), 1);
    match &features[0].1 {
        FeatureCad::Extrusion { croquis, mode, .. } => {
            assert_eq!(*croquis, 2, "changement de croquis autorisé dès v1");
            assert_eq!(*mode, ModeExtrusion::Soustraction);
        }
        autre => panic!("feature extrusion attendue : {autre:?}"),
    }
}

#[test]
fn poche_a_travers_perce_le_corps() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "corps".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let corps = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(12.0) },
        couleur: None,
    };
    session.execute(&corps).expect("corps extrudé");
    session
        .execute(&CreateSketch {
            nom: "poche".to_string(),
        })
        .expect("croquis 2");
    pose_cercle(&mut session, 20.0, 12.5, 5.0);
    let poche = AddFeature {
        croquis: 2,
        mode: ModeExtrusion::Soustraction,
        profondeur: ProfondeurFeature::ATravers,
        couleur: None,
    };
    session.execute(&poche).expect("poche percée");
    // reconstruction : l'enveloppe « à travers » vaut 1,1 fois la
    // diagonale de la boîte du corps courant (40×25×12) — jamais
    // journalisée : dérivée déterministe (ADR-0024 D4)
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    let resultat = session
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    let solide = resultat.corps.expect("corps percé");
    let diagonale = (40.0_f64 * 40.0 + 25.0 * 25.0 + 12.0 * 12.0).sqrt();
    let attendu = 40.0 * 25.0 * 12.0 - std::f64::consts::PI * 25.0 * diagonale * 1.1;
    assert!(
        (solide.volume_mm3 - attendu).abs() < 1e-2 * attendu.abs(),
        "volume attendu ≈ {attendu}, obtenu {}",
        solide.volume_mm3
    );
}

#[test]
fn feature_en_echec_marquee_le_reste_du_corps_survit() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "corps".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let extrusion = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(12.0) },
        couleur: None,
    };
    session.execute(&extrusion).expect("corps extrudé");
    session
        .execute(&CreateSketch {
            nom: "poche".to_string(),
        })
        .expect("croquis 2");
    pose_cercle(&mut session, 20.0, 12.5, 5.0);
    // profondeur d'expression qui n'évalue pas : 1/0 (OF-VAL-0004)
    let poche = AddFeature {
        croquis: 2,
        mode: ModeExtrusion::Soustraction,
        profondeur: ProfondeurFeature::Expression {
            texte: "1 / 0".to_string(),
        },
        couleur: None,
    };
    session.execute(&poche).expect("feature posée");
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    assert!(
        !profondeurs.contains_key(&1),
        "la profondeur 1/0 n'évalue pas"
    );
    let resultat = session
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    // la feature 0 survit, la 1 est EN ÉCHEC typée OF-CAD-0005
    assert!(matches!(resultat.etats[0].1, EtatFeature::Ok));
    let code = resultat.etats[1].1.code().expect("échec typée");
    assert_eq!(code, "OF-CAD-0005");
    let solide = resultat.corps.expect("le corps rendu survit");
    assert!((solide.volume_mm3 - 12000.0).abs() < 1e-9);
}

#[test]
fn edition_cad_invalidee_par_toute_ecriture_au_journal() {
    let mut session = Session::new("tests");
    let avant = session.cad().edition();
    session
        .execute(&SetInput {
            name: "a".to_string(),
            value: 1.0,
            unite: None,
        })
        .expect("entrée");
    assert!(
        session.cad().edition() > avant,
        "toute écriture au journal invalide le fil de travail CAO"
    );
}

#[test]
fn rejeu_reconstruit_le_meme_corps() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "corps".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let extrusion = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    session.execute(&extrusion).expect("feature posée");
    // conteneur → relecture migrée → rejeu : la reconstruction est
    // identique (dérivé déterministe, ADR-0016 D8 étendu au CAO)
    let octets = session.save_to_bytes().expect("conteneur sérialisé");
    let document = Document::from_bytes_with(&octets, registre_migrations_courant())
        .expect("journal relu et migré");
    let mut relue = Session::from_document("tests", document).expect("rejeu");
    assert_eq!(relue.cad().features().len(), 1);
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = relue.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = relue.profondeurs_pour_cad();
    let resultat = relue
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    let solide = resultat.corps.expect("corps rejoué");
    assert!((solide.volume_mm3 - 4000.0).abs() <= 1e-6);
}

#[test]
fn rejeu_marque_en_echec_les_profondeurs_hors_bornes() {
    // A5-04 (audit n°5, ADR-0036) : journal hostile — la profondeur
    // fixe d'une feature est nulle, négative ou colossale. Le rejeu
    // marque la feature EN ÉCHEC OF-CAD-0005 ; le corps rendu par
    // les features valides survit.
    for profondeur in [0.0, -5.0, 1e300] {
        let mut document = Document::new();
        // rectangle sur le croquis implicite 0 — entités au schéma
        // v3 courant, même forme que la sérialisation d'esquisse
        for (id, x, y) in [
            (0, 0.0, 0.0),
            (1, 40.0, 0.0),
            (2, 40.0, 25.0),
            (3, 0.0, 25.0),
        ] {
            document.append(NewEvent::new(
                "sketch.entity.added",
                3,
                "tests",
                serde_json::json!({"genre": "point", "croquis": 0, "id": id, "x": x, "y": y}),
            ));
        }
        for (id, p1, p2) in [(4, 0, 1), (5, 1, 2), (6, 2, 3), (7, 3, 0)] {
            document.append(NewEvent::new(
                "sketch.entity.added",
                3,
                "tests",
                serde_json::json!({"genre": "ligne", "croquis": 0, "id": id, "p1": p1, "p2": p2}),
            ));
        }
        // feature valide (id 0) puis feature hostile (id 1) : même
        // forme que « cad.feature.added » v1, couleur absente
        document.append(NewEvent::new(
            "cad.feature.added",
            1,
            "tests",
            serde_json::json!({
                "id": 0,
                "feature": {
                    "type": "extrusion",
                    "croquis": 0,
                    "mode": "addition",
                    "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": 4.0}},
                }
            }),
        ));
        document.append(NewEvent::new(
            "cad.feature.added",
            1,
            "tests",
            serde_json::json!({
                "id": 1,
                "feature": {
                    "type": "extrusion",
                    "croquis": 0,
                    "mode": "addition",
                    "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": profondeur}},
                }
            }),
        ));
        let mut session = Session::from_document("tests", document).expect("rejeu tolérant");
        let mut solveur = Systeme2D::new().expect("solveur");
        let regions = session.croquis_figes_pour_cad(&mut solveur);
        let profondeurs = session.profondeurs_pour_cad();
        let resultat =
            session
                .cad()
                .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
        assert!(
            matches!(resultat.etats[0].1, EtatFeature::Ok),
            "profondeur {profondeur} : la feature valide doit passer"
        );
        let code = resultat.etats[1]
            .1
            .code()
            .unwrap_or_else(|| panic!("profondeur {profondeur} : échec typé attendu"));
        assert_eq!(code, "OF-CAD-0005", "profondeur {profondeur}");
        let solide = resultat
            .corps
            .unwrap_or_else(|| panic!("profondeur {profondeur} : le corps valide survit"));
        assert!(
            (solide.volume_mm3 - 4000.0).abs() < 1e-9,
            "profondeur {profondeur} : volume attendu 4 000 mm³, obtenu {}",
            solide.volume_mm3
        );
    }
}

#[test]
fn rejeu_marque_en_echec_les_couleurs_hors_bornes() {
    // A5-04 (audit n°5, ADR-0036 D2) : journal hostile — une
    // composante RGBA hors 0..=1. Le rejeu marque la feature EN
    // ÉCHEC OF-CAD-0005 ; le corps rendu par les features valides
    // survit.
    for rgba in [
        [2.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0, -0.5],
        [0.2, 0.4, 0.6, 1.5],
    ] {
        let mut document = Document::new();
        // rectangle sur le croquis implicite 0 — entités au schéma
        // v3 courant, même construction que le test des profondeurs
        for (id, x, y) in [
            (0, 0.0, 0.0),
            (1, 40.0, 0.0),
            (2, 40.0, 25.0),
            (3, 0.0, 25.0),
        ] {
            document.append(NewEvent::new(
                "sketch.entity.added",
                3,
                "tests",
                serde_json::json!({"genre": "point", "croquis": 0, "id": id, "x": x, "y": y}),
            ));
        }
        for (id, p1, p2) in [(4, 0, 1), (5, 1, 2), (6, 2, 3), (7, 3, 0)] {
            document.append(NewEvent::new(
                "sketch.entity.added",
                3,
                "tests",
                serde_json::json!({"genre": "ligne", "croquis": 0, "id": id, "p1": p1, "p2": p2}),
            ));
        }
        // feature valide (id 0, sans couleur) puis feature hostile
        // (id 1, couleur hors bornes) : même forme que
        // « cad.feature.added » v1
        document.append(NewEvent::new(
            "cad.feature.added",
            1,
            "tests",
            serde_json::json!({
                "id": 0,
                "feature": {
                    "type": "extrusion",
                    "croquis": 0,
                    "mode": "addition",
                    "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": 4.0}},
                }
            }),
        ));
        document.append(NewEvent::new(
            "cad.feature.added",
            1,
            "tests",
            serde_json::json!({
                "id": 1,
                "feature": {
                    "type": "extrusion",
                    "croquis": 0,
                    "mode": "addition",
                    "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": 4.0}},
                    "couleur": {"rgba": [rgba[0], rgba[1], rgba[2], rgba[3]], "speculaire": 0.5},
                }
            }),
        ));
        let mut session = Session::from_document("tests", document).expect("rejeu tolérant");
        let mut solveur = Systeme2D::new().expect("solveur");
        let regions = session.croquis_figes_pour_cad(&mut solveur);
        let profondeurs = session.profondeurs_pour_cad();
        let resultat =
            session
                .cad()
                .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
        assert!(
            matches!(resultat.etats[0].1, EtatFeature::Ok),
            "rgba {rgba:?} : la feature valide doit passer"
        );
        let code = resultat.etats[1]
            .1
            .code()
            .unwrap_or_else(|| panic!("rgba {rgba:?} : échec typé attendu"));
        assert_eq!(code, "OF-CAD-0005", "rgba {rgba:?}");
        let solide = resultat
            .corps
            .unwrap_or_else(|| panic!("rgba {rgba:?} : le corps valide survit"));
        assert!(
            (solide.volume_mm3 - 4000.0).abs() < 1e-9,
            "rgba {rgba:?} : volume attendu 4 000 mm³, obtenu {}",
            solide.volume_mm3
        );
    }
}

#[test]
fn rejeu_deux_fois_reconstruit_les_memes_volumes() {
    // Critère de sortie de l'ordre 1 (ADR-0036) : deux rejeux du
    // même conteneur reconstruisent des volumes identiques bit à
    // bit — les gardes de D1/D2 échouent de façon pure, elles ne
    // peuvent pas introduire de non-déterminisme au rejeu.
    // Journal forgé : un corps valide (id 0) et une poche hostile
    // (id 1, couleur hors bornes).
    let mut document = Document::new();
    // rectangle sur le croquis implicite 0 — entités au schéma
    // v3 courant, même construction que les tests hostiles
    for (id, x, y) in [
        (0, 0.0, 0.0),
        (1, 40.0, 0.0),
        (2, 40.0, 25.0),
        (3, 0.0, 25.0),
    ] {
        document.append(NewEvent::new(
            "sketch.entity.added",
            3,
            "tests",
            serde_json::json!({"genre": "point", "croquis": 0, "id": id, "x": x, "y": y}),
        ));
    }
    for (id, p1, p2) in [(4, 0, 1), (5, 1, 2), (6, 2, 3), (7, 3, 0)] {
        document.append(NewEvent::new(
            "sketch.entity.added",
            3,
            "tests",
            serde_json::json!({"genre": "ligne", "croquis": 0, "id": id, "p1": p1, "p2": p2}),
        ));
    }
    // feature valide (id 0, sans couleur) puis feature hostile
    // (id 1, couleur hors bornes) : même forme que
    // « cad.feature.added » v1
    document.append(NewEvent::new(
        "cad.feature.added",
        1,
        "tests",
        serde_json::json!({
            "id": 0,
            "feature": {
                "type": "extrusion",
                "croquis": 0,
                "mode": "addition",
                "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": 4.0}},
            }
        }),
    ));
    document.append(NewEvent::new(
        "cad.feature.added",
        1,
        "tests",
        serde_json::json!({
            "id": 1,
            "feature": {
                "type": "extrusion",
                "croquis": 0,
                "mode": "addition",
                "profondeur": {"type": "cote", "valeur": {"type": "fixe", "mm": 4.0}},
                "couleur": {"rgba": [2.0, 0.0, 0.0, 1.0], "speculaire": 0.5},
            }
        }),
    ));
    // un seul conteneur sérialisé, rejoué deux fois : chaque rejeu
    // relit et migre les mêmes octets
    let octets = document.to_bytes().expect("conteneur sérialisé");
    let mut volumes = [0.0_f64; 2];
    for (rejeu, volume) in volumes.iter_mut().enumerate() {
        let relu = Document::from_bytes_with(&octets, registre_migrations_courant())
            .expect("journal relu et migré");
        let mut rejouee = Session::from_document("tests", relu).expect("rejeu tolérant");
        let mut solveur = Systeme2D::new().expect("solveur");
        let regions = rejouee.croquis_figes_pour_cad(&mut solveur);
        let profondeurs = rejouee.profondeurs_pour_cad();
        let resultat =
            rejouee
                .cad()
                .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
        assert!(
            matches!(resultat.etats[0].1, EtatFeature::Ok),
            "rejeu {rejeu} : la feature valide doit passer"
        );
        let code = resultat.etats[1]
            .1
            .code()
            .unwrap_or_else(|| panic!("rejeu {rejeu} : échec typé attendu"));
        assert_eq!(code, "OF-CAD-0005", "rejeu {rejeu}");
        let solide = resultat
            .corps
            .unwrap_or_else(|| panic!("rejeu {rejeu} : le corps valide survit"));
        *volume = solide.volume_mm3;
    }
    assert_eq!(
        volumes[0].to_bits(),
        volumes[1].to_bits(),
        "deux rejeux du même conteneur : volumes identiques"
    );
    assert!(
        (volumes[0] - 4000.0).abs() < 1e-9,
        "volume attendu 4 000 mm³, obtenu {}",
        volumes[0]
    );
}

/// Moteur dont la boîte englobante est dégénérée (diagonale 0) :
/// l'enveloppe « à travers » résout une profondeur nulle — jamais
/// transmise au moteur, la garde D1 la marque en échec (ADR-0036
/// D2 : « à travers » suit D1).
struct MoteurBoiteDegeneree;

impl MoteurCad for MoteurBoiteDegeneree {
    type Solide = ();

    fn extruder(
        &self,
        _region: &Region,
        _mode: ModeExtrusion,
        _profondeur: Length,
    ) -> Result<(), ErreurMoteurCad> {
        Ok(())
    }

    fn fusionner(&self, _base: &(), _autre: &()) -> Result<(), ErreurMoteurCad> {
        Ok(())
    }

    fn soustraire(&self, _base: &(), _outil: &()) -> Result<(), ErreurMoteurCad> {
        Err(echec("soustraction sans objet"))
    }

    fn tesseller(&self, _solide: &(), _tolerance: Length) -> Result<Mesh, ErreurMoteurCad> {
        Ok(Mesh::new())
    }

    fn boite_englobante(&self, _solide: &()) -> Result<BoiteEnglobante, ErreurMoteurCad> {
        let coin = Point3::new(mm(0.0), mm(0.0), mm(0.0)).expect("origine");
        Ok(BoiteEnglobante {
            min: coin,
            max: coin,
        })
    }
}

#[test]
fn a_travers_sur_boite_degeneree_part_en_echec() {
    // ADR-0036 D2 : « à travers » suit D1 — une boîte dégénérée
    // (diagonale 0) résout une profondeur nulle : EnÉchec
    // OF-CAD-0005, jamais transmise au moteur ; le corps courant
    // survit.
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "corps".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    session
        .execute(&AddFeature {
            croquis: 1,
            mode: ModeExtrusion::Addition,
            profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
            couleur: None,
        })
        .expect("corps extrudé");
    session
        .execute(&AddFeature {
            croquis: 1,
            mode: ModeExtrusion::Addition,
            profondeur: ProfondeurFeature::ATravers,
            couleur: None,
        })
        .expect("à travers posée");
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    let resultat = session
        .cad()
        .reconstruire(&MoteurBoiteDegeneree, &regions, &profondeurs);
    assert!(matches!(resultat.etats[0].1, EtatFeature::Ok));
    let code = resultat.etats[1]
        .1
        .code()
        .unwrap_or_else(|| panic!("boîte dégénérée : échec typé attendu"));
    assert_eq!(code, "OF-CAD-0005");
    resultat.corps.expect("le corps valide survit");
}

#[test]
fn couleur_de_feature_journalisee() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: Some(CouleurFeature {
            rgba: [0.2, 0.4, 0.6, 1.0],
            speculaire: 0.5,
        }),
    };
    session.execute(&commande).expect("feature colorée");
    let events = session.document().events();
    let dernier = events.last().expect("journal non vide");
    assert_eq!(
        dernier.data["feature"]["couleur"]["rgba"],
        serde_json::json!([0.2, 0.4, 0.6, 1.0])
    );
    let speculaire = dernier.data["feature"]["couleur"]["speculaire"]
        .as_f64()
        .expect("spéculaire");
    assert!((speculaire - 0.5).abs() < 1e-9);
    let features = session.cad().features();
    match &features[0].1 {
        FeatureCad::Extrusion {
            couleur: Some(couleur),
            ..
        } => {
            assert_eq!(couleur.rgba, [0.2, 0.4, 0.6, 1.0]);
            assert!((couleur.speculaire - 0.5).abs() < 1e-9);
        }
        autre => panic!("feature extrusion attendue : {autre:?}"),
    }
}

#[test]
fn of_cad_0005_au_registre_central() {
    // garde bidirectionnelle : le code est stable et documenté au
    // registre public (gabarit groupe B)
    let erreur = ErreurMoteurCad::Echec {
        raison: "contour impropre".to_string(),
    };
    assert_eq!(erreur.code(), "OF-CAD-0005");
    assert!(
        REGISTRY.contains("| OF-CAD-0005 |"),
        "le code doit figurer au registre des codes publics"
    );
}

#[test]
fn add_feature_refuse_croquis_deux_regions_disjointes() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    // ADR-0037 D3 : une feature extrude exactement une région —
    // deux rectangles disjoints = deux régions, refus au plan
    pose_rectangle(&mut session, 0.0, 0.0, 10.0, 10.0);
    for (x, y) in [(20.0, 0.0), (30.0, 0.0), (30.0, 10.0), (20.0, 10.0)] {
        pose_point(&mut session, x, y);
    }
    for p1 in 8..12u32 {
        pose_ligne(&mut session, p1, 8 + (p1 + 1) % 4);
    }
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    let avant = session.document().events().len();
    let erreur = session
        .execute(&commande)
        .expect_err("croquis multi-régions refusé au plan");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("plusieurs régions"),
        "message attendu sur les régions, obtenu : {erreur}"
    );
    // refus au plan : rien au journal, rien projeté
    assert_eq!(session.document().events().len(), avant);
    assert!(session.cad().features().is_empty(), "rien projeté");
}

#[test]
fn update_feature_refuse_croquis_deux_regions() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    // une région unique : la feature 0 passe au plan
    pose_rectangle(&mut session, 0.0, 0.0, 10.0, 10.0);
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    // le même croquis porte désormais deux régions disjointes
    for (x, y) in [(20.0, 0.0), (30.0, 0.0), (30.0, 10.0), (20.0, 10.0)] {
        pose_point(&mut session, x, y);
    }
    for p1 in 8..12u32 {
        pose_ligne(&mut session, p1, 8 + (p1 + 1) % 4);
    }
    let maj = UpdateFeature {
        id: 0,
        croquis: 1,
        mode: ModeExtrusion::Soustraction,
        profondeur: ProfondeurFeature::Fixe { mm: mm(12.0) },
        couleur: None,
    };
    let avant = session.document().events().len();
    let erreur = session
        .execute(&maj)
        .expect_err("mise à jour multi-régions refusée au plan");
    assert_eq!(erreur.code(), "OF-SES-0001");
    assert!(
        erreur.to_string().contains("plusieurs régions"),
        "message attendu sur les régions, obtenu : {erreur}"
    );
    // refus non destructif : le journal est intact, la feature 0
    // survit, inchangée
    assert_eq!(session.document().events().len(), avant);
    assert_eq!(session.cad().features().len(), 1);
}

#[test]
fn anneau_extrude_aire_nette_et_rejeu_deux_fois() {
    let mut session = Session::new("tests");
    session
        .execute(&CreateSketch {
            nom: "croquis-1".to_string(),
        })
        .expect("croquis");
    // ADR-0037 D3 : rectangle 40×25 percé d'un trou rectangulaire
    // (15,7)-(25,17) — une seule région, l'aire nette vaut
    // 1000 - 100 = 900 mm²
    pose_rectangle(&mut session, 0.0, 0.0, 40.0, 25.0);
    for (x, y) in [(15.0, 7.0), (25.0, 7.0), (25.0, 17.0), (15.0, 17.0)] {
        pose_point(&mut session, x, y);
    }
    for p1 in 8..12u32 {
        pose_ligne(&mut session, p1, 8 + (p1 + 1) % 4);
    }
    let commande = AddFeature {
        croquis: 1,
        mode: ModeExtrusion::Addition,
        profondeur: ProfondeurFeature::Fixe { mm: mm(4.0) },
        couleur: None,
    };
    session.execute(&commande).expect("feature posée");
    // reconstruction : volume analytique = aire nette × profondeur
    let mut solveur = Systeme2D::new().expect("solveur");
    let regions = session.croquis_figes_pour_cad(&mut solveur);
    let profondeurs = session.profondeurs_pour_cad();
    let corps = session
        .cad()
        .reconstruire(&MoteurPrismesAnalytiques, &regions, &profondeurs);
    let solide = corps.corps.expect("corps reconstruit");
    assert!(
        (solide.volume_mm3 - 3600.0).abs() < 1e-9,
        "volume attendu 3 600 mm³ (extérieur moins trou), obtenu {}",
        solide.volume_mm3
    );
    assert_eq!(corps.etats.len(), 1);
    assert!(matches!(corps.etats[0].1, EtatFeature::Ok));
    // rejeu : les mêmes octets reconstruisent deux fois le même
    // volume — critère de sortie de l'ordre 1 (ADR-0036)
    let octets = session.document().to_bytes().expect("conteneur sérialisé");
    for rejeu in 0..2 {
        let relu = Document::from_bytes_with(&octets, registre_migrations_courant())
            .expect("journal relu et migré");
        let mut rejouee = Session::from_document("tests", relu).expect("rejeu tolérant");
        let mut solveur_rejeu = Systeme2D::new().expect("solveur");
        let regions_rejees = rejouee.croquis_figes_pour_cad(&mut solveur_rejeu);
        let profondeurs_rejees = rejouee.profondeurs_pour_cad();
        let rejoue = rejouee.cad().reconstruire(
            &MoteurPrismesAnalytiques,
            &regions_rejees,
            &profondeurs_rejees,
        );
        let solide = rejoue
            .corps
            .unwrap_or_else(|| panic!("rejeu {rejeu} : le corps doit survivre"));
        assert!(
            (solide.volume_mm3 - 3600.0).abs() < 1e-9,
            "rejeu {rejeu} : volume attendu 3 600 mm³, obtenu {}",
            solide.volume_mm3
        );
        assert!(
            matches!(rejoue.etats[0].1, EtatFeature::Ok),
            "rejeu {rejeu} : la feature doit passer"
        );
    }
}
