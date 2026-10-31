//! Croquis contraint 2D de la session (ADR-0018, jalon 2 phase D) :
//! entités, contraintes et graines de position au journal, positions
//! résolues comme état dérivé jamais sérialisé.
//!
//! - **journal** : quatre types d'événements (`sketch.entity.added`,
//!   `sketch.constraint.added`, `sketch.constraint.updated`,
//!   `sketch.point.moved`), identifiants
//!   séquentiels émis par la commande — l'ordre d'ajout est figé au
//!   journal et fait foi pour la reproductibilité du rejeu ;
//! - **graines** : positions de départ des points, posées par les
//!   glissements (`MovePoint`) — jamais la vérité : la vérité =
//!   entités + contraintes + dernières graines, toutes au journal ;
//! - **résolution dérivée** : [`EsquisseProjection::resoudre`]
//!   reconstruit le système depuis le journal et résout via un
//!   [`Solveur2D`] **injecté** (ADR-0018 D3 : forge-session
//!   n'instancie jamais le solveur natif, frontière LGPL de
//!   l'ADR-0013) — aucune position résolue n'est jamais écrite ni
//!   sérialisée.

use crate::SessionError;
use crate::command::{
    EVENT_SKETCH_CONSTRAINT_ADDED, EVENT_SKETCH_CONSTRAINT_UPDATED, EVENT_SKETCH_ENTITY_ADDED,
    EVENT_SKETCH_POINT_MOVED,
};
use crate::projection::{
    Projection, TypesIgnores, champ_entier, champ_nombre, champ_str, verifie_schema,
    verifie_schema_parmi,
};
use forge_doc::NewEvent;
use forge_gcs::{
    Algorithme, Contrainte, GcsError, IdCercle, IdLigne, IdPoint, Solveur2D, TagContrainte,
};
use forge_units::{Angle, Length};
use std::collections::BTreeMap;

/// Version de schéma courante de `sketch.entity.added` : la v2
/// (ADR-0020) donne à l'arc un balayage signé — `angle_fin` peut
/// précéder `angle_debut`, le sens de l'arc suit le geste —, la v1
/// reste lisible (balayage trigonométrique) pour les journaux non
/// migrés.
pub(crate) const VERSION_SCHEMA_ENTITE: u32 = 2;

/// Versions acceptées à la relecture d'une entité (ADR-0020) : les
/// journaux v1 non migrés rejouent, le registre aligne leurs arcs en
/// v2.
const VERSIONS_ENTITE: &[u32] = &[1, VERSION_SCHEMA_ENTITE];

/// Version de schéma de `sketch.point.moved` (ADR-0018) : non
/// concerné par le balayage signé des arcs (ADR-0020).
const VERSION_SCHEMA_DEPLACEMENT: u32 = 1;

/// Version de schéma courante de `sketch.constraint.added` : la v2
/// (ADR-0019) encode la cote de `Distance` comme objet typé — fixe
/// ou expression pilotée —, la v1 reste lisible (nombre nu) pour les
/// journaux non migrés.
const VERSION_SCHEMA_CONTRAINTE: u32 = 2;

/// Versions acceptées à la relecture d'une contrainte (ADR-0019 §1) :
/// les journaux v1 non migrés rejouent, le registre les aligne en v2.
const VERSIONS_CONTRAINTE: &[u32] = &[1, VERSION_SCHEMA_CONTRAINTE];

/// Seuil de coincidence de deux graines d'une même passe (re-test
/// du 2026-10-23) : en dessous, le gradient d'une cote de distance
/// aux graines devient indéfini — la passe répare la paire.
const SEUIL_GRAINES_CONFONDUES_MM: f64 = 1e-6;

/// Écart déterministe de la réparation des graines confondues
/// (re-test du 2026-10-23) : la paire est écartée vers +x, un
/// écart invisible à l'écran — les graines ne sont que des points
/// de départ.
const ECART_GRAINES_CONFONDUES_MM: f64 = 1e-3;
/// Borne de magnitude des coordonnées d'esquisse, en millimètres :
/// miroir de la constante équivalente de forge-geom (forge-session
/// ne dépend pas de forge-geom, la valeur est dupliquée à dessein) —
/// au-delà, la coordonnée est refusée (A4-13b, audit n°4).
const BORNE_COORDONNEE_MM: f64 = 1e9;

/// Version de schéma de `sketch.constraint.updated` : type nouveau de
/// la phase E, né en v1 — son champ `valeur` suit le schéma des
/// contraintes (objet typé en pratique, jamais un nombre nu émis).
const VERSION_SCHEMA_MISE_A_JOUR: u32 = 1;

/// Genre d'une entité du croquis (sélection et affichage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreEntite {
    /// Point libre.
    Point,
    /// Ligne entre deux points.
    Ligne,
    /// Cercle par centre et rayon.
    Cercle,
    /// Arc par centre, extrémités et angles.
    Arc,
}

/// Entité géométrique du croquis (ADR-0018 D1), sérialisable dans
/// l'événement `sketch.entity.added` : les coordonnées et cotes sont
/// des newtypes de forge-units, jamais de `f64` nu.
#[derive(Debug, Clone, PartialEq)]
pub enum EntiteEsquisse {
    /// Point libre.
    Point {
        /// Abscisse de départ (millimètres).
        x: Length,
        /// Ordonnée de départ (millimètres).
        y: Length,
    },
    /// Ligne entre deux points existants (extrémités partagées).
    Ligne {
        /// Premier point.
        p1: u32,
        /// Second point.
        p2: u32,
    },
    /// Cercle par centre et rayon.
    Cercle {
        /// Point centre.
        centre: u32,
        /// Rayon.
        rayon: Length,
    },
    /// Arc par centre, extrémités, rayon et angles aux extrémités.
    Arc {
        /// Point centre.
        centre: u32,
        /// Point de début.
        debut: u32,
        /// Point de fin.
        fin: u32,
        /// Rayon.
        rayon: Length,
        /// Angle de début (radians).
        angle_debut: Angle,
        /// Angle de fin (radians).
        angle_fin: Angle,
    },
}

/// Valeur d'une cote de `Distance` (ADR-0019, cotes pilotées) : une
/// longueur fixe ou une expression du moteur de liaisons — le nœud
/// `esquisse.<alias>` du graphe des valeurs porte l'expression, la
/// contrainte n'en est que la vitrine au croquis.
#[derive(Debug, Clone, PartialEq)]
pub enum ValeurCote {
    /// Longueur fixe.
    Fixe {
        /// Longueur de la cote (millimètres).
        mm: Length,
    },
    /// Expression pilotée du moteur de liaisons.
    Expression {
        /// Texte de l'expression (sous-ensemble déterministe).
        texte: String,
        /// Nom du nœud portant l'expression, sous `esquisse.`.
        alias: String,
    },
}

impl ValeurCote {
    /// Longueur fixe de la cote, ou `None` si elle est pilotée par
    /// une expression : la résolution exige alors de la figer d'abord
    /// ([`crate::Session::croquis_pour_resolution`]).
    pub fn mm(&self) -> Option<Length> {
        match self {
            ValeurCote::Fixe { mm } => Some(*mm),
            ValeurCote::Expression { .. } => None,
        }
    }
}

/// Contrainte géométrique du croquis (ADR-0018 D1) : les onze genres du
/// trait `forge_gcs::Contrainte` plus l'ancrage `PointFixe` (pont vers
/// `Solveur2D::fixer_point`). Les cotes sont des newtypes.
#[derive(Debug, Clone, PartialEq)]
pub enum ContrainteEsquisse {
    /// Coïncidence de deux points.
    Coincidence {
        /// Premier point.
        p1: u32,
        /// Second point.
        p2: u32,
    },
    /// Ligne horizontale.
    Horizontale {
        /// Ligne contrainte.
        ligne: u32,
    },
    /// Ligne verticale.
    Verticale {
        /// Ligne contrainte.
        ligne: u32,
    },
    /// Distance entre deux points (cote, pilotable en phase E).
    Distance {
        /// Premier point.
        p1: u32,
        /// Second point.
        p2: u32,
        /// Valeur de la cote : fixe ou expression pilotée (ADR-0019).
        valeur: ValeurCote,
    },
    /// Angle entre deux lignes (cote, pilotable en phase E).
    Angle {
        /// Première ligne.
        ligne1: u32,
        /// Seconde ligne.
        ligne2: u32,
        /// Valeur de l'angle.
        valeur: Angle,
    },
    /// Deux lignes de même longueur.
    MemeLongueur {
        /// Première ligne.
        ligne1: u32,
        /// Seconde ligne.
        ligne2: u32,
    },
    /// Point sur une ligne.
    PointSurLigne {
        /// Point contraint.
        point: u32,
        /// Ligne support.
        ligne: u32,
    },
    /// Deux lignes parallèles.
    Paralleles {
        /// Première ligne.
        ligne1: u32,
        /// Seconde ligne.
        ligne2: u32,
    },
    /// Deux lignes perpendiculaires.
    Perpendiculaires {
        /// Première ligne.
        ligne1: u32,
        /// Seconde ligne.
        ligne2: u32,
    },
    /// Rayon d'un cercle (cote, pilotable en phase E).
    RayonCercle {
        /// Cercle contraint.
        cercle: u32,
        /// Valeur du rayon.
        valeur: Length,
    },
    /// Point sur un cercle.
    PointSurCercle {
        /// Point contraint.
        point: u32,
        /// Cercle support.
        cercle: u32,
    },
    /// Point fixé : ancrage du croquis (pont vers
    /// `Solveur2D::fixer_point`, ADR-0018 D1).
    PointFixe {
        /// Point ancré.
        point: u32,
    },
}

