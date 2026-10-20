//! Commandes de la session (ADR-0008, ADR-0011) : une intention
//! utilisateur = une validation contre l'état courant, puis 1..n
//! événements et d'éventuels blobs à déposer.
//!
//! Phase E (ADR-0019) : les liaisons passent au schéma v2 — chaque
//! valeur déclare son unité (ADR-0007 phase 1) — la suppression
//! d'alias arrive en trois modes ([`RemoveAlias`]) et les cotes
//! d'esquisse se pilotent par expressions
//! ([`crate::esquisse::ValeurCote`]).

use crate::esquisse::{ContrainteEsquisse, EntiteEsquisse, ValeurCote};
use crate::{Session, SessionError};
use forge_doc::NewEvent;
use forge_units::Length;
use forge_values::ValuesError;
use forge_values::{ModeSuppression, PREFIXES_RESERVES, Unite, remplacer_identifiants};
use std::collections::BTreeSet;

/// Type d'événement : valeur d'entrée fixée (schéma v2 : `name`,
/// `value`, `unit` — ADR-0019, ADR-0007 phase 1).
pub const EVENT_INPUT_SET: &str = "value.input_set";
/// Type d'événement : expression de liaison fixée (schéma v2 :
/// `name`, `expression`, `unit`).
pub const EVENT_EXPRESSION_SET: &str = "value.expression_set";
/// Type d'événement : alias supprimé du graphe des liaisons
/// (schéma v1 : `name` — un événement par alias, ADR-0019).
pub const EVENT_ALIAS_REMOVED: &str = "value.alias_removed";
/// Type d'événement : tracés importés (schéma v1 : `format`, `blob`,
/// `traces`, `sommets`).
pub const EVENT_TRACES_IMPORTED: &str = "geom.traces.imported";
/// Type d'événement : maillage importé (schéma v2 : `format`, `blob`,
/// `triangles`, `echelle` — ADR-0017).
pub const EVENT_MESH_IMPORTED: &str = "geom.mesh.imported";
/// Type d'événement : entité ajoutée au croquis contraint (schéma
/// v1 : `genre`, `id`, puis les champs de l'entité — ADR-0018).
pub const EVENT_SKETCH_ENTITY_ADDED: &str = "sketch.entity.added";
/// Type d'événement : contrainte ajoutée au croquis contraint
/// (schéma v2 : `genre`, `id`, puis les champs de la contrainte ;
/// la cote `distance` est polymorphe — ADR-0019).
pub const EVENT_SKETCH_CONSTRAINT_ADDED: &str = "sketch.constraint.added";
/// Type d'événement : contrainte remplacée à identifiant conservé
/// (schéma v1, même forme que `sketch.constraint.added` v2 — la
/// projection remplace en place, ADR-0019).
pub const EVENT_SKETCH_CONSTRAINT_UPDATED: &str = "sketch.constraint.updated";
/// Type d'événement : germe de position d'un point du croquis
/// (schéma v1 : `point`, `x`, `y` — ADR-0018).
pub const EVENT_SKETCH_POINT_MOVED: &str = "sketch.point.moved";

/// Production d'une commande : ses événements et les blobs à déposer.
/// L'exécution est tout-ou-rien (ADR-0011) : si une projection refuse
/// ou un blob est refusé, ni blobs ni événements ne sont écrits.
#[derive(Debug, Clone, Default)]
pub struct CommandPlan {
    /// Événements à écrire au journal (1..n).
    pub events: Vec<NewEvent>,
    /// Blobs à déposer (nom, octets) avant l'écriture des événements.
    pub blobs: Vec<(String, Vec<u8>)>,
}

impl CommandPlan {
    /// Plan sans blob (ergonomie des commandes du domaine des liaisons).
    pub fn new(events: Vec<NewEvent>) -> Self {
        Self {
            events,
            blobs: Vec::new(),
        }
    }
}

/// Une intention utilisateur. La commande se **planifie** contre l'état
/// courant : elle valide ses données, puis produit 1..n événements et
/// d'éventuels blobs. La session attribue ensuite à chaque événement
/// l'acteur et un identifiant de transaction (une commande = un groupe
/// d'undo) — la commande ne s'en occupe pas.
pub trait Command {
    /// Valide la commande contre l'état courant et produit son plan.
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError>;
}

