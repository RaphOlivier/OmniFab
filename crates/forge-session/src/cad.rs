//! Domaine CAO de la session (ADR-0024, phase B lot 2) : features
//! d'extrusion rejouées depuis le journal, reconstruction du corps
//! par un moteur CAO **injecté** ([`MoteurCad`]).
//!
//! - **journal** : `cad.feature.added` et `cad.feature.updated` — la
//!   feature porte son croquis, son mode, sa profondeur journalisée
//!   et sa couleur ; le corps reconstruit n'est JAMAIS sérialisé
//!   (état dérivé, ADR-0016 D8 étendu au CAO, ADR-0024 D4) ;
//! - **moteur injecté** : forge-session n'instancie jamais le noyau
//!   OCCT (même frontière que le solveur 2D, ADR-0018 D3 / ADR-0013)
//!   — le lot 2 teste contre un moteur trivial aux volumes
//!   analytiques, le lot 3 branche le vrai noyau (ADR-0023) ;
//! - **profondeurs en expression** : l'alias
//!   `cad.extrusion{id}.profondeur` vit dans le graphe des liaisons
//!   comme nœud système, posé dans la même transaction que la
//!   feature (un groupe d'undo, ADR-0011) ;
//! - **« à travers »** : enveloppe dérivée =
//!   [`FACTEUR_ATRAVERS`] × diagonale de la boîte du corps courant —
//!   jamais journalisée (dérivé déterministe, ADR-0024 D4).

use crate::SessionError;
use crate::command::{EVENT_FEATURE_ADDED, EVENT_FEATURE_UPDATED, EVENT_SKETCH_CREATED};
use crate::esquisse::{
    EntiteEsquisse, VERSION_SCHEMA_CROQUIS, ValeurCote, champ_id, nombre_cote, texte_cote,
};
use crate::projection::{Projection, TypesIgnores, verifie_schema};
use forge_doc::NewEvent;
use forge_geom::{
    ErreurContour, MAGNITUDE_MAX_MM, Mesh, Point2, Point3, Region, Segment2, SensHoraire,
};
use forge_units::{Angle, Length};
use std::collections::BTreeMap;

/// Version de schéma courante des événements `cad.feature.added` et
/// `cad.feature.updated` (ADR-0024 §7, lot 2) : nés en v1.
pub(crate) const VERSION_SCHEMA_FEATURE: u32 = 1;

/// Plan d'un croquis (ADR-0024 §7, lot 2) : seul « xy » est supporté —
/// un plan inconnu d'un journal plus récent est retenu tel quel,
/// jamais deviné, jamais refusé.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanCroquis {
    /// Plan xy, origine du corps (le lot 2 extrude en +z).
    Xy,
    /// Plan inconnu, genre retenu tel quel.
    Inconnu {
        /// Genre du plan lu au journal.
        genre: String,
    },
}

/// Mode d'une extrusion (ADR-0024 §7) : addition au corps courant ou
/// soustraction (poche, perçage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeExtrusion {
    /// Ajout au corps courant (première feature : corps initial).
    Addition,
    /// Soustraction du corps courant (poche, perçage).
    Soustraction,
}

/// Profondeur d'une extrusion, entrée des commandes (ADR-0024 D4) :
/// cote fixe, expression pilotée ou « à travers » — l'expression vit
/// dans le nœud système `cad.extrusion{id}.profondeur` du graphe des
/// liaisons, la valeur journalisée est la [`ProfondeurCad`] dérivée.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfondeurFeature {
    /// Cote fixe, finie et strictement positive.
    Fixe {
        /// Profondeur (millimètres).
        mm: Length,
    },
    /// Expression pilotée du moteur de liaisons.
    Expression {
        /// Texte de l'expression (sous-ensemble déterministe).
        texte: String,
    },
    /// À travers : enveloppe dérivée du corps courant.
    ATravers,
}

/// Profondeur journalisée d'une extrusion (ADR-0024 D4) : la cote
/// (figée ou pilotée par alias) ou « à travers » — jamais l'enveloppe
/// dérivée, recomputée à chaque reconstruction.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfondeurCad {
    /// Cote : fixe ou expression pilotée (alias sous `cad.`).
    Cote(ValeurCote),
    /// À travers : enveloppe dérivée à la reconstruction.
    ATravers,
}

