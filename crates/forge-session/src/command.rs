//! Commandes de la session (ADR-0008, ADR-0011) : une intention
//! utilisateur = une validation contre l'état courant, puis 1..n
//! événements et d'éventuels blobs à déposer.
//!
//! Phase E (ADR-0019) : les liaisons passent au schéma v2 — chaque
//! valeur déclare son unité (ADR-0007 phase 1) — la suppression
//! d'alias arrive en trois modes ([`RemoveAlias`]) et les cotes
//! d'esquisse se pilotent par expressions
//! ([`crate::esquisse::ValeurCote`]).

use crate::cad::{
    CouleurFeature, FeatureCad, ModeExtrusion, ProfondeurCad, ProfondeurFeature, feature_en_json,
    premier_contour_ferme, segments_du_croquis,
};
use crate::esquisse::{ContrainteEsquisse, EntiteEsquisse, ValeurCote};
use crate::{Session, SessionError};
use forge_doc::NewEvent;
use forge_geom::MAGNITUDE_MAX_MM;
use forge_units::Length;
use forge_values::ValuesError;
use forge_values::{
    LONGUEUR_MAX_EXPRESSION, ModeSuppression, PREFIXES_RESERVES, Unite, remplacer_identifiants,
};
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
/// (schéma v2 : `croquis`, `point`, `x`, `y` — ADR-0024 §7, lot 2).
pub const EVENT_SKETCH_POINT_MOVED: &str = "sketch.point.moved";
/// Type d'événement : croquis nommé créé (schéma v1 : `nom`, `plan`
/// — ADR-0024 §7, lot 2) ; le croquis créé devient l'actif.
pub const EVENT_SKETCH_CREATED: &str = "sketch.created";
/// Type d'événement : feature CAO ajoutée (schéma v1 : `id`,
/// `feature` — ADR-0024 §7, lot 2).
pub const EVENT_FEATURE_ADDED: &str = "cad.feature.added";
/// Type d'événement : feature CAO remplacée à identifiant conservé
/// (schéma v1, même forme que `cad.feature.added` — ADR-0024 §7,
/// lot 2).
pub const EVENT_FEATURE_UPDATED: &str = "cad.feature.updated";

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
    // Un alias nommé comme une fonction du langage (`min`, `abs`…)
    // serait créable mais jamais référençable : le lexeur lit
    // « min( » comme un appel — refus au plan, rien au journal
    // (audit n°2, N2).
    if forge_values::est_nom_de_fonction(name) {
        return Err(SessionError::CommandRejected {
            reason: format!("le nom « {name} » désigne une fonction des expressions"),
        });
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
        verifie_nom_chemin(&self.name)?;
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
        verifie_nom_chemin(&self.name)?;
        verifie_prefixe_libre(&self.name)?;
        if self.expression.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "l'expression ne peut pas être vide".to_string(),
            });
        }
        // Borne de longueur (audit n°3, A3-02) : refus au plan, rien
        // au journal — la borne vit aussi dans forge-values, un
        // journal forgé ne passe jamais le parseur.
        if self.expression.chars().count() > LONGUEUR_MAX_EXPRESSION {
            return Err(SessionError::CommandRejected {
                reason: format!("l'expression dépasse {LONGUEUR_MAX_EXPRESSION} caractères"),
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
                // Cote d'esquisse ou profondeur de feature : la
                // contrainte ou la feature porte le texte de
                // l'expression, elle suit la substitution à
                // identifiant conservé (ADR-0019 §1, ADR-0024 §7).
                if let Some((croquis, id, mut contrainte)) =
                    session.esquisse().contrainte_de_cote_situation(&dep)
                {
                    if let ContrainteEsquisse::Distance {
                        valeur: ValeurCote::Expression { texte, .. },
                        ..
                    } = &mut contrainte
                    {
                        *texte = nouveau;
                    }
                    events.push(NewEvent::new(
                        EVENT_SKETCH_CONSTRAINT_UPDATED,
                        crate::esquisse::VERSION_SCHEMA_MISE_A_JOUR,
                        "",
                        crate::esquisse::contrainte_en_json(croquis, id, &contrainte),
                    ));
                } else if let Some((id, mut feature)) = session.cad().feature_de_alias(&dep) {
                    if let FeatureCad::Extrusion {
                        profondeur: ProfondeurCad::Cote(ValeurCote::Expression { texte, .. }),
                        ..
                    } = &mut feature
                    {
                        *texte = nouveau;
                    }
                    events.push(NewEvent::new(
                        EVENT_FEATURE_UPDATED,
                        crate::cad::VERSION_SCHEMA_FEATURE,
                        "",
                        serde_json::json!({
                            "id": id,
                            "feature": feature_en_json(&feature),
                        }),
                    ));
                }
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
        // Croquis actif depuis le lot 2 (ADR-0024 §7) : l'entité vit
        // dans le croquis visé, identifiant séquentiel PAR croquis.
        let croquis = esquisse.croquis_actif();
        let id = esquisse.prochaine_id_entite();
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_SKETCH_ENTITY_ADDED,
            crate::esquisse::VERSION_SCHEMA_ENTITE,
            "",
            crate::esquisse::entite_en_json(croquis, id, &self.entite),
        )]))
    }
}