/// Vérifie qu'un nom de valeur est référençable par le lexeur
/// d'expressions (ADR-0016 A8) : `[A-Za-z_][A-Za-z0-9_]*`, à élargir
/// en Unicode à l'implémentation de l'ADR-0006. Un nom hors de ce
/// jeu n'est pas créable : il ne serait jamais référençable.
fn verifie_nom(name: &str) -> Result<(), SessionError> {
    let mut caracteres = name.chars();
    let premier = caracteres.next();
    let premier_valide = premier.is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let reste_valide = caracteres.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if premier_valide && reste_valide {
        return Ok(());
    }
    Err(SessionError::CommandRejected {
        reason: format!("le nom « {name} » n'est pas un identifiant référençable"),
    })
}

/// Vérifie qu'un nom en chemin à points (ADR-0006 : espaces de noms
/// par point) est référençable : chaque segment est un identifiant
/// référençable (ADR-0016 A8).
fn verifie_nom_chemin(name: &str) -> Result<(), SessionError> {
    for segment in name.split('.') {
        verifie_nom(segment)?;
    }
    Ok(())
}

/// Refuse la création utilisateur sous un préfixe réservé (ADR-0019
/// §2 : OF-VAL-0008) — les nœuds système ne se créent que par les
/// commandes qui en ont la charge (cotes d'esquisse).
fn verifie_prefixe_libre(name: &str) -> Result<(), SessionError> {
    if PREFIXES_RESERVES.iter().any(|p| name.starts_with(p)) {
        let reserve = ValuesError::AliasReserve {
            name: name.to_string(),
        };
        return Err(SessionError::CommandRejected {
            reason: reserve.to_string(),
        });
    }
    Ok(())
}

/// Rejoue une erreur du moteur de liaisons en refus de commande :
/// le message porte le code OF-VAL-*, stable et traduisible côté
/// interface (ADR-0003).
fn refus_values(erreur: ValuesError) -> SessionError {
    SessionError::CommandRejected {
        reason: erreur.to_string(),
    }
}

/// Événement `value.alias_removed` : l'alias nommé quitte le graphe
/// à l'application (la projection retire le nœud sans vérification —
/// le plan a validé, le journal fait foi, ADR-0019 §1).
fn evenement_alias_retire(name: &str) -> NewEvent {
    NewEvent::new(
        EVENT_ALIAS_REMOVED,
        1,
        "",
        serde_json::json!({ "name": name }),
    )
}

/// Fixe la valeur d'entrée nommée du graphe de liaisons.
///
/// L'unité déclarée (ADR-0007 phase 1) est une métadonnée de
/// validation, jamais un état d'exécution : elle vit dans l'événement
/// `value.input_set` schéma v2 et le graphe la vérifie à la
/// définition ; les newtypes de forge-units restent la frontière
/// programmatique des grandeurs dimensionnées.
#[derive(Debug, Clone)]
pub struct SetInput {
    /// Nom de la valeur.
    pub name: String,
    /// Valeur d'entrée ; un nombre non fini est refusé au plan.
    pub value: f64,
    /// Unité déclarée de la valeur (`None` = sans unité).
    pub unite: Option<Unite>,
}

impl Command for SetInput {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        verifie_nom(&self.name)?;
        verifie_prefixe_libre(&self.name)?;
        if !self.value.is_finite() {
            return Err(SessionError::CommandRejected {
                reason: "la valeur doit être un nombre fini".to_string(),
            });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_INPUT_SET,
            2,
            "",
            serde_json::json!({
                "name": self.name,
                "value": self.value,
                "unit": self.unite.map(|u| u.code()),
            }),
        )]))
    }
}

/// Définit l'expression de la valeur nommée du graphe de liaisons.
///
/// La validation syntaxique et la résolution des références ont lieu à
/// l'application (projection forge-values) : une expression fautive est
/// rejetée par [`SessionError::ProjectionFailed`] sans toucher au
/// journal. L'unité déclarée est vérifiée à la définition
/// (OF-VAL-0007, ADR-0019).
#[derive(Debug, Clone)]
pub struct SetExpression {
    /// Nom de la valeur.
    pub name: String,
    /// Expression dans le sous-ensemble déterministe de forge-values.
    pub expression: String,
    /// Unité déclarée de la valeur (`None` = sans unité).
    pub unite: Option<Unite>,
}

impl Command for SetExpression {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        verifie_nom(&self.name)?;
        verifie_prefixe_libre(&self.name)?;
        if self.expression.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "l'expression ne peut pas être vide".to_string(),
            });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_EXPRESSION_SET,
            2,
            "",
            serde_json::json!({
                "name": self.name,
                "expression": self.expression,
                "unit": self.unite.map(|u| u.code()),
            }),
        )]))
    }
}

