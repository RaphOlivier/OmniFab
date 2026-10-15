//! Projections par domaine (ADR-0008) : état métier reconstruit par rejeu
//! du journal d'événements.

use crate::SessionError;
use crate::command::{
    EVENT_EXPRESSION_SET, EVENT_INPUT_SET, EVENT_MESH_IMPORTED, EVENT_TRACES_IMPORTED,
};
use forge_doc::NewEvent;
use forge_values::ValueGraph;

/// Projection d'un domaine métier : l'état du domaine reconstruit par
/// rejeu du journal. Les domaines futurs (esquisse, CAO, CAM…)
/// enregistrent leurs projections auprès de la session sans modifier
/// cette crate.
pub trait Projection {
    /// Nom du domaine, pour les messages d'erreur.
    fn name(&self) -> &'static str;
    /// Remet la projection à vide, avant un rejeu.
    fn reset(&mut self);
    /// Applique un événement. Les types inconnus sont tolérés
    /// (compatibilité avant : journal écrit par une version plus
    /// récente) ; un refus est une erreur typée.
    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError>;
    /// Noms des blobs référencés par l'état courant de la projection.
    /// La session s'en sert pour purger les blobs orphelins (ADR-0011).
    fn referenced_blobs(&self) -> Vec<String> {
        Vec::new()
    }
    /// Types d'événements que cette projection sait traiter. La session
    /// s'en sert pour l'intersection des types « ignorés » par
    /// l'interface : un type n'est ignoré que s'il est inconnu de
    /// toutes les projections (ADR-0011). Défaut : aucun type déclaré.
    fn types_connus(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Projection du domaine des liaisons (forge-values) — la projection
/// enregistrée au jalon 1.
#[derive(Debug, Default)]
pub struct ValuesProjection {
    /// Graphe de liaisons reconstruit par rejeu.
    graph: ValueGraph,
    /// Types d'événements ignorés, uniques, dans l'ordre de rencontre.
    unknown: Vec<String>,
}

impl ValuesProjection {
    /// Nouvelle projection vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Graphe de liaisons projeté.
    pub fn graph(&self) -> &ValueGraph {
        &self.graph
    }

    /// Graphe de liaisons projeté, mutable (lecture paresseuse et
    /// recalculs, cf. ADR-0002).
    pub fn graph_mut(&mut self) -> &mut ValueGraph {
        &mut self.graph
    }

    /// Types d'événements ignorés par cette projection.
    pub fn unknown_types(&self) -> &[String] {
        &self.unknown
    }
}

impl Projection for ValuesProjection {
    fn name(&self) -> &'static str {
        "values"
    }

    fn reset(&mut self) {
        self.graph = ValueGraph::new();
        self.unknown.clear();
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        match event.event_type.as_str() {
            EVENT_INPUT_SET => {
                let name = champ_str(event, "name")?;
                let value = champ_nombre(event, "value")?;
                self.graph.set_input(name, value);
                Ok(())
            }
            EVENT_EXPRESSION_SET => {
                let name = champ_str(event, "name")?;
                let expression = champ_str(event, "expression")?;
                self.graph
                    .set_expression(name, expression)
                    .map_err(|source| SessionError::ProjectionFailed {
                        projection: "values".to_string(),
                        message: source.to_string(),
                    })
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
        &[EVENT_INPUT_SET, EVENT_EXPRESSION_SET]
    }
}

/// Tracés importés courants (référence + compteurs, jamais les objets).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TracesImportees {
    /// Format d'origine (`svg` ou `dxf`).
    pub format: String,
    /// Nom du blob contenant les octets d'origine.
    pub blob: String,
    /// Nombre de tracés.
    pub traces: usize,
    /// Nombre total de sommets.
    pub sommets: usize,
}

/// Maillage importé courant (référence + compteurs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaillageImporte {
    /// Format d'origine (`stl`).
    pub format: String,
    /// Nom du blob contenant les octets d'origine.
    pub blob: String,
    /// Nombre de triangles.
    pub triangles: usize,
}

/// Projection du domaine de la géométrie importée (ADR-0011) : portée par
/// la session au même titre que [`ValuesProjection`], sans dépendance à
/// forge-geom — elle ne porte que références (nom du blob) et compteurs ;
/// les objets lourds re-parsés vivent à la lecture, côté application.
#[derive(Debug, Default)]
pub struct GeomProjection {
    /// Tracés importés courants, le cas échéant.
    traces_courantes: Option<TracesImportees>,
    /// Maillage importé courant, le cas échéant.
    maillage_courant: Option<MaillageImporte>,
    /// Types d'événements ignorés, uniques, dans l'ordre de rencontre.
    unknown: Vec<String>,
}

impl GeomProjection {
    /// Nouvelle projection vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tracés importés courants (référence + compteurs).
    pub fn traces(&self) -> Option<&TracesImportees> {
        self.traces_courantes.as_ref()
    }

    /// Maillage importé courant (référence + compteurs).
    pub fn maillage(&self) -> Option<&MaillageImporte> {
        self.maillage_courant.as_ref()
    }

    /// Types d'événements ignorés par cette projection.
    pub fn unknown_types(&self) -> &[String] {
        &self.unknown
    }
}

impl Projection for GeomProjection {
    fn name(&self) -> &'static str {
        "geom"
    }

    fn reset(&mut self) {
        self.traces_courantes = None;
        self.maillage_courant = None;
        self.unknown.clear();
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        match event.event_type.as_str() {
            EVENT_TRACES_IMPORTED => {
                let format = champ_str(event, "format")?;
                if format != "svg" && format != "dxf" {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("format « {format} » ni svg ni dxf"),
                    });
                }
                let blob = champ_str(event, "blob")?;
                let traces = champ_nombre(event, "traces")? as usize;
                let sommets = champ_nombre(event, "sommets")? as usize;
                self.traces_courantes = Some(TracesImportees {
                    format: format.to_string(),
                    blob: blob.to_string(),
                    traces,
                    sommets,
                });
                Ok(())
            }
            EVENT_MESH_IMPORTED => {
                let format = champ_str(event, "format")?;
                if format != "stl" {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("format « {format} » différent de stl"),
                    });
                }
                let blob = champ_str(event, "blob")?;
                let triangles = champ_nombre(event, "triangles")? as usize;
                self.maillage_courant = Some(MaillageImporte {
                    format: format.to_string(),
                    blob: blob.to_string(),
                    triangles,
                });
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

    fn referenced_blobs(&self) -> Vec<String> {
        let mut references = Vec::new();
        if let Some(traces) = &self.traces_courantes {
            references.push(traces.blob.clone());
        }
        if let Some(maillage) = &self.maillage_courant {
            references.push(maillage.blob.clone());
        }
        references
    }

    fn types_connus(&self) -> &'static [&'static str] {
        &[EVENT_TRACES_IMPORTED, EVENT_MESH_IMPORTED]
    }
}

/// Lit un champ chaîne du schéma v1 du type d'événement.
fn champ_str<'a>(event: &'a NewEvent, champ: &str) -> Result<&'a str, SessionError> {
    event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (chaîne) absent"),
        })
}

/// Lit un champ nombre du schéma v1 du type d'événement.
fn champ_nombre(event: &NewEvent, champ: &str) -> Result<f64, SessionError> {
    event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (nombre) absent"),
        })
}
