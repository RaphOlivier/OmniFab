//! Projections par domaine (ADR-0008) : état métier reconstruit par rejeu
//! du journal d'événements.

use crate::SessionError;
use crate::command::{
    EVENT_ALIAS_REMOVED, EVENT_EXPRESSION_SET, EVENT_INPUT_SET, EVENT_MESH_IMPORTED,
    EVENT_TRACES_IMPORTED,
};
use forge_doc::NewEvent;
use forge_values::{Origine, PREFIXES_RESERVES, Unite, ValueGraph};
use std::collections::HashSet;

/// Version de schéma des événements restés en v1 —
/// `geom.traces.imported` (ADR-0016 A3), `value.alias_removed`
/// (ADR-0019) : toute autre version d'un type connu est refusée,
/// jamais lue comme du v1.
const VERSION_SCHEMA_CONNUE: u32 = 1;
/// Version de schéma de `geom.mesh.imported` (ADR-0017) : le champ
/// `echelle` est ajouté en v2 ; les événements v1 sont migrés à
/// l'ouverture par le registre de migrations.
const VERSION_SCHEMA_MESH: u32 = 2;
/// Version de schéma courante des événements de liaisons
/// `value.input_set` et `value.expression_set` (ADR-0019) : la v2
/// ajoute le champ `unit` (ADR-0007 phase 1).
const VERSION_SCHEMA_VALEURS: u32 = 2;
/// Versions acceptées à la relecture des événements de liaisons
/// (ADR-0019 §1) : les journaux v1 non migrés rejouent sans unité,
/// le registre les aligne en v2 à l'ouverture.
const VERSIONS_VALEURS: &[u32] = &[1, VERSION_SCHEMA_VALEURS];

/// Plafond de types inconnus retenus : au-delà, les suivants ne sont
/// plus mémorisés un à un, seulement comptés (audit n°4 A4-03,
/// constaté le 2026-10-30).
pub(crate) const MAX_TYPES_INCONNUS: usize = 256;

/// Types d'événements ignorés, plafonnés : les premiers retenus un à
/// un, les suivants seulement comptés — un journal hostile ne fait
/// plus croître la mémoire linéairement ni payer le dédoublonnage
/// O(n²) du `Vec::contains` historique (audit n°4 A4-03).
#[derive(Debug, Default, Clone)]
pub(crate) struct TypesIgnores {
    /// Types retenus, uniques, dans l'ordre de rencontre.
    retenus: Vec<String>,
    /// Types déjà vus, pour le dédoublonnage en O(1).
    vus: HashSet<String>,
    /// Événements ignorés au-delà du plafond, seulement comptés.
    supplementaires: usize,
}

impl TypesIgnores {
    /// Note un type ignoré : retenu s'il est nouveau tant que le
    /// plafond [`MAX_TYPES_INCONNUS`] n'est pas atteint, sinon
    /// seulement compté.
    pub(crate) fn noter(&mut self, nom: &str) {
        if self.vus.len() >= MAX_TYPES_INCONNUS {
            self.supplementaires += 1;
            return;
        }
        if self.vus.insert(nom.to_string()) {
            self.retenus.push(nom.to_string());
        }
    }

    /// Types retenus, uniques, dans l'ordre de rencontre.
    pub(crate) fn retenus(&self) -> &[String] {
        &self.retenus
    }

    /// Événements ignorés au-delà du plafond, seulement comptés.
    pub(crate) fn supplementaires(&self) -> usize {
        self.supplementaires
    }