/// Couleur d'affichage d'une feature (ADR-0024 D4) : RGBA et
/// composante spéculaire — métadonnée d'affichage, jamais de la
/// géométrie.
#[derive(Debug, Clone, PartialEq)]
pub struct CouleurFeature {
    /// Composantes rouge, vert, bleu, alpha (0..=1).
    pub rgba: [f64; 4],
    /// Composante spéculaire du rendu.
    pub speculaire: f64,
}

/// Feature CAO du journal (ADR-0024 §7) : extrusion d'un croquis
/// fermé ; un genre inconnu d'un journal plus récent rejoue sans
/// refus (jamais deviné).
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureCad {
    /// Extrusion d'un croquis fermé.
    Extrusion {
        /// Croquis extrudé.
        croquis: u32,
        /// Mode : addition ou soustraction.
        mode: ModeExtrusion,
        /// Profondeur journalisée.
        profondeur: ProfondeurCad,
        /// Couleur d'affichage.
        couleur: Option<CouleurFeature>,
    },
    /// Feature de genre inconnu (journal plus récent).
    Inconnu {
        /// Genre retenu tel quel.
        genre: String,
    },
}

/// Boîte englobante d'un solide (millimètres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoiteEnglobante {
    /// Coin minimal.
    pub min: Point3,
    /// Coin maximal.
    pub max: Point3,
}

impl BoiteEnglobante {
    /// Diagonale de la boîte, en millimètres — base de l'enveloppe
    /// « à travers » (ADR-0024 D4).
    pub fn diagonale(&self) -> f64 {
        let dx = self.max.x().millimeters() - self.min.x().millimeters();
        let dy = self.max.y().millimeters() - self.min.y().millimeters();
        let dz = self.max.z().millimeters() - self.min.z().millimeters();
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Erreur du moteur CAO injecté (OF-CAD-0005) : échec d'extrusion, de
/// booléen, de tessellation ou de boîte englobante — la feature
/// fautive est marquée en échec, le corps courant survit.
#[derive(Debug, thiserror::Error)]
pub enum ErreurMoteurCad {
    /// Échec générique du moteur.
    #[error("OF-CAD-0005 : échec du moteur CAO : {raison}")]
    Echec {
        /// Explication de l'échec.
        raison: String,
    },
}

impl ErreurMoteurCad {
    /// Code stable de l'erreur (ADR-0003), registre
    /// `docs/error-codes.md`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Echec { .. } => "OF-CAD-0005",
        }
    }
}

/// État d'une feature après reconstruction (ADR-0024 D4).
#[derive(Debug, Clone, PartialEq)]
pub enum EtatFeature {
    /// Feature appliquée au corps.
    Ok,
    /// Feature en échec (OF-CAD-0005) : le corps courant survit.
    EnEchec {
        /// Message de l'échec du moteur.
        message: String,
    },
}

impl EtatFeature {
    /// Code stable de l'état : `None` si la feature est saine,
    /// OF-CAD-0005 si elle est en échec.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Ok => None,
            Self::EnEchec { .. } => Some("OF-CAD-0005"),
        }
    }
}

/// Garde commune des profondeurs d'extrusion (ADR-0036 D1, A5-04 —
/// audit n°5) : finie, strictement positive, bornée haute à la
/// magnitude maximale du projet ([`MAGNITUDE_MAX_MM`], 1e9 mm).
/// Appelée avant chaque appel moteur au rejeu — invalide : feature
/// [`EtatFeature::EnEchec`] (OF-CAD-0005), le corps survit — et à
/// l'écriture (`feature_valide`, refus OF-SES-0001). La garde vit
/// à la frontière, jamais dans le moteur.
pub(crate) fn profondeur_valide(mm: f64) -> bool {
    mm.is_finite() && mm > 0.0 && mm <= MAGNITUDE_MAX_MM
}

/// Garde commune des couleurs d'affichage (ADR-0036 D2, A5-04 —
/// audit n°5) : composantes RGBA finies et dans 0..=1, composante
/// spéculaire finie. Appelée avant chaque extrusion au rejeu —
/// invalide : feature [`EtatFeature::EnEchec`] (OF-CAD-0005), le
/// corps survit — et à l'écriture (`feature_valide`, refus
/// OF-SES-0001). La garde vit à la frontière, jamais dans le
/// moteur.
pub(crate) fn couleur_valide(couleur: &CouleurFeature) -> bool {
    couleur
        .rgba
        .iter()
        .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
        && couleur.speculaire.is_finite()
}

