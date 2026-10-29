//! # forge-session — session, commandes et undo/redo
//!
//! (ADR-0008, ouverture du jalon 1 ; ADR-0011, intégration au
//! démonstrateur ; ADR-0019, liaisons v2 — unités déclarées,
//! suppression d'alias en trois modes, cotes pilotées.) La session
//! possède le document (forge-doc) et les
//! projections métier ; les applications ne touchent plus `Document`
//! directement : elles construisent des [commandes](Command) et lisent
//! les projections.
//!
//! - **commande** : une intention utilisateur se planifie contre l'état
//!   courant et produit 1..n événements et d'éventuels blobs
//!   ([`CommandPlan`]) ; la session attribue à chaque événement l'acteur
//!   et un identifiant de transaction (une commande = un groupe d'undo) ;
//! - **validation avant écriture, tout-ou-rien** (ADR-0011, ADR-0016 A1) :
//!   les événements sont d'abord appliqués aux projections, les blobs
//!   ensuite déposés ; si l'une refuse, **rien n'est écrit ni tronqué** —
//!   la pile redo et les blobs déjà déposés restent intacts, et un
//!   journal écrit par la session se rejoue toujours ;
//! - **undo/redo par curseur** : le curseur `applied` sépare le journal
//!   appliqué des événements défaits, qui restent en mémoire pour le
//!   redo ; une nouvelle commande après un undo **abandonne** les
//!   événements défaits et purge les blobs orphelins (ADR-0011) ;
//! - **géométrie dans des blobs, jamais dans le journal** (ADR-0001) :
//!   les commandes d'import déposent les octets d'origine en blobs, la
//!   projection [`GeomProjection`] ne porte que références et compteurs ;
//! - **sauvegarde** : le fichier contient le journal linéaire visible ;
//!   la session n'est pas mutée — la pile redo survit à
//!   l'enregistrement (ADR-0016 C3) — et les blobs orphelins du
//!   préfixe appliqué ne figurent pas dans le fichier.

mod command;
mod error;
mod esquisse;
mod projection;

pub use command::{
    AddConstraint, AddEntity, AncrerPoint, Command, CommandPlan, EVENT_ALIAS_REMOVED,
    EVENT_EXPRESSION_SET, EVENT_INPUT_SET, EVENT_MESH_IMPORTED, EVENT_SKETCH_CONSTRAINT_ADDED,
    EVENT_SKETCH_CONSTRAINT_UPDATED, EVENT_SKETCH_ENTITY_ADDED, EVENT_SKETCH_POINT_MOVED,
    EVENT_TRACES_IMPORTED, ImportMesh, ImportTraces, MovePoint, RemoveAlias, SetExpression,
    SetInput,
};
pub use error::SessionError;
pub use esquisse::{
    ContrainteEsquisse, CroquisResolu, EntiteEsquisse, EsquisseProjection, GenreEntite, ValeurCote,
};
pub use forge_values::{InfoNoeud, ModeSuppression, Unite};
pub use projection::{
    GeomProjection, MaillageImporte, Projection, TracesImportees, ValuesProjection,
};

use forge_doc::{DocError, Document, Event, MigrationRegistry, NewEvent};
use forge_units::Length;
use forge_values::ValueGraph;

/// Registre de migrations courant de la session (ADR-0016 A3) : les
/// types d'événement connus, déclarés à leur version courante — les
/// liaisons passent en v2 avec le champ `unit` (ADR-0019), la
/// contrainte de croquis migre sa cote de distance vers l'objet typé,
/// l'entité migre son arc vers le balayage signé (ADR-0020),
/// `value.alias_removed` et `sketch.constraint.updated` naissent
/// en v1 (ADR-0019) ; le maillage importé migre du v1 au v2
/// (ADR-0017). À passer à l'ouverture d'un projet
/// (Document::from_bytes_with, Document::open_from_path_with,
/// feature native) : un journal écrit par une version plus récente
/// est refusé par SchemaTooNew (ADR-0001) au lieu d'être lu comme
/// du v1.
pub fn registre_migrations_courant() -> MigrationRegistry {
    let mut registre = MigrationRegistry::new();
    registre.declare(EVENT_INPUT_SET, 2);
    registre.register(EVENT_INPUT_SET, 1, migre_unite_v1_v2);
    registre.declare(EVENT_EXPRESSION_SET, 2);
    registre.register(EVENT_EXPRESSION_SET, 1, migre_unite_v1_v2);
    registre.declare(EVENT_ALIAS_REMOVED, 1);
    registre.declare(EVENT_TRACES_IMPORTED, 1);
    registre.declare(EVENT_MESH_IMPORTED, 2);
    registre.register(EVENT_MESH_IMPORTED, 1, migre_mesh_v1_v2);
    registre.declare(EVENT_SKETCH_ENTITY_ADDED, 2);
    registre.register(EVENT_SKETCH_ENTITY_ADDED, 1, migre_entite_v1_v2);
    registre.declare(EVENT_SKETCH_CONSTRAINT_ADDED, 2);
    registre.register(EVENT_SKETCH_CONSTRAINT_ADDED, 1, migre_contrainte_v2);
    registre.declare(EVENT_SKETCH_CONSTRAINT_UPDATED, 1);
    registre.declare(EVENT_SKETCH_POINT_MOVED, 1);
    registre
}

