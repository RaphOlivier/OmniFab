//! # `app` — application egui du démonstrateur omnifab-desktop
//!
//! Conforme à `docs/architecture.md` et aux ADR-0011/0014 : la
//! fenêtre egui met en scêne le noyau complet, branché de bout en
//! bout :
//!
//! - l'application porte une **session** forge-session (ADR-0008) :
//!   commandes, projections, undo/redo par curseur ;
//! - **annuler / rétablir** visibles : boutons et raccourcis Ctrl+Z,
//!   Ctrl+Y, Ctrl+Maj+Z (une annulation impossible par raccourci est
//!   ignorée silencieusement, ADR-0011) ;
//! - **import/export SVG/DXF/STL** par forge-io (ADR-0009) : les octets
//!   d'origine partent dans des blobs, le journal ne porte que
//!   références et compteurs (ADR-0001) ; l'export ne change pas l'état
//!   — rien au journal ; **les fichiers sont choisis par l'utilisateur
//!   via les sélecteurs natifs** (ADR-0014) ;
//! - **fil de travail** (ADR-0016 C1) : lectures, parses et exports
//!   détachés du fil d'interface, échecs de parse mémorisés — un
//!   fichier invalide n'est plus re-parsé à chaque frame ; le fil
//!   survit à une panique du travail — échec typé OF-WORK-0001, le
//!   travail suivant est traité (audit n°3 N8) ;
//! - **résolution du croquis contraint** (ADR-0018 D3) : la demande
//!   part au fil de travail, qui instancie le solveur natif —
//!   jamais sur le fil d’interface (ADR-0016 C1), jamais dans
//!   forge-session (frontière LGPL de l’ADR-0013) ; les positions
//!   résolues restent un état dérivé en mémoire, jamais
//!   sérialisées ;
//! - **mode esquisse** (ADR-0018, lot 3c) : outils du croquis dans
//!   le panneau de gauche (vue 2D), un clic pose une entité, le
//!   croquis se dessine par-dessus la scène ;
//! - **protection du travail** (ADR-0016 C3) : indicateur « non
//!   enregistré » posé par toute commande effective et levé par la
//!   sauvegarde — le titre de la fenêtre porte `*`, « Réinitialiser »,
//!   « Ouvrir » et « Reprendre » demandent confirmation, et
//!   l'enregistrement ne casse plus la pile redo ; la croix est
//!   interceptée : confirmation Enregistrer / Quitter sans
//!   enregistrer / Annuler avant de fermer (audit n°3 A3-06) ;
//! - **messages d'état fidèles** (ADR-0016 C4) : une commande refusée
//!   n'affiche jamais de succès, l'action suivante efface l'erreur —
//!   plus de rouge persistant après correction ;
//! - **unités du maillage STL** (ADR-0017) : un modèle dont la plus
//!   grande dimension est sous 1 mm propose l'échelle mètres
//!   (× 1000) à l'import — le facteur vit dans l'événement
//!   `geom.mesh.imported` schéma v2, appliqué au rendu et à l'export
//!   (toujours en mm) ;
//! - **bibliothèque de projets** (ADR-0014) : écran d'accueil listant
//!   les `.omnifab` du dossier `OmniFab/Projets` du dossier personnel,
//!   « Enregistrer sous… » y propose par défaut ; le chemin courant
//!   est un état de l'app UI, jamais un événement ;
//! - **viewport wgpu** (ADR-0015, phases C1 et C2) : vue 2D — grille
//!   de fond, tracés importés et wireframe du maillage STL en
//!   projection de dessus dessinés par le GPU via forge-render, le
//!   rectangle paramétrique par le painter, caméra à la molette
//!   (zoom au curseur) et au glisser (pan) ;
//!   vue 3D — wireframe du maillage STL par le GPU, arêtes
//!   dédupliquées, caméra orbite, plafond d'affichage supprimé ;
//! - les dimensions manipulées restent des `Length` (forge-units, jamais
//!   de `f64` nu dans l'UI) ;
//! - toute l'interface passe par le catalogue Fluent embarqué
//!   (forge-i18n, ADR-0010) : aucune chaîne d'interface dans le code ;
//! - chaque widget interactif porte un `help_id` : un clic droit ouvre
//!   sa page d'aide (convention jalon 1, ADR-0010).
//!
//! Découpage 6/6 de `main.rs` (fiche
//! `preparation/decoupage-main-rs.md`) : `DemoApp`, ses méthodes
//! cœur et son impl `eframe::App` vivent ici — le lanceur ne porte
//! que `main`.

use crate::aide;
use crate::aide::avec_aide;
use crate::atelier_esquisse;
use crate::bibliotheque;
use crate::dialogues::{DialogueNouvelAlias, DialogueSuppression};
use crate::fichiers;
use crate::i18n;
use crate::theme::ACTEUR;
#[cfg(test)]
use crate::theme::COULEUR_GRILLE;
#[cfg(test)]
use crate::theme::COULEUR_RECTANGLE_3D;
#[cfg(test)]
use crate::theme::COULEUR_TRACES;
#[cfg(test)]
use crate::theme::COULEUR_WIREFRAME;
use crate::theme::DEFAULT_DIAGONAL_EXPR;
use crate::theme::ECHELLE_METRES;
#[cfg(test)]
use crate::theme::EVENEMENTS_VIERGES;
#[cfg(test)]
use crate::theme::GRILLE_CIBLE_PX;
use crate::travail::Retour;
use crate::travail::Travail;
use crate::travail::fil_de_travail;
use crate::travail::garde_actuels;
use crate::travail::propose_echelle_metres;
use crate::travail::raccourcis_annulation;
use crate::travail::touche_les_liaisons;
use eframe::egui;
use forge_doc::Document;
use forge_geom::{Mesh, Path2};
use forge_i18n::Argument;
#[cfg(test)]
use forge_io::svg;
#[cfg(test)]
use forge_render::Couleur;
#[cfg(test)]
use forge_render::{Boite2, Scene2D, Scene3D, pas_de_grille};
use forge_render::{Camera2D, Camera3D, Renderer2D, Segment3D};
use forge_session::{
    Command, CroquisResolu, ImportMesh, ImportTraces, Session, SetExpression, SetInput, Unite,
};
#[cfg(test)]
use forge_session::{EsquisseProjection, ValeurCote};
use forge_units::Length;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// État de rendu wgpu d'eframe : alias pour alléger la signature du
/// constructeur (ADR-0015).
pub(crate) type RenderState = eframe::egui_wgpu::RenderState;

/// Écran affiché : accueil (bibliothèque) ou atelier (ADR-0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ecran {
    /// Écran d'accueil : bibliothèque de projets.
    Accueil,
    /// Atelier : viewport et panneaux.
    Atelier,
}

/// Vue du viewport : plan 2D ou volume 3D (ADR-0015, phase C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vue {
    /// Vue 2D : grille, rectangle et tracés au sol.
    DeuxD,
    /// Vue 3D : wireframe du maillage en volume, caméra orbite.
    TroisD,
}

/// Action destructive : elle ne part qu'après confirmation quand du
/// travail non enregistré existe (ADR-0016 C3).
#[derive(Debug, Clone)]
pub(crate) enum ActionDestructrice {
    /// Nouveau projet : la session courante est remplacée.
    Nouveau,
    /// Ouverture d'un projet : chemin déjà choisi, ou sélecteur natif.
    Ouvrir {
        /// Chemin du projet à ouvrir, s'il est déjà connu.
        chemin: Option<PathBuf>,
    },
    /// Reprise de la session en cours depuis l'écran d'accueil.
    Reprendre,
    /// Fermeture de la fenêtre demandée à la croix (audit n°3
    /// A3-06) : « Enregistrer » sauve puis ferme, « Quitter sans
    /// enregistrer » ferme, « Annuler » ramène au travail.
    Quitter,
}