/// Supprime un alias du graphe des liaisons (ADR-0019 §1, amendement
/// du 2026-10-20) : `Simple` refuse un alias encore référencé
/// (OF-VAL-0009), `Cascade` supprime aussi les formules qui le
/// lisent — récursivement —, `Remplacer` substitue un autre alias
/// dans les formules et les cotes avant de supprimer. Tout se joue
/// en une seule transaction : une commande = un groupe d'undo.
#[derive(Debug, Clone)]
pub struct RemoveAlias {
    /// Nom de l'alias supprimé.
    pub name: String,
    /// Mode de suppression : `Simple`, `Cascade` ou `Remplacer`.
    pub mode: ModeSuppression,
}

impl Command for RemoveAlias {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        let graph = session.graph();
        let Some(info) = graph.noeud(&self.name) else {
            return Err(refus_values(ValuesError::UnknownValue {
                name: self.name.clone(),
            }));
        };
        if info.systeme {
            return Err(refus_values(ValuesError::AliasReserve {
                name: self.name.clone(),
            }));
        }
        match &self.mode {
            ModeSuppression::Simple => {
                let references = graph.dependants(&self.name).unwrap_or_default();
                if !references.is_empty() {
                    return Err(refus_values(ValuesError::AliasReference {
                        name: self.name.clone(),
                        references: references.join(", "),
                    }));
                }
                Ok(CommandPlan::new(vec![evenement_alias_retire(&self.name)]))
            }
            ModeSuppression::Cascade => {
                // Dépendants transitifs : tout ce qui lit l'alias,
                // directement ou non. Une cote système qui le lit
                // refuse la cascade entière — une cote d'esquisse ne
                // se supprime pas indirectement (ADR-0019 §1).
                let mut supprimes = BTreeSet::new();
                supprimes.insert(self.name.clone());
                let mut queue = vec![self.name.clone()];
                while let Some(courant) = queue.pop() {
                    for dep in graph.dependants(&courant).unwrap_or_default() {
                        if supprimes.insert(dep.clone()) {
                            queue.push(dep);
                        }
                    }
                }
                let systeme: Vec<String> = supprimes
                    .iter()
                    .filter(|nom| graph.noeud(nom).is_some_and(|i| i.systeme))
                    .cloned()
                    .collect();
                if !systeme.is_empty() {
                    return Err(refus_values(ValuesError::AliasReference {
                        name: self.name.clone(),
                        references: systeme.join(", "),
                    }));
                }
                let events = supprimes
                    .iter()
                    .map(|nom| evenement_alias_retire(nom))
                    .collect();
                Ok(CommandPlan::new(events))
            }
            ModeSuppression::Remplacer { par } => self.plan_remplacer(session, par),
        }
    }
}

impl RemoveAlias {
    /// Plan du mode `Remplacer` : chaque formule qui lit l'alias est
    /// réécrite vers l'alias de remplacement (événements
    /// `value.expression_set` réémis, unités conservées), les cotes
    /// d'esquisse suivent (`sketch.constraint.updated`, identifiant
    /// conservé), puis l'alias disparaît — le tout en une seule
    /// transaction (ADR-0019 §1).
    fn plan_remplacer(&self, session: &Session, par: &str) -> Result<CommandPlan, SessionError> {
        let graph = session.graph();
        if par == self.name {
            return Err(refus_values(ValuesError::Cycle {
                path: format!("{} → {} → {}", self.name, par, self.name),
            }));
        }
        let Some(info_par) = graph.noeud(par) else {
            return Err(refus_values(ValuesError::UnknownValue {
                name: par.to_string(),
            }));
        };
        if info_par.systeme {
            return Err(refus_values(ValuesError::AliasReserve {
                name: par.to_string(),
            }));
        }
        if graph
            .deps_transitifs(par)
            .unwrap_or_default()
            .contains(&self.name)
        {
            return Err(refus_values(ValuesError::Cycle {
                path: format!("{} → {} → {}", self.name, par, self.name),
            }));
        }
        let mut events = Vec::new();
        for dep in graph.dependants(&self.name).unwrap_or_default() {
            let Some(info_dep) = graph.noeud(&dep) else {
                continue;
            };
            let Some(texte) = &info_dep.expression else {
                continue;
            };
            let nouveau = remplacer_identifiants(texte, &self.name, par);
            let nom_dep = dep.clone();
            events.push(NewEvent::new(
                EVENT_EXPRESSION_SET,
                2,
                "",
                serde_json::json!({
                    "name": nom_dep,
                    "expression": nouveau.clone(),
                    "unit": info_dep.unite.map(|u| u.code()),
                }),
            ));
            if info_dep.systeme {
                // Cote d'esquisse : la contrainte porte le texte de
                // l'expression, elle suit la substitution à
                // identifiant conservé (ADR-0019 §1).
                let Some((id, mut contrainte)) = session.esquisse().contrainte_de_cote(&dep)
                else {
                    continue;
                };
                if let ContrainteEsquisse::Distance {
                    valeur: ValeurCote::Expression { texte, .. },
                    ..
                } = &mut contrainte
                {
                    *texte = nouveau;
                }
                events.push(NewEvent::new(
                    EVENT_SKETCH_CONSTRAINT_UPDATED,
                    1,
                    "",
                    crate::esquisse::contrainte_en_json(id, &contrainte),
                ));
            }
        }
        events.push(evenement_alias_retire(&self.name));
        Ok(CommandPlan::new(events))
    }
}