/// Sérialise une entité pour l'événement `sketch.entity.added` :
/// schéma plat v1 — `genre`, `id`, puis les champs de l'entité
/// (coordonnées en millimètres, angles en radians).
pub(crate) fn entite_en_json(id: u32, entite: &EntiteEsquisse) -> serde_json::Value {
    match entite {
        EntiteEsquisse::Point { x, y } => serde_json::json!({
            "genre": "point",
            "id": id,
            "x": x.millimeters(),
            "y": y.millimeters(),
        }),
        EntiteEsquisse::Ligne { p1, p2 } => serde_json::json!({
            "genre": "ligne",
            "id": id,
            "p1": p1,
            "p2": p2,
        }),
        EntiteEsquisse::Cercle { centre, rayon } => serde_json::json!({
            "genre": "cercle",
            "id": id,
            "centre": centre,
            "rayon": rayon.millimeters(),
        }),
        EntiteEsquisse::Arc {
            centre,
            debut,
            fin,
            rayon,
            angle_debut,
            angle_fin,
        } => serde_json::json!({
            "genre": "arc",
            "id": id,
            "centre": centre,
            "debut": debut,
            "fin": fin,
            "rayon": rayon.millimeters(),
            "angle_debut": angle_debut.radians(),
            "angle_fin": angle_fin.radians(),
        }),
    }
}

/// Sérialise une contrainte pour les événements `sketch.constraint.added`
/// et `sketch.constraint.updated` : schéma plat — `genre`, `id`, puis
/// les champs de la contrainte (cotes en millimètres, angles en
/// radians) ; la cote de `Distance` est un objet typé depuis la v2
/// (ADR-0019).
pub(crate) fn contrainte_en_json(id: u32, contrainte: &ContrainteEsquisse) -> serde_json::Value {
    match contrainte {
        ContrainteEsquisse::Coincidence { p1, p2 } => serde_json::json!({
            "genre": "coincidence",
            "id": id,
            "p1": p1,
            "p2": p2,
        }),
        ContrainteEsquisse::Horizontale { ligne } => serde_json::json!({
            "genre": "horizontale",
            "id": id,
            "ligne": ligne,
        }),
        ContrainteEsquisse::Verticale { ligne } => serde_json::json!({
            "genre": "verticale",
            "id": id,
            "ligne": ligne,
        }),
        ContrainteEsquisse::Distance { p1, p2, valeur } => serde_json::json!({
            "genre": "distance",
            "id": id,
            "p1": p1,
            "p2": p2,
            "valeur": valeur_cote_en_json(valeur),
        }),
        ContrainteEsquisse::Angle {
            ligne1,
            ligne2,
            valeur,
        } => serde_json::json!({
            "genre": "angle",
            "id": id,
            "ligne1": ligne1,
            "ligne2": ligne2,
            "valeur": valeur.radians(),
        }),
        ContrainteEsquisse::MemeLongueur { ligne1, ligne2 } => serde_json::json!({
            "genre": "meme-longueur",
            "id": id,
            "ligne1": ligne1,
            "ligne2": ligne2,
        }),
        ContrainteEsquisse::PointSurLigne { point, ligne } => serde_json::json!({
            "genre": "point-sur-ligne",
            "id": id,
            "point": point,
            "ligne": ligne,
        }),
        ContrainteEsquisse::Paralleles { ligne1, ligne2 } => serde_json::json!({
            "genre": "paralleles",
            "id": id,
            "ligne1": ligne1,
            "ligne2": ligne2,
        }),
        ContrainteEsquisse::Perpendiculaires { ligne1, ligne2 } => serde_json::json!({
            "genre": "perpendiculaires",
            "id": id,
            "ligne1": ligne1,
            "ligne2": ligne2,
        }),
        ContrainteEsquisse::RayonCercle { cercle, valeur } => serde_json::json!({
            "genre": "rayon-cercle",
            "id": id,
            "cercle": cercle,
            "valeur": valeur.millimeters(),
        }),
        ContrainteEsquisse::PointSurCercle { point, cercle } => serde_json::json!({
            "genre": "point-sur-cercle",
            "id": id,
            "point": point,
            "cercle": cercle,
        }),
        ContrainteEsquisse::PointFixe { point } => serde_json::json!({
            "genre": "point-fixe",
            "id": id,
            "point": point,
        }),
    }
}

/// Sérialise une cote de distance (schéma v2, ADR-0019) : fixe
/// (`{"type": "fixe", "mm": …}`) ou expression pilotée
/// (`{"type": "expression", "texte": …, "alias": …}`).
fn valeur_cote_en_json(valeur: &ValeurCote) -> serde_json::Value {
    match valeur {
        ValeurCote::Fixe { mm } => serde_json::json!({
            "type": "fixe",
            "mm": mm.millimeters(),
        }),
        ValeurCote::Expression { texte, alias } => serde_json::json!({
            "type": "expression",
            "texte": texte,
            "alias": alias,
        }),
    }
}

/// Relit une cote de distance : nombre nu du schéma v1 (ADR-0018,
/// cote fixe) ou objet typé du schéma v2 — fixe ou expression
/// pilotée (ADR-0019).
fn valeur_cote_de_journal(event: &NewEvent) -> Result<ValeurCote, SessionError> {
    match event.data.get("valeur") {
        None => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « valeur » (cote) absent".to_string(),
        }),
        Some(serde_json::Value::Number(_)) => Ok(ValeurCote::Fixe {
            mm: Length::from_millimeters(champ_nombre(event, "valeur")?),
        }),
        Some(serde_json::Value::Object(objet)) => match objet.get("type") {
            Some(serde_json::Value::String(type_cote)) => match type_cote.as_str() {
                "fixe" => Ok(ValeurCote::Fixe {
                    mm: Length::from_millimeters(nombre_cote(event, objet, "mm")?),
                }),
                "expression" => Ok(ValeurCote::Expression {
                    texte: texte_cote(event, objet, "texte")?.to_string(),
                    alias: texte_cote(event, objet, "alias")?.to_string(),
                }),
                autre => Err(SessionError::InvalidEventData {
                    event_type: event.event_type.clone(),
                    reason: format!("type de cote inconnu : « {autre} »"),
                }),
            },
            _ => Err(SessionError::InvalidEventData {
                event_type: event.event_type.clone(),
                reason: "type de cote absent".to_string(),
            }),
        },
        Some(_) => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « valeur » ni nombre nu ni objet typé".to_string(),
        }),
    }
}

/// Lit un champ nombre d'un objet cote typé (schéma v2).
fn nombre_cote(
    event: &NewEvent,
    objet: &serde_json::Map<String, serde_json::Value>,
    champ: &str,
) -> Result<f64, SessionError> {
    objet
        .get(champ)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (nombre) absent de la cote"),
        })
}

/// Lit un champ texte d'un objet cote typé (schéma v2) : l'emprunt vit
/// sur l'objet, pas sur l'événement.
fn texte_cote<'a>(
    event: &NewEvent,
    objet: &'a serde_json::Map<String, serde_json::Value>,
    champ: &str,
) -> Result<&'a str, SessionError> {
    objet
        .get(champ)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (texte) absent de la cote"),
        })
}

/// Alias d'une cote pilotée, chaîne vide pour une cote fixe.
fn alias_de_cote(valeur: &ValeurCote) -> &str {
    match valeur {
        ValeurCote::Fixe { .. } => "",
        ValeurCote::Expression { alias, .. } => alias,
    }
}

/// Relit une entité depuis l'événement `sketch.entity.added` (le
/// champ `genre` choisit la variante, chaque champ est revalidé par
/// la projection après lecture).
fn entite_de_journal(event: &NewEvent) -> Result<(u32, EntiteEsquisse), SessionError> {
    let genre = champ_str(event, "genre")?;
    let id = champ_id(event, "id")?;
    let entite = match genre {
        "point" => EntiteEsquisse::Point {
            x: Length::from_millimeters(champ_nombre(event, "x")?),
            y: Length::from_millimeters(champ_nombre(event, "y")?),
        },
        "ligne" => EntiteEsquisse::Ligne {
            p1: champ_id(event, "p1")?,
            p2: champ_id(event, "p2")?,
        },
        "cercle" => EntiteEsquisse::Cercle {
            centre: champ_id(event, "centre")?,
            rayon: Length::from_millimeters(champ_nombre(event, "rayon")?),
        },
        "arc" => EntiteEsquisse::Arc {
            centre: champ_id(event, "centre")?,
            debut: champ_id(event, "debut")?,
            fin: champ_id(event, "fin")?,
            rayon: Length::from_millimeters(champ_nombre(event, "rayon")?),
            angle_debut: Angle::from_radians(champ_nombre(event, "angle_debut")?),
            angle_fin: Angle::from_radians(champ_nombre(event, "angle_fin")?),
        },
        _ => {
            return Err(SessionError::InvalidEventData {
                event_type: event.event_type.clone(),
                reason: format!("genre d'entité inconnu : « {genre} »"),
            });
        }
    };
    Ok((id, entite))
}