/// Décision de la garde de fermeture de la fenêtre (audit n°3
/// A3-06) : la croix ne doit pas perdre un travail non
/// enregistré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecisionFermeture {
    /// Laisser la fenêtre se fermer : rien n'est à perdre.
    LaisserFermer,
    /// Bloquer la fermeture : la confirmation est déjà ouverte.
    Bloquer,
    /// Bloquer la fermeture et ouvrir la confirmation.
    BloquerEtConfirmer,
}

/// Garde de fermeture de la fenêtre (audit n°3 A3-06) : du
/// travail non enregistré ne se perd pas au clic sur la croix —
/// la fermeture est annulée (ViewportCommand::CancelClose,
/// exemple egui « confirm_exit ») et la confirmation demandée,
/// sauf si elle est déjà ouverte.
pub(crate) fn garde_fermeture(dirty: bool, confirmation_en_cours: bool) -> DecisionFermeture {
    if !dirty {
        return DecisionFermeture::LaisserFermer;
    }
    if confirmation_en_cours {
        DecisionFermeture::Bloquer
    } else {
        DecisionFermeture::BloquerEtConfirmer
    }
}

/// Choix des unités d'un maillage minuscule demandé à l'import
/// (ADR-0017) : le fil de travail a lu les octets et mesuré la plus
/// grande dimension ; l'import attend la réponse de l'utilisateur.
/// La demande ne garde que le chemin et la dimension mesurée
/// (ADR-0038 D2) : pas de copie d'octets — le fichier est relu au
/// fil de travail sous le budget du format à la confirmation.
#[derive(Debug, Clone)]
pub(crate) struct DemandeEchelle {
    /// Fichier importé, à relire à la confirmation d'échelle.
    pub(crate) chemin: PathBuf,
    /// Plus grande dimension du maillage, en millimètres.
    pub(crate) dimension: f64,
}

/// `help_id` de la page d'aide du code d'erreur **terminal** du
/// message : les refus de la session enchaînent les codes — un refus
/// de projection enveloppe celui de la cause (OF-SES-0002 puis
/// OF-VAL-0007) —, la page d'aide de la cause terminale, le dernier
/// code du message, s'ouvre (ADR-0019 §6, correctif ADR-0020).
pub(crate) fn aide_du_code(message: &str) -> String {
    let code = message
        .split(' ')
        .rfind(|mot| mot.starts_with("OF-"))
        .unwrap_or_else(|| message.split(' ').next().unwrap_or(""));
    format!("aide-{}", code.to_lowercase())
}