/// Facteur de l'enveloppe « à travers » (ADR-0024 D4) : la profondeur
/// dérivée vaut ce facteur × la diagonale de la boîte du corps
/// courant — jamais journalisée, recomputée à chaque reconstruction.
pub const FACTEUR_ATRAVERS: f64 = 1.1;

/// Moteur CAO injecté (ADR-0024 §7, même frontière que le solveur 2D
/// — ADR-0018 D3) : forge-session n'instancie jamais le noyau. Le
/// lot 2 teste contre des volumes analytiques de prisme, le lot 3
/// branche OCCT (ADR-0023).
pub trait MoteurCad {
    /// Type du solide produit par le moteur.
    type Solide;

    /// Extrue la région du croquis (ADR-0037 D3) — l'extérieur
    /// trigonométrique et ses trous horaires — dans le plan xy du
    /// croquis, vers +z (ADR-0024 D3), de la profondeur donnée ; le
    /// moteur analytique du lot 2 calcule l'aire nette (extérieur
    /// moins trous), le noyau OCCT du lot 3 recevra la face à trous.
    ///
    /// # Erreurs
    /// [`ErreurMoteurCad::Echec`] (OF-CAD-0005) si le moteur refuse.
    fn extruder(
        &self,
        region: &Region,
        mode: ModeExtrusion,
        profondeur: Length,
    ) -> Result<Self::Solide, ErreurMoteurCad>;

    /// Fusionne deux solides (mode addition).
    ///
    /// # Erreurs
    /// [`ErreurMoteurCad::Echec`] (OF-CAD-0005) si le moteur refuse.
    fn fusionner(
        &self,
        base: &Self::Solide,
        autre: &Self::Solide,
    ) -> Result<Self::Solide, ErreurMoteurCad>;

    /// Soustrait l'outil du corps (mode soustraction).
    ///
    /// # Erreurs
    /// [`ErreurMoteurCad::Echec`] (OF-CAD-0005) si le moteur refuse.
    fn soustraire(
        &self,
        base: &Self::Solide,
        outil: &Self::Solide,
    ) -> Result<Self::Solide, ErreurMoteurCad>;

    /// Tesselation du solide à la tolérance donnée.
    ///
    /// # Erreurs
    /// [`ErreurMoteurCad::Echec`] (OF-CAD-0005) si le moteur refuse.
    fn tesseller(&self, solide: &Self::Solide, tolerance: Length) -> Result<Mesh, ErreurMoteurCad>;

    /// Boîte englobante du solide — base de l'enveloppe « à travers ».
    ///
    /// # Erreurs
    /// [`ErreurMoteurCad::Echec`] (OF-CAD-0005) si le moteur refuse.
    fn boite_englobante(&self, solide: &Self::Solide) -> Result<BoiteEnglobante, ErreurMoteurCad>;
}

/// Corps reconstruit et état de chaque feature (ADR-0024 D4) : le
/// corps est un état dérivé jamais sérialisé ; une feature en échec
/// est marquée [`EtatFeature::EnEchec`] (OF-CAD-0005) pendant que le
/// corps courant survit — la reconstruction est déterministe, le
/// rejeu du journal redonne le même corps.
pub struct CorpsCad<S> {
    /// Corps courant après la dernière feature saine, `None` si
    /// aucune feature n'a abouti.
    pub corps: Option<S>,
    /// État de chaque feature, dans l'ordre du journal.
    pub etats: Vec<(u32, EtatFeature)>,
}

/// Projection du domaine CAO (ADR-0024 §7, lot 2) : features rejouées
/// depuis le journal ; le corps reconstruit est un état dérivé,
/// recalculé à la demande par un moteur CAO **injecté** — le fil de
/// travail de l'application cache le résultat par le numéro
/// d'édition.
#[derive(Debug, Clone, Default)]
pub struct CadProjection {
    /// Features dans l'ordre d'ajout au journal.
    features: Vec<(u32, FeatureCad)>,
    /// Types d'événements ignorés, plafonnés (audit n°4 A4-03).
    unknown: TypesIgnores,
    /// Numéro d'édition : croît à CHAQUE événement appliqué, tout
    /// type — toute écriture au journal invalide le fil de travail
    /// CAO (ADR-0024 D4).
    edition: u64,
}