    /// Remet à vide.
    pub(crate) fn clear(&mut self) {
        self.retenus.clear();
        self.vus.clear();
        self.supplementaires = 0;
    }
}

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
    /// Types d'événements ignorés, plafonnés (audit n°4 A4-03).
    unknown: TypesIgnores,
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
        self.unknown.retenus()
    }

    /// Événements ignorés au-delà du plafond, seulement comptés.
    pub fn unknown_supplementaires(&self) -> usize {
        self.unknown.supplementaires()
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
                verifie_schema_parmi(event, VERSIONS_VALEURS)?;
                let name = champ_str(event, "name")?;
                let value = champ_nombre(event, "value")?;
                let unite = lit_unite(event)?;
                self.graph
                    .set_input_unite(name, value, unite, origine_de(name))
                    .map_err(projection_values)?;
                Ok(())
            }
            EVENT_EXPRESSION_SET => {
                verifie_schema_parmi(event, VERSIONS_VALEURS)?;
                let name = champ_str(event, "name")?;
                let expression = champ_str(event, "expression")?;
                let unite = lit_unite(event)?;
                self.graph
                    .set_expression_unite(name, expression, unite, origine_de(name))
                    .map_err(projection_values)?;
                Ok(())
            }
            EVENT_ALIAS_REMOVED => {
                verifie_schema(event, VERSION_SCHEMA_CONNUE)?;
                let name = champ_str(event, "name")?;
                if self.graph.noeud(name).is_none() {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("alias « {name} » absent du graphe des liaisons"),
                    });
                }
                // Le plan a validé (références, nœuds système) : le
                // journal fait foi, la projection retire sans revérifier
                // (ADR-0019 §1).
                self.graph.retirer_noeud(name);
                Ok(())
            }
            _ => {
                self.unknown.noter(&event.event_type);
                Ok(())
            }
        }
    }

    fn types_connus(&self) -> &'static [&'static str] {
        &[EVENT_INPUT_SET, EVENT_EXPRESSION_SET, EVENT_ALIAS_REMOVED]
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

/// Maillage importé courant (référence + compteurs, échelle d'import).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaillageImporte {
    /// Format d'origine (`stl`).
    pub format: String,
    /// Nom du blob contenant les octets d'origine.
    pub blob: String,
    /// Nombre de triangles.
    pub triangles: usize,
    /// Échelle d'import appliquée aux coordonnées (ADR-0017) :
    /// 1,0 = millimètres, 1000,0 = mètres.
    pub echelle: f64,
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
    /// Types d'événements ignorés, plafonnés (audit n°4 A4-03).
    unknown: TypesIgnores,
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
        self.unknown.retenus()
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
                verifie_schema(event, VERSION_SCHEMA_CONNUE)?;
                let format = champ_str(event, "format")?;
                if format != "svg" && format != "dxf" {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("format « {format} » ni svg ni dxf"),
                    });
                }
                let blob = champ_str(event, "blob")?;
                let traces = champ_entier(event, "traces")?;
                let sommets = champ_entier(event, "sommets")?;
                self.traces_courantes = Some(TracesImportees {
                    format: format.to_string(),
                    blob: blob.to_string(),
                    traces,
                    sommets,
                });
                Ok(())
            }
            EVENT_MESH_IMPORTED => {
                verifie_schema(event, VERSION_SCHEMA_MESH)?;
                let format = champ_str(event, "format")?;
                if format != "stl" {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("format « {format} » différent de stl"),
                    });
                }
                let blob = champ_str(event, "blob")?;
                let triangles = champ_entier(event, "triangles")?;
                let echelle = champ_nombre(event, "echelle")?;
                self.maillage_courant = Some(MaillageImporte {
                    format: format.to_string(),
                    blob: blob.to_string(),
                    triangles,
                    echelle,
                });
                Ok(())
            }
            _ => {
                self.unknown.noter(&event.event_type);
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
pub(crate) fn champ_str<'a>(event: &'a NewEvent, champ: &str) -> Result<&'a str, SessionError> {
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
pub(crate) fn champ_nombre(event: &NewEvent, champ: &str) -> Result<f64, SessionError> {
    event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (nombre) absent"),
        })
}