pub(crate) struct DemoApp {
    /// Session de travail : document, projections, undo/redo
    /// (ADR-0008/0011).
    pub(crate) session: Session,
    /// Tampons d'édition des expressions des alias calculés : le
    /// champ texte affiche l'expression du début de frame, un
    /// rechargement du graphe à chaque frame écraserait la saisie
    /// en cours (leçon round 6 egui) — chaque alias garde son
    /// texte et son activité d'une frame à l'autre.
    pub(crate) tampon_expressions: BTreeMap<String, (String, bool)>,
    /// Dernière erreur à afficher dans le viewport.
    pub(crate) error: Option<String>,
    /// Dernier message d'information à afficher dans le viewport.
    pub(crate) info: Option<String>,
    /// Catalogue Fluent embarqué.
    pub(crate) i18n: i18n::Catalogue,
    /// Page d'aide ouverte, si une page est demandée.
    pub(crate) aide: Option<String>,
    /// Écran affiché : accueil ou atelier (ADR-0014).
    pub(crate) ecran: Ecran,
    /// Fichier projet courant, choisi par l'utilisateur : état de
    /// l'app UI, jamais un événement (ADR-0014).
    pub(crate) chemin_projet: Option<PathBuf>,
    /// Cache mémoire non autoritaire des tracés re-parsés (nom du
    /// blob + tracés ou échec de parse), invalidé quand la référence
    /// change (ADR-0011) — un échec est mémorisé, pas re-parsé à
    /// chaque frame (ADR-0016 C1).
    pub(crate) cache: Option<(String, Result<Vec<Path2>, String>)>,
    /// Cache mémoire non autoritaire du maillage re-parsé (nom du
    /// blob + maillage ou échec de parse), invalidé quand la référence
    /// change (ADR-0014, ADR-0016 C1).
    pub(crate) cache_maillage: Option<(String, Result<Mesh, String>)>,
    /// Wireframe 3D du maillage en cache, à clé d'édition (ADR-0027,
    /// patron `camera_cle`) : les segments ne sont reconstruits que
    /// quand la référence du maillage change, jamais à chaque frame
    /// — un STL de centaines de milliers de triangles ne repaye
    /// plus le dédoublonnage à chaque draw (audit n°4 A4-02).
    pub(crate) wireframe_maillage: Option<(Option<String>, Vec<Segment3D>)>,
    /// Caméra du viewport : ajustée automatiquement quand la scêne
    /// change, pilotée ensuite par l'utilisateur (ADR-0015).
    pub(crate) camera: Camera2D,
    /// Clé du dernier ajustement : références des blobs tracés et
    /// maillage — la caméra se réajuste quand elles changent.
    pub(crate) camera_cle: Option<(Option<String>, Option<String>)>,
    /// Caméra orbite de la vue 3D : ajustée automatiquement quand la
    /// scêne change, pilotée ensuite par l'utilisateur (ADR-0015,
    /// phase C2).
    pub(crate) camera3d: Camera3D,
    /// Vue affichée par le viewport : 2D ou 3D (ADR-0015, phase C2).
    pub(crate) vue: Vue,
    /// Pipeline wgpu du viewport créé : le dessin 2D passe par le GPU,
    /// sinon le viewport affiche le message d'indisponibilité.
    pub(crate) viewport_wgpu: bool,
    /// Tampons d'édition des champs de dimensions : le DragValue affiche
    /// la valeur du début de frame, un rechargement du graphe à chaque
    /// frame figeait le chiffre pendant le geste — chaque champ garde
    /// donc sa valeur et son activité d'une frame à l'autre
    /// (validation C1 round 6).
    pub(crate) tampon_dimensions: BTreeMap<String, (f64, bool)>,
    /// Émetteur vers le fil de travail : lectures, parses et exports
    /// hors du fil d'interface (ADR-0016 C1) ; `None` si le fil n'a
    /// pas pu démarrer.
    pub(crate) travaux: Option<mpsc::Sender<Travail>>,
    /// Retours du fil de travail, consommés à chaque frame.
    pub(crate) retours: mpsc::Receiver<Retour>,
    /// Travaux confiés non revenus : le viewport demande un
    /// rafraîchissement tant qu'il y en a (ADR-0016 C1).
    pub(crate) travaux_en_attente: u32,
    /// Génération de l'état : incrémentée à chaque réinitialisation
    /// ou ouverture — les travaux en vol deviennent obsolètes
    /// (ADR-0016 C1).
    pub(crate) generation: u64,
    /// Travail non enregistré : posé par toute commande effective,
    /// levé par la sauvegarde (ADR-0016 C3).
    pub(crate) dirty: bool,
    /// Le titre de la fenêtre reflète-t-il déjà `dirty` ? La commande
    /// de titre n'est émise que quand il change (ADR-0016 C3).
    pub(crate) titre_sale: bool,
    /// Action destructive demandée, en attente de confirmation
    /// (ADR-0016 C3).
    pub(crate) demande: Option<ActionDestructrice>,
    /// Fermeture de la fenêtre confirmée par le dialogue (audit n°3
    /// A3-06) : la garde ne bloque plus la demande de fermeture
    /// qu'elle vient d'émettre.
    pub(crate) fermeture_confirmee: bool,
    /// Choix des unités d'un maillage minuscule, en attente de la
    /// réponse de l'utilisateur (ADR-0017).
    pub(crate) demande_echelle: Option<DemandeEchelle>,
    /// Cache des positions résolues du croquis, clé par (numéro
    /// d’édition, génération des liaisons) (ADR-0018 D3,
    /// ADR-0019 §6) : état dérivé en mémoire, jamais sérialisé ; un
    /// échec est mémorisé à sa clé — pas re-résolu à chaque
    /// frame — et les graines du journal restent la vérité
    /// affichable (ADR-0018 D7). Une cote pilotée re-figée repart
    /// en résolution même sans changement du croquis.
    pub(crate) croquis_resolu: Option<((u64, u64), Result<CroquisResolu, String>)>,
    /// Dernière résolution RÉUSSIE du croquis, toute édition
    /// confondue (re-test du 24-10) : l'affichage y retombe quand
    /// la résolution fraîche du journal courant échoue. Un échec
    /// ne doit jamais effacer la dernière forme saine — sinon le
    /// croquis saute aux graines du journal au moment même de la
    /// pose (l'ancre figée loin des graines fait échouer DogLeg,
    /// l'erreur remplace la dernière réussite au cache et
    /// l'affichage retombe sur des graines périmées). L'erreur
    /// reste signalée par le message d'état, le DoF du panneau
    /// reste sur la résolution fraîche — seule l'affichage garde la
    /// forme.
    pub(crate) derniere_resolution_bonne: Option<CroquisResolu>,
    /// Clé (édition, liaisons) en cours de résolution au fil de
    /// travail : une seule résolution en vol (ADR-0016 C1).
    pub(crate) resolution_en_attente: Option<(u64, u64)>,
    /// Outil actif du mode esquisse (ADR-0018, lot 3c) : état de
    /// l’app UI, jamais un événement.
    pub(crate) outil: atelier_esquisse::OutilEsquisse,
    /// Geste en cours de l'outil de croquis (ADR-0018, lot 3c-2) :
    /// l'origine posée attend la suite — état de l'app UI,
    /// jamais un événement, abandonné au changement d'outil.
    pub(crate) geste: Option<atelier_esquisse::GesteEsquisse>,
    /// Entités du croquis sélectionnées au clic (ADR-0018 D5,
    /// lot 3c-4) : au plus deux — les contraintes relient au plus
    /// deux entités ; la poignée au glissement garde la sélection
    /// — état de l'app UI, jamais un événement.
    pub(crate) selection: Vec<u32>,
    /// Point saisi par le glissement d'une poignée (ADR-0018 D4) :
    /// l'aperçu local le suit sans écrire, le relâchement pose sa
    /// graine — état de l'app UI, jamais un événement.
    pub(crate) saisie: Option<u32>,
    /// Surcharge d'affichage du point relâché, pendant le vol de sa
    /// résolution (anti-flash du relâchement, re-validation §7) :
    /// le relâchement pose la graine par `MovePoint` et la
    /// résolution repart au fil de travail — sans elle, l'affichage
    /// retomberait sur la résolution d'avant le glissement et la
    /// forme bondirait en arrière le temps du vol. L'édition du
    /// relâchement la caducise dès qu'une autre écriture touche le
    /// journal, le frais la remplace — état dérivé d'affichage,
    /// jamais sérialisé.
    pub(crate) surcharge_relachee: Option<(u64, u32, Length, Length)>,
    /// Aperçu de glissement : dernière résolution d'affichage du
    /// point saisi pendant le glissement (re-test du 2026-10-23) —
    /// la forme liée suit le geste en direct, la sémantique est
    /// exactement celle du relâchement ; l'échec est silencieux.
    /// Depuis le re-test du 24-10, l'aperçu tient AUSSI le vol du
    /// relâchement (purge seulement quand le frais est arrivé) :
    /// le dessin ne montre que des états résolus, jamais une
    /// position brute du curseur. État dérivé d'affichage, jamais
    /// sérialisé.
    pub(crate) croquis_apercu: Option<Result<CroquisResolu, String>>,
    /// Clé du dernier aperçu envoyé au fil de travail : point saisi
    /// et position visée en millimètres — la même position ne
    /// repart pas, un seul aperçu en vol.
    pub(crate) apercu_envoye: Option<(u32, f64, f64)>,
    /// Aperçu en vol au fil de travail.
    pub(crate) apercu_en_attente: bool,
    /// Balayage signé accumulé du geste de l'arc en cours (ADR-0020)
    /// : positif dans le sens trigonométrique, négatif dans le sens
    /// horaire — état de l'app UI, jamais un événement.
    pub(crate) balayage_arc: f64,
    /// Dernier angle du curseur autour du centre de l'arc en cours,
    /// pour accumuler le balayage au fil du mouvement (ADR-0020).
    pub(crate) angle_curseur_arc: f64,
    /// Identifiants des contraintes redondantes ou partiellement
    /// redondantes affichées (ADR-0018 D5, lot 3c-4) : le diagnostic
    /// du solveur est instable (FPA #106) — l'affichage exige le
    /// même diagnostic sur deux résolutions consécutives
    /// (hystérésis). État dérivé d'affichage, jamais sérialisé.
    pub(crate) redondances_affichees: Option<Vec<u32>>,
    /// Redondances de la résolution précédente, pour l'hystérésis
    /// (ADR-0018 D5, lot 3c-4) : jamais affichées telles quelles.
    pub(crate) redondances_precedentes: Option<Vec<u32>>,
    /// Valeur en millimètres de la cote de distance en cours de
    /// saisie (ADR-0018 D5, lot 3c-4) : état de l'app UI, jamais un
    /// événement — elle préremplit le dialogue de cote.
    pub(crate) cote_distance: f64,
    /// Dernière contrainte touchée et instant de pose (ADR-0021
    /// D2) : surbrillance momentanée dans la liste des contraintes
    /// — état d'affichage, jamais sérialisé.
    pub(crate) derniere_contrainte: Option<(u32, f64)>,
    /// Instant de la trame courante, en secondes egui monotones :
    /// sert à estomper la surbrillance momentanée.
    pub(crate) instant: f64,
    /// Génération des liaisons (ADR-0019 §6) : avance à chaque
    /// commande ou annulation qui touche le graphe des valeurs —
    /// les cotes pilotées se re-figent et la résolution repart
    /// même sans changement du croquis (checklist §8).
    pub(crate) generation_liaisons: u64,
    /// Dialogue « + Nouvel alias » ouvert (ADR-0019 §6) : état de
    /// l'app UI, jamais un événement tant que « Poser » n'est pas
    /// cliqué.
    pub(crate) dialogue_nouvel_alias: Option<DialogueNouvelAlias>,
    /// Dialogue de suppression d'un alias référencé (ADR-0019 §6) :
    /// cascade, remplacement ou annulation.
    pub(crate) dialogue_suppression: Option<DialogueSuppression>,
    /// Dialogue de cote de l'atelier esquisse : valeur fixe ou
    /// expression pilotée (ADR-0019 §6).
    pub(crate) dialogue_cote: Option<atelier_esquisse::DialogueCote>,
}