/// Migration `sketch.entity.added` v1 → v2 (ADR-0020) : l'arc v1
/// balaie toujours dans le sens trigonométrique — `angle_fin` lu
/// comme balayage signé v2, il est réencodé en `début + balayage
/// positif` ; les autres genres sont inchangés.
fn migre_entite_v1_v2(donnees: &serde_json::Value) -> Result<serde_json::Value, DocError> {
    let mut donnees = donnees.clone();
    if donnees.get("genre").and_then(serde_json::Value::as_str) == Some("arc")
        && let Some(objet) = donnees.as_object_mut()
    {
        let debut = objet.get("angle_debut").and_then(serde_json::Value::as_f64);
        let fin = objet.get("angle_fin").and_then(serde_json::Value::as_f64);
        if let (Some(debut), Some(fin)) = (debut, fin) {
            let mut balayage = (fin - debut).rem_euclid(std::f64::consts::TAU);
            if balayage == 0.0 {
                balayage = std::f64::consts::TAU;
            }
            objet.insert("angle_fin".to_string(), serde_json::json!(debut + balayage));
        }
    }
    Ok(donnees)
}

/// Migration `geom.mesh.imported` v1 → v2 (ADR-0017) : le champ
/// `echelle` est posé à 1,0 — les journaux v1 relisent leurs maillages
/// en millimètres, sans changement de comportement.
fn migre_mesh_v1_v2(donnees: &serde_json::Value) -> Result<serde_json::Value, DocError> {
    let mut donnees = donnees.clone();
    if let Some(objet) = donnees.as_object_mut() {
        objet.insert("echelle".to_string(), serde_json::json!(1.0));
    }
    Ok(donnees)
}

/// Migration `value.input_set` et `value.expression_set` v1 → v2
/// (ADR-0019) : le champ `unit` est posé à `null` — les journaux
/// v1 relisent leurs valeurs sans unité déclarée, sans changement de
/// comportement.
fn migre_unite_v1_v2(donnees: &serde_json::Value) -> Result<serde_json::Value, DocError> {
    let mut donnees = donnees.clone();
    if let Some(objet) = donnees.as_object_mut() {
        objet.insert("unit".to_string(), serde_json::Value::Null);
    }
    Ok(donnees)
}

/// Migration `sketch.constraint.added` v1 → v2 (ADR-0019) : la cote
/// de `Distance` — nombre nu en v1 (ADR-0018) — devient l'objet typé
/// `{"type": "fixe", "mm": …}`.
fn migre_contrainte_v2(donnees: &serde_json::Value) -> Result<serde_json::Value, DocError> {
    let mut donnees = donnees.clone();
    let cote_nue = match (donnees.get("genre"), donnees.get("valeur")) {
        (Some(serde_json::Value::String(genre)), Some(valeur)) => {
            genre == "distance" && valeur.is_number()
        }
        _ => false,
    };
    if cote_nue && let Some(objet) = donnees.as_object_mut() {
        let millimetres = objet.get("valeur").cloned().unwrap_or_default();
        objet.insert(
            "valeur".to_string(),
            serde_json::json!({ "type": "fixe", "mm": millimetres }),
        );
    }
    Ok(donnees)
}