/// Ajoute une contrainte au croquis contraint (ADR-0018 D1) : les
/// onze genres du solveur plus l'ancrage d'un point. Un refus
/// (entités référencées absentes, cote invalide) ne laisse rien au
/// journal (OF-SES-0001). Orientation (ADR-0021 D1, révisée le
/// 2026-10-22 — retour checklist jalon 2, alignement FreeCAD) :
/// une ligne portant déjà une contrainte d'orientation refuse
/// toute pose d'orientation — même genre (redondance) ou genre
/// opposé ; l'ancienne décision de bascule est retirée, et cette
/// commande n'émet plus `sketch.constraint.updated` (les anciens
/// journaux qui l'ont porté se rejouent tels quels).
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
        // Orientation (ADR-0021 D1, révisée le 2026-10-22 — retour
        // checklist jalon 2, alignement FreeCAD) : une ligne porte
        // au plus une contrainte d'orientation, et toute pose
        // d'orientation sur une ligne déjà orientée est REFUSÉE —
        // même genre (redondance) ou genre opposé (la bascule est
        // retirée). Rien au journal dans les deux cas.
        let orientation = match &self.contrainte {
            ContrainteEsquisse::Horizontale { ligne } => Some((*ligne, true)),
            ContrainteEsquisse::Verticale { ligne } => Some((*ligne, false)),
            _ => None,
        };
        if let Some((ligne, horizontale)) = orientation
            && let Some((_, horiz_existante)) =
                esquisse
                    .contraintes()
                    .iter()
                    .find_map(|(id, contrainte)| match contrainte {
                        ContrainteEsquisse::Horizontale { ligne: l } if *l == ligne => {
                            Some((*id, true))
                        }
                        ContrainteEsquisse::Verticale { ligne: l } if *l == ligne => {
                            Some((*id, false))
                        }
                        _ => None,
                    })
        {
            let existant = if horiz_existante {
                "horizontale"
            } else {
                "verticale"
            };
            if horiz_existante == horizontale {
                return Err(SessionError::CommandRejected {
                    reason: format!("la ligne {ligne} porte déjà une contrainte {existant}"),
                });
            }
            let demande = if horizontale {
                "horizontale"
            } else {
                "verticale"
            };
            let raison = format!("la ligne {ligne} porte déjà une contrainte {existant}");
            return Err(SessionError::CommandRejected {
                reason: format!("{raison} : annulez-la (Ctrl+Z) avant d'en poser une {demande}"),
            });
        }
        let id = esquisse.prochaine_id_contrainte();
        // Croquis actif depuis le lot 2 (ADR-0024 §7) : la contrainte
        // vit dans le croquis visé, identifiant séquentiel PAR croquis.
        let croquis = esquisse.croquis_actif();
        let valeur = match &self.contrainte {
            ContrainteEsquisse::Distance { valeur, .. } => Some(valeur),
            _ => None,
        };
        let Some(ValeurCote::Expression { texte, alias }) = valeur else {
            return Ok(CommandPlan::new(vec![NewEvent::new(
                EVENT_SKETCH_CONSTRAINT_ADDED,
                crate::esquisse::VERSION_SCHEMA_CONTRAINTE,
                "",
                crate::esquisse::contrainte_en_json(croquis, id, &self.contrainte),
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
            crate::esquisse::VERSION_SCHEMA_CONTRAINTE,
            "",
            crate::esquisse::contrainte_en_json(croquis, id, &self.contrainte),
        ));
        Ok(CommandPlan::new(events))
    }
}

/// Ancre un point à sa position affichée (ADR-0021 D2, révisée le
/// 2026-10-23 — re-test jalon 2, point 8) : la pose capture la
/// position que l'utilisateur voit — résolue de la passe épinglée
/// après un geste, graine du journal sinon — et la fige avant la
/// première résolution : la forme ne saute jamais, le point reste
/// là où il est dessiné, le reste du croquis absorbe (alignement
/// FreeCAD). Deux événements dans la même transaction — la graine
/// part à la position capturée puis la contrainte l'épingle (même
/// mécanique que la cote pilotée, ADR-0019 décision 4) : contrainte
/// et déplacement s'annulent et se rétablissent ensemble, et le
/// journal rejoué reconstruit la capture. Un refus (point absent,
/// point déjà ancré, coordonnées non finies) ne laisse rien au
/// journal (OF-SES-0001).
#[derive(Debug, Clone)]
pub struct AncrerPoint {
    /// Identifiant du point ancré.
    pub point: u32,
    /// Abscisse affichée capturée (millimètres).
    pub x: Length,
    /// Ordonnée affichée capturée (millimètres).
    pub y: Length,
}

impl Command for AncrerPoint {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        let esquisse = session.esquisse();
        // L'ancrage est unique par point : reposer un ancrage sur
        // un point déjà fixé est refusé, rien au journal.
        if esquisse.point_ancre(self.point) {
            return Err(SessionError::CommandRejected {
                reason: format!(
                    "le point {} est déjà ancré par une contrainte PointFixe",
                    self.point
                ),
            });
        }
        if let Some(reason) = esquisse.raison_refus_deplacement(self.point, self.x, self.y) {
            return Err(SessionError::CommandRejected { reason });
        }
        let id = esquisse.prochaine_id_contrainte();
        // Croquis actif depuis le lot 2 (ADR-0024 §7) : le
        // déplacement capturé et la contrainte d'ancrage visent le
        // même croquis, dans la même transaction.
        let croquis = esquisse.croquis_actif();
        Ok(CommandPlan::new(vec![
            NewEvent::new(
                EVENT_SKETCH_POINT_MOVED,
                crate::esquisse::VERSION_SCHEMA_DEPLACEMENT,
                "",
                serde_json::json!({
                    "croquis": croquis,
                    "point": self.point,
                    "x": self.x.millimeters(),
                    "y": self.y.millimeters(),
                }),
            ),
            NewEvent::new(
                EVENT_SKETCH_CONSTRAINT_ADDED,
                crate::esquisse::VERSION_SCHEMA_CONTRAINTE,
                "",
                crate::esquisse::contrainte_en_json(
                    croquis,
                    id,
                    &ContrainteEsquisse::PointFixe { point: self.point },
                ),
            ),
        ]))
    }
}