impl DemoApp {
    /// Nouveau démonstrateur, sur le catalogue français embarqué :
    /// session vierge posée, écran d'accueil affiché (ADR-0014) ; le
    /// pipeline wgpu du viewport est créé ici quand le rendu wgpu est
    /// actif (ADR-0015).
    pub(crate) fn new(i18n: i18n::Catalogue, render_state: Option<&RenderState>) -> Self {
        // Pipeline wgpu du viewport : créé une fois ici, partagé avec
        // les callbacks d'egui-wgpu via les ressources du renderer
        // (ADR-0015).
        let mut viewport_wgpu = false;
        if let Some(etat) = render_state {
            let device = etat.device.as_ref();
            let format = etat.target_format;
            etat.renderer
                .write()
                .callback_resources
                .insert(Renderer2D::nouveau(device, format));
            viewport_wgpu = true;
        }
        // Fil de travail (ADR-0016 C1) : lectures, parses et exports
        // détachés du fil d'interface ; le fil s'éteint quand
        // l'émetteur des travaux est abandonné.
        let (emetteur_travaux, recepteur_travaux) = mpsc::channel::<Travail>();
        let (emetteur_retours, recepteur_retours) = mpsc::channel::<Retour>();
        let travaux = std::thread::Builder::new()
            .name("omnifab-travail".to_string())
            .spawn(move || fil_de_travail(recepteur_travaux, emetteur_retours))
            .is_ok()
            .then_some(emetteur_travaux);
        let mut app = Self {
            session: Session::new(ACTEUR),
            tampon_expressions: BTreeMap::new(),
            error: None,
            info: None,
            i18n,
            aide: None,
            ecran: Ecran::Accueil,
            chemin_projet: None,
            cache: None,
            cache_maillage: None,
            wireframe_maillage: None,
            camera: Camera2D::nouvelle(),
            camera_cle: None,
            camera3d: Camera3D::nouvelle(),
            vue: Vue::DeuxD,
            viewport_wgpu,
            tampon_dimensions: BTreeMap::new(),
            travaux,
            retours: recepteur_retours,
            travaux_en_attente: 0,
            generation: 0,
            dirty: false,
            titre_sale: false,
            demande: None,
            fermeture_confirmee: false,
            demande_echelle: None,
            croquis_resolu: None,
            derniere_resolution_bonne: None,
            resolution_en_attente: None,
            outil: atelier_esquisse::OutilEsquisse::Selection,
            geste: None,
            selection: Vec::new(),
            saisie: None,
            surcharge_relachee: None,
            croquis_apercu: None,
            apercu_envoye: None,
            apercu_en_attente: false,
            balayage_arc: 0.0,
            angle_curseur_arc: 0.0,
            redondances_affichees: None,
            redondances_precedentes: None,
            cote_distance: 100.0,
            derniere_contrainte: None,
            instant: 0.0,
            generation_liaisons: 0,
            dialogue_nouvel_alias: None,
            dialogue_suppression: None,
            dialogue_cote: None,
        };
        app.reinitialiser();
        app
    }

    /// Remet le démonstrateur à l'état initial (nouveau projet) :
    /// nouvelle session, dimensions et expression par défaut — trois
    /// groupes d'undo séparés (ADR-0011).
    pub(crate) fn reinitialiser(&mut self) {
        self.generation += 1;
        self.session = Session::new(ACTEUR);
        self.tampon_dimensions.clear();
        self.tampon_expressions.clear();
        self.generation_liaisons += 1;
        self.dialogue_nouvel_alias = None;
        self.dialogue_suppression = None;
        self.dialogue_cote = None;
        self.effacer_messages();
        self.cache = None;
        self.cache_maillage = None;
        self.camera = Camera2D::nouvelle();
        self.camera_cle = None;
        self.camera3d = Camera3D::nouvelle();
        self.vue = Vue::DeuxD;
        self.aide = None;
        self.demande_echelle = None;
        self.croquis_resolu = None;
        self.derniere_resolution_bonne = None;
        self.resolution_en_attente = None;
        self.outil = atelier_esquisse::OutilEsquisse::Selection;
        self.geste = None;
        self.selection = Vec::new();
        self.saisie = None;
        self.surcharge_relachee = None;
        self.croquis_apercu = None;
        self.apercu_envoye = None;
        self.apercu_en_attente = false;
        self.balayage_arc = 0.0;
        self.angle_curseur_arc = 0.0;
        self.redondances_affichees = None;
        self.redondances_precedentes = None;
        self.chemin_projet = None;
        let _ = self.executer(&SetInput {
            name: "largeur".into(),
            // 240 × 160 mm : le rectangle de démonstration tient
            // plusieurs positions dans une vue d'écran (ADR-0020,
            // validation jalon 2 — 800 × 600 couvrait tout l'écran).
            value: 240.0,
            unite: Some(Unite::Millimetre),
        });
        let _ = self.executer(&SetInput {
            name: "hauteur".into(),
            value: 160.0,
            unite: Some(Unite::Millimetre),
        });
        let _ = self.executer(&SetExpression {
            name: "diagonale".into(),
            expression: DEFAULT_DIAGONAL_EXPR.into(),
            unite: Some(Unite::Millimetre),
        });
        // La session posée est à jour : rien à perdre (ADR-0016 C3).
        self.dirty = false;
    }

    /// Efface les messages d'état de l'action précédente : chaque
    /// action utilisateur repart sans message — l'erreur rouge ne
    /// survit pas à l'action qui la corrige (ADR-0016 C4).
    pub(crate) fn effacer_messages(&mut self) {
        self.error = None;
        self.info = None;
    }

    /// Exécute une commande de session ; un refus remonte au canal
    /// d'erreur du viewport sans interrompre l'interface. Toute
    /// commande effective pose l'indicateur de travail non enregistré
    /// (ADR-0016 C3). Le `Result` porte le verdict : seul l'appelant
    /// affiche un succès, jamais après un refus (ADR-0016 C4). Une
    /// commande qui redéfinit le graphe des liaisons avance la
    /// génération : les cotes pilotées se re-figent (ADR-0019 §6).
    pub(crate) fn executer(&mut self, commande: &dyn Command) -> Result<(), String> {
        self.effacer_messages();
        let empreinte_avant = self.empreinte_journal();
        if let Err(error) = self.session.execute(commande) {
            let message = error.to_string();
            self.error = Some(message.clone());
            return Err(message);
        }
        if self.empreinte_journal() != empreinte_avant {
            self.dirty = true;
            let journal = self.session.document().events();
            let avance = journal.len() > empreinte_avant.0
                && journal[empreinte_avant.0..]
                    .iter()
                    .any(|evenement| touche_les_liaisons(&evenement.event_type));
            if avance {
                self.generation_liaisons += 1;
            }
        }
        Ok(())
    }

    /// Empreinte du journal appliqué : longueur et bornes undo/redo —
    /// toute divergence marque le travail non enregistré
    /// (ADR-0016 C3).
    pub(crate) fn empreinte_journal(&self) -> (usize, bool, bool) {
        let journal = self.session.document().events().len();
        (journal, self.session.can_undo(), self.session.can_redo())
    }

    /// Annule le dernier groupe d'actions (bouton ou raccourci) : le
    /// tampon d'édition est rafraîchi et le travail non enregistré
    /// suit le journal (ADR-0016 C3).
    pub(crate) fn annuler(&mut self) {
        self.effacer_messages();
        if self.session.undo().is_ok() {
            self.dirty = true;
            self.raz_liaisons();
        }
    }

    /// Rétablit le groupe défait (bouton ou raccourci) : le tampon
    /// d'édition est rafraîchi et le travail non enregistré suit le
    /// journal (ADR-0016 C3).
    pub(crate) fn retablir(&mut self) {
        self.effacer_messages();
        if self.session.redo().is_ok() {
            self.dirty = true;
            self.raz_liaisons();
        }
    }