impl CadProjection {
    /// Nouvelle projection vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Features dans l'ordre d'ajout au journal.
    pub fn features(&self) -> &[(u32, FeatureCad)] {
        &self.features
    }

    /// Feature de l'identifiant donné, ou `None`.
    pub fn feature(&self, id: u32) -> Option<&FeatureCad> {
        self.features
            .iter()
            .find(|(identifiant, _)| *identifiant == id)
            .map(|(_, feature)| feature)
    }

    /// Feature d'extrusion dont la profondeur pilotée s'appuie sur le
    /// nœud donné, clonée avec son identifiant — la substitution
    /// d'alias (`RemoveAlias`) l'atteint : la feature suit la cote
    /// remplacée (ADR-0024 §7, lot 2).
    pub fn feature_de_alias(&self, alias: &str) -> Option<(u32, FeatureCad)> {
        self.features
            .iter()
            .find(|(_, feature)| {
                if let FeatureCad::Extrusion {
                    profondeur: ProfondeurCad::Cote(ValeurCote::Expression { alias: cible, .. }),
                    ..
                } = feature
                {
                    cible == alias
                } else {
                    false
                }
            })
            .cloned()
    }

    /// Prochain identifiant de feature (séquentiel : l'ordre d'ajout
    /// au journal fait foi, ADR-0024 §7).
    pub fn prochaine_id_feature(&self) -> u32 {
        self.features.len() as u32
    }

    /// Numéro d'édition : croît à chaque événement appliqué, tout
    /// type — clé de cache monotone pour le fil de travail CAO.
    pub fn edition(&self) -> u64 {
        self.edition
    }

    /// Types d'événements ignorés par cette projection.
    pub fn unknown_types(&self) -> &[String] {
        self.unknown.retenus()
    }

    /// Reconstruit le corps dans l'ordre du journal (ADR-0024 D4) :
    /// chaque feature s'applique au corps courant — l'addition
    /// fusionne (la première devient le corps initial), la
    /// soustraction exige un corps courant, « à travers » prend
    /// l'enveloppe [`FACTEUR_ATRAVERS`] × diagonale de la boîte du
    /// corps courant (jamais journalisée). La profondeur en
    /// expression se fige par la valeur du nœud
    /// `cad.extrusion{id}.profondeur` (carte `profondeurs`, échecs
    /// sautés) ; une région absente de la carte ou non fermée
    /// marque la feature en échec (OF-CAD-0005) — le corps courant
    /// survit, le reste des features continue.
    pub fn reconstruire<S>(
        &self,
        moteur: &dyn MoteurCad<Solide = S>,
        regions: &BTreeMap<u32, Region>,
        profondeurs: &BTreeMap<u32, Length>,
    ) -> CorpsCad<S> {
        let mut corps: Option<S> = None;
        let mut etats: Vec<(u32, EtatFeature)> = Vec::new();
        for (id, feature) in &self.features {
            let FeatureCad::Extrusion {
                croquis,
                mode,
                profondeur,
                couleur,
            } = feature
            else {
                etats.push((
                    *id,
                    EtatFeature::EnEchec {
                        message: "feature de genre inconnu au moteur".to_string(),
                    },
                ));
                continue;
            };
            let echec = |message: String| (*id, EtatFeature::EnEchec { message });
            let Some(region) = regions.get(croquis) else {
                etats.push(echec(format!(
                    "le croquis {croquis} n'offre pas exactement une région à extruder"
                )));
                continue;
            };
            // Défense en profondeur : la région arrive fermée de
            // forge-geom (ADR-0037 D3), la garde survit au rejeu
            // d'un journal hostile.
            if !region.exterieur().is_closed()
                || region.trous().iter().any(|trou| !trou.is_closed())
            {
                etats.push(echec(format!(
                    "la région du croquis {croquis} n'est pas fermée"
                )));
                continue;
            }
            // ADR-0036 D2 (A5-04) : garde sur la couleur d'affichage —
            // le rejeu d'un journal hostile n'extrue jamais une
            // couleur hors bornes : échec typé OF-CAD-0005, le corps
            // courant survit.
            if let Some(couleur) = couleur
                && !couleur_valide(couleur)
            {
                etats.push(echec(
                    "la couleur exige ses composantes RGBA finies dans 0..=1".to_string(),
                ));
                continue;
            }
            let profondeur = match profondeur {
                ProfondeurCad::Cote(ValeurCote::Fixe { mm }) => *mm,
                ProfondeurCad::Cote(ValeurCote::Expression { alias, .. }) => {
                    let Some(mm) = profondeurs.get(id) else {
                        etats.push(echec(format!(
                            "la profondeur « {alias} » n'évalue pas en une longueur"
                        )));
                        continue;
                    };
                    *mm
                }
                ProfondeurCad::ATravers => {
                    let Some(courant) = &corps else {
                        etats.push(echec("« à travers » exige un corps courant".to_string()));
                        continue;
                    };
                    let Ok(boite) = moteur.boite_englobante(courant) else {
                        etats.push(echec(
                            "la boîte du corps courant est indéterminée".to_string(),
                        ));
                        continue;
                    };
                    Length::from_millimeters(FACTEUR_ATRAVERS * boite.diagonale())
                }
            };
            // ADR-0036 D1 (A5-04) : garde sur la valeur résolue —
            // fixe, expression figée ou enveloppe « à travers ».
            // Le rejeu d'un journal hostile ne l'extrue jamais :
            // échec typé OF-CAD-0005, le corps courant survit.
            let profondeur_mm = profondeur.millimeters();
            if !profondeur_valide(profondeur_mm) {
                etats.push(echec(format!(
                    "la profondeur {profondeur_mm} mm doit être finie, strictement positive et au plus {MAGNITUDE_MAX_MM} mm"
                )));
                continue;
            }
            let solide = match moteur.extruder(region, *mode, profondeur) {
                Ok(solide) => solide,
                Err(erreur) => {
                    etats.push(echec(erreur.to_string()));
                    continue;
                }
            };
            let resultat: Result<S, ErreurMoteurCad> = match (&corps, mode) {
                (Some(base), ModeExtrusion::Addition) => moteur.fusionner(base, &solide),
                (Some(base), ModeExtrusion::Soustraction) => moteur.soustraire(base, &solide),
                (None, ModeExtrusion::Addition) => Ok(solide),
                (None, ModeExtrusion::Soustraction) => {
                    etats.push(echec("une soustraction exige un corps courant".to_string()));
                    continue;
                }
            };
            match resultat {
                Ok(nouveau) => {
                    corps = Some(nouveau);
                    etats.push((*id, EtatFeature::Ok));
                }
                Err(erreur) => etats.push(echec(erreur.to_string())),
            }
        }
        CorpsCad { corps, etats }
    }
}