/// Session de travail (ADR-0008) : possède le document forge-doc et les
/// projections par domaine ; les applications construisent des
/// [commandes](Command) et lisent les projections — elles ne touchent
/// jamais `Document` directement.
pub struct Session {
    /// Document possédé : journal, blobs, snapshot.
    doc: Document,
    /// Projection du domaine des liaisons (jalon 1).
    values: ValuesProjection,
    /// Projection du domaine de la géométrie importée (ADR-0011).
    geom: GeomProjection,
    /// Projection du croquis contraint (ADR-0018, phase D).
    esquisse: EsquisseProjection,
    /// Projections des domaines futurs, enregistrées.
    projections: Vec<Box<dyn Projection>>,
    /// Nombre d'événements appliqués aux projections ; les événements
    /// suivants du journal sont défaits, en attente de redo.
    applied: usize,
    /// Identifiant de l'acteur attribué à chaque événement écrit.
    actor: String,
}

impl Session {
    /// Session vide, avec les projections des liaisons, de la
    /// géométrie importée et du croquis contraint.
    ///
    /// `actor` identifie l'application ou la zone UI à l'origine des
    /// événements (ex. `desktop`, `android.machine`).
    pub fn new(actor: &str) -> Self {
        Self {
            doc: Document::new(),
            values: ValuesProjection::new(),
            geom: GeomProjection::new(),
            esquisse: EsquisseProjection::new(),
            projections: Vec::new(),
            applied: 0,
            actor: actor.to_string(),
        }
    }

    /// Session à partir d'un document existant : le journal est rejoué
    /// en entier dans les projections.
    pub fn from_document(actor: &str, doc: Document) -> Result<Self, SessionError> {
        let mut session = Self::new(actor);
        session.doc = doc;
        session.applied = session.doc.events().len();
        session.replay_applied()?;
        Ok(session)
    }

    /// Enregistre la projection d'un domaine futur ; le journal appliqué
    /// est rejoué dans la nouvelle projection. Une projection qui refuse
    /// un événement n'est pas enregistrée.
    pub fn register_projection(
        &mut self,
        mut projection: Box<dyn Projection>,
    ) -> Result<(), SessionError> {
        projection.reset();
        let events: Vec<NewEvent> = self.doc.events()[..self.applied]
            .iter()
            .map(as_new)
            .collect();
        for event in &events {
            projection.apply(event)?;
        }
        self.projections.push(projection);
        Ok(())
    }

    /// Exécute une commande : planification contre l'état courant,
    /// application aux projections, dépôt des blobs, puis écriture au
    /// journal. L'exécution est tout-ou-rien (ADR-0011) : un refus au
    /// plan, dans une projection ou au dépôt d'un blob laisse le journal
    /// **et la pile redo** intacts (ADR-0016 A1). L'abandon des
    /// événements défaisables (ADR-0008) et la purge des blobs orphelins
    /// (ADR-0011) n'ont lieu qu'après une validation complète.
    pub fn execute(&mut self, command: &dyn Command) -> Result<(), SessionError> {
        let plan = command.plan(self)?;
        if plan.events.is_empty() && plan.blobs.is_empty() {
            return Ok(());
        }
        // Références de l'état courant, capturées avant application :
        // un undo peut revenir à cet état — leurs blobs doivent survivre.
        let mut references = self.referenced_blobs();
        for event in &plan.events {
            if let Err(error) = self.apply_all(event) {
                // Rien n'est écrit au journal et la pile redo est encore
                // en place : le rejeu du journal appliqué rétablit
                // exactement l'état d'avant commande (ADR-0016 A1).
                self.replay_applied()?;
                return Err(error);
            }
        }
        let mut deposes: Vec<String> = Vec::new();
        for (nom, donnees) in plan.blobs {
            if let Err(erreur) = self.doc.set_blob(&nom, donnees) {
                // Toujours rien au journal : les blobs déjà déposés par
                // le plan sont retirés — aucun ne survit isolément — et
                // le rejeu rétablit l'état d'avant commande.
                for nom in &deposes {
                    self.doc.remove_blob(nom);
                }
                self.replay_applied()?;
                return Err(SessionError::BlobRejected {
                    reason: erreur.to_string(),
                });
            }
            deposes.push(nom);
        }
        // Validation complète : seule une commande réussie abandonne la
        // pile redo (ADR-0008) et purge les blobs orphelins (ADR-0011).
        references.extend(deposes.iter().cloned());
        self.truncate_pending();
        self.gc_blobs(&references);
        // UUIDv7 horodaté par l'horloge du document (ADR-0016 A7).
        let txn = self.doc.nouvel_uuid_v7();
        for mut event in plan.events {
            event.actor = self.actor.clone();
            event.txn = Some(txn.clone());
            self.doc.append(event);
        }
        self.applied = self.doc.events().len();
        Ok(())
    }