/// Pose la graine de position d'un point du croquis (ADR-0018 D1) :
/// la nouvelle position devient le point de départ de la prochaine
/// résolution — jamais une position résolue au journal. Un refus
/// (point absent, coordonnées non finies, point ancré par un
/// `PointFixe`, ADR-0021) ne laisse rien au journal (OF-SES-0001).
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
        // Un point ancré ne se déplace plus (ADR-0021 D3) : la
        // graine d'un `PointFixe` ne bouge pas — l'ancrage reste
        // effectif face à un glisser tenté depuis l'atelier comme
        // à une commande scriptée.
        if esquisse.point_ancre(self.point) {
            return Err(SessionError::CommandRejected {
                reason: format!(
                    "le point {} est ancré par une contrainte PointFixe",
                    self.point
                ),
            });
        }
        if let Some(reason) = esquisse.raison_refus_deplacement(self.point, self.x, self.y) {
            return Err(SessionError::CommandRejected { reason });
        }
        // Croquis actif depuis le lot 2 (ADR-0024 §7) : le
        // déplacement vise le croquis courant.
        let croquis = esquisse.croquis_actif();
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_SKETCH_POINT_MOVED,
            crate::esquisse::VERSION_SCHEMA_DEPLACEMENT,
            "",
            serde_json::json!({
                "croquis": croquis,
                "point": self.point,
                "x": self.x.millimeters(),
                "y": self.y.millimeters(),
            }),
        )]))
    }
}