impl Projection for CadProjection {
    fn name(&self) -> &'static str {
        "cad"
    }

    fn reset(&mut self) {
        self.features.clear();
        self.unknown.clear();
        // `edition` n'est jamais remis à zéro : clé de cache monotone —
        // deux contenus différents du fil CAO portent des éditions
        // différentes, même après un rejeu complet.
    }

    fn apply(&mut self, event: &NewEvent) -> Result<(), SessionError> {
        // Toute écriture au journal invalide le fil de travail CAO :
        // l'édition croît à chaque événement appliqué, tout type
        // (ADR-0024 D4).
        self.edition += 1;
        match event.event_type.as_str() {
            // Croquis créé : le domaine CAO n'y accroche rien — la
            // projection d'esquisse porte l'état, l'édition suffit
            // ici (validation de schéma : un journal plus récent n'est
            // jamais lu comme du v1).
            EVENT_SKETCH_CREATED => verifie_schema(event, VERSION_SCHEMA_CROQUIS),
            EVENT_FEATURE_ADDED => {
                verifie_schema(event, VERSION_SCHEMA_FEATURE)?;
                let id = champ_id(event, "id")?;
                let feature = feature_de_journal(event)?;
                if id != self.features.len() as u32 {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("identifiant de feature {id} non séquentiel"),
                    });
                }
                self.features.push((id, feature));
                Ok(())
            }
            EVENT_FEATURE_UPDATED => {
                verifie_schema(event, VERSION_SCHEMA_FEATURE)?;
                let id = champ_id(event, "id")?;
                let feature = feature_de_journal(event)?;
                let Some(position) = self
                    .features
                    .iter()
                    .position(|(identifiant, _)| *identifiant == id)
                else {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("feature {id} inconnue au fil CAO"),
                    });
                };
                // Identifiant conservé : seule la feature change,
                // l'ordre d'ajout au journal fait foi (ADR-0024 §7).
                self.features[position].1 = feature;
                Ok(())
            }
            _ => {
                self.unknown.noter(&event.event_type);
                Ok(())
            }
        }
    }

    fn types_connus(&self) -> &'static [&'static str] {
        &[
            EVENT_SKETCH_CREATED,
            EVENT_FEATURE_ADDED,
            EVENT_FEATURE_UPDATED,
        ]
    }
}