    /// Annule le dernier groupe (une commande entière, ou un événement
    /// isolé du journal) : l'état est reconstruit par rejeu du journal
    /// appliqué. Les événements défaits restent en mémoire pour le
    /// redo (ADR-0008).
    pub fn undo(&mut self) -> Result<(), SessionError> {
        if !self.can_undo() {
            return Err(SessionError::UndoEmpty);
        }
        let events = self.doc.events();
        self.applied = groupe_start(events, self.applied);
        self.replay_applied()
    }

    /// Rétablit le groupe défait suivant.
    pub fn redo(&mut self) -> Result<(), SessionError> {
        if !self.can_redo() {
            return Err(SessionError::RedoEmpty);
        }
        let events = self.doc.events();
        self.applied = groupe_end(events, self.applied);
        self.replay_applied()
    }

    /// `true` s'il reste un groupe à annuler.
    pub fn can_undo(&self) -> bool {
        self.applied > 0
    }

    /// `true` s'il reste un groupe défait à rétablir.
    pub fn can_redo(&self) -> bool {
        self.applied < self.doc.events().len()
    }

    /// Journal complet, y compris les événements défaits en attente de
    /// redo.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Graphe de liaisons projeté (lecture).
    pub fn graph(&self) -> &ValueGraph {
        self.values.graph()
    }

    /// Graphe de liaisons projeté, mutable : lecture paresseuse et
    /// recalculs (ADR-0002).
    pub fn graph_mut(&mut self) -> &mut ValueGraph {
        self.values.graph_mut()
    }

    /// Projection de la géométrie importée (références et compteurs,
    /// lecture) — jamais les objets lourds (ADR-0011).
    pub fn geom(&self) -> &GeomProjection {
        &self.geom
    }

    /// Projection du croquis contraint (entités, contraintes et
    /// graines, lecture) — les positions résolues sont un état dérivé
    /// (ADR-0018 D3).
    pub fn esquisse(&self) -> &EsquisseProjection {
        &self.esquisse
    }

    /// Croquis prêt pour la résolution (ADR-0019) : clone de la
    /// projection, cotes pilotées figées par le moteur de liaisons —
    /// le solveur ne voit jamais une expression (frontière
    /// forge-values → forge-gcs), les positions résolues restent un
    /// état dérivé jamais sérialisé (ADR-0018 D3). Une cote qui
    /// n'évalue pas ou n'évalue pas en une longueur finie et
    /// strictement positive refuse la résolution entière.
    pub fn croquis_pour_resolution(&mut self) -> Result<EsquisseProjection, SessionError> {
        let mut croquis = self.esquisse.clone();
        for alias in croquis.alias_des_cotes() {
            let millimetres = self.graph_mut().value(&alias).map_err(|source| {
                SessionError::ProjectionFailed {
                    projection: "values".to_string(),
                    message: source.to_string(),
                }
            })?;
            if !millimetres.is_finite() || millimetres <= 0.0 {
                return Err(SessionError::CommandRejected {
                    reason: format!("la cote « {alias} » doit être finie et strictement positive"),
                });
            }
            croquis.figer_cote(&alias, Length::from_millimeters(millimetres));
        }
        Ok(croquis)
    }

    /// Types d'événements « ignorés » par l'interface : inconnus de
    /// toutes les projections (values, geom, esquisse, enregistrées) — un type
    /// connu d'une seule projection n'est pas un type ignoré (ADR-0011).
    pub fn unknown_types(&self) -> Vec<String> {
        self.values
            .unknown_types()
            .iter()
            .filter(|event_type| {
                !self.geom.types_connus().contains(&event_type.as_str())
                    && !self.esquisse.types_connus().contains(&event_type.as_str())
                    && self
                        .projections
                        .iter()
                        .all(|projection| !projection.types_connus().contains(&event_type.as_str()))
            })
            .cloned()
            .collect()
    }

    /// Sérialise la session en conteneur ZIP (en mémoire), **sans la
    /// muter** : le fichier contient le journal linéaire visible
    /// (ADR-0008) et la pile redo survit à l'enregistrement
    /// (ADR-0016 C3) — les blobs orphelins du préfixe ne figurent
    /// pas dans le fichier.
    pub fn save_to_bytes(&self) -> Result<Vec<u8>, DocError> {
        self.doc.to_bytes_prefix(self.applied)
    }