/// Relit une contrainte depuis l'événement `sketch.constraint.added`.
fn contrainte_de_journal(event: &NewEvent) -> Result<(u32, ContrainteEsquisse), SessionError> {
    let genre = champ_str(event, "genre")?;
    let id = champ_id(event, "id")?;
    let contrainte = match genre {
        "coincidence" => ContrainteEsquisse::Coincidence {
            p1: champ_id(event, "p1")?,
            p2: champ_id(event, "p2")?,
        },
        "horizontale" => ContrainteEsquisse::Horizontale {
            ligne: champ_id(event, "ligne")?,
        },
        "verticale" => ContrainteEsquisse::Verticale {
            ligne: champ_id(event, "ligne")?,
        },
        "distance" => ContrainteEsquisse::Distance {
            p1: champ_id(event, "p1")?,
            p2: champ_id(event, "p2")?,
            valeur: valeur_cote_de_journal(event)?,
        },
        "angle" => ContrainteEsquisse::Angle {
            ligne1: champ_id(event, "ligne1")?,
            ligne2: champ_id(event, "ligne2")?,
            valeur: Angle::from_radians(champ_nombre(event, "valeur")?),
        },
        "meme-longueur" => ContrainteEsquisse::MemeLongueur {
            ligne1: champ_id(event, "ligne1")?,
            ligne2: champ_id(event, "ligne2")?,
        },
        "point-sur-ligne" => ContrainteEsquisse::PointSurLigne {
            point: champ_id(event, "point")?,
            ligne: champ_id(event, "ligne")?,
        },
        "paralleles" => ContrainteEsquisse::Paralleles {
            ligne1: champ_id(event, "ligne1")?,
            ligne2: champ_id(event, "ligne2")?,
        },
        "perpendiculaires" => ContrainteEsquisse::Perpendiculaires {
            ligne1: champ_id(event, "ligne1")?,
            ligne2: champ_id(event, "ligne2")?,
        },
        "rayon-cercle" => ContrainteEsquisse::RayonCercle {
            cercle: champ_id(event, "cercle")?,
            valeur: Length::from_millimeters(champ_nombre(event, "valeur")?),
        },
        "point-sur-cercle" => ContrainteEsquisse::PointSurCercle {
            point: champ_id(event, "point")?,
            cercle: champ_id(event, "cercle")?,
        },
        "point-fixe" => ContrainteEsquisse::PointFixe {
            point: champ_id(event, "point")?,
        },
        _ => {
            return Err(SessionError::InvalidEventData {
                event_type: event.event_type.clone(),
                reason: format!("genre de contrainte inconnu : « {genre} »"),
            });
        }
    };
    Ok((id, contrainte))
}

/// Lit un champ identifiant (entier non négatif tenant dans un `u32`)
/// du schéma v1.
fn champ_id(event: &NewEvent, champ: &str) -> Result<u32, SessionError> {
    let valeur = champ_entier(event, champ)?;
    u32::try_from(valeur).map_err(|_| SessionError::InvalidEventData {
        event_type: event.event_type.clone(),
        reason: format!("champ « {champ} » hors des bornes de u32"),
    })
}

/// Genre d'une entité.
fn genre_de(entite: &EntiteEsquisse) -> GenreEntite {
    match entite {
        EntiteEsquisse::Point { .. } => GenreEntite::Point,
        EntiteEsquisse::Ligne { .. } => GenreEntite::Ligne,
        EntiteEsquisse::Cercle { .. } => GenreEntite::Cercle,
        EntiteEsquisse::Arc { .. } => GenreEntite::Arc,
    }
}

/// Erreur d'identifiant absent du croquis reconstruit dans le solveur.
fn manquant(entite: &str, id: u32) -> GcsError {
    GcsError::IdInconnu {
        contexte: format!("{entite} {id} n'existe pas dans le croquis reconstruit"),
    }
}

/// Reconstruction du croquis dans le solveur : identifiants solveur
/// par identifiant de croquis (l'ordre d'ajout fait foi).
#[derive(Debug, Default)]
struct Reconstruction {
    /// Points du croquis.
    points: BTreeMap<u32, IdPoint>,
    /// Lignes du croquis.
    lignes: BTreeMap<u32, IdLigne>,
    /// Cercles du croquis.
    cercles: BTreeMap<u32, IdCercle>,
}

impl Reconstruction {
    /// Identifiant solveur d'un point du croquis.
    fn id_point(&self, id: u32) -> Result<IdPoint, GcsError> {
        self.points
            .get(&id)
            .copied()
            .ok_or_else(|| manquant("le point", id))
    }

    /// Identifiant solveur d'une ligne du croquis.
    fn id_ligne(&self, id: u32) -> Result<IdLigne, GcsError> {
        self.lignes
            .get(&id)
            .copied()
            .ok_or_else(|| manquant("la ligne", id))
    }

    /// Identifiant solveur d'un cercle du croquis.
    fn id_cercle(&self, id: u32) -> Result<IdCercle, GcsError> {
        self.cercles
            .get(&id)
            .copied()
            .ok_or_else(|| manquant("le cercle", id))
    }
}

/// État dérivé d'un croquis résolu (ADR-0018 D3) : positions des
/// points et diagnostic. Jamais sérialisé : le rejeu du journal le
/// recalcule toujours.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CroquisResolu {
    /// Positions résolues des points, par identifiant de croquis.
    positions: BTreeMap<u32, (Length, Length)>,
    /// Degrés de liberté restants (0 : croquis entièrement contraint).
    dof: usize,
    /// Reproductibilité garantie par l'arène (FreeCAD #33399).
    reproductible: bool,
    /// Tags des contraintes conflictuelles (surentraînement).
    conflictuelles: Vec<TagContrainte>,
    /// Tags des contraintes redondantes.
    redondantes: Vec<TagContrainte>,
    /// Tags des contraintes partiellement redondantes.
    partiellement_redondantes: Vec<TagContrainte>,
    /// Tag solveur de chaque contrainte posée, par identifiant de
    /// contrainte du croquis ; l’ancrage `PointFixe` n’a pas de
    /// tag — ce n’est pas une contrainte du solveur, le diagnostic
    /// ne peut jamais le référencer.
    tags: BTreeMap<u32, TagContrainte>,
}

impl CroquisResolu {
    /// Position résolue d'un point du croquis, ou `None` si ce n'est
    /// pas un point connu.
    pub fn position(&self, id: u32) -> Option<(Length, Length)> {
        self.positions.get(&id).copied()
    }

    /// Degrés de liberté restants (0 : croquis entièrement contraint).
    pub fn dof(&self) -> usize {
        self.dof
    }

    /// Vrai si la reproductibilité des positions est garantie
    /// (adresses de paramètres croissantes, FreeCAD #33399).
    pub fn reproductible(&self) -> bool {
        self.reproductible
    }

    /// Tags des contraintes conflictuelles (surentraînement).
    pub fn conflictuelles(&self) -> &[TagContrainte] {
        &self.conflictuelles
    }

    /// Tags des contraintes redondantes.
    pub fn redondantes(&self) -> &[TagContrainte] {
        &self.redondantes
    }

    /// Tags des contraintes partiellement redondantes.
    pub fn partiellement_redondantes(&self) -> &[TagContrainte] {
        &self.partiellement_redondantes
    }

    /// Tag solveur de la contrainte du croquis, ou `None` si elle
    /// n’en a pas (ancrage `PointFixe`) ou est inconnue.
    pub fn tag(&self, id: u32) -> Option<TagContrainte> {
        self.tags.get(&id).copied()
    }
}

/// Capture d'ancrage en attente (ADR-0021 D2, révisée le
/// 2026-10-23) : le déplacement qui précède immédiatement la
/// contrainte `PointFixe` du même point, avec l'état de geste
/// d'avant ce déplacement — restauré à la consommation, la capture
/// d'ancrage n'étant pas un geste (re-test du 24-10 : la pose du
/// point fixé ne doit pas faire sauter la forme).
#[derive(Debug, Clone)]
struct CaptureAncrage {
    /// Point déplacé, futur point ancré.
    point: u32,
    /// Position affichée capturée : la graine du point ancré.
    position: (Length, Length),
    /// Dernier point déplacé avant le déplacement capturé.
    dernier_deplace: Option<u32>,
    /// Graine d'avant geste avant le déplacement capturé.
    graine_avant_dernier: Option<(Length, Length)>,
}

/// Paramètres d'une passe de résolution (cf.
/// [`EsquisseProjection::resoudre`]) : point épinglé au geste,
/// translation de suivi du geste, ancrages figés à leur position
/// résolue, graine substituée du repli, positions initiales
/// d'une reproduction d'affichage, ancrage différé et ancrage
/// exclu du système (re-test du 24-10).
#[derive(Clone, Copy)]
struct PasseDeResolution<'a> {
    /// Point épinglé au geste : sa graine tient, le reste absorbe.
    epingle: Option<u32>,
    /// Vecteur du geste : translate les graines non épinglées.
    translation: Option<(Length, Length)>,
    /// Ancrages figés à leur position résolue de la passe de base.
    ancres: Option<&'a [(u32, (Length, Length))]>,
    /// Graine substituée du repli de graine d'avant geste.
    graine_substituee: Option<(u32, (Length, Length))>,
    /// Positions résolues d'un affichage reproduit : graines de passe.
    positions_initiales: Option<&'a BTreeMap<u32, (Length, Length)>>,
    /// Les ancrages capturés ne sont figés qu'après la résolution.
    differe_captures: bool,
    /// Point dont la contrainte `PointFixe` est hors du système.
    exclu: Option<u32>,
}

impl PasseDeResolution<'_> {
    /// Passe neutre : aucune épingle, aucune translation, aucun
    /// ancrage figé — seules les graines du journal et l'exclusion.
    fn neutre(exclu: Option<u32>) -> Self {
        Self {
            epingle: None,
            translation: None,
            ancres: None,
            graine_substituee: None,
            positions_initiales: None,
            differe_captures: false,
            exclu,
        }
    }
}