/// Crée un croquis nommé (ADR-0024 §7, lot 2) : il devient le croquis
/// actif — les commandes d'esquisse s'y adressent jusqu'au prochain
/// `sketch.created`. Un nom vide ou déjà pris est refusé, rien au
/// journal (OF-SES-0001) ; au rejeu d'un journal hostile, le nom
/// dupliqué est un refus de données (OF-SES-0005) — le journal fait
/// foi, la projection ne devine jamais.
#[derive(Debug, Clone)]
pub struct CreateSketch {
    /// Nom du croquis, unique dans la session.
    pub nom: String,
}

impl Command for CreateSketch {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        if self.nom.trim().is_empty() {
            return Err(SessionError::CommandRejected {
                reason: "le nom du croquis ne peut pas être vide".to_string(),
            });
        }
        if session.esquisse().croquis_de_nom(&self.nom).is_some() {
            return Err(SessionError::CommandRejected {
                reason: format!("le nom de croquis « {} » est déjà pris", self.nom),
            });
        }
        Ok(CommandPlan::new(vec![NewEvent::new(
            EVENT_SKETCH_CREATED,
            crate::esquisse::VERSION_SCHEMA_CROQUIS,
            "",
            serde_json::json!({
                "nom": self.nom,
                "plan": { "type": "xy" },
            }),
        )]))
    }
}

/// Profondeur journalisée d'une commande de feature (ADR-0024 D4) :
/// la profondeur en expression devient une cote pilotée par l'alias
/// système `cad.extrusion{id}.profondeur`, posé dans la même
/// transaction — le préfixe `cad.` est réservé (OF-VAL-0008), la
/// création utilisateur y est refusée.
fn profondeur_de_commande(id: u32, profondeur: &ProfondeurFeature) -> ProfondeurCad {
    match profondeur {
        ProfondeurFeature::Fixe { mm } => ProfondeurCad::Cote(ValeurCote::Fixe { mm: *mm }),
        ProfondeurFeature::Expression { texte } => ProfondeurCad::Cote(ValeurCote::Expression {
            texte: texte.clone(),
            alias: format!("cad.extrusion{id}.profondeur"),
        }),
        ProfondeurFeature::ATravers => ProfondeurCad::ATravers,
    }
}

/// Garde commune des commandes de feature (ADR-0024 §7, lot 2) :
/// croquis connu et pourvu d'un contour fermé aux graines du journal,
/// profondeur valide, couleur aux composantes RGBA finies et dans
/// 0..=1 (ADR-0036 D2) — un refus ne laisse rien au journal
/// (OF-SES-0001).
fn feature_valide(
    session: &Session,
    croquis: u32,
    profondeur: &ProfondeurFeature,
    couleur: &Option<CouleurFeature>,
) -> Result<(), SessionError> {
    let esquisse = session.esquisse();
    let Some(entites) = esquisse.entites_du_croquis(croquis) else {
        return Err(SessionError::CommandRejected {
            reason: format!("le croquis {croquis} n'existe pas"),
        });
    };
    let positions = |point: u32| esquisse.position_graine_du_croquis(croquis, point);
    if premier_contour_ferme(&segments_du_croquis(entites, &positions)).is_none() {
        return Err(SessionError::CommandRejected {
            reason: format!("le croquis {croquis} n'a pas de contour fermé à extruder"),
        });
    }
    match profondeur {
        ProfondeurFeature::Fixe { mm } => {
            // ADR-0036 D1 (A5-04) : la borne haute manquait — 1e300 mm
            // passait au plan. La garde commune refuse les profondeurs
            // non finies, négatives et démesurées, rien au journal.
            if !crate::cad::profondeur_valide(mm.millimeters()) {
                return Err(SessionError::CommandRejected {
                    reason: format!(
                        "la profondeur doit être finie, strictement positive et au plus {MAGNITUDE_MAX_MM} mm"
                    ),
                });
            }
        }
        ProfondeurFeature::Expression { texte } => {
            if texte.trim().is_empty() {
                return Err(SessionError::CommandRejected {
                    reason: "l'expression de la profondeur ne peut pas être vide".to_string(),
                });
            }
            // Borne de longueur (audit n°3, A3-02) : même garde que
            // `SetExpression`, refus au plan, rien au journal.
            if texte.chars().count() > LONGUEUR_MAX_EXPRESSION {
                return Err(SessionError::CommandRejected {
                    reason: format!("l'expression dépasse {LONGUEUR_MAX_EXPRESSION} caractères"),
                });
            }
        }
        ProfondeurFeature::ATravers => {}
    }
    // ADR-0036 D2 (A5-04) : composantes RGBA finies ET dans 0..=1 —
    // un rouge à 2.0 ou un alpha négatif passaient au plan. Même
    // garde commune que le rejeu (`crate::cad::couleur_valide`),
    // refus OF-SES-0001, rien au journal.
    if let Some(couleur) = couleur && !crate::cad::couleur_valide(couleur) {
        return Err(SessionError::CommandRejected {
            reason: "les composantes RGBA de la couleur doivent être finies et dans 0..=1"
                .to_string(),
        });
    }
    Ok(())
}