/// Importe des tracés vectoriels (SVG ou DXF) comme géométrie courante
/// (ADR-0011) : les octets d'origine partent dans un blob, le journal ne
/// porte que la référence et les compteurs. La validation du contenu
/// (forge-io, sous budgets) est faite par l'application **avant** la
/// commande — la commande ne dépend pas de forge-geom et ne porte que
/// format, octets et compteurs.
#[derive(Debug, Clone)]
pub struct ImportTraces {
    /// Format d'origine : `svg` ou `dxf`.
    pub format: String,
    /// Octets d'origine du fichier importé.
    pub octets: Vec<u8>,
    /// Nombre de tracés importés.
    pub traces: usize,
    /// Nombre total de sommets importés.
    pub sommets: usize,
}

impl Command for ImportTraces {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.format != "svg" && self.format != "dxf" {
            return Err(SessionError::CommandRejected {
                reason: format!("format de tracés non supporté : « {} »", self.format),
            });
        }
        // Nom de blob unique par import (ADR-0011) : deux imports ne
        // s'écrasent jamais — un undo restaure une référence vers des
        // octets intacts, la purge reste au GC des blobs orphelins.
        let blob = format!("geometry/traces-{}", uuid::Uuid::now_v7());
        let mut plan = CommandPlan::new(vec![NewEvent::new(
            EVENT_TRACES_IMPORTED,
            1,
            "",
            serde_json::json!({
                "format": self.format,
                "blob": blob,
                "traces": self.traces,
                "sommets": self.sommets,
            }),
        )]);
        plan.blobs.push((blob, self.octets.clone()));
        Ok(plan)
    }
}

/// Importe un maillage (STL) comme géométrie courante (ADR-0011) :
/// octets d'origine dans un blob, référence et compteurs au journal.
/// L'échelle d'import (ADR-0017) vit dans l'événement : le blob garde
/// les octets bruts et la relecture les re-parse — le facteur doit
/// survivre au rejeu.
#[derive(Debug, Clone)]
pub struct ImportMesh {
    /// Format d'origine : `stl`.
    pub format: String,
    /// Octets d'origine du fichier importé.
    pub octets: Vec<u8>,
    /// Nombre de triangles importés.
    pub triangles: usize,
    /// Échelle d'import appliquée aux coordonnées (ADR-0017) :
    /// 1,0 = millimètres, 1000,0 = mètres.
    pub echelle: f64,
}

impl Command for ImportMesh {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.format != "stl" {
            return Err(SessionError::CommandRejected {
                reason: format!("format de maillage non supporté : « {} »", self.format),
            });
        }
        if !self.echelle.is_finite() || self.echelle <= 0.0 {
            return Err(SessionError::CommandRejected {
                reason: "l'échelle doit être finie et positive".to_string(),
            });
        }
        // Nom de blob unique par import, même logique que les tracés.
        let blob = format!("geometry/maillage-{}", uuid::Uuid::now_v7());
        let mut plan = CommandPlan::new(vec![NewEvent::new(
            EVENT_MESH_IMPORTED,
            2,
            "",
            serde_json::json!({
                "format": self.format,
                "blob": blob,
                "triangles": self.triangles,
                "echelle": self.echelle,
            }),
        )]);
        plan.blobs.push((blob, self.octets.clone()));
        Ok(plan)
    }
}