/// Sérialise une feature pour les événements `cad.feature.*` :
/// objet typé — `type`, puis les champs de la feature (ADR-0024 §7).
pub(crate) fn feature_en_json(feature: &FeatureCad) -> serde_json::Value {
    match feature {
        FeatureCad::Extrusion {
            croquis,
            mode,
            profondeur,
            couleur,
        } => serde_json::json!({
            "type": "extrusion",
            "croquis": croquis,
            "mode": mode_en_json(mode),
            "profondeur": profondeur_en_json(profondeur),
            "couleur": couleur.as_ref().map(couleur_en_json),
        }),
        FeatureCad::Inconnu { genre } => serde_json::json!({
            "type": genre,
        }),
    }
}

/// Mode d'extrusion en chaîne du journal (`addition` /
/// `soustraction`, ADR-0024 §7).
fn mode_en_json(mode: &ModeExtrusion) -> &'static str {
    match mode {
        ModeExtrusion::Addition => "addition",
        ModeExtrusion::Soustraction => "soustraction",
    }
}

/// Profondeur journalisée : cote typée ou « à travers »
/// (ADR-0024 D4).
fn profondeur_en_json(profondeur: &ProfondeurCad) -> serde_json::Value {
    match profondeur {
        ProfondeurCad::Cote(valeur) => serde_json::json!({
            "type": "cote",
            "valeur": crate::esquisse::valeur_cote_en_json(valeur),
        }),
        ProfondeurCad::ATravers => serde_json::json!({
            "type": "a_travers",
        }),
    }
}

/// Couleur d'affichage de la feature (ADR-0024 D4).
fn couleur_en_json(couleur: &CouleurFeature) -> serde_json::Value {
    serde_json::json!({
        "rgba": [
            couleur.rgba[0],
            couleur.rgba[1],
            couleur.rgba[2],
            couleur.rgba[3],
        ],
        "speculaire": couleur.speculaire,
    })
}

/// Relit une feature depuis le champ « feature » d'un événement
/// `cad.feature.*` : le genre choisit la variante — un genre inconnu
/// d'un journal plus récent rejoue comme [`FeatureCad::Inconnu`],
/// jamais refusé, jamais deviné.
fn feature_de_journal(event: &NewEvent) -> Result<FeatureCad, SessionError> {
    let Some(serde_json::Value::Object(feature)) = event.data.get("feature") else {
        return Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « feature » (objet) absent".to_string(),
        });
    };
    let Some(genre) = feature.get("type").and_then(serde_json::Value::as_str) else {
        return Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « type » absent de la feature".to_string(),
        });
    };
    if genre != "extrusion" {
        return Ok(FeatureCad::Inconnu {
            genre: genre.to_string(),
        });
    }
    let croquis = entier_de_objet(event, feature, "croquis")?;
    let mode = match feature.get("mode").and_then(serde_json::Value::as_str) {
        Some("addition") => ModeExtrusion::Addition,
        Some("soustraction") => ModeExtrusion::Soustraction,
        _ => {
            return Err(SessionError::InvalidEventData {
                event_type: event.event_type.clone(),
                reason: "champ « mode » absent ou inconnu de l'extrusion".to_string(),
            });
        }
    };
    let profondeur = profondeur_de_journal(event, feature)?;
    let couleur = couleur_de_journal(event, feature)?;
    Ok(FeatureCad::Extrusion {
        croquis,
        mode,
        profondeur,
        couleur,
    })
}

/// Relit la profondeur journalisée d'une extrusion : cote typée ou
/// « à travers » (ADR-0024 D4).
fn profondeur_de_journal(
    event: &NewEvent,
    feature: &serde_json::Map<String, serde_json::Value>,
) -> Result<ProfondeurCad, SessionError> {
    let Some(serde_json::Value::Object(profondeur)) = feature.get("profondeur") else {
        return Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « profondeur » (objet typé) absent de la feature".to_string(),
        });
    };
    match profondeur.get("type").and_then(serde_json::Value::as_str) {
        Some("cote") => Ok(ProfondeurCad::Cote(valeur_cote_de_objet(
            event, profondeur,
        )?)),
        Some("a_travers") => Ok(ProfondeurCad::ATravers),
        _ => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "profondeur de type inconnu".to_string(),
        }),
    }
}

