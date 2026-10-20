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
//!
//! Phase E (ADR-0019) : chaque nœud porte en outre une **unité**
//! optionnelle (ADR-0007 phase 1 : métadonnée de validation, cohérence
//! vérifiée à la définition) et une **origine** — utilisateur ou
//! système, les préfixes `esquisse.`, `param.` et `doc.` étant
//! réservés aux nœuds système. La suppression d'un alias se fait en
//! trois modes ([`ModeSuppression`]).

mod error;
mod expr;

pub use error::ValuesError;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use expr::Expr;

/// Préfixes de noms réservés aux nœuds système (ADR-0019 §2).
pub const PREFIXES_RESERVES: [&str; 3] = ["esquisse.", "param.", "doc."];

/// Identifiant interne d'un nœud du graphe.
pub(crate) type NodeId = u64;

/// Unité physique déclarée d'un nœud (ADR-0007 phase 1 : métadonnée de
/// validation, jamais état d'expression).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unite {
    /// Millimètre (code `mm`).
    Millimetre,
    /// Degré (code `deg`).
    Degre,
}

impl Unite {
    /// Code stable de l'unité, tel qu'accepté dans les événements v2.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Millimetre => "mm",
            Self::Degre => "deg",
        }
    }

    /// Retourne l'unité portant ce code, si le code est reconnu.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "mm" => Some(Self::Millimetre),
            "deg" => Some(Self::Degre),
            _ => None,
        }
    }
}

impl fmt::Display for Unite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Origine d'un nœud : créé par l'utilisateur ou par le système.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// Nœud créé par une commande utilisateur.
    Utilisateur,
    /// Nœud créé par le système (cotes d'esquisse, etc.).
    Systeme,
}

/// Mode de suppression d'un alias (ADR-0019 §1, amendement 2026-10-20).
#[derive(Debug, Clone)]
pub enum ModeSuppression {
    /// Refuse si l'alias est encore référencé (OF-VAL-0009).
    Simple,
    /// Supprime aussi, récursivement, chaque formule qui le référence.
    Cascade,
    /// Substitue `par` dans les formules et cotes, puis supprime.
    Remplacer {
        /// Nom de l'alias de remplacement, déjà existant.
        par: String,
    },
}

/// Bilan d'une suppression : noms supprimés et noms réécrits.
#[derive(Debug, Default)]
pub struct ResultatSuppression {
    /// Noms des nœuds supprimés, l'alias demandé en premier.
    pub supprimes: Vec<String>,
    /// Noms des nœuds dont l'expression a été réécrite (mode Remplacer).
    pub modifies: Vec<String>,
}

/// Informations publiques sur un nœud du graphe.
#[derive(Debug, Clone, PartialEq)]
pub struct InfoNoeud {
    /// Unité déclarée à la définition (`None` = sans unité).
    pub unite: Option<Unite>,
    /// Texte de l'expression si le nœud est calculé, `None` sinon.
    pub expression: Option<String>,
    /// `true` si le nœud a été créé par le système.
    pub systeme: bool,
}

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
    /// Expression calculée : arbre parsé et texte source conservé.
    Computed {
        /// Arbre de syntaxe parsé.
        arbre: Expr,
        /// Texte source, conservé pour l'affichage et la réécriture.
        texte: String,
    },
}

/// Nœud du graphe de liaisons.
#[derive(Debug)]
struct Node {
    name: String,
    kind: NodeKind,
    state: NodeState,
    cached: Option<f64>,
    unite: Option<Unite>,
    origine: Origine,
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
    /// Variante de compatibilité : sans unité déclarée, origine
    /// utilisateur. Un nombre non fini (NaN ou infini) est refusé :
    /// l'invariant de finitude tient au niveau du graphe, pas seulement
    /// de la commande (ADR-0016 A8).
    pub fn set_input(&mut self, name: &str, value: f64) -> Result<(), ValuesError> {
        self.set_input_unite(name, value, None, Origine::Utilisateur)
    }

    /// Fixe la valeur d'entrée nommée avec son unité et son origine.
    pub fn set_input_unite(
        &mut self,
        name: &str,
        value: f64,
        unite: Option<Unite>,
        origine: Origine,
    ) -> Result<(), ValuesError> {
        if !value.is_finite() {
            return Err(ValuesError::NonFiniteInput {
                name: name.to_string(),
            });
        }
        self.verifie_cree(name, origine)?;
        let id = self.ensure_node(name, origine);
        let node = match self.nodes.get_mut(&id) {
            Some(node) => node,
            None => return Ok(()),
        };
        let was_computed = matches!(node.kind, NodeKind::Computed { .. });
        node.kind = NodeKind::Input(value);
        node.unite = unite;
        if was_computed {
            self.clear_dependencies(id);
        }
        self.invalidate(id);
        Ok(())
    }