    /// Remet à zéro les tampons d'édition des liaisons et avance la
    /// génération : une annulation, un rétablissement ou une
    /// ouverture peut avoir redéfini le graphe des valeurs — les
    /// cotes pilotées se re-figent à la prochaine résolution
    /// (ADR-0019 §6).
    pub(crate) fn raz_liaisons(&mut self) {
        self.tampon_dimensions.clear();
        self.tampon_expressions.clear();
        self.generation_liaisons += 1;
    }

    /// Traduit une clé du catalogue embarqué.
    #[expect(clippy::expect_used)]
    pub(crate) fn texte(&self, cle: &str) -> String {
        self.i18n
            .traduction(cle)
            .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)")
    }

    /// Traduit une clé du catalogue embarqué, avec arguments.
    #[expect(clippy::expect_used)]
    pub(crate) fn texte_avec(&self, cle: &str, arguments: &[(&str, Argument)]) -> String {
        self.i18n
            .traduction_avec(cle, arguments)
            .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)")
    }

    /// Ouvre un projet depuis un chemin choisi (sélecteur natif ou
    /// bibliothèque) : rejeu du journal, mémorisation du chemin —
    /// état d'app UI, pas un événement (ADR-0014).
    pub(crate) fn ouvrir_projet(&mut self, chemin: &Path) {
        self.effacer_messages();
        // Registre des types connus (ADR-0016 A3) : un journal écrit
        // par une version plus récente est refusé à l'ouverture.
        let registre = forge_session::registre_migrations_courant();
        let document = match Document::open_from_path_with(chemin, registre) {
            Ok(document) => document,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let nombre = document.events().len();
        let session = match Session::from_document(ACTEUR, document) {
            Ok(session) => session,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        self.session = session;
        self.generation += 1;
        self.cache = None;
        self.cache_maillage = None;
        self.camera = Camera2D::nouvelle();
        self.camera_cle = None;
        self.camera3d = Camera3D::nouvelle();
        self.croquis_resolu = None;
        self.derniere_resolution_bonne = None;
        self.resolution_en_attente = None;
        self.surcharge_relachee = None;
        self.redondances_affichees = None;
        self.redondances_precedentes = None;
        self.chemin_projet = Some(chemin.to_path_buf());
        self.ecran = Ecran::Atelier;
        self.raz_liaisons();
        self.info = Some(self.texte_avec("info-recharge", &[("n", Argument::from(nombre))]));
        self.dirty = false;
    }

    /// Enregistre le projet dans `chemin` : conteneur ZIP, les
    /// actions annulées sont abandonnées (ADR-0008) ; le chemin
    /// devient le projet courant (ADR-0014).
    pub(crate) fn enregistrer_dans(&mut self, chemin: &Path) {
        self.effacer_messages();
        if let Err(error) = self.session.save_to_path(chemin) {
            self.error = Some(error.to_string());
            return;
        }
        self.chemin_projet = Some(chemin.to_path_buf());
        let affiche = chemin.display().to_string();
        self.info = Some(self.texte_avec(
            "info-enregistre",
            &[("chemin", Argument::from(affiche.as_str()))],
        ));
        self.dirty = false;
    }

    /// « Enregistrer » : dans le projet courant, sinon via le
    /// dialogue « Enregistrer sous… » (bibliothèque proposée —
    /// ADR-0014).
    pub(crate) fn enregistrer(&mut self) {
        if let Some(chemin) = self.chemin_projet.clone() {
            self.enregistrer_dans(&chemin);
            return;
        }
        self.enregistrer_sous();
    }

    /// « Enregistrer sous… » : dialogue natif, dossier de la
    /// bibliothèque proposé par défaut (ADR-0014).
    pub(crate) fn enregistrer_sous(&mut self) {
        let libelle = self.texte("filtre-projet");
        let dossier = bibliotheque::dossier();
        let Some(chemin) = fichiers::chemin_projet(&libelle, dossier.as_deref()) else {
            return;
        };
        self.enregistrer_dans(&chemin);
    }

    /// Demande une action destructive : confirmation d'abord si du
    /// travail non enregistré existe, exécution directe sinon
    /// (ADR-0016 C3).
    pub(crate) fn demander(&mut self, action: ActionDestructrice) {
        if self.dirty {
            self.demande = Some(action);
        } else {
            self.executer_action(action);
        }
    }

    /// Exécute l'action destructive confirmée (ou immédiate quand
    /// rien n'est à perdre) : nouveau projet, ouverture — chemin
    /// reçu ou sélecteur natif — ou retour à l'atelier (ADR-0016 C3).
    pub(crate) fn executer_action(&mut self, action: ActionDestructrice) {
        match action {
            ActionDestructrice::Nouveau => {
                self.reinitialiser();
                self.ecran = Ecran::Atelier;
            }
            ActionDestructrice::Ouvrir { chemin } => {
                let Some(chemin) = chemin.or_else(|| self.demander_chemin()) else {
                    return;
                };
                self.ouvrir_projet(&chemin);
            }
            ActionDestructrice::Reprendre => {
                self.effacer_messages();
                self.ecran = Ecran::Atelier;
            }
            // La croix ne passe jamais par ici : la garde de l'update
            // l'annule et la fenêtre dédiée pilote enregistrer /
            // quitter / annuler avant la fermeture (audit n°3 A3-06).
            ActionDestructrice::Quitter => {}
        }
    }

    /// Chemin d'ouverture demandé au sélecteur natif quand l'action
    /// destructive n'en porte pas (ADR-0014, ADR-0016 C3).
    pub(crate) fn demander_chemin(&self) -> Option<PathBuf> {
        let libelle = self.texte("filtre-projet");
        fichiers::choisir_projet(&libelle)
    }

    /// Fenêtre de confirmation d'une action destructive (ADR-0016 C3) :
    /// « Continuer » exécute l'action, « Annuler » la rejette — du
    /// travail non enregistré serait perdu. Fermeture demandée à la
    /// croix (audit n°3 A3-06) : « Enregistrer » sauve puis ferme,
    /// « Quitter sans enregistrer » ferme, « Annuler » ramène au
    /// travail.
    pub(crate) fn fenetre_confirmation(&mut self, ctx: &egui::Context) {
        /// Choix du dialogue de fermeture (audit n°3 A3-06).
        #[derive(Debug, Clone, Copy)]
        enum ChoixFermeture {
            /// Enregistrer puis fermer.
            Enregistrer,
            /// Fermer sans enregistrer.
            Quitter,
            /// Rester dans le travail courant.
            Annuler,
        }
        let Some(action) = self.demande.clone() else {
            return;
        };
        let fermeture = matches!(action, ActionDestructrice::Quitter);
        let mut decision: Option<bool> = None;
        let mut choix: Option<ChoixFermeture> = None;
        let cle_texte = if fermeture {
            "confirmation-fermeture-texte"
        } else {
            "confirmation-texte"
        };
        egui::Window::new(self.texte("confirmation-titre"))
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(self.texte(cle_texte));
                ui.horizontal(|ui| {
                    if fermeture {
                        let reponse = ui.button(self.texte("confirmation-enregistrer"));
                        avec_aide(&reponse, "aide-confirmation-enregistrer", &mut self.aide);
                        if reponse.clicked() {
                            choix = Some(ChoixFermeture::Enregistrer);
                        }
                        let reponse = ui.button(self.texte("confirmation-quitter"));
                        avec_aide(&reponse, "aide-confirmation-quitter", &mut self.aide);
                        if reponse.clicked() {
                            choix = Some(ChoixFermeture::Quitter);
                        }
                        let reponse = ui.button(self.texte("confirmation-annuler"));
                        if reponse.clicked() {
                            choix = Some(ChoixFermeture::Annuler);
                        }
                    } else {
                        let reponse = ui.button(self.texte("confirmation-confirmer"));
                        avec_aide(&reponse, "aide-confirmation-perte", &mut self.aide);
                        if reponse.clicked() {
                            decision = Some(true);
                        }
                        let reponse = ui.button(self.texte("confirmation-annuler"));
                        if reponse.clicked() {
                            decision = Some(false);
                        }
                    }
                });
            });
        match decision {
            Some(true) => {
                self.demande = None;
                self.executer_action(action);
            }
            Some(false) => self.demande = None,
            None => {}
        }
        match choix {
            Some(ChoixFermeture::Enregistrer) => {
                // Enregistrer d'abord : sans fichier courant,
                // « Enregistrer sous… » s'ouvre — la fenêtre ne
                // ferme que si la sauvegarde a eu lieu.
                self.enregistrer();
                if !self.dirty {
                    self.demande = None;
                    self.fermeture_confirmee = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Some(ChoixFermeture::Quitter) => {
                self.demande = None;
                self.fermeture_confirmee = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Some(ChoixFermeture::Annuler) => self.demande = None,
            None => {}
        }
    }

    /// Fenêtre du choix des unités d'un maillage minuscule (ADR-0017) :
    /// « Mètres » importe à l'échelle × 1000, « Millimètres » importe
    /// tel quel, « Annuler » n'écrit rien.
    pub(crate) fn fenetre_echelle(&mut self, ctx: &egui::Context) {
        let Some(demande) = self.demande_echelle.clone() else {
            return;
        };
        let mut decision: Option<f64> = None;
        let mut annule = false;
        let dimension = format!("{:.2}", demande.dimension);
        egui::Window::new(self.texte("echelle-titre"))
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(self.texte_avec(
                    "echelle-texte",
                    &[("dimension", Argument::from(dimension.as_str()))],
                ));
                ui.horizontal(|ui| {
                    let reponse = ui.button(self.texte("echelle-metres"));
                    avec_aide(&reponse, "aide-echelle-metres", &mut self.aide);
                    if reponse.clicked() {
                        decision = Some(ECHELLE_METRES);
                    }
                    let reponse = ui.button(self.texte("echelle-millimetres"));
                    avec_aide(&reponse, "aide-echelle-millimetres", &mut self.aide);
                    if reponse.clicked() {
                        decision = Some(1.0);
                    }
                    let reponse = ui.button(self.texte("echelle-annuler"));
                    avec_aide(&reponse, "aide-echelle-annuler", &mut self.aide);
                    if reponse.clicked() {
                        annule = true;
                    }
                });
            });
        if annule {
            self.demande_echelle = None;
        } else if let Some(echelle) = decision {
            // ADR-0038 D2 : la demande ne garde pas d'octets — le
            // fichier est relu au fil de travail sous le budget du
            // format ; consommée ici, un échec de relecture devient
            // un message d'import, jamais une écriture au journal.
            self.demande_echelle = None;
            self.envoyer_travail(Travail::ConfirmerMaillage {
                generation: self.generation,
                chemin: demande.chemin,
                echelle,
            });
        }
    }

    /// Lit puis importe le fichier de tracés choisi dans le sélecteur
    /// natif : lecture et parse partent au fil de travail, le format
    /// suit le bouton (ADR-0014, ADR-0016 C1).
    pub(crate) fn charger_traces(&mut self, format: &'static str, chemin: &Path) {
        self.effacer_messages();
        self.envoyer_travail(Travail::ImportTraces {
            generation: self.generation,
            format,
            chemin: chemin.to_path_buf(),
        });
    }
    /// Lit puis importe le maillage STL choisi dans le sélecteur
    /// natif : lecture et parse au fil de travail (ADR-0014,
    /// ADR-0016 C1).
    pub(crate) fn charger_maillage(&mut self, chemin: &Path) {
        self.effacer_messages();
        self.envoyer_travail(Travail::ImportMaillage {
            generation: self.generation,
            chemin: chemin.to_path_buf(),
        });
    }
    /// Confie un travail au fil de travail : compteur des travaux en
    /// attente et message d'information (ADR-0016 C1) ; sans fil
    /// disponible, la demande est abandonnée en silence. Le fil mort
    /// est relancé au prochain envoi (audit n°3 N8) : il laissait un
    /// « travail en cours » permanent — compteur jamais
    /// décrémenté, repaint infini — et des envois abandonnés en
    /// silence.
    pub(crate) fn envoyer_travail(&mut self, travail: Travail) {
        let Some(emetteur) = &self.travaux else {
            return;
        };
        match emetteur.send(travail) {
            Ok(()) => {
                self.travaux_en_attente += 1;
                self.info = Some(self.texte("info-travail-en-cours"));
            }
            Err(erreur) => {
                let travail = erreur.0;
                self.relancer_fil_de_travail();
                if let Some(emetteur) = &self.travaux
                    && emetteur.send(travail).is_ok()
                {
                    self.travaux_en_attente += 1;
                    self.info = Some(self.texte("info-travail-en-cours"));
                }
            }
        }
    }

    /// Relance le fil de travail mort (audit n°3 N8) : un fil neuf
    /// repart sur des canaux neufs — les travaux en vol du fil mort
    /// ne reviendront jamais, le compteur des travaux en attente
    /// repart de zéro.
    pub(crate) fn relancer_fil_de_travail(&mut self) {
        let (emetteur_travaux, recepteur_travaux) = mpsc::channel::<Travail>();
        let (emetteur_retours, recepteur_retours) = mpsc::channel::<Retour>();
        self.travaux = std::thread::Builder::new()
            .name("omnifab-travail".to_string())
            .spawn(move || fil_de_travail(recepteur_travaux, emetteur_retours))
            .is_ok()
            .then_some(emetteur_travaux);
        self.retours = recepteur_retours;
        self.travaux_en_attente = 0;
    }
    /// Applique un retour du fil de travail à l'état : commande de
    /// session pour un import, message pour un export, canal
    /// d'erreur pour un échec (ADR-0016 C1).
    pub(crate) fn appliquer_retour(&mut self, retour: Retour) {
        // Fin d'une action : le verdict remplace l'état précédent
        // (ADR-0016 C4).
        self.effacer_messages();
        match retour {
            Retour::ImportTraces {
                format,
                octets,
                traces,
                sommets,
                ..
            } => {
                // Le succès n'est affiché que si la commande passe :
                // un refus ne pose jamais de message (ADR-0016 C4).
                let resultat = self.executer(&ImportTraces {
                    format: format.to_string(),
                    octets,
                    traces,
                    sommets,
                });
                if resultat.is_ok() {
                    self.cache = None;
                    self.info = Some(self.texte_avec(
                        "info-import-traces",
                        &[
                            ("traces", Argument::from(traces)),
                            ("sommets", Argument::from(sommets)),
                        ],
                    ));
                }
            }
            Retour::ImportMaillage {
                chemin,
                octets,
                triangles,
                dimension,
                ..
            } => {
                // Un maillage minuscule ressemble à un export en
                // mètres : l'import attend le choix des unités
                // (ADR-0017) ; rien n'est écrit avant la réponse.
                // La demande ne garde que chemin et dimension —
                // pas de copie d'octets (ADR-0038 D2).
                if propose_echelle_metres(dimension) {
                    self.demande_echelle = Some(DemandeEchelle { chemin, dimension });
                } else {
                    self.importer_maillage(octets, triangles, 1.0);
                }
            }
            Retour::ConfirmerMaillage {
                octets,
                triangles,
                echelle,
                ..
            } => {
                // La confirmation d'échelle a relu le fichier au fil
                // de travail sous le budget du format (ADR-0038 D2) :
                // l'import part des octets relus, tout-ou-rien.
                self.importer_maillage(octets, triangles, echelle);
            }
            Retour::Export { chemin, .. } => {
                self.info = Some(self.texte_avec(
                    "info-export",
                    &[("chemin", Argument::from(chemin.as_str()))],
                ));
            }
            Retour::Erreur { message, .. } => {
                self.error = Some(message);
            }
            Retour::EsquisseResolue { cle, resultat, .. } => {
                if self.resolution_en_attente == Some(cle) {
                    self.resolution_en_attente = None;
                }
                // Redondances du diagnostic (ADR-0018 D5, lot 3c-4) :
                // l'hystérésis n'affiche que le diagnostic stable sur
                // deux résolutions consécutives (FPA #106) ; un échec
                // remet l'hystérésis à zéro.
                match &resultat {
                    Ok(croquis) => {
                        let esquisse = self.session.esquisse();
                        let courantes =
                            atelier_esquisse::contraintes_redondantes(croquis, esquisse);
                        let stables = atelier_esquisse::redondances_stables(
                            self.redondances_precedentes.as_deref(),
                            &courantes,
                        );
                        self.redondances_affichees = stables;
                        self.redondances_precedentes = Some(courantes);
                    }
                    Err(_) => {
                        self.redondances_affichees = None;
                        self.redondances_precedentes = None;
                    }
                }
                // L’échec est mémorisé à sa clé (ADR-0018 D7) : le
                // journal est intact — et la résolution ne repart
                // pas à chaque frame (ADR-0016 C1).
                if let Err(message) = &resultat {
                    self.error = Some(message.clone());
                }
                // La dernière résolution RÉUSSIE survit à l’échec
                // (re-test du 24-10) : l’affichage ne retombe
                // jamais sur les graines périmées du journal — la
                // forme saine reste dessinée, l’erreur est
                // signalée par le message d’état.
                if let Ok(croquis) = &resultat {
                    self.derniere_resolution_bonne = Some(croquis.clone());
                }
                self.croquis_resolu = Some((cle, resultat));
                // La surcharge du relâchement a tenu le vol : le
                // frais la remplace (anti-flash du relâchement,
                // re-validation §7).
                if cle == (self.session.esquisse().edition(), self.generation_liaisons) {
                    self.surcharge_relachee = None;
                }
            }
            Retour::EsquisseApercue { resultat, .. } => {
                // Aperçu de glissement (re-test du 2026-10-23) :
                // affichage seul — l'échec ne remonte jamais, le
                // dessin retombe sur la surcharge du glissement.
                self.apercu_en_attente = false;
                self.croquis_apercu = Some(resultat);
            }
        }
    }
    /// Demande la résolution du croquis contraint au fil de travail
    /// si l’édition de la projection ou le graphe des liaisons a
    /// changé (ADR-0018 D3, ADR-0019 §6) : le solveur natif est
    /// instancié dans le fil de travail — jamais sur le fil
    /// d’interface, jamais dans forge-session (ADR-0016 C1). Une
    /// seule résolution en vol par clé ; un échec déjà mémorisé ne
    /// repart pas à chaque frame.
    pub(crate) fn demande_resolution_esquisse(&mut self) {
        if self.session.esquisse().entites().is_empty() {
            self.croquis_resolu = None;
            self.derniere_resolution_bonne = None;
            self.resolution_en_attente = None;
            return;
        }
        let cle = (self.session.esquisse().edition(), self.generation_liaisons);
        if self
            .croquis_resolu
            .as_ref()
            .is_some_and(|(cle_cache, _)| *cle_cache == cle)
        {
            return;
        }
        if self.resolution_en_attente == Some(cle) {
            return;
        }
        let Some(emetteur) = &self.travaux else {
            return;
        };
        // Cotes pilotées figées par le moteur de liaisons (ADR-0019) :
        // le fil de travail ne voit jamais une expression, la
        // frontière forge-values → forge-gcs reste intacte.
        let croquis = match self.session.croquis_pour_resolution() {
            Ok(croquis) => croquis,
            Err(erreur) => {
                self.error = Some(erreur.to_string());
                return;
            }
        };
        let travail = Travail::ResoudreEsquisse {
            generation: self.generation,
            cle,
            croquis,
        };
        if emetteur.send(travail).is_ok() {
            self.resolution_en_attente = Some(cle);
            self.travaux_en_attente += 1;
        }
    }

    /// Demande l'aperçu de glissement du croquis au fil de travail
    /// (re-test du 2026-10-23) : la projection clonée reçoit la
    /// position visée comme relâchement provisoire — la forme
    /// liée suit le point saisi pendant le glissement, exactement
    /// comme au relâchement. Un seul aperçu en vol ; la même
    /// position ne repart pas ; l'échec est silencieux — le
    /// résultat sert l'affichage seul.
    pub(crate) fn demande_apercu_glissement(&mut self, point: u32, x: Length, y: Length) {
        let cle = (point, x.millimeters(), y.millimeters());
        if self.apercu_en_attente || self.apercu_envoye == Some(cle) {
            return;
        }
        let Some(emetteur) = &self.travaux else {
            return;
        };
        // Cotes pilotées figées comme pour toute résolution
        // (ADR-0019) : le fil de travail ne voit jamais
        // d'expression.
        let croquis = match self.session.croquis_pour_resolution() {
            Ok(croquis) => croquis,
            Err(_) => return,
        };
        let Some(croquis) = croquis.apercu_de_glissement(point, x, y) else {
            return;
        };
        let travail = Travail::ResoudreApercu {
            generation: self.generation,
            croquis,
        };
        if emetteur.send(travail).is_ok() {
            self.apercu_en_attente = true;
            self.apercu_envoye = Some(cle);
            self.travaux_en_attente += 1;
        }
    }

    /// Purge de l'aperçu de glissement une fois le geste fini
    /// (re-test du 24-10) : l'aperçu tient le glissement ET le vol
    /// du relâchement — il se purge seulement quand les deux sont
    /// finis, le frais du relâchement a pris le relais.
    pub(crate) fn purge_apercu_si_geste_fini(&mut self) {
        if self.saisie.is_none() && atelier_esquisse::surcharge_du_relachement(self).is_none() {
            self.croquis_apercu = None;
            self.apercu_envoye = None;
            self.apercu_en_attente = false;
        }
    }

    /// Résolution d'affichage du geste de poignée (re-test du
    /// 24-10) : l'aperçu de glissement reste la résolution
    /// d'affichage tant que le glissement tient OU que le vol du
    /// relâchement dure — il est l'état résolu du relâchement, le
    /// frais le remplace à son arrivée. Pendant un glissement, le
    /// dessin ne montre ainsi que des états résolus — jamais une
    /// position brute du curseur qui violerait une contrainte.
    pub(crate) fn resolution_du_geste(&self) -> Option<&CroquisResolu> {
        if self.saisie.is_some() || atelier_esquisse::surcharge_du_relachement(self).is_some() {
            self.croquis_apercu
                .as_ref()
                .and_then(|resultat| resultat.as_ref().ok())
        } else {
            None
        }
    }

    /// Importe le maillage à l'échelle donnée : commande tout-ou-rien,
    /// le succès n'est affiché que si elle passe (ADR-0016 C4) ; une
    /// demande d'échelle en attente est consommée (ADR-0017).
    pub(crate) fn importer_maillage(&mut self, octets: Vec<u8>, triangles: usize, echelle: f64) {
        self.demande_echelle = None;
        let resultat = self.executer(&ImportMesh {
            format: "stl".to_string(),
            octets,
            triangles,
            echelle,
        });
        if resultat.is_ok() {
            self.cache_maillage = None;
            self.info = Some(self.texte_avec(
                "info-import-maillage",
                &[("triangles", Argument::from(triangles))],
            ));
        }
    }
    /// Exporte les tracés courants au format demandé, vers le chemin
    /// choisi : re-parse du blob d'origine puis sérialisation au fil
    /// de travail. L'export ne change pas l'état du projet : rien au
    /// journal (ADR-0011, ADR-0016 C1).
    pub(crate) fn exporter_traces(&mut self, format: &'static str, chemin: &Path) {
        self.effacer_messages();
        let Some(reference) = self.session.geom().traces() else {
            return;
        };
        let blob = reference.blob.clone();
        let Some(octets) = self.session.document().blob(&blob) else {
            self.error = Some(format!("blob absent du document : {blob}"));
            return;
        };
        let format_origine = match reference.format.as_str() {
            "svg" => "svg",
            _ => "dxf",
        };
        self.envoyer_travail(Travail::ExportTraces {
            generation: self.generation,
            format_origine,
            format,
            octets: octets.to_vec(),
            chemin: chemin.to_path_buf(),
        });
    }
    /// Exporte le maillage courant en STL binaire, vers le chemin
    /// choisi : re-parse et écriture au fil de travail, depuis les
    /// octets d'origine (ADR-0011, ADR-0016 C1).
    pub(crate) fn exporter_maillage(&mut self, chemin: &Path) {
        self.effacer_messages();
        let Some(reference) = self.session.geom().maillage() else {
            return;
        };
        let blob = reference.blob.clone();
        let Some(octets) = self.session.document().blob(&blob) else {
            self.error = Some(format!("blob absent du document : {blob}"));
            return;
        };
        let echelle = reference.echelle;
        self.envoyer_travail(Travail::ExportMaillage {
            generation: self.generation,
            octets: octets.to_vec(),
            echelle,
            chemin: chemin.to_path_buf(),
        });
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Instant de la trame (ADR-0021 D2) : alimente l'estompage
        // de la surbrillance momentanée des contraintes posées.
        self.instant = ctx.input(|i| i.time);
        // Raccourcis : Cmd (Ctrl sous Windows/Linux) annule, Cmd+Y ou
        // Cmd+Maj+Z rétablit ; aucun raccourci quand un champ de texte
        // a le focus — la saisie en cours n'est pas écrasée
        // (ADR-0016 C2) ; un raccourci impossible est ignoré
        // silencieusement (ADR-0011).
        let clavier_pris = ctx.wants_keyboard_input();
        let (annuler, retablir) = ctx.input(|i| {
            raccourcis_annulation(
                &i.modifiers,
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                clavier_pris,
            )
        });
        if annuler && self.session.can_undo() {
            self.annuler();
        }
        if retablir && self.session.can_redo() {
            self.retablir();
        }
        // Retours du fil de travail : imports, exports et échecs
        // (ADR-0016 C1) ; les travaux d'une génération dépassée
        // (réinitialisation, ouverture) sont ignorés.
        let recus: Vec<Retour> = self.retours.try_iter().collect();
        self.travaux_en_attente = self.travaux_en_attente.saturating_sub(recus.len() as u32);
        for retour in garde_actuels(recus, self.generation) {
            self.appliquer_retour(retour);
        }
        // Résolution du croquis contraint (ADR-0018 D3) : la demande
        // ne part que si l’édition du croquis a changé.
        self.demande_resolution_esquisse();
        if self.travaux_en_attente > 0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
        });
        // Panneaux de l'atelier seulement : l'accueil garde tout
        // l'espace pour la bibliothèque (ADR-0014).
        if self.ecran == Ecran::Atelier {
            egui::SidePanel::left("liaisons")
                .resizable(true)
                .min_width(220.0)
                .default_width(300.0)
                .show(ctx, |ui| {
                    self.liaisons_panel(ui);
                });
            egui::SidePanel::right("journal")
                .default_width(330.0)
                .show(ctx, |ui| {
                    self.journal_panel(ui);
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.ecran == Ecran::Accueil {
                self.accueil(ui);
            } else {
                self.viewport(ui);
            }
        });
        // Garde de fermeture (audit n°3 A3-06) : la fenêtre ne se
        // ferme pas tant que du travail non enregistré existe — la
        // demande de fermeture est annulée (CancelClose) et la
        // confirmation demandée (exemple egui « confirm_exit ») ;
        // l'annonce d'avant le correctif, « fermeture non
        // interceptable », était fausse (audit n°3, D6). Une
        // fermeture confirmée par le dialogue passe : la garde ne
        // bloque plus la demande qu'elle vient d'émettre.
        if !self.fermeture_confirmee && ctx.input(|i| i.viewport().close_requested()) {
            match garde_fermeture(self.dirty, self.demande.is_some()) {
                DecisionFermeture::LaisserFermer => {}
                DecisionFermeture::Bloquer => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
                DecisionFermeture::BloquerEtConfirmer => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.demande = Some(ActionDestructrice::Quitter);
                }
            }
        }
        // Titre de la fenêtre : « * » signale le travail non
        // enregistré, levé par la sauvegarde (ADR-0016 C3).
        if self.titre_sale != self.dirty {
            let cle = if self.dirty {
                "titre-fenetre-modifie"
            } else {
                "titre-fenetre"
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.texte(cle)));
            self.titre_sale = self.dirty;
        }
        self.fenetre_confirmation(ctx);
        self.fenetre_echelle(ctx);
        self.fenetre_dialogues_liaisons(ctx);
        atelier_esquisse::fenetre_cote(ctx, self);
        aide::fenetre(ctx, &self.i18n, &mut self.aide);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Point2, Point3};
    use forge_session::{AddConstraint, AddEntity, ContrainteEsquisse, EntiteEsquisse, MovePoint};

    /// Découpage 6/6 de `main.rs` (fiche
    /// `preparation/decoupage-main-rs.md`) : `DemoApp` vit dans son
    /// propre module `app`, le lanceur ne porte que `main` — garde
    /// structurelle du refactor.
    #[test]
    fn demoapp_vit_dans_le_module_app() {
        let _demo = crate::app::DemoApp::new(i18n::catalogue(), None);
    }

    /// SVG minimal de test : un triangle fermé.
    const SVG_TEST: &[u8] =
        b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M 0 0 L 10 0 L 0 10 Z\"/></svg>\n";

    /// Importe le SVG de test via l'application, comme le bouton
    /// « Importer SVG ».
    fn importer_svg_de_test(app: &mut DemoApp) {
        let traces = svg::import(SVG_TEST, &svg::SvgBudgets::default()).expect("svg de test");
        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
        app.executer(&ImportTraces {
            format: "svg".into(),
            octets: SVG_TEST.to_vec(),
            traces: traces.len(),
            sommets,
        })
        .expect("import de test");
    }

    /// La croix ne doit pas perdre le travail non enregistré (audit
    /// n°3 A3-06) : la fermeture est bloquée, la confirmation
    /// demandée — trois cas.
    #[test]
    fn doit_bloquer_la_fermeture() {
        // Travail non enregistré, confirmation déjà ouverte : la
        // fermeture reste bloquée, sans rouvrir la confirmation.
        assert_eq!(
            garde_fermeture(true, true),
            DecisionFermeture::Bloquer,
            "la fermeture reste bloquée tant que la confirmation est ouverte"
        );