    /// Enregistre la session dans un fichier, de manière atomique
    /// (feature `native`, cf. [`Document::save_to_path`]) **sans
    /// la muter** : la pile redo survit à l'enregistrement
    /// (ADR-0016 C3).
    #[cfg(feature = "native")]
    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), DocError> {
        self.doc.save_to_path_prefix(path, self.applied)
    }

    /// Abandonne les événements défaisables : le journal devient le
    /// journal visible (une nouvelle action ou une sauvegarde les
    /// supprime, ADR-0008).
    fn truncate_pending(&mut self) {
        if self.applied < self.doc.events().len() {
            self.doc.pop_after(self.applied as u64);
        }
    }

    /// Références de blobs portées par l'état courant de toutes les
    /// projections (géométrie importée, domaines enregistrés).
    fn referenced_blobs(&self) -> Vec<String> {
        let mut references = self.geom.referenced_blobs();
        for projection in &self.projections {
            references.extend(projection.referenced_blobs());
        }
        references
    }

    /// Purge les blobs orphelins (ADR-0011) : ne survit qu'un blob
    /// présent dans `references` — l'état courant des projections au
    /// moment de la planification, augmenté des blobs fraîchement
    /// déposés par une commande validée — ou référencé par un
    /// événement appliqué du journal (clé `data.blob`, comme
    /// `to_bytes_prefix`, A3-01 : le rejeu d'un undo profond doit
    /// retrouver les blobs des imports antérieurs).
    fn gc_blobs(&mut self, references: &[String]) {
        let appliques: Vec<String> = self.doc.events()[..self.applied]
            .iter()
            .filter_map(|event| event.data.get("blob"))
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect();
        for nom in self.doc.blob_names() {
            if !references.contains(&nom) && !appliques.contains(&nom) {
                self.doc.remove_blob(&nom);
            }
        }
    }

    /// Applique un événement à toutes les projections.
    fn apply_all(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        self.values.apply(event)?;
        self.geom.apply(event)?;
        self.esquisse.apply(event)?;
        for projection in &mut self.projections {
            projection.apply(event)?;
        }
        Ok(())
    }

    /// Réinitialise toutes les projections.
    fn reset_all(&mut self) {
        self.values.reset();
        self.geom.reset();
        self.esquisse.reset();
        for projection in &mut self.projections {
            projection.reset();
        }
    }

    /// Rejoue le journal appliqué (`events[0..applied]`) dans toutes
    /// les projections, après les avoir réinitialisées.
    fn replay_applied(&mut self) -> Result<(), SessionError> {
        self.reset_all();
        let events: Vec<NewEvent> = self.doc.events()[..self.applied]
            .iter()
            .map(as_new)
            .collect();
        for event in &events {
            self.apply_all(event)?;
        }
        Ok(())
    }
}

/// Convertit un événement persisté en événement neutre pour
/// projection : la projection ne voit ni numéro de séquence, ni
/// horodatage.
fn as_new(event: &Event) -> NewEvent {
    NewEvent {
        event_type: event.event_type.clone(),
        schema_version: event.schema_version,
        actor: event.actor.clone(),
        data: event.data.clone(),
        txn: event.txn.clone(),
    }
}

/// Indice de début (exclusif) du groupe undo qui s'achève à `applied` :
/// une transaction entière (événements contigus de même identifiant),
/// ou l'événement isolé précédent.
fn groupe_start(events: &[Event], applied: usize) -> usize {
    let last = applied - 1;
    match events.get(last).and_then(|event| event.txn.as_deref()) {
        Some(txn) => {
            let mut start = last;
            while start > 0 && events[start - 1].txn.as_deref() == Some(txn) {
                start -= 1;
            }
            start
        }
        None => last,
    }
}

/// Indice de fin (exclusif) du groupe redo qui commence à `applied`.
fn groupe_end(events: &[Event], applied: usize) -> usize {
    if applied >= events.len() {
        return applied;
    }
    match events.get(applied).and_then(|event| event.txn.as_deref()) {
        Some(txn) => {
            let mut end = applied + 1;
            while end < events.len() && events[end].txn.as_deref() == Some(txn) {
                end += 1;
            }
            end
        }
        None => applied + 1,
    }
}