/// Projection du croquis contraint (ADR-0018) : entités, contraintes
/// et graines de position rejouées depuis le journal ; les positions
/// résolues sont un état dérivé, calculé à la demande via
/// [`EsquisseProjection::resoudre`] et jamais sérialisé.
#[derive(Debug, Clone, Default)]
pub struct EsquisseProjection {
    /// Entités du croquis, dans l'ordre d'ajout (l'identifiant est
    /// l'indice d'ajout, ADR-0018 D1).
    entites: Vec<(u32, EntiteEsquisse)>,
    /// Contraintes du croquis, dans l'ordre d'ajout.
    contraintes: Vec<(u32, ContrainteEsquisse)>,
    /// Dernières positions de départ des points, posées par les
    /// glissements (identifiant du point → position germe).
    graines: BTreeMap<u32, (Length, Length)>,
    /// Dernier point déplacé par un glissement, dérivé du journal :
    /// la résolution l'épingle pour qu'il reste où on le lâche, le
    /// reste du croquis absorbant l'ajustement.
    dernier_deplace: Option<u32>,
    /// Position du dernier point déplacé juste AVANT son dernier
    /// glissement (graine précédente ou origine de l'entité) :
    /// avec la graine courante, elle donne le vecteur du dernier
    /// geste — un croquis rigide en translation le suit en entier.
    graine_avant_dernier: Option<(Length, Length)>,
    /// Types d'événements ignorés, plafonnés (audit n°4 A4-03).
    unknown: TypesIgnores,
    /// Ancrages capturés à la pose (ADR-0021 D2, révisée le
    /// 2026-10-23) : identifiant du point ancré → position affichée
    /// au moment de la pose, portée au journal par le
    /// `sketch.point.moved` qui précède immédiatement la contrainte
    /// de la même transaction (commande AncrerPoint). La graine du
    /// point EST la position capturée : la passe de base le fige
    /// avant la première résolution — la forme ne saute jamais.
    ancrages_captures: BTreeMap<u32, (Length, Length)>,
    /// Capture d'ancrage en attente (ADR-0021 D2, révisée le
    /// 2026-10-23) : posée par le dernier `sketch.point.moved`
    /// appliqué, consommée par la contrainte `PointFixe` du même
    /// point qui suit immédiatement — tout autre événement appliqué
    /// l'abandonne. L'état de geste d'avant le déplacement capturé
    /// voyage avec : la capture n'est pas un geste, la consommation
    /// lui rend ses droits — sinon le déplacement capturé devenait
    /// le dernier geste et la forme sautait à la pose (re-test du
    /// 24-10 : la passe épinglée du vrai geste disparaissait au
    /// profit de la seule passe de base).
    capture_en_attente: Option<CaptureAncrage>,
    /// Pose d'ancrage capturé pas encore suivie d'un autre événement
    /// de croquis (re-test du 24-10 : le rectangle flippe en
    /// symétrie autour du point fixé) : point ancré → position
    /// capturée. La résolution qui suit la pose reproduit d'abord
    /// l'affichage d'avant pose — le système sans cet ancrage,
    /// exactement la résolution qui tenait l'écran — puis fige
    /// l'ancre à sa capture sur ces positions : les graines du
    /// journal des autres points peuvent être loin de l'affiché
    /// (croquis rigide translaté par le geste) et DogLeg retiendrait
    /// la branche miroir la plus proche de ces graines périmées.
    /// Tout événement de croquis appliqué efface la fraîcheur.
    pose_ancrage_fraiche: Option<(u32, (Length, Length))>,
    /// Numéro d'édition : croît à chaque événement appliqué, jamais
    /// remis à zéro (clé de cache monotone pour l'application).
    edition: u64,
}

impl EsquisseProjection {
    /// Nouvelle projection vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Entités du croquis, dans l'ordre d'ajout au journal.
    pub fn entites(&self) -> &[(u32, EntiteEsquisse)] {
        &self.entites
    }

    /// Contraintes du croquis, dans l'ordre d'ajout au journal.
    pub fn contraintes(&self) -> &[(u32, ContrainteEsquisse)] {
        &self.contraintes
    }

    /// Un point porte-t-il un ancrage `PointFixe` (ADR-0021 D3) ?
    /// L'ancrage interdit le déplacement de sa graine : le point
    /// n'est plus une inconnue du solveur ni une poignée.
    pub fn point_ancre(&self, id: u32) -> bool {
        self.contraintes.iter().any(|(_, contrainte)| {
            matches!(contrainte, ContrainteEsquisse::PointFixe { point } if *point == id)
        })
    }

    /// Genre d'une entité du croquis, ou `None` si l'identifiant est
    /// inconnu.
    pub fn genre_entite(&self, id: u32) -> Option<GenreEntite> {
        self.entites
            .iter()
            .find(|(identifiant, _)| *identifiant == id)
            .map(|(_, entite)| genre_de(entite))
    }

    /// Position de départ d'un point du croquis (ADR-0018 D7) : la
    /// dernière graine posée par un déplacement `MovePoint`, les
    /// coordonnées d'origine de l'entité sinon — la vérité
    /// affichable quand aucune résolution n'est disponible.
    /// `None` si l'identifiant n'est pas un point du croquis.
    pub fn position_graine(&self, id: u32) -> Option<(Length, Length)> {
        if let Some((x, y)) = self.graines.get(&id) {
            return Some((*x, *y));
        }
        self.entites
            .iter()
            .find(|(identifiant, _)| *identifiant == id)
            .and_then(|(_, entite)| match entite {
                EntiteEsquisse::Point { x, y } => Some((*x, *y)),
                _ => None,
            })
    }

    /// Prochain identifiant d'entité (séquentiel : l'ordre d'ajout
    /// au journal fait foi, ADR-0018 D1).
    pub fn prochaine_id_entite(&self) -> u32 {
        self.entites.len() as u32
    }

    /// Prochain identifiant de contrainte (séquentiel).
    pub fn prochaine_id_contrainte(&self) -> u32 {
        self.contraintes.len() as u32
    }

    /// Alias des cotes pilotées du croquis (contraintes `Distance` en
    /// expression), dans l'ordre d'ajout (ADR-0019).
    pub fn alias_des_cotes(&self) -> Vec<String> {
        self.contraintes
            .iter()
            .filter_map(|(_, contrainte)| match contrainte {
                ContrainteEsquisse::Distance {
                    valeur: ValeurCote::Expression { alias, .. },
                    ..
                } => Some(alias.clone()),
                _ => None,
            })
            .collect()
    }

    /// Contrainte portant la cote pilotée de l'alias donné, avec son
    /// identifiant, ou `None` si aucune cote ne porte cet alias
    /// (ADR-0019) : la substitution d'alias réécrit la contrainte à
    /// identifiant conservé.
    pub fn contrainte_de_cote(&self, alias: &str) -> Option<(u32, ContrainteEsquisse)> {
        self.contraintes
            .iter()
            .find(|(_, contrainte)| {
                matches!(
                    contrainte,
                    ContrainteEsquisse::Distance {
                        valeur: ValeurCote::Expression { alias: a, .. },
                        ..
                    } if a.as_str() == alias
                )
            })
            .map(|(id, contrainte)| (*id, contrainte.clone()))
    }

    /// Fige une cote pilotée : son expression devient la longueur
    /// donnée, évaluée par le moteur de liaisons côté session — la
    /// résolution ne voit jamais une expression (frontière
    /// forge-values → forge-gcs, ADR-0019).
    pub fn figer_cote(&mut self, alias: &str, mm: Length) {
        for (_, contrainte) in &mut self.contraintes {
            if let ContrainteEsquisse::Distance { valeur, .. } = contrainte
                && let ValeurCote::Expression { alias: a, .. } = valeur
                && a.as_str() == alias
            {
                *valeur = ValeurCote::Fixe { mm };
            }
        }
    }

    /// Numéro d'édition du croquis : croît à chaque événement appliqué
    /// et n'est jamais remis à zéro — deux contenus différents du
    /// croquis portent toujours des éditions différentes.
    pub fn edition(&self) -> u64 {
        self.edition
    }

    /// Types d'événements ignorés par cette projection.
    pub fn unknown_types(&self) -> &[String] {
        self.unknown.retenus()
    }

    /// Raison du refus d'une entité au plan (ADR-0018 D2), ou `None`
    /// si elle est valide contre l'état courant.
    pub fn raison_refus_entite(&self, entite: &EntiteEsquisse) -> Option<String> {
        match entite {
            EntiteEsquisse::Point { x, y } => {
                if let Some(reason) = Self::raison_finie(x.millimeters(), "l'abscisse du point") {
                    return Some(reason);
                }
                Self::raison_finie(y.millimeters(), "l'ordonnée du point")
            }
            EntiteEsquisse::Ligne { p1, p2 } => {
                if let Some(reason) = self.raison_deux_points(*p1, *p2) {
                    return Some(reason);
                }
                if p1 == p2 {
                    return Some("une ligne relie deux points distincts".to_string());
                }
                None
            }
            EntiteEsquisse::Cercle { centre, rayon } => {
                if let Some(reason) = self.raison_point(*centre) {
                    return Some(reason);
                }
                Self::raison_positive(rayon.millimeters(), "le rayon du cercle")
            }
            EntiteEsquisse::Arc {
                centre,
                debut,
                fin,
                rayon,
                angle_debut,
                angle_fin,
            } => {
                if let Some(reason) = self.raison_point(*centre) {
                    return Some(reason);
                }
                if let Some(reason) = self.raison_point(*debut) {
                    return Some(reason);
                }
                if let Some(reason) = self.raison_point(*fin) {
                    return Some(reason);
                }
                if let Some(reason) = Self::raison_angle(angle_debut.radians()) {
                    return Some(reason);
                }
                if let Some(reason) = Self::raison_angle(angle_fin.radians()) {
                    return Some(reason);
                }
                Self::raison_positive(rayon.millimeters(), "le rayon de l'arc")
            }
        }
    }