/// Ajoute une entité au croquis contraint (ADR-0018 D1). L'identifiant
/// est attribué par le plan : séquentiel, l'ordre d'ajout au journal
/// fait foi pour la reproductibilité du rejeu. Un refus (entité
/// invalide, références absentes) ne laisse rien au journal
/// (OF-SES-0001).
#[derive(Debug, Clone)]
pub struct AddEntity {
    /// Entité à ajouter : point, ligne, cercle ou arc.
    pub entite: EntiteEsquisse,
}

impl Command for AddEntity {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        let esquisse = session.esquisse();
        if let Some(reason) = esquisse.raison_refus_entite(&self.entite) {
            return Err(SessionError::CommandRejected { reason });
        }
        let id = esquisse.prochaine_id_entite();
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_SKETCH_ENTITY_ADDED,
            1,
            "",
            crate::esquisse::entite_en_json(id, &self.entite),
        )]))
    }
}

/// Ajoute une contrainte au croquis contraint (ADR-0018 D1) : les
/// onze genres du solveur plus l'ancrage d'un point. Un refus
/// (entités référencées absentes, cote invalide) ne laisse rien au
/// journal (OF-SES-0001).
///
/// Cote pilotée (ADR-0019 décision 4) : une `Distance` en expression
/// crée AUSSI le nœud `esquisse.<alias>` dans le graphe des liaisons
/// — deux événements dans la même transaction : contrainte et nœud
/// s'annulent et se rétablissent ensemble. L'expression est validée
/// à l'application par forge-values (références, cohérence des
/// unités) : un refus ne laisse rien au journal.
#[derive(Debug, Clone)]
pub struct AddConstraint {
    /// Contrainte à ajouter.
    pub contrainte: ContrainteEsquisse,
}

impl Command for AddConstraint {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        let esquisse = session.esquisse();
        if let Some(reason) = esquisse.raison_refus_contrainte(&self.contrainte) {
            return Err(SessionError::CommandRejected { reason });
        }
        let id = esquisse.prochaine_id_contrainte();
        let valeur = match &self.contrainte {
            ContrainteEsquisse::Distance { valeur, .. } => Some(valeur),
            _ => None,
        };
        let Some(ValeurCote::Expression { texte, alias }) = valeur else {
            return Ok(CommandPlan::new(vec![NewEvent::new(
                EVENT_SKETCH_CONSTRAINT_ADDED,
                2,
                "",
                crate::esquisse::contrainte_en_json(id, &self.contrainte),
            )]));
        };
        if !alias.starts_with("esquisse.") {
            return Err(SessionError::CommandRejected {
                reason: format!(
                    "l'alias d'une cote vit sous le préfixe réservé « esquisse. » : « {alias} »"
                ),
            });
        }
        verifie_nom_chemin(alias)?;
        if texte.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "l'expression de la cote ne peut pas être vide".to_string(),
            });
        }
        if session.graph().noeud(alias).is_some() {
            return Err(refus_values(ValuesError::AliasReserve {
                name: alias.clone(),
            }));
        }
        let mut events = vec![NewEvent::new(
            EVENT_EXPRESSION_SET,
            2,
            "",
            serde_json::json!({
                "name": alias,
                "expression": texte,
                "unit": "mm",
            }),
        )];
        events.push(NewEvent::new(
            EVENT_SKETCH_CONSTRAINT_ADDED,
            2,
            "",
            crate::esquisse::contrainte_en_json(id, &self.contrainte),
        ));
        Ok(CommandPlan::new(events))
    }
}

/// Pose la graine de position d'un point du croquis (ADR-0018 D1) :
/// la nouvelle position devient le point de départ de la prochaine
/// résolution — jamais une position résolue au journal. Un refus
/// (point absent, coordonnées non finies) ne laisse rien au journal
/// (OF-SES-0001).
#[derive(Debug, Clone)]
pub struct MovePoint {
    /// Identifiant du point déplacé.
    pub point: u32,
    /// Nouvelle abscisse de départ (millimètres).
    pub x: Length,
    /// Nouvelle ordonnée de départ (millimètres).
    pub y: Length,
}

impl Command for MovePoint {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        let esquisse = session.esquisse();
        if let Some(reason) = esquisse.raison_refus_deplacement(self.point, self.x, self.y) {
            return Err(SessionError::CommandRejected { reason });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_SKETCH_POINT_MOVED,
            1,
            "",
            serde_json::json!({
                "point": self.point,
                "x": self.x.millimeters(),
                "y": self.y.millimeters(),
            }),
        )]))
    }
}