/// Relit la cote d'une profondeur depuis l'objet typé qui la porte.
fn valeur_cote_de_objet(
    event: &NewEvent,
    profondeur: &serde_json::Map<String, serde_json::Value>,
) -> Result<ValeurCote, SessionError> {
    let Some(serde_json::Value::Object(cote)) = profondeur.get("valeur") else {
        return Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "cote de la profondeur absente".to_string(),
        });
    };
    match cote.get("type").and_then(serde_json::Value::as_str) {
        Some("fixe") => Ok(ValeurCote::Fixe {
            mm: Length::from_millimeters(nombre_cote(event, cote, "mm")?),
        }),
        Some("expression") => Ok(ValeurCote::Expression {
            texte: texte_cote(event, cote, "texte")?.to_string(),
            alias: texte_cote(event, cote, "alias")?.to_string(),
        }),
        autre => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("type de cote inconnu : « {autre:?} »"),
        }),
    }
}

/// Relit la couleur d'une extrusion : absente, `null` ou objet RGBA.
fn couleur_de_journal(
    event: &NewEvent,
    feature: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<CouleurFeature>, SessionError> {
    match feature.get("couleur") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Object(couleur)) => {
            let Some(rgba) = couleur.get("rgba").and_then(serde_json::Value::as_array) else {
                return Err(SessionError::InvalidEventData {
                    event_type: event.event_type.clone(),
                    reason: "champ « rgba » absent de la couleur".to_string(),
                });
            };
            if rgba.len() != 4 {
                return Err(SessionError::InvalidEventData {
                    event_type: event.event_type.clone(),
                    reason: format!("« rgba » n'a pas quatre composantes : {}", rgba.len()),
                });
            }
            let mut composantes = [0.0; 4];
            for (i, composante) in rgba.iter().enumerate() {
                let Some(valeur) = composante.as_f64() else {
                    return Err(SessionError::InvalidEventData {
                        event_type: event.event_type.clone(),
                        reason: format!("composante {i} de « rgba » non numérique"),
                    });
                };
                composantes[i] = valeur;
            }
            let speculaire = nombre_de_objet(event, couleur, "speculaire")?;
            Ok(Some(CouleurFeature {
                rgba: composantes,
                speculaire,
            }))
        }
        _ => Err(SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: "champ « couleur » ni null ni objet".to_string(),
        }),
    }
}

/// Lit un champ entier non négatif d'un objet imbriqué, borné `u32`.
fn entier_de_objet(
    event: &NewEvent,
    objet: &serde_json::Map<String, serde_json::Value>,
    champ: &str,
) -> Result<u32, SessionError> {
    let valeur = objet
        .get(champ)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (entier non négatif) absent ou invalide"),
        })?;
    u32::try_from(valeur).map_err(|_| SessionError::InvalidEventData {
        event_type: event.event_type.clone(),
        reason: format!("champ « {champ} » hors des bornes de u32"),
    })
}

/// Lit un champ nombre d'un objet imbriqué.
fn nombre_de_objet(
    event: &NewEvent,
    objet: &serde_json::Map<String, serde_json::Value>,
    champ: &str,
) -> Result<f64, SessionError> {
    objet
        .get(champ)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| SessionError::InvalidEventData {
            event_type: event.event_type.clone(),
            reason: format!("champ « {champ} » (nombre) absent ou invalide"),
        })
}