    /// Raison du refus d'une contrainte au plan (ADR-0018 D2), ou
    /// `None` si elle est valide contre l'état courant.
    pub fn raison_refus_contrainte(&self, contrainte: &ContrainteEsquisse) -> Option<String> {
        match contrainte {
            ContrainteEsquisse::Coincidence { p1, p2 } => {
                if let Some(reason) = self.raison_deux_points(*p1, *p2) {
                    return Some(reason);
                }
                if p1 == p2 {
                    return Some("une coïncidence relie deux points distincts".to_string());
                }
                None
            }
            ContrainteEsquisse::Horizontale { ligne } => self.raison_ligne(*ligne),
            ContrainteEsquisse::Verticale { ligne } => self.raison_ligne(*ligne),
            ContrainteEsquisse::Distance { p1, p2, valeur } => {
                if let Some(reason) = self.raison_deux_points(*p1, *p2) {
                    return Some(reason);
                }
                if p1 == p2 {
                    return Some("une cote de distance relie deux points distincts".to_string());
                }
                Self::raison_cote(valeur)
            }
            ContrainteEsquisse::Angle {
                ligne1,
                ligne2,
                valeur,
            } => {
                if let Some(reason) = self.raison_deux_lignes(*ligne1, *ligne2) {
                    return Some(reason);
                }
                if ligne1 == ligne2 {
                    return Some("une cote d'angle relie deux lignes distinctes".to_string());
                }
                Self::raison_angle(valeur.radians())
            }
            ContrainteEsquisse::MemeLongueur { ligne1, ligne2 } => {
                if let Some(reason) = self.raison_deux_lignes(*ligne1, *ligne2) {
                    return Some(reason);
                }
                if ligne1 == ligne2 {
                    return Some(
                        "une égalité de longueur relie deux lignes distinctes".to_string(),
                    );
                }
                None
            }
            ContrainteEsquisse::PointSurLigne { point, ligne } => {
                if let Some(reason) = self.raison_point(*point) {
                    return Some(reason);
                }
                self.raison_ligne(*ligne)
            }
            ContrainteEsquisse::Paralleles { ligne1, ligne2 } => {
                if let Some(reason) = self.raison_deux_lignes(*ligne1, *ligne2) {
                    return Some(reason);
                }
                if ligne1 == ligne2 {
                    return Some("un parallélisme relie deux lignes distinctes".to_string());
                }
                None
            }
            ContrainteEsquisse::Perpendiculaires { ligne1, ligne2 } => {
                if let Some(reason) = self.raison_deux_lignes(*ligne1, *ligne2) {
                    return Some(reason);
                }
                if ligne1 == ligne2 {
                    return Some("une perpendicularité relie deux lignes distinctes".to_string());
                }
                None
            }
            ContrainteEsquisse::RayonCercle { cercle, valeur } => {
                if let Some(reason) = self.raison_cercle(*cercle) {
                    return Some(reason);
                }
                Self::raison_positive(valeur.millimeters(), "le rayon")
            }
            ContrainteEsquisse::PointSurCercle { point, cercle } => {
                if let Some(reason) = self.raison_point(*point) {
                    return Some(reason);
                }
                self.raison_cercle(*cercle)
            }
            ContrainteEsquisse::PointFixe { point } => self.raison_point(*point),
        }
    }

    /// Raison du refus d'un déplacement de point au plan (ADR-0018 D2),
    /// ou `None` si le point existe et les coordonnées sont finies.
    pub fn raison_refus_deplacement(&self, point: u32, x: Length, y: Length) -> Option<String> {
        if let Some(reason) = self.raison_point(point) {
            return Some(reason);
        }
        if let Some(reason) = Self::raison_finie(x.millimeters(), "l'abscisse du point") {
            return Some(reason);
        }
        Self::raison_finie(y.millimeters(), "l'ordonnée du point")
    }

    /// Résout le croquis : reconstruit le système depuis le journal
    /// (ordre d'ajout figé, reproductibilité garantie par l'arène
    /// de forge-gcs) et le résout via le solveur **injecté**
    /// (ADR-0018 D3 : forge-session n'instancie jamais le solveur
    /// natif). Algorithme par défaut : DogLeg, le plus robuste.
    ///
    /// Le dernier point déplacé reste où on le lâche : une seconde
    /// passe l'épingle (paramètres fixes) et laisse le reste du
    /// croquis absorber l'ajustement. Le diagnostic (degrés de
    /// liberté, conflits, redondances) reste celui de la passe de
    /// base ; la passe épinglée n'est retenue que si elle résout
    /// sans conflit, sinon on revient à la passe de base.
    ///
    /// Un croquis **rigide en translation** (deux degrés de liberté
    /// restants à la passe de base, aucun ancrage) suit le geste en
    /// entier : les graines de la passe épinglée partent translatées
    /// du vecteur du dernier geste — le carré suit le coin tiré, il
    /// ne s'arrête plus « à mi-chemin ».
    ///
    /// Les points ancrés (`PointFixe`) ne bougent jamais d'une passe
    /// à l'autre : la passe épinglée fige leur position résolue de
    /// la passe de base (révision d'ADR-0021 du 2026-10-22, CI #275).
    ///
    /// Un ancrage posé par `AncrerPoint` — qui capture la position
    /// affichée (ADR-0021 D2, révisée le 2026-10-23 — re-test jalon
    /// 2, point 8) — fige son point à la position capturée dès la
    /// passe de base : la forme ne saute pas entre l'affichage
    /// d'avant pose et celui d'après, le reste du croquis absorbe.
    /// La pose ne consomme pas l'état de geste : le dernier vrai
    /// geste garde sa passe épinglée (re-test du 24-10).
    ///
    /// Une passe de base que DogLeg ne vient pas à bout — graines
    /// dégénérées, typiquement un relâchement accroché sur un autre
    /// point — est rejouée depuis la graine d'avant geste : les
    /// graines ne sont que des points de départ (re-test du
    /// 2026-10-23, cf. passe_de_base). Un ancrage capturé posé en
    /// dernier peut seul mixer les graines — le point figé à sa
    /// position affichée, les autres aux graines du journal : sa
    /// passe est d'abord rejouée en difféant l'ancrage, la
    /// résolution ne doit jamais échouer quand le croquis a une
    /// solution (re-test du 24-10, cf. passe_de_base).
    ///
    /// La pose d'un ancrage capturé ne flippe jamais le croquis en
    /// symétrie autour du point fixé (re-test du 24-10) : la
    /// résolution reproduit d'abord l'affichage d'avant pose — le
    /// système sans l'ancrage frais, exactement la résolution qui
    /// tenait l'écran — puis fige l'ancre à sa capture sur ces
    /// positions ; les graines périmées du journal ne peuvent plus
    /// faire retenir la branche miroir (cf. `pose_ancrage_fraiche`).
    pub fn resoudre(&self, solveur: &mut dyn Solveur2D) -> Result<CroquisResolu, GcsError> {
        self.resoudre_interne(solveur, None)
    }

    /// Résolution interne : `exclu` sort un ancrage `PointFixe` du
    /// système pour cette résolution seulement — la reproduction de
    /// l'affichage d'avant pose d'un ancrage frais (re-test du
    /// 24-10, cf. `pose_ancrage_fraiche`).
    fn resoudre_interne(
        &self,
        solveur: &mut dyn Solveur2D,
        exclu: Option<u32>,
    ) -> Result<CroquisResolu, GcsError> {
        // Pose fraîche d'un ancrage capturé : reproduire l'affichage
        // d'avant pose, puis figer l'ancre à sa capture dessus. Le
        // moindre échec retombe sur le comportement ordinaire.
        if exclu.is_none()
            && let Some((point, position)) = self.pose_ancrage_fraiche
            && let Ok(avant_pose) = self.resoudre_interne(solveur, Some(point))
            && let Some(figee) = self.figee_l_ancrage_frais(solveur, point, position, &avant_pose)
        {
            return Ok(figee);
        }
        let base = self.passe_de_base(solveur, exclu)?;
        let ancres = self.ancrages_resolus(&base, exclu);
        let translation = if base.dof() == 2 && ancres.is_empty() {
            self.translation_du_dernier()
        } else {
            None
        };
        if let Some(point) = self.dernier_deplace
            && !(self.point_ancre(point) && exclu != Some(point))
            && self.graines.contains_key(&point)
            && let Ok(epingle) = self.passe_epinglee(solveur, point, translation, &ancres, exclu)
            && epingle.conflictuelles.is_empty()
        {
            return Ok(CroquisResolu {
                positions: epingle.positions,
                ..base
            });
        }
        Ok(base)
    }

    /// Fige l'ancrage frais sur l'affichage d'avant pose (re-test du
    /// 24-10 : le rectangle flippe en symétrie autour du point
    /// fixé) : les positions résolues de l'affichage reconstitué
    /// deviennent les graines de la passe, l'ancre part figée à sa
    /// capture, les autres ancrages à leur position affichée — le
    /// solveur ne peut plus retenir une branche miroir, la forme ne
    /// bouge pas d'un poil. `None` si la passe figée échoue ou
    /// conflictue : le comportement ordinaire reprend.
    fn figee_l_ancrage_frais(
        &self,
        solveur: &mut dyn Solveur2D,
        point: u32,
        position: (Length, Length),
        avant_pose: &CroquisResolu,
    ) -> Option<CroquisResolu> {
        let mut ancres = self.ancrages_resolus(avant_pose, None);
        match ancres.iter_mut().find(|(ancre, _)| *ancre == point) {
            Some(ancre) => ancre.1 = position,
            None => ancres.push((point, position)),
        }
        let figee = PasseDeResolution {
            ancres: Some(&ancres),
            positions_initiales: Some(&avant_pose.positions),
            ..PasseDeResolution::neutre(None)
        };
        let resolu = self.resoudre_passe(solveur, &figee).ok()?;
        if resolu.conflictuelles.is_empty() {
            Some(resolu)
        } else {
            None
        }
    }

    /// Aperçu de glissement (re-test du 2026-10-23) : une
    /// projection où la position visée devient la graine du point
    /// pour une résolution d'affichage SEULEMENT — rien au
    /// journal. La sémantique est exactement celle du relâchement
    /// (`MovePoint` puis [`EsquisseProjection::resoudre`]) :
    /// suivi du geste d'un croquis rigide, ancrages respectés,
    /// repli sur la passe de base. La forme liée suit le point
    /// saisi PENDANT le glissement au lieu d'attendre le
    /// relâchement. `None` si le point n'existe pas.
    pub fn apercu_de_glissement(
        &self,
        point: u32,
        x: Length,
        y: Length,
    ) -> Option<EsquisseProjection> {
        let avant = self.position_graine(point)?;
        let mut apercu = self.clone();
        apercu.graine_avant_dernier = Some(avant);
        apercu.graines.insert(point, (x, y));
        apercu.dernier_deplace = Some(point);
        // L'aperçu est un geste à lui tout seul : sa résolution suit
        // le comportement ordinaire, pas la fraîcheur de pose d'un
        // ancrage (re-test du 24-10, cf. `pose_ancrage_fraiche`).
        apercu.pose_ancrage_fraiche = None;
        Some(apercu)
    }

