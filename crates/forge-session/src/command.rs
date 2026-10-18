//! Commandes de la session (ADR-0008, ADR-0011) : une intention
//! utilisateur = une validation contre l'état courant, puis 1..n
//! événements et d'éventuels blobs à déposer.

use crate::{Session, SessionError};
use forge_doc::NewEvent;

/// Type d'événement : valeur d'entrée fixée (schéma v1 : `name`, `value`).
pub const EVENT_INPUT_SET: &str = "value.input_set";
/// Type d'événement : expression de liaison fixée (schéma v1 : `name`,
/// `expression`).
pub const EVENT_EXPRESSION_SET: &str = "value.expression_set";
/// Type d'événement : tracés importés (schéma v1 : `format`, `blob`,
/// `traces`, `sommets`).
pub const EVENT_TRACES_IMPORTED: &str = "geom.traces.imported";
/// Type d'événement : maillage importé (schéma v2 : `format`, `blob`,
/// `triangles`, `echelle` — ADR-0017).
pub const EVENT_MESH_IMPORTED: &str = "geom.mesh.imported";

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

/// Fixe la valeur d'entrée nommée du graphe de liaisons.
///
/// Le nombre est sans unité (ADR-0007 phase 1 : l'unité sera déclarée par
/// le nœud au jalon 2) ; les newtypes de forge-units restent la frontière
/// programmatique des grandeurs dimensionnées.
#[derive(Debug, Clone)]
pub struct SetInput {
    /// Nom de la valeur.
    pub name: String,
    /// Valeur d'entrée ; un nombre non fini est refusé au plan.
    pub value: f64,
}

impl Command for SetInput {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        verifie_nom(&self.name)?;
        if !self.value.is_finite() {
            return Err(SessionError::CommandRejected {
                reason: "la valeur doit être un nombre fini".to_string(),
            });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_INPUT_SET,
            1,
            "",
            serde_json::json!({"name": self.name, "value": self.value}),
        )]))
    }
}

/// Définit l'expression de la valeur nommée du graphe de liaisons.
///
/// La validation syntaxique et la résolution des références ont lieu à
/// l'application (projection forge-values) : une expression fautive est
/// rejetée par [`SessionError::ProjectionFailed`] sans toucher au
/// journal.
#[derive(Debug, Clone)]
pub struct SetExpression {
    /// Nom de la valeur.
    pub name: String,
    /// Expression dans le sous-ensemble déterministe de forge-values.
    pub expression: String,
}

impl Command for SetExpression {
    fn plan(&self, _session: &Session) -> Result<CommandPlan, SessionError> {
        if self.name.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom de la valeur ne peut pas être vide".to_string(),
            });
        }
        verifie_nom(&self.name)?;
        if self.expression.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "l'expression ne peut pas être vide".to_string(),
            });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_EXPRESSION_SET,
            1,
            "",
            serde_json::json!({"name": self.name, "expression": self.expression}),
        )]))
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