    /// Définit l'expression de la valeur nommée. Les noms référencés doivent
    /// déjà exister ; une référence inconnue est une erreur OF-VAL-0002.
    /// Variante de compatibilité : sans unité déclarée, origine utilisateur.
    pub fn set_expression(&mut self, name: &str, text: &str) -> Result<(), ValuesError> {
        self.set_expression_unite(name, text, None, Origine::Utilisateur)
    }

    /// Définit l'expression de la valeur nommée, avec son unité déclarée
    /// et son origine. La cohérence des unités est vérifiée à la
    /// définition (OF-VAL-0007) : une unité déclarée ne peut lire que
    /// des nœuds de la même unité ou sans unité.
    pub fn set_expression_unite(
        &mut self,
        name: &str,
        text: &str,
        unite: Option<Unite>,
        origine: Origine,
    ) -> Result<(), ValuesError> {
        self.verifie_cree(name, origine)?;
        let id = self.ensure_node(name, origine);
        self.definir_expression(id, text, unite)
    }

    /// Définit une expression système (cote d'esquisse, etc.), sous un
    /// préfixe réservé en général. Un nœud système n'est jamais
    /// supprimable directement (OF-VAL-0008).
    pub fn set_expression_systeme(
        &mut self,
        name: &str,
        text: &str,
        unite: Option<Unite>,
    ) -> Result<(), ValuesError> {
        self.set_expression_unite(name, text, unite, Origine::Systeme)
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

    /// Informations sur un nœud : unité déclarée, texte de l'expression
    /// et origine système.
    pub fn noeud(&self, name: &str) -> Option<InfoNoeud> {
        let node = self.noeud_par_nom(name)?;
        Some(InfoNoeud {
            unite: node.unite,
            expression: match &node.kind {
                NodeKind::Computed { texte, .. } => Some(texte.clone()),
                NodeKind::Input(_) => None,
            },
            systeme: node.origine == Origine::Systeme,
        })
    }

    /// Noms des dépendants directs du nœud, triés.
    pub fn dependants(&self, name: &str) -> Option<Vec<String>> {
        let id = *self.names.get(name)?;
        Some(self.noms_depuis_ids(&self.dependants_ids(id)))
    }

    /// Noms des dépendances transitives du nœud (tout ce qu'il lit,
    /// directement ou non), triés, le nœud lui-même exclu.
    pub fn deps_transitifs(&self, name: &str) -> Option<Vec<String>> {
        let id = *self.names.get(name)?;
        let mut vus = BTreeSet::new();
        self.collecte_deps_transitifs(id, &mut vus);
        Some(self.noms_depuis_ids(&vus))
    }

    /// Supprime un nœud selon le mode demandé (ADR-0019 §1). Les nœuds
    /// système ne sont jamais supprimables directement (OF-VAL-0008) ;
    /// le mode `Simple` refuse un alias encore référencé (OF-VAL-0009),
    /// `Cascade` supprime aussi les formules qui le lisent, `Remplacer`
    /// substitue un autre alias dans les formules avant de supprimer.
    pub fn supprimer(
        &mut self,
        name: &str,
        mode: &ModeSuppression,
    ) -> Result<ResultatSuppression, ValuesError> {
        let Some(id) = self.names.get(name).copied() else {
            return Err(ValuesError::UnknownValue {
                name: name.to_string(),
            });
        };
        let origine = match self.noeud_par_id(id) {
            Some(node) => node.origine,
            None => {
                return Err(ValuesError::UnknownValue {
                    name: name.to_string(),
                });
            }
        };
        if origine == Origine::Systeme {
            return Err(ValuesError::AliasReserve {
                name: name.to_string(),
            });
        }
        match mode {
            ModeSuppression::Simple => {
                let references = self.noms_depuis_ids(&self.dependants_ids(id));
                if !references.is_empty() {
                    return Err(ValuesError::AliasReference {
                        name: name.to_string(),
                        references: references.join(", "),
                    });
                }
                self.retirer_id(id);
                let mut resultat = ResultatSuppression::default();
                resultat.supprimes.push(name.to_string());
                Ok(resultat)
            }
            ModeSuppression::Cascade => {
                let transitifs = self.dependants_transitifs_ids(id);
                let mut systeme = Vec::new();
                for tid in &transitifs {
                    let node = match self.noeud_par_id(*tid) {
                        Some(node) => node,
                        None => continue,
                    };
                    if node.origine == Origine::Systeme {
                        systeme.push(node.name.clone());
                    }
                }
                if !systeme.is_empty() {
                    return Err(ValuesError::AliasReference {
                        name: name.to_string(),
                        references: systeme.join(", "),
                    });
                }
                let mut resultat = ResultatSuppression::default();
                resultat.supprimes.push(name.to_string());
                let mut noms = self.noms_depuis_ids(&transitifs);
                noms.sort();
                resultat.supprimes.extend(noms);
                for tid in &transitifs {
                    self.retirer_id(*tid);
                }
                self.retirer_id(id);
                Ok(resultat)
            }
            ModeSuppression::Remplacer { par } => {
                let Some(cible) = self.names.get(par).copied() else {
                    return Err(ValuesError::UnknownValue {
                        name: par.clone(),
                    });
                };
                let mut vus = BTreeSet::new();
                self.collecte_deps_transitifs(cible, &mut vus);
                if cible == id || vus.contains(&id) {
                    return Err(ValuesError::Cycle {
                        path: format!("{name} → {par} → {name}"),
                    });
                }
                // validation complète avant toute modification : la
                // substitution ne doit rendre aucune formule invalide
                let dependants = self.dependants_ids(id);
                let mut reecritures: Vec<(NodeId, String, Option<Unite>)> = Vec::new();
                for dep in &dependants {
                    let texte = match self.noeud_par_id(*dep) {
                        Some(node) => match &node.kind {
                            NodeKind::Computed { texte, .. } => texte.clone(),
                            NodeKind::Input(_) => continue,
                        },
                        None => continue,
                    };
                    let nouveau = remplacer_identifiants(&texte, name, par);
                    let unite = self.unite_de(*dep);
                    self.prepare_expression(*dep, &nouveau, unite)?;
                    reecritures.push((*dep, nouveau, unite));
                }
                let mut resultat = ResultatSuppression::default();
                for (dep, nouveau, unite) in reecritures {
                    self.definir_expression(dep, &nouveau, unite)?;
                    if let Some(node) = self.noeud_par_id(dep) {
                        resultat.modifies.push(node.name.clone());
                    }
                }
                self.retirer_id(id);
                resultat.supprimes.push(name.to_string());
                Ok(resultat)
            }
        }
    }

    /// Retire du graphe le nœud nommé, sans aucune vérification
    /// (les dépendances qui le lisent deviennent des références
    /// mortes : réservé aux projections qui reconstruisent l'état).
    pub fn retirer_noeud(&mut self, name: &str) {
        if let Some(id) = self.names.get(name).copied() {
            self.retirer_id(id);
        }
    }

    /// Refuse la création utilisateur sous un préfixe réservé et tout
    /// écrasement d'un nœud d'une autre origine (OF-VAL-0008).
    fn verifie_cree(&self, name: &str, origine: Origine) -> Result<(), ValuesError> {
        if origine == Origine::Utilisateur
            && PREFIXES_RESERVES.iter().any(|p| name.starts_with(p))
        {
            return Err(ValuesError::AliasReserve {
                name: name.to_string(),
            });
        }
        if let Some(node) = self.noeud_par_nom(name) {
            if node.origine != origine {
                return Err(ValuesError::AliasReserve {
                    name: name.to_string(),
                });
            }
        }
        Ok(())
    }

    fn noeud_par_id(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    fn noeud_par_nom(&self, name: &str) -> Option<&Node> {
        let id = self.names.get(name)?;
        self.nodes.get(id)
    }

    fn unite_de(&self, id: NodeId) -> Option<Unite> {
        match self.noeud_par_id(id) {
            Some(node) => node.unite,
            None => None,
        }
    }

    /// Parse et valide une expression pour le nœud `id` : références
    /// résolues (OF-VAL-0002/0003) et cohérence des unités vérifiée à
    /// la définition (OF-VAL-0007).
    fn prepare_expression(
        &self,
        id: NodeId,
        text: &str,
        unite: Option<Unite>,
    ) -> Result<(Expr, BTreeSet<NodeId>), ValuesError> {
        let nom = match self.noeud_par_id(id) {
            Some(node) => node.name.clone(),
            None => {
                return Err(ValuesError::UnknownValue {
                    name: format!("nœud interne #{id}"),
                });
            }
        };
        let parsed = expr::parse(text, &nom, &|n| self.names.get(n).copied())?;
        let mut refs = BTreeSet::new();
        expr::collect_references(&parsed, &mut refs);
        for dep in &refs {
            if *dep == id {
                continue;
            }
            let (nom_dep, unite_dep) = match self.noeud_par_id(*dep) {
                Some(node) => (node.name.clone(), node.unite),
                None => continue,
            };
            if let (Some(u), Some(u_dep)) = (unite, unite_dep) {
                if u != u_dep {
                    return Err(ValuesError::UniteIncoherente {
                        node: nom.clone(),
                        unite: u,
                        reference: nom_dep,
                        unite_reference: u_dep,
                    });
                }
            }
        }
        Ok((parsed, refs))
    }

    /// Installe l'expression validée sur le nœud `id` et marque
    /// périmés le nœud et ses dépendants.
    fn installer_expression(
        &mut self,
        id: NodeId,
        parsed: Expr,
        texte: &str,
        refs: BTreeSet<NodeId>,
        unite: Option<Unite>,
    ) {
        self.clear_dependencies(id);
        for dep in &refs {
            self.dependents.entry(*dep).or_default().insert(id);
        }
        self.deps.insert(id, refs);
        if let Some(node) = self.nodes.get_mut(&id) {
            node.kind = NodeKind::Computed {
                arbre: parsed,
                texte: texte.to_string(),
            };
            node.unite = unite;
        }
        self.invalidate(id);
    }

    /// Valide puis installe une expression sur le nœud `id`.
    fn definir_expression(
        &mut self,
        id: NodeId,
        text: &str,
        unite: Option<Unite>,
    ) -> Result<(), ValuesError> {
        let (parsed, refs) = self.prepare_expression(id, text, unite)?;
        self.installer_expression(id, parsed, text, refs, unite);
        Ok(())
    }

    fn ensure_node(&mut self, name: &str, origine: Origine) -> NodeId {
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
                unite: None,
                origine,
            },
        );
        id
    }