/// Vérifie la version de schéma d'un événement de type connu
/// (ADR-0016 A3) : le registre de migrations le refuse déjà à
/// l'ouverture, la projection est la dernière ligne de défense —
/// un journal plus récent n'est jamais lu comme du v1.
pub(crate) fn verifie_schema(event: &NewEvent, version: u32) -> Result<(), SessionError> {
    if event.schema_version != version {
        return Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!(
                "version de schéma {} non supportée (attendue : {})",
                event.schema_version, version
            ),
        });
    }
    Ok(())
}

/// Vérifie la version de schéma d'un événement acceptant plusieurs
/// versions : les journaux plus anciens rejouent s'ils sont listés,
/// toute version absente de la liste est refusée — jamais lue en
/// devinant (ADR-0019 §1).
pub(crate) fn verifie_schema_parmi(event: &NewEvent, versions: &[u32]) -> Result<(), SessionError> {
    if versions.contains(&event.schema_version) {
        return Ok(());
    }
    Err(SessionError::InvalidEventData {
        event_type: event.event_type.clone(),
        reason: format!(
            "version de schéma {} non supportée pour ce type",
            event.schema_version
        ),
    })
}

/// Lit le champ `unit` d'un événement de liaisons v2 : absent, `null`
/// ou code inconnu refusé, un code connu = l'unité déclarée, sinon
/// `None` = sans unité (ADR-0007 phase 1, ADR-0019).
fn lit_unite(event: &NewEvent) -> Result<Option<Unite>, SessionError> {
    match event.data.get("unit") {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(code)) => match Unite::from_code(code) {
            Some(unite) => Ok(Some(unite)),
            None => Err(SessionError::InvalidEventData {
                event_type: event.event_type.clone(),
                reason: format!("unité inconnue : « {code} »"),
            }),
        },
        Some(_) => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « unit » ni null ni code d'unité".to_string(),
        }),
    }
}

/// Origine du nœud porté par un événement de liaisons : les noms sous
/// un préfixe réservé sont toujours système (ADR-0019) — le plan des
/// commandes refuse déjà leur création par l'utilisateur, le journal
/// fait foi.
fn origine_de(name: &str) -> Origine {
    if PREFIXES_RESERVES.iter().any(|p| name.starts_with(p)) {
        Origine::Systeme
    } else {
        Origine::Utilisateur
    }
}

/// Traduit un refus du moteur de liaisons en échec de projection :
/// le message d'origine (codes OF-VAL-*) traverse tel quel.
fn projection_values(source: forge_values::ValuesError) -> SessionError {
    SessionError::ProjectionFailed {
        projection: "values".to_string(),
        message: source.to_string(),
    }
}

/// Lit un champ entier non négatif du schéma v1 : les nombres
/// négatifs, fractionnaires ou hors `usize` sont refusés — jamais
/// de conversion `as` silencieuse depuis un `f64` de fichier
/// (ADR-0016 A3).
pub(crate) fn champ_entier(event: &NewEvent, champ: &str) -> Result<usize, SessionError> {
    let valeur = event
        .data
        .get(champ)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (entier non négatif) absent ou invalide"),
        })?;
    usize::try_from(valeur).map_err(|_| SessionError::InvalidEventData {
        event_type: event.event_type.clone(),
        reason: format!("champ « {champ} » hors des bornes de la plateforme"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un journal hostile fourmille de types inconnus distincts :
    /// au-delà du plafond, les types supplémentaires ne sont plus
    /// retenus un à un (audit n°4 A4-03).
    #[test]
    fn types_inconnus_plafonnes() {
        let mut projection = ValuesProjection::new();
        for i in 0..300 {
            let event = NewEvent::new(
                &format!("forge.journal.faux.{i}"),
                1,
                "test",
                serde_json::Value::Object(serde_json::Map::new()),
            );
            projection.apply(&event).expect("type inconnu toléré");
        }
        assert_eq!(
            projection.unknown_types().len(),
            MAX_TYPES_INCONNUS,
            "seuls {MAX_TYPES_INCONNUS} types retenus"
        );
        assert_eq!(
            projection.unknown_supplementaires(),
            44,
            "les 44 événements au-delà du plafond sont comptés"
        );
    }
}
