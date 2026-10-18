#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-values — moteur unique de liaisons paramétriques
//!
//! (ADR-0000 verrou nº6, ADR-0002.) Un seul graphe de dépendances réactif
//! alimente les deux vues du paramétrage : alias/expressions et éditeur
//! nodal. Mode d'évaluation : **paresseux avec marquage dirty** —
//!
//! - écrire une valeur ([`ValueGraph::set_input`]) ou une expression
//!   ([`ValueGraph::set_expression`]) marque le nœud et, transitivement,
//!   tous ses dépendants « périmés », sans rien recalculer ;
//! - lire une valeur ([`ValueGraph::value`]) déclenche le recalcul du
//!   sous-graphe sale atteint, en remontant les dépendances (ordre
//!   topologique implicite du parcours en profondeur, chaque nœud n'est
//!   évalué qu'une fois) ;
//! - un cycle est une erreur typée [`ValuesError::Cycle`]
//!   (OF-VAL-0001), jamais une boucle infinie ;
//! - [`ValueGraph::evaluate_all`] force le recalcul complet et expose
//!   les erreurs cachées (ADR-0002 : commande « recalculer tout », à
//!   rendre obligatoire avant usinage) ;
//! - les expressions se limitent à un sous-ensemble **déterministe**
//!   (arithmétique, fonctions fixes), garantissant un rejeu identique —
//!   exigé par l'event sourcing (ADR-0001).

mod error;
mod expr;

pub use error::ValuesError;

use std::collections::{BTreeMap, BTreeSet};

use expr::Expr;

/// Identifiant interne d'un nœud du graphe.
pub(crate) type NodeId = u64;

/// État d'un nœud vis-à-vis du cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeState {
    /// Cache valide.
    Clean,
    /// À recalculer à la prochaine lecture.
    Dirty,
    /// En cours de recalcul ; le re-rencontrer pendant l'évaluation = cycle.
    InProgress,
}

/// Contenu d'un nœud.
#[derive(Debug, Clone)]
enum NodeKind {
    /// Valeur d'entrée fixée par l'utilisateur.
    Input(f64),
    /// Expression calculée à partir d'autres nœuds.
    Computed(Expr),
}

/// Nœud du graphe de liaisons.
#[derive(Debug)]
struct Node {
    name: String,
    kind: NodeKind,
    state: NodeState,
    cached: Option<f64>,
}

/// Graphe de liaisons paramétriques (moteur unique, ADR-0000 verrou nº6).
///
/// Les nœuds sont identifiés par nom ; une valeur d'entrée est fixée par
/// `set_input`, une valeur calculée par `set_expression`.
#[derive(Debug, Default)]
pub struct ValueGraph {
    next_id: NodeId,
    nodes: BTreeMap<NodeId, Node>,
    names: BTreeMap<String, NodeId>,
    deps: BTreeMap<NodeId, BTreeSet<NodeId>>,
    dependents: BTreeMap<NodeId, BTreeSet<NodeId>>,
}

impl ValueGraph {
    /// Nouveau graphe vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fixe la valeur d'entrée nommée et marque périmés ses dépendants.
    /// Un nombre non fini (NaN ou infini) est refusé : l'invariant de
    /// finitude tient au niveau du graphe, pas seulement de la
    /// commande (ADR-0016 A8).
    pub fn set_input(&mut self, name: &str, value: f64) -> Result<(), ValuesError> {
        if !value.is_finite() {
            return Err(ValuesError::NonFiniteInput {
                name: name.to_string(),
            });
        }
        let id = self.ensure_node(name);
        let node = match self.nodes.get_mut(&id) {
            Some(node) => node,
            None => return,
        };
        let was_computed = matches!(node.kind, NodeKind::Computed(_));
        node.kind = NodeKind::Input(value);
        if was_computed {
            self.clear_dependencies(id);
        }
        self.invalidate(id);
        Ok(())
    }

    /// Définit l'expression de la valeur nommée. Les noms référencés doivent
    /// déjà exister ; une référence inconnue est une erreur OF-VAL-0002.
    pub fn set_expression(&mut self, name: &str, text: &str) -> Result<(), ValuesError> {
        let parsed = expr::parse(text, name, &|n| self.names.get(n).copied())?;
        let mut new_deps = BTreeSet::new();
        expr::collect_references(&parsed, &mut new_deps);
        let id = self.ensure_node(name);
        self.clear_dependencies(id);
        self.deps.insert(id, new_deps.clone());
        for dep in &new_deps {
            self.dependents.entry(*dep).or_default().insert(id);
        }
        if let Some(node) = self.nodes.get_mut(&id) {
            node.kind = NodeKind::Computed(parsed);
        }
        self.invalidate(id);
        Ok(())
    }

