#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-session — session, commandes et undo/redo
//!
//! (ADR-0008, ouverture du jalon 1.) La session possède le document
//! (forge-doc) et les projections métier ; les applications ne touchent
//! plus `Document` directement : elles construisent des
//! [commandes](Command) et lisent les projections.
//!
//! - **commande** : une intention utilisateur se planifie contre l'état
//!   courant et produit 1..n événements ; la session attribue à chacun
//!   l'acteur et un identifiant de transaction (une commande = un groupe
//!   d'undo) ;
//! - **validation avant écriture** : les événements sont d'abord
//!   appliqués aux projections ; si l'une refuse, **rien n'est écrit au
//!   journal** — un journal écrit par la session se rejoue toujours ;
//! - **undo/redo par curseur** : le curseur `applied` sépare le journal
//!   appliqué des événements défaits, qui restent en mémoire pour le
//!   redo ; une nouvelle commande après un undo **abandonne** les
//!   événements défaits ;
//! - **sauvegarde** : le fichier contient le journal linéaire visible ;
//!   enregistrer abandonne les événements en attente de redo.

mod command;
mod error;
mod projection;

pub use command::{Command, EVENT_EXPRESSION_SET, EVENT_INPUT_SET, SetExpression, SetInput};
pub use error::SessionError;
pub use projection::{Projection, ValuesProjection};

use forge_doc::{DocError, Document, Event, NewEvent};
use forge_values::ValueGraph;

/// Session de travail (ADR-0008) : possède le document forge-doc et les
/// projections par domaine ; les applications construisent des
/// [commandes](Command) et lisent les projections — elles ne touchent
/// jamais `Document` directement.
pub struct Session {
    /// Document possédé : journal, blobs, snapshot.
    doc: Document,
    /// Projection du domaine des liaisons (jalon 1).
    values: ValuesProjection,
    /// Projections des domaines futurs, enregistrées.
    projections: Vec<Box<dyn Projection>>,
    /// Nombre d'événements appliqués aux projections ; les événements
    /// suivants du journal sont défaits, en attente de redo.
    applied: usize,
    /// Identifiant de l'acteur attribué à chaque événement écrit.
    actor: String,
}

impl Session {
    /// Session vide, avec la projection des liaisons (jalon 1).
    ///
    /// `actor` identifie l'application ou la zone UI à l'origine des
    /// événements (ex. `desktop`, `android.machine`).
    pub fn new(actor: &str) -> Self {
        Self {
            doc: Document::new(),
            values: ValuesProjection::new(),
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
    /// application aux projections, puis écriture au journal. Un refus
    /// (au plan ou dans une projection) laisse le journal intact.
    /// Comme toute nouvelle action, une commande exécutée après un undo
    /// abandonne les événements défaits (ADR-0008).
    pub fn execute(&mut self, command: &dyn Command) -> Result<(), SessionError> {
        let events = command.plan(self)?;
        if events.is_empty() {
            return Ok(());
        }
        self.truncate_pending();
        for event in &events {
            if let Err(error) = self.apply_all(event) {
                // Rien n'a été écrit au journal : le rejeu du journal
                // appliqué rétablit exactement l'état d'avant commande.
                self.replay_applied()?;
                return Err(error);
            }
        }
        let txn = uuid::Uuid::now_v7().to_string();
        for mut event in events {
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

    /// Types d'événements ignorés par la projection des liaisons
    /// (journal écrit par une version plus récente ?).
    pub fn unknown_types(&self) -> &[String] {
        self.values.unknown_types()
    }

    /// Sérialise la session en conteneur ZIP (en mémoire). Les
    /// événements défaisables sont **abandonnés** : le fichier contient
    /// le journal linéaire visible (ADR-0008).
    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>, DocError> {
        self.truncate_pending();
        self.doc.to_bytes()
    }

    /// Enregistre la session dans un fichier, de manière atomique
    /// (feature `native`, cf. [`Document::save_to_path`]). Les
    /// événements défaisables sont abandonnés.
    #[cfg(feature = "native")]
    pub fn save_to_path(&mut self, path: &std::path::Path) -> Result<(), DocError> {
        self.truncate_pending();
        self.doc.save_to_path(path)
    }

    /// Abandonne les événements défaisables : le journal devient le
    /// journal visible (une nouvelle action ou une sauvegarde les
    /// supprime, ADR-0008).
    fn truncate_pending(&mut self) {
        if self.applied < self.doc.events().len() {
            self.doc.pop_after(self.applied as u64);
        }
    }

    /// Applique un événement à toutes les projections.
    fn apply_all(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        self.values.apply(event)?;
        for projection in &mut self.projections {
            projection.apply(event)?;
        }
        Ok(())
    }

    /// Réinitialise toutes les projections.
    fn reset_all(&mut self) {
        self.values.reset();
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