    /// Graine d'avant geste du point déplacé : le point de départ
    /// du repli quand les graines courantes font échouer la passe
    /// de base (re-test du 2026-10-23). Renvoie None si le journal
    /// ne détermine ni point déplacé ni graine d'avant.
    fn graine_avant_geste(&self) -> Option<(u32, (Length, Length))> {
        let point = self.dernier_deplace?;
        let avant = self.graine_avant_dernier?;
        Some((point, avant))
    }

    /// Passe de base de la résolution, avec replis : les graines ne
    /// sont que des points de départ, seul l'échec du DERNIER repli
    /// remonte. (1) Repli d'ancrage différé (re-test du 24-10) :
    /// l'ancrage capturé fige son point à sa position affichée
    /// AVANT la résolution — quand les autres points partent des
    /// graines du journal, DogLeg peut échouer sur ces graines
    /// mélangées et l'erreur remontait alors que le croquis a une
    /// solution ; la passe est rejouée en difféant l'ancrage : le
    /// croquis se résout d'abord SANS point figé (le système
    /// d'avant pose, celui qui tenait l'affichage), l'ancre est
    /// figée à sa position résolue — la capture — puis un second
    /// passage la confirme. (2) Repli de graine d'avant geste :
    /// graines dégénérées, typiquement un relâchement accroché sur
    /// un autre point — deux graines confondues rendent le gradient
    /// d'une cote de distance indéfini — la passe est rejouée
    /// depuis la graine d'avant geste du point déplacé. Un ancrage
    /// `exclu` (reproduction de l'affichage d'avant pose d'un
    /// ancrage frais, re-test du 24-10) ne fait partie du système
    /// d'aucune de ces passes.
    fn passe_de_base(
        &self,
        solveur: &mut dyn Solveur2D,
        exclu: Option<u32>,
    ) -> Result<CroquisResolu, GcsError> {
        let neutre = PasseDeResolution::neutre(exclu);
        match self.resoudre_passe(solveur, &neutre) {
            Ok(base) => Ok(base),
            Err(erreur) => {
                if !self.ancrages_captures.is_empty()
                    && let Ok(differee) = self.resoudre_passe(
                        solveur,
                        &PasseDeResolution {
                            differe_captures: true,
                            ..neutre
                        },
                    )
                {
                    return Ok(differee);
                }
                match self.graine_avant_geste() {
                    Some(graine) => self.resoudre_passe(
                        solveur,
                        &PasseDeResolution {
                            graine_substituee: Some(graine),
                            ..neutre
                        },
                    ),
                    None => Err(erreur),
                }
            }
        }
    }

    /// Répare les graines confondues d'une passe (re-test du
    /// 2026-10-23) : deux graines exactement superposées — un
    /// relâchement accroché sur un autre point, un carré affaissé
    /// par deux orientations posées sur des côtés opposés —
    /// rendent le gradient d'une cote de distance indéfini
    /// ((p1 − p2)/‖p1 − p2‖ = NaN) et DogLeg échoue alors que le
    /// croquis a une solution. Les graines ne sont que des points
    /// de départ : toute paire liée par une cote de distance,
    /// comme toute ligne de longueur nulle, part écartée d'un
    /// epsilon déterministe vers +x — jamais l'épinglé ni un point
    /// ancré, jamais le journal.
    fn reparer_graines_confondues(
        &self,
        graines: &mut BTreeMap<u32, (Length, Length)>,
        epingle: Option<u32>,
        ancres: Option<&[(u32, (Length, Length))]>,
    ) {
        let fige = |point: u32| {
            epingle.is_some_and(|e| e == point)
                || ancres.is_some_and(|a| a.iter().any(|(p, _)| *p == point))
        };
        let mut paires: Vec<(u32, u32)> = Vec::new();
        for (_, contrainte) in &self.contraintes {
            if let ContrainteEsquisse::Distance { p1, p2, .. } = contrainte {
                paires.push((*p1, *p2));
            }
        }
        for (_, entite) in &self.entites {
            if let EntiteEsquisse::Ligne { p1, p2 } = entite {
                paires.push((*p1, *p2));
            }
        }
        for (p1, p2) in paires {
            let Some((x1, y1)) = graines.get(&p1) else {
                continue;
            };
            let Some((x2, y2)) = graines.get(&p2) else {
                continue;
            };
            let ecart =
                (x2.millimeters() - x1.millimeters()).hypot(y2.millimeters() - y1.millimeters());
            if ecart >= SEUIL_GRAINES_CONFONDUES_MM {
                continue;
            }
            let cible = if fige(p2) { p1 } else { p2 };
            if let Some((graine_x, _)) = graines.get_mut(&cible) {
                let ecarte = graine_x.millimeters() + ECART_GRAINES_CONFONDUES_MM;
                *graine_x = Length::from_millimeters(ecarte);
            }
        }
    }

    /// Positions résolues des points ancrés (`PointFixe`) dans la
    /// passe de base : la passe épinglée fige ces positions — un
    /// point ancré ne bouge jamais d'une passe à l'autre, même
    /// quand le geste tire la forme (révision d'ADR-0021 du
    /// 2026-10-22, CI #275). Les ancrages capturés (ADR-0021 D2,
    /// révisée le 2026-10-23) ne changent rien ici : leur position
    /// capturée est déjà leur position résolue de la passe de base,
    /// le point étant figé à sa graine avant la résolution. Un
    /// ancrage `exclu` (re-test du 24-10, reproduction de
    /// l'affichage d'avant pose d'un ancrage frais) n'y figure
    /// pas : il ne fait pas partie du système de la passe.
    fn ancrages_resolus(
        &self,
        base: &CroquisResolu,
        exclu: Option<u32>,
    ) -> Vec<(u32, (Length, Length))> {
        let mut ancres = Vec::new();
        for (_, contrainte) in &self.contraintes {
            if let ContrainteEsquisse::PointFixe { point } = contrainte
                && Some(*point) != exclu
                && let Some(position) = base.position(*point)
            {
                ancres.push((*point, position));
            }
        }
        ancres
    }

    /// Passe épinglée : le point tiré garde sa graine et les points
    /// ancrés restent figés à leur position résolue de la passe de
    /// base — le conflit éventuel fait rejeter la passe (repli).
    fn passe_epinglee(
        &self,
        solveur: &mut dyn Solveur2D,
        point: u32,
        translation: Option<(Length, Length)>,
        ancres: &[(u32, (Length, Length))],
        exclu: Option<u32>,
    ) -> Result<CroquisResolu, GcsError> {
        self.resoudre_passe(
            solveur,
            &PasseDeResolution {
                epingle: Some(point),
                translation,
                ancres: Some(ancres),
                ..PasseDeResolution::neutre(exclu)
            },
        )
    }

    /// Vecteur du dernier geste : de la graine d'avant (graine
    /// précédente du point déplacé ou origine de son entité) vers sa
    /// graine courante. `None` si le journal ne le détermine pas.
    fn translation_du_dernier(&self) -> Option<(Length, Length)> {
        let point = self.dernier_deplace?;
        let avant = self.graine_avant_dernier?;
        let apres = self.graines.get(&point).copied()?;
        Some((
            Length::from_millimeters(apres.0.millimeters() - avant.0.millimeters()),
            Length::from_millimeters(apres.1.millimeters() - avant.1.millimeters()),
        ))
    }