    /// Lit la valeur demandée, en recalculant paresseusement ce qui est
    /// périmé. Les erreurs (cycle, division par zéro…) n'apparaissent qu'à
    /// cette lecture (ADR-0002).
    pub fn value(&mut self, name: &str) -> Result<f64, ValuesError> {
        let id = *self
            .names
            .get(name)
            .ok_or_else(|| ValuesError::UnknownValue {
                name: name.to_string(),
            })?;
        let mut stack = Vec::new();
        self.eval_node(id, &mut stack)
    }

    /// Recalcule toutes les valeurs et retourne la première erreur
    /// éventuelle (ADR-0002 : commande « recalculer tout »).
    pub fn evaluate_all(&mut self) -> Result<(), ValuesError> {
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            let mut stack = Vec::new();
            self.eval_node(id, &mut stack)?;
        }
        Ok(())
    }

    /// Noms de toutes les valeurs, triés.
    pub fn names(&self) -> Vec<String> {
        self.names.keys().cloned().collect()
    }

    /// `Some(true)` si la valeur est périmée, `Some(false)` si elle est à
    /// jour, `None` si elle n'existe pas.
    pub fn is_dirty(&self, name: &str) -> Option<bool> {
        let id = *self.names.get(name)?;
        self.nodes
            .get(&id)
            .map(|node| node.state == NodeState::Dirty)
    }

    fn ensure_node(&mut self, name: &str) -> NodeId {
        if let Some(id) = self.names.get(name) {
            return *id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.names.insert(name.to_string(), id);
        self.nodes.insert(
            id,
            Node {
                name: name.to_string(),
                kind: NodeKind::Input(0.0),
                state: NodeState::Dirty,
                cached: None,
            },
        );
        id
    }

    /// Retire les arêtes de dépendance sortantes d'un nœud.
    fn clear_dependencies(&mut self, id: NodeId) {
        let old_deps = self.deps.remove(&id).unwrap_or_default();
        for dep in &old_deps {
            if let Some(dependents) = self.dependents.get_mut(dep) {
                dependents.remove(&id);
            }
        }
    }

    /// Marque périmé un nœud et, transitivement, tous ses dépendants.
    fn invalidate(&mut self, id: NodeId) {
        let mut queue = vec![id];
        let mut visited = BTreeSet::new();
        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(&current) {
                node.state = NodeState::Dirty;
                node.cached = None;
            }
            if let Some(dependents) = self.dependents.get(&current) {
                for dependent in dependents {
                    queue.push(*dependent);
                }
            }
        }
    }

    /// Évalue un nœud : cache, ou remontée récursive de ses dépendances
    /// sales. La pile `stack` sert à la fois d'ordre topologique implicite
    /// et de détection de cycle.
    fn eval_node(&mut self, id: NodeId, stack: &mut Vec<NodeId>) -> Result<f64, ValuesError> {
        let state = match self.nodes.get(&id) {
            Some(node) => node.state,
            None => {
                return Err(ValuesError::UnknownValue {
                    name: format!("nœud interne #{id}"),
                });
            }
        };
        match state {
            NodeState::Clean => {
                let cached = match self.nodes.get(&id) {
                    Some(node) => node.cached,
                    None => None,
                };
                return Ok(cached.unwrap_or(0.0));
            }
            NodeState::InProgress => {
                let start = stack.iter().rposition(|i| *i == id).unwrap_or(0);
                let mut path: Vec<String> = Vec::new();
                for i in &stack[start..] {
                    if let Some(node) = self.nodes.get(i) {
                        path.push(node.name.clone());
                    }
                }
                if let Some(node) = self.nodes.get(&id) {
                    path.push(node.name.clone());
                }
                return Err(ValuesError::Cycle {
                    path: path.join(" → "),
                });
            }
            NodeState::Dirty => {}
        }
        if let Some(node) = self.nodes.get_mut(&id) {
            node.state = NodeState::InProgress;
        }
        stack.push(id);
        let node_data = match self.nodes.get(&id) {
            Some(node) => (node.name.clone(), node.kind.clone()),
            None => {
                stack.pop();
                return Err(ValuesError::UnknownValue {
                    name: format!("nœud interne #{id}"),
                });
            }
        };
        let (name, kind) = node_data;
        let result = match kind {
            NodeKind::Input(value) => Ok(value),
            NodeKind::Computed(parsed) => {
                expr::evaluate(&parsed, &name, &mut |dep| self.eval_node(dep, stack))
            }
        };
        stack.pop();
        match result {
            Ok(value) => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.state = NodeState::Clean;
                    node.cached = Some(value);
                }
                Ok(value)
            }
            Err(error) => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.state = NodeState::Dirty;
                }
                Err(error)
            }
        }
    }
}