/// Segments du croquis aux positions données (ADR-0024 D4) : les
/// entités deviennent des segments typés — lignes entre points,
/// cercle en arc à balayage nul (le cercle complet), arc par son
/// centre et ses angles (les points de début et de fin de l'entité ne
/// portent que la sélection, la géométrie suit les angles — le sens
/// est trigonométrique si l'angle de fin précède ou égale celui de
/// début, horaire sinon). Une position absente saute l'entité : le
/// croquis incomplet ne donne pas de contour.
pub(crate) fn segments_du_croquis(
    entites: &[(u32, EntiteEsquisse)],
    position: &dyn Fn(u32) -> Option<(Length, Length)>,
) -> Vec<Segment2> {
    let point =
        |id: u32| -> Option<Point2> { position(id).and_then(|(x, y)| Point2::new(x, y).ok()) };
    let mut segments = Vec::new();
    for (_, entite) in entites {
        match entite {
            EntiteEsquisse::Point { .. } => {}
            EntiteEsquisse::Ligne { p1, p2 } => {
                if let (Some(depart), Some(arrivee)) = (point(*p1), point(*p2)) {
                    segments.push(Segment2::Ligne {
                        p1: depart,
                        p2: arrivee,
                    });
                }
            }
            EntiteEsquisse::Cercle { centre, rayon } => {
                if let Some(centre) = point(*centre) {
                    segments.push(Segment2::Arc {
                        centre,
                        rayon: *rayon,
                        debut: Angle::ZERO,
                        fin: Angle::ZERO,
                        sens: SensHoraire::Trigonometrique,
                    });
                }
            }
            EntiteEsquisse::Arc {
                centre,
                rayon,
                angle_debut,
                angle_fin,
                ..
            } => {
                if let Some(centre) = point(*centre) {
                    segments.push(Segment2::Arc {
                        centre,
                        rayon: *rayon,
                        debut: *angle_debut,
                        fin: *angle_fin,
                        sens: if angle_fin.radians() >= angle_debut.radians() {
                            SensHoraire::Trigonometrique
                        } else {
                            SensHoraire::Horaire
                        },
                    });
                }
            }
        }
    }
    segments
}

/// Régions du croquis (ADR-0037 D3) : les segments, non ordonnés,
/// sont chaînés par le graphe d'extrémités de forge-geom (élagage
/// des queues, refus d'embranchement OF-GEO-0008, plafond
/// OF-GEO-0009) puis classés en régions par orientation et parité —
/// l'extérieur trigonométrique, les trous horaires. Zéro région :
/// aucun cycle fermé ; plusieurs régions : croquis multi-corps,
/// refusé au plan (OF-SES-0001, ADR-0037 D3) ; les refus de
/// discrétisation (OF-GEO-0004/0005) sont propagés.
pub(crate) fn regions_du_croquis(segments: &[Segment2]) -> Result<Vec<Region>, ErreurContour> {
    let cycles = forge_geom::contours(segments)?;
    forge_geom::regions(&cycles)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Positions de deux points (ids 0 et 1), absentes ailleurs.
    fn positions(x0: f64, y0: f64, x1: f64, y1: f64) -> impl Fn(u32) -> Option<(Length, Length)> {
        move |id| match id {
            0 => Some((Length::from_millimeters(x0), Length::from_millimeters(y0))),
            1 => Some((Length::from_millimeters(x1), Length::from_millimeters(y1))),
            _ => None,
        }
    }

    #[test]
    fn arc_effondre_omis_par_segments_du_croquis() {
        // A5-02 + A4-18 (audit n°5, ADR-0037 D1) : un arc effondré
        // (fin = début) n'est plus un cercle complet — omis des
        // segments du croquis, il ne referme aucun contour.
        let entites = vec![(
            0,
            EntiteEsquisse::Arc {
                centre: 0,
                debut: 1,
                fin: 1,
                rayon: Length::from_millimeters(5.0),
                angle_debut: Angle::from_degrees(30.0),
                angle_fin: Angle::from_degrees(30.0),
            },
        )];
        let segments = segments_du_croquis(&entites, &positions(0.0, 0.0, 1.0, 2.0));
        assert!(segments.is_empty(), "l'arc effondré ne devient pas un cercle");
    }

    #[test]
    fn cercle_d_entite_devient_cercle_explicite() {
        // Le cercle complet d'esquisse devient le variant explicite
        // de Segment2 (ADR-0037 D1), jamais un arc à balayage nul.
        let entites = vec![(
            0,
            EntiteEsquisse::Cercle {
                centre: 0,
                rayon: Length::from_millimeters(5.0),
            },
        )];
        let segments = segments_du_croquis(&entites, &positions(1.0, 2.0, 0.0, 0.0));
        match segments.as_slice() {
            [Segment2::Cercle { centre, rayon, sens }] => {
                assert_eq!(centre.x().millimeters(), 1.0);
                assert_eq!(centre.y().millimeters(), 2.0);
                assert_eq!(rayon.millimeters(), 5.0);
                assert_eq!(*sens, SensHoraire::Trigonometrique);
            }
            autre => panic!("un cercle explicite attendu : {autre:?}"),
        }
    }
}