    /// Une passe de résolution : reconstruit le système (solveur
    /// vidé au préalable — le même solveur peut servir à plusieurs
    /// passes), pose les contraintes, résout, puis relève positions
    /// et diagnostic. Si `epingle` désigne un point, il est fixé
    /// après la reconstruction : il n'est plus une inconnue et
    /// garde sa position germe. Si `translation` est donnée, les
    /// graines de tous les points SAUF l'épinglé partent translatées
    /// de ce vecteur (suivi du geste d'un croquis rigide).
    ///
    /// L'ancrage `PointFixe` non capturé (journaux hérités) se pose
    /// APRÈS la résolution de base : il épingle la position RÉSOLUE
    /// du point, celle que l'utilisateur voit — jamais sa graine,
    /// sinon la forme saute de plusieurs cases au moment de
    /// l'ancrage quand les cotes ont tiré le croquis loin de ses
    /// graines (retour checklist jalon 2, point 8) ; un second
    /// passage confirme la solution. L'ancrage CAPTURÉ (posé par
    /// `AncrerPoint`, ADR-0021 D2 révisée le 2026-10-23) se fige
    /// avant la première résolution : sa graine est la position
    /// affichée capturée à la pose — la forme ne saute jamais, le
    /// reste du croquis absorbe.
    ///
    /// En passe épinglée (`ancres` fourni), les points ancrés partent
    /// de leur position résolue de la passe de base et y restent
    /// figés : un geste qui les contredit fait rejeter la passe
    /// (repli sur la base) — l'ancrage ne suit jamais le geste.
    ///
    /// Si graine_substituee désigne un point, sa graine est
    /// remplacée par la position donnée pour cette passe seulement :
    /// le repli de la passe de base rejoue la résolution depuis la
    /// graine d'avant geste du point déplacé (re-test du 2026-10-23).
    ///
    /// Si differe_captures est vrai, les ancrages capturés ne sont
    /// PAS figés avant la résolution : ils suivent le sort des
    /// ancres non capturées — figés à leur position résolue après
    /// un premier passage, confirmés par un second. C'est le repli
    /// d'ancrage différé de la passe de base (re-test du 24-10) :
    /// la position résolue du premier passage est celle que tenait
    /// l'affichage avant la pose — la capture.
    ///
    /// Les graines confondues d'une paire liée par une cote de
    /// distance — ou d'une ligne de longueur nulle — sont écartées
    /// d'un epsilon déterministe avant la résolution : leur
    /// gradient est indéfini aux graines, DogLeg échouerait alors
    /// que le croquis a une solution (re-test du 2026-10-23).
    ///
    /// Si `positions_initiales` est fourni, les graines de la passe
    /// partent de ces positions résolues — la reproduction d'un
    /// affichage d'avant pose, figeage de l'ancrage frais (re-test
    /// du 24-10) ; si `exclu` désigne un point, sa contrainte
    /// `PointFixe` ne fait pas partie du système de la passe.
    fn resoudre_passe(
        &self,
        solveur: &mut dyn Solveur2D,
        passe: &PasseDeResolution,
    ) -> Result<CroquisResolu, GcsError> {
        if self.entites.is_empty() {
            return Ok(CroquisResolu {
                positions: BTreeMap::new(),
                dof: 0,
                reproductible: true,
                conflictuelles: Vec::new(),
                redondantes: Vec::new(),
                partiellement_redondantes: Vec::new(),
                tags: BTreeMap::new(),
            });
        }
        solveur.vider()?;
        let mut reconstruction = Reconstruction::default();
        // Graines de la passe (re-test du 2026-10-23) : journal — ou
        // position résolue de la passe de base pour un point ancré,
        // graine substituée du repli, positions initiales d'une
        // reproduction d'affichage (re-test du 24-10) — puis
        // translation du suivi du geste ; la réparation des graines
        // confondues s'applique en dernier, une fois la translation
        // posée.
        let mut graines_passe: BTreeMap<u32, (Length, Length)> = BTreeMap::new();
        for (id, entite) in &self.entites {
            if let EntiteEsquisse::Point { x, y } = entite {
                let graine = match passe.ancres.and_then(|a| a.iter().find(|(p, _)| p == id)) {
                    Some((_, position)) => *position,
                    None => match passe.graine_substituee {
                        Some((point, position)) if point == *id => position,
                        _ => match passe.positions_initiales {
                            Some(positions) => positions.get(id).copied().unwrap_or((*x, *y)),
                            None => self.graines.get(id).copied().unwrap_or((*x, *y)),
                        },
                    },
                };
                graines_passe.insert(*id, graine);
            }
        }
        self.reparer_graines_confondues(&mut graines_passe, passe.epingle, passe.ancres);
        for (id, entite) in &self.entites {
            match entite {
                EntiteEsquisse::Point { .. } => {
                    // Suivi du geste : les points non épinglés
                    // partent translatés du vecteur du dernier
                    // geste, l'épinglé reste à sa graine.
                    let (dx, dy) = match (passe.translation, passe.epingle) {
                        (Some((dx, dy)), Some(e)) if *id != e => (dx, dy),
                        _ => (Length::from_millimeters(0.0), Length::from_millimeters(0.0)),
                    };
                    let (graine_x, graine_y) = graines_passe
                        .get(id)
                        .copied()
                        .expect("graine de passe de tout point");
                    let identifiant = solveur.ajouter_point(
                        Length::from_millimeters(graine_x.millimeters() + dx.millimeters()),
                        Length::from_millimeters(graine_y.millimeters() + dy.millimeters()),
                    )?;
                    reconstruction.points.insert(*id, identifiant);
                }
                EntiteEsquisse::Ligne { p1, p2 } => {
                    let identifiant = solveur.ajouter_ligne(
                        reconstruction.id_point(*p1)?,
                        reconstruction.id_point(*p2)?,
                    )?;
                    reconstruction.lignes.insert(*id, identifiant);
                }
                EntiteEsquisse::Cercle { centre, rayon } => {
                    let identifiant =
                        solveur.ajouter_cercle(reconstruction.id_point(*centre)?, *rayon)?;
                    reconstruction.cercles.insert(*id, identifiant);
                }
                EntiteEsquisse::Arc {
                    centre,
                    debut,
                    fin,
                    rayon,
                    angle_debut,
                    angle_fin,
                } => {
                    // Aucune contrainte ne référence un arc en v1 : son
                    // identifiant solveur n'a pas besoin d'être gardé.
                    solveur.ajouter_arc(
                        reconstruction.id_point(*centre)?,
                        reconstruction.id_point(*debut)?,
                        reconstruction.id_point(*fin)?,
                        *rayon,
                        *angle_debut,
                        *angle_fin,
                    )?;
                }
            }
        }
        if let Some(id) = passe.epingle {
            solveur.fixer_point(reconstruction.id_point(id)?)?;
        }
        if let Some(ancres) = passe.ancres {
            for (point, _) in ancres {
                solveur.fixer_point(reconstruction.id_point(*point)?)?;
            }
        }
        let mut tags = BTreeMap::new();
        let mut ancrages = Vec::new();
        for (id, contrainte) in &self.contraintes {
            match contrainte {
                // L'ancrage capturé (ADR-0021 D2, révisée le
                // 2026-10-23 — re-test jalon 2, point 8) se fige avant
                // la première résolution : la graine du point EST la
                // position affichée capturée à la pose, la forme ne
                // saute pas, le reste absorbe. L'ancrage non capturé
                // (journaux hérités) s'épingle après la résolution de
                // base : il fige la position résolue du point (celle
                // que l'utilisateur voit), pas sa graine — la forme
                // ne saute jamais au moment de l'ancrage (retour
                // checklist jalon 2). Le repli d'ancrage différé
                // (re-test du 24-10) fait suivre aux ancrages
                // capturés le sort des hérités : figés à leur
                // position résolue après un premier passage sans
                // point figé — la résolution ne doit jamais échouer
                // sur des graines mélangées quand le croquis a une
                // solution.
                ContrainteEsquisse::PointFixe { point } => {
                    if passe.ancres.is_none() && Some(*point) != passe.exclu {
                        if !passe.differe_captures && self.ancrages_captures.contains_key(point) {
                            solveur.fixer_point(reconstruction.id_point(*point)?)?;
                        } else {
                            ancrages.push(reconstruction.id_point(*point)?);
                        }
                    }
                }
                _ => {
                    if let Some(tag) = applique_contrainte(solveur, &reconstruction, contrainte)? {
                        tags.insert(*id, tag);
                    }
                }
            }
        }
        solveur.resoudre(Algorithme::DogLeg)?;
        // L'ancrage n'altère le résultat que s'il existe : sans
        // PointFixe, un second passage ne servirait à rien et le
        // comportement reste exactement celui d'avant l'ancrage
        // post-résolution.
        if !ancrages.is_empty() {
            for point in ancrages {
                solveur.fixer_point(point)?;
            }
            solveur.resoudre(Algorithme::DogLeg)?;
        }
        let diagnostic = solveur.diagnostic(Algorithme::DogLeg)?;
        let mut positions = BTreeMap::new();
        for (id, identifiant) in &reconstruction.points {
            let position = solveur.position_point(*identifiant)?;
            positions.insert(*id, position);
        }
        Ok(CroquisResolu {
            positions,
            dof: diagnostic.dof(),
            reproductible: diagnostic.reproductible(),
            conflictuelles: diagnostic.conflictuelles().to_vec(),
            redondantes: diagnostic.redondantes().to_vec(),
            partiellement_redondantes: diagnostic.partiellement_redondantes().to_vec(),
            tags,
        })
    }

    /// Raison pour laquelle un identifiant n'est pas un point connu.
    fn raison_point(&self, id: u32) -> Option<String> {
        if self.genre_entite(id) == Some(GenreEntite::Point) {
            None
        } else {
            Some(format!("le point {id} n'existe pas"))
        }
    }

    /// Raison pour laquelle un identifiant n'est pas une ligne connue.
    fn raison_ligne(&self, id: u32) -> Option<String> {
        if self.genre_entite(id) == Some(GenreEntite::Ligne) {
            None
        } else {
            Some(format!("la ligne {id} n'existe pas"))
        }
    }

    /// Raison pour laquelle un identifiant n'est pas un cercle connu.
    fn raison_cercle(&self, id: u32) -> Option<String> {
        if self.genre_entite(id) == Some(GenreEntite::Cercle) {
            None
        } else {
            Some(format!("le cercle {id} n'existe pas"))
        }
    }

    /// Raison pour laquelle l'un des deux identifiants n'est pas un
    /// point connu.
    fn raison_deux_points(&self, p1: u32, p2: u32) -> Option<String> {
        self.raison_point(p1).or_else(|| self.raison_point(p2))
    }

    /// Raison pour laquelle l'un des deux identifiants n'est pas une
    /// ligne connue.
    fn raison_deux_lignes(&self, ligne1: u32, ligne2: u32) -> Option<String> {
        self.raison_ligne(ligne1)
            .or_else(|| self.raison_ligne(ligne2))
    }

    /// Raison pour laquelle une coordonnée n'est pas un nombre fini.
    fn raison_finie(valeur: f64, nom: &str) -> Option<String> {
        if !valeur.is_finite() {
            Some(format!("{nom} doit être un nombre fini"))
        } else if valeur.abs() > BORNE_COORDONNEE_MM {
            Some(format!("{nom} dépasse la borne de magnitude"))
        } else {
            None
        }
    }

    /// Raison pour laquelle une cote n'est pas finie et strictement
    /// positive.
    fn raison_positive(valeur: f64, nom: &str) -> Option<String> {
        if valeur.is_finite() && valeur > 0.0 {
            None
        } else {
            Some(format!("{nom} doit être finie et strictement positive"))
        }
    }

    /// Raison du refus d'une cote de distance (ADR-0019) : fixe
    /// invalide ou expression vide.
    fn raison_cote(valeur: &ValeurCote) -> Option<String> {
        match valeur {
            ValeurCote::Fixe { mm } => Self::raison_positive(mm.millimeters(), "la distance"),
            ValeurCote::Expression { texte, .. } => {
                if texte.trim().is_empty() {
                    Some("l'expression de la cote ne peut pas être vide".to_string())
                } else {
                    None
                }
            }
        }
    }