/// Plan d'une commande de feature (ADR-0024 §7, lot 2) : la profondeur
/// en expression pose D'ABORD le nœud système
/// `cad.extrusion{id}.profondeur` — même mécanique que la cote
/// pilotée (ADR-0019 décision 4) — puis la feature elle-même ; deux
/// événements, une transaction, un seul groupe d'undo (ADR-0011).
fn plan_feature(id: u32, feature: &FeatureCad, event_type: &str) -> CommandPlan {
    let mut events = Vec::new();
    if let FeatureCad::Extrusion {
        profondeur: ProfondeurCad::Cote(ValeurCote::Expression { texte, alias }),
        ..
    } = feature
    {
        events.push(NewEvent::new(
            EVENT_EXPRESSION_SET,
            2,
            "",
            serde_json::json!({
                "name": alias,
                "expression": texte,
                "unit": "mm",
            }),
        ));
    }
    events.push(NewEvent::new(
        event_type,
        crate::cad::VERSION_SCHEMA_FEATURE,
        "",
        serde_json::json!({
            "id": id,
            "feature": feature_en_json(feature),
        }),
    ));
    CommandPlan::new(events)
}

/// Ajoute une feature d'extrusion au corps CAO (ADR-0024 §7, lot 2) :
/// le croquis doit exister et porter un contour fermé, la profondeur
/// est une cote fixe, pilotée par expression ou « à travers »
/// (enveloppe dérivée du corps courant, jamais journalisée,
/// ADR-0024 D4). Un refus ne laisse rien au journal (OF-SES-0001).
#[derive(Debug, Clone)]
pub struct AddFeature {
    /// Croquis extrudé.
    pub croquis: u32,
    /// Mode : addition au corps courant ou soustraction.
    pub mode: ModeExtrusion,
    /// Profondeur d'extrusion.
    pub profondeur: ProfondeurFeature,
    /// Couleur d'affichage (facultative).
    pub couleur: Option<CouleurFeature>,
}

impl Command for AddFeature {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        feature_valide(session, self.croquis, &self.profondeur, &self.couleur)?;
        let id = session.cad().prochaine_id_feature();
        let feature = FeatureCad::Extrusion {
            croquis: self.croquis,
            mode: self.mode,
            profondeur: profondeur_de_commande(id, &self.profondeur),
            couleur: self.couleur.clone(),
        };
        Ok(plan_feature(id, &feature, EVENT_FEATURE_ADDED))
    }
}

/// Remplace une feature à identifiant conservé (ADR-0024 §7, lot 2) :
/// mêmes gardes que [`AddFeature`] — le changement de croquis est
/// autorisé dès la v1, l'ordre d'ajout au journal fait foi. Un refus
/// ne laisse rien au journal (OF-SES-0001).
#[derive(Debug, Clone)]
pub struct UpdateFeature {
    /// Identifiant de la feature remplacée.
    pub id: u32,
    /// Croquis extrudé (peut changer).
    pub croquis: u32,
    /// Mode : addition au corps courant ou soustraction.
    pub mode: ModeExtrusion,
    /// Profondeur d'extrusion.
    pub profondeur: ProfondeurFeature,
    /// Couleur d'affichage (facultative).
    pub couleur: Option<CouleurFeature>,
}

impl Command for UpdateFeature {
    fn plan(&self, session: &Session) -> Result<CommandPlan, SessionError> {
        if session.cad().feature(self.id).is_none() {
            return Err(SessionError::CommandRejected {
                reason: format!("la feature {} n'existe pas", self.id),
            });
        }
        feature_valide(session, self.croquis, &self.profondeur, &self.couleur)?;
        let feature = FeatureCad::Extrusion {
            croquis: self.croquis,
            mode: self.mode,
            profondeur: profondeur_de_commande(self.id, &self.profondeur),
            couleur: self.couleur.clone(),
        };
        Ok(plan_feature(self.id, &feature, EVENT_FEATURE_UPDATED))
    }
}
