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
    Projection, champ_entier, champ_nombre, champ_str, verifie_schema, verifie_schema_parmi,
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
    /// Types d'événements ignorés, uniques, dans l'ordre de rencontre.
    unknown: Vec<String>,
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
        &self.unknown
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
    pub fn resoudre(&self, solveur: &mut dyn Solveur2D) -> Result<CroquisResolu, GcsError> {
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
        let mut reconstruction = Reconstruction::default();
        for (id, entite) in &self.entites {
            match entite {
                EntiteEsquisse::Point { x, y } => {
                    let graine = self.graines.get(id).copied().unwrap_or((*x, *y));
                    let identifiant = solveur.ajouter_point(graine.0, graine.1)?;
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
        let mut tags = BTreeMap::new();
        for (id, contrainte) in &self.contraintes {
            if let Some(tag) = applique_contrainte(solveur, &reconstruction, contrainte)? {
                tags.insert(*id, tag);
            }
        }
        solveur.resoudre(Algorithme::DogLeg)?;
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
        if valeur.is_finite() {
            None
        } else {
            Some(format!("{nom} doit être un nombre fini"))
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
        self.unknown.clear();
        // `edition` n'est jamais remis à zéro : clé de cache monotone —
        // deux contenus différents du croquis portent des éditions
        // différentes, même après un rejeu complet.
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
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
                self.graines.insert(point, (x, y));
                self.edition += 1;
                Ok(())
            }
            _ => {
                if !self.unknown.contains(&event.event_type) {
                    self.unknown.push(event.event_type.clone());
                }
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