    fn dependants_ids(&self, id: NodeId) -> BTreeSet<NodeId> {
        match self.dependents.get(&id) {
            Some(dependents) => dependents.clone(),
            None => BTreeSet::new(),
        }
    }

    /// Dépendants transitifs du nœud : tous les nœuds qui le lisent,
    /// directement ou non.
    fn dependants_transitifs_ids(&self, id: NodeId) -> BTreeSet<NodeId> {
        let mut vus = BTreeSet::new();
        let mut queue = vec![id];
        while let Some(current) = queue.pop() {
            if let Some(dependents) = self.dependents.get(&current) {
                for dependent in dependents {
                    if vus.insert(*dependent) {
                        queue.push(*dependent);
                    }
                }
            }
        }
        vus
    }

    /// Dépendances transitives du nœud : tout ce qu'il lit, directement
    /// ou non (le nœud lui-même est exclu).
    fn collecte_deps_transitifs(&self, id: NodeId, vus: &mut BTreeSet<NodeId>) {
        if let Some(deps) = self.deps.get(&id) {
            for dep in deps {
                if vus.insert(*dep) {
                    self.collecte_deps_transitifs(*dep, vus);
                }
            }
        }
    }

    fn noms_depuis_ids(&self, ids: &BTreeSet<NodeId>) -> Vec<String> {
        let mut noms: Vec<String> = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(node) = self.noeud_par_id(*id) {
                noms.push(node.name.clone());
            }
        }
        noms.sort();
        noms
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
            NodeKind::Computed { arbre, .. } => {
                expr::evaluate(&arbre, &name, &mut |dep| self.eval_node(dep, stack))
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

/// Remplace dans `texte` chaque occurrence de l'identifiant `de` par
/// `par`, sans toucher les identifiants qui contiennent `de` (bordures
/// respectées : ni voisin alphanumérique avant, ni après).
pub fn remplacer_identifiants(texte: &str, de: &str, par: &str) -> String {
    let cible: Vec<char> = de.chars().collect();
    if cible.is_empty() {
        return texte.to_string();
    }
    let est_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let lettres: Vec<char> = texte.chars().collect();
    let mut resultat = String::with_capacity(texte.len());
    let mut i = 0;
    while i < lettres.len() {
        let fin = i + cible.len();
        let correspond = fin <= lettres.len()
            && (i == 0 || !est_ident(lettres[i - 1]))
            && (fin == lettres.len() || !est_ident(lettres[fin]))
            && lettres[i..fin] == cible[..];
        if correspond {
            resultat.push_str(par);
            i = fin;
        } else {
            resultat.push(lettres[i]);
            i += 1;
        }
    }
    resultat
}