    /// Raison pour laquelle un angle n'est pas fini.
    fn raison_angle(valeur: f64) -> Option<String> {
        if valeur.is_finite() {
            None
        } else {
            Some("l'angle doit être fini".to_string())
        }
    }
}

/// Traduit une contrainte du croquis vers le solveur et l’ajoute
/// (ADR-0018 D1 : les onze genres du trait plus l’ancrage
/// `PointFixe`). Renvoie le tag solveur de la contrainte posée ;
/// l’ancrage n’a pas de tag — ce n’est pas une contrainte du
/// solveur, le diagnostic ne peut jamais le référencer.
fn applique_contrainte(
    solveur: &mut dyn Solveur2D,
    reconstruction: &Reconstruction,
    contrainte: &ContrainteEsquisse,
) -> Result<Option<TagContrainte>, GcsError> {
    match contrainte {
        ContrainteEsquisse::Coincidence { p1, p2 } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Coincidence {
                p1: reconstruction.id_point(*p1)?,
                p2: reconstruction.id_point(*p2)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Horizontale { ligne } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Horizontale {
                ligne: reconstruction.id_ligne(*ligne)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Verticale { ligne } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Verticale {
                ligne: reconstruction.id_ligne(*ligne)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Distance { p1, p2, valeur } => {
            // Frontière forge-values → forge-gcs (ADR-0019) : le
            // solveur ne voit jamais une expression — la session fige
            // les cotes pilotées avant de résoudre.
            let ValeurCote::Fixe { mm } = valeur else {
                return Err(GcsError::AbiInvalide {
                    contexte: format!(
                        "la cote pilotée « {} » doit être figée avant la résolution",
                        alias_de_cote(valeur)
                    ),
                });
            };
            let tag = solveur.ajouter_contrainte(Contrainte::Distance {
                p1: reconstruction.id_point(*p1)?,
                p2: reconstruction.id_point(*p2)?,
                valeur: *mm,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::PointSurLigne { point, ligne } => {
            let tag = solveur.ajouter_contrainte(Contrainte::PointSurLigne {
                point: reconstruction.id_point(*point)?,
                ligne: reconstruction.id_ligne(*ligne)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::MemeLongueur { ligne1, ligne2 } => {
            let tag = solveur.ajouter_contrainte(Contrainte::MemeLongueur {
                ligne1: reconstruction.id_ligne(*ligne1)?,
                ligne2: reconstruction.id_ligne(*ligne2)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Paralleles { ligne1, ligne2 } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Paralleles {
                ligne1: reconstruction.id_ligne(*ligne1)?,
                ligne2: reconstruction.id_ligne(*ligne2)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::PointSurCercle { point, cercle } => {
            let tag = solveur.ajouter_contrainte(Contrainte::PointSurCercle {
                point: reconstruction.id_point(*point)?,
                cercle: reconstruction.id_cercle(*cercle)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::RayonCercle { cercle, valeur } => {
            let tag = solveur.ajouter_contrainte(Contrainte::RayonCercle {
                cercle: reconstruction.id_cercle(*cercle)?,
                valeur: *valeur,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Perpendiculaires { ligne1, ligne2 } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Perpendiculaires {
                ligne1: reconstruction.id_ligne(*ligne1)?,
                ligne2: reconstruction.id_ligne(*ligne2)?,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::Angle {
            ligne1,
            ligne2,
            valeur,
        } => {
            let tag = solveur.ajouter_contrainte(Contrainte::Angle {
                ligne1: reconstruction.id_ligne(*ligne1)?,
                ligne2: reconstruction.id_ligne(*ligne2)?,
                valeur: *valeur,
            })?;
            Ok(Some(tag))
        }
        ContrainteEsquisse::PointFixe { point } => {
            solveur.fixer_point(reconstruction.id_point(*point)?)?;
            Ok(None)
        }
    }
}

impl Projection for EsquisseProjection {
    fn name(&self) -> &'static str {
        "esquisse"
    }

    fn reset(&mut self) {
        self.entites.clear();
        self.contraintes.clear();
        self.graines.clear();
        self.dernier_deplace = None;
        self.graine_avant_dernier = None;
        self.unknown.clear();
        self.ancrages_captures.clear();
        self.capture_en_attente = None;
        self.pose_ancrage_fraiche = None;
        // `edition` n'est jamais remis à zéro : clé de cache monotone —
        // deux contenus différents du croquis portent des éditions
        // différentes, même après un rejeu complet.
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        // Capture d'ancrage (ADR-0021 D2, révisée) : le déplacement
        // posé par l'événement précédent ne survit qu'à la contrainte
        // `PointFixe` du même point qui le suit immédiatement — tout
        // autre événement l'abandonne.
        let capture = self.capture_en_attente.take();
        // La fraîcheur de pose (re-test du 24-10, cf.
        // `pose_ancrage_fraiche`) ne survit à aucun événement de
        // croquis : seule la pose d'un ancrage capturé la pose.
        let _pose_fraiche = self.pose_ancrage_fraiche.take();
        match event.event_type.as_str() {
            EVENT_SKETCH_ENTITY_ADDED => {
                verifie_schema_parmi(event, VERSIONS_ENTITE)?;
                let (id, entite) = entite_de_journal(event)?;
                if id != self.entites.len() as u32 {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("identifiant d'entité {id} non séquentiel"),
                    });
                }
                if let Some(reason) = self.raison_refus_entite(&entite) {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason,
                    });
                }
                self.entites.push((id, entite));
                self.edition += 1;
                Ok(())
            }
            EVENT_SKETCH_CONSTRAINT_ADDED => {
                verifie_schema_parmi(event, VERSIONS_CONTRAINTE)?;
                let (id, contrainte) = contrainte_de_journal(event)?;
                if id != self.contraintes.len() as u32 {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("identifiant de contrainte {id} non séquentiel"),
                    });
                }
                if let Some(reason) = self.raison_refus_contrainte(&contrainte) {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason,
                    });
                }
                // Capture d'ancrage (ADR-0021 D2, révisée) : le
                // `PointFixe` du point déplacé à l'événement précédent
                // fige la position affichée au moment de la pose — et
                // rend ses droits à l'état de geste d'avant le
                // déplacement capturé : l'ancrage n'est pas un
                // geste, la passe épinglée du dernier vrai geste
                // continue de tenir l'affichage (re-test du 24-10 :
                // la forme sautait à la pose du point fixé).
                if let ContrainteEsquisse::PointFixe { point } = &contrainte
                    && let Some(capture) = &capture
                    && capture.point == *point
                {
                    self.dernier_deplace = capture.dernier_deplace;
                    self.graine_avant_dernier = capture.graine_avant_dernier;
                    self.ancrages_captures.insert(*point, capture.position);
                    self.pose_ancrage_fraiche = Some((*point, capture.position));
                }
                self.contraintes.push((id, contrainte));
                self.edition += 1;
                Ok(())
            }
            EVENT_SKETCH_CONSTRAINT_UPDATED => {
                verifie_schema(event, VERSION_SCHEMA_MISE_A_JOUR)?;
                let (id, contrainte) = contrainte_de_journal(event)?;
                let Some(position) = self
                    .contraintes
                    .iter()
                    .position(|(identifiant, _)| *identifiant == id)
                else {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("contrainte {id} inconnue au croquis"),
                    });
                };
                if let Some(reason) = self.raison_refus_contrainte(&contrainte) {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason,
                    });
                }
                // Identifiant conservé : seule la contrainte change,
                // l'ordre d'ajout au journal fait foi (ADR-0019 §1).
                self.contraintes[position].1 = contrainte;
                self.edition += 1;
                Ok(())
            }
            EVENT_SKETCH_POINT_MOVED => {
                verifie_schema(event, VERSION_SCHEMA_DEPLACEMENT)?;
                let point = champ_id(event, "point")?;
                let x = Length::from_millimeters(champ_nombre(event, "x")?);
                let y = Length::from_millimeters(champ_nombre(event, "y")?);
                if let Some(reason) = self.raison_refus_deplacement(point, x, y) {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason,
                    });
                }
                // Capture d'ancrage (ADR-0021 D2, révisée) : le
                // déplacement reste en attente de la contrainte
                // `PointFixe` qui suit immédiatement, s'il vient —
                // l'état de geste d'avant voyage avec : la capture
                // n'est pas un geste (re-test du 24-10).
                let capture = CaptureAncrage {
                    point,
                    position: (x, y),
                    dernier_deplace: self.dernier_deplace,
                    graine_avant_dernier: self.graine_avant_dernier,
                };
                self.graine_avant_dernier = self.position_graine(point);
                self.graines.insert(point, (x, y));
                self.dernier_deplace = Some(point);
                self.capture_en_attente = Some(capture);
                self.edition += 1;
                Ok(())
            }
            _ => {
                self.unknown.noter(&event.event_type);
                Ok(())
            }
        }
    }

    fn types_connus(&self) -> &'static [&'static str] {
        &[
            EVENT_SKETCH_ENTITY_ADDED,
            EVENT_SKETCH_CONSTRAINT_ADDED,
            EVENT_SKETCH_CONSTRAINT_UPDATED,
            EVENT_SKETCH_POINT_MOVED,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A4-13b (audit n°4) : une coordonnée finie mais au-delà de la
    /// borne commune est refusée au plan, comme une coordonnée non
    /// finie — un point à 10^300 mm empoisonnerait la caméra.
    #[test]
    fn point_au_dela_de_la_borne_refuse() {
        let projection = EsquisseProjection::new();
        let point = EntiteEsquisse::Point {
            x: Length::from_millimeters(1e300),
            y: Length::ZERO,
        };
        assert!(
            projection.raison_refus_entite(&point).is_some(),
            "une coordonnée hors borne doit être refusée"
        );
    }
}
