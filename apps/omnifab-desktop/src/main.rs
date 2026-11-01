#![forbid(unsafe_code)]

//! # omnifab-desktop — démonstrateur des jalons 1 et 2
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

mod aide;
mod atelier_esquisse;
mod bibliotheque;
mod esquisse;
mod fichiers;
mod i18n;
mod travail;

use aide::{avec_aide, avec_aide_sur_calque};
use bibliotheque::EntreeProjet;
use eframe::egui;
use forge_doc::Document;
use forge_geom::{Mesh, Path2};
use forge_i18n::Argument;
use forge_io::{dxf, stl, svg};
use forge_render::{
    Boite2, Boite3D, Camera2D, Camera3D, Couleur, Renderer2D, Scene2D, Scene3D, Segment2D,
    Segment3D, dessiner_scene, dessiner_wireframe, pas_de_grille, pas_decimation,
};
use forge_session::{
    AddConstraint, AddEntity, Command, ContrainteEsquisse, CroquisResolu, EntiteEsquisse,
    ImportMesh, ImportTraces, ModeSuppression, RemoveAlias, Session, SetExpression, SetInput, Unite,
    ValeurCote,
};
#[cfg(test)]
use forge_session::EsquisseProjection;
use forge_units::Length;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use travail::Retour;
use travail::Travail;
use travail::fil_de_travail;
use travail::garde_actuels;
use travail::maillage_echelle;
use travail::met_en_cache;
use travail::propose_echelle_metres;
use travail::raccourcis_annulation;
use travail::touche_les_liaisons;

/// État de rendu wgpu d'eframe : alias pour alléger la signature du
/// constructeur (ADR-0015).
type RenderState = eframe::egui_wgpu::RenderState;

/// Acteur des événements écrits par le démonstrateur (ADR-0011).
const ACTEUR: &str = "desktop.demo";
/// Expression initiale de la diagonale du rectangle.
const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(largeur * largeur + hauteur * hauteur)";
/// Couleur du wireframe 3D du maillage, RGBA normalisé — parité avec
/// le painter egui (40, 220, 220) (ADR-0015, phase C2).
const COULEUR_WIREFRAME: Couleur = [40.0 / 255.0, 220.0 / 255.0, 220.0 / 255.0, 1.0];
/// Couleur du contour du rectangle paramétrique posé sur le plan z = 0
/// de la vue 3D, RGBA normalisé (ADR-0015, phase C2).
const COULEUR_RECTANGLE_3D: Couleur = [1.0, 1.0, 1.0, 1.0];
/// Couleur des tracés importés, RGBA normalisé — parité avec le painter
/// egui (240, 180, 20) (ADR-0015).
const COULEUR_TRACES: Couleur = [240.0 / 255.0, 180.0 / 255.0, 20.0 / 255.0, 1.0];
/// Largeur du trait des tracés importés, en pixels logiques (ADR-0015).
const LARGEUR_TRACES_PX: f32 = 1.5;
/// Couleur de la grille de fond 3D, RGBA normalisé (ADR-0015).
const COULEUR_GRILLE: Couleur = [0.35, 0.40, 0.47, 1.0];
/// Couleur de la grille 2D, plus claire que la feuille bleue du
/// rectangle paramétrique pour éviter la confusion (ADR-0015, phase C2).
const COULEUR_GRILLE_2D: Couleur = [0.55, 0.62, 0.75, 1.0];
/// Espacement visé entre deux lignes de grille, en pixels (ADR-0015).
/// 40 px (re-validation §7) : des cellules plus fines au même niveau
/// de zoom restent lisibles, le pas s'adapte par puissances de dix.
const GRILLE_CIBLE_PX: f64 = 40.0;
/// Seuil de détection d'un export en mètres : un maillage dont la
/// plus grande dimension est sous ce seuil (en millimètres) propose
/// l'échelle × 1000 à l'import (ADR-0017). 10 mm : un objet de 3 m
/// exporté en mètres depuis Blender mesure 3,0 « mm » après lecture
/// (retour checklist jalon 2) — le seuil couvre les objets jusqu'à
/// ~10 m en mètres ; une pièce réellement plus petite déclenche
/// aussi le dialogue, au prix d'un clic « Millimètres ».
const SEUIL_METRES: f64 = 10.0;
/// Échelle proposée pour un maillage exporté en mètres (Blender) :
/// les coordonnées sont multipliées par 1000 pour revenir en
/// millimètres (ADR-0017).
const ECHELLE_METRES: f64 = 1000.0;
/// Nombre d'événements posés par une session vierge : au-delà, la
/// session est considérée « en cours » sur l'écran d'accueil.
const EVENEMENTS_VIERGES: usize = 3;

/// Écran affiché : accueil (bibliothèque) ou atelier (ADR-0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ecran {
    /// Écran d'accueil : bibliothèque de projets.
    Accueil,
    /// Atelier : viewport et panneaux.
    Atelier,
}

/// Vue du viewport : plan 2D ou volume 3D (ADR-0015, phase C2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vue {
    /// Vue 2D : grille, rectangle et tracés au sol.
    DeuxD,
    /// Vue 3D : wireframe du maillage en volume, caméra orbite.
    TroisD,
}

/// Action destructive : elle ne part qu'après confirmation quand du
/// travail non enregistré existe (ADR-0016 C3).
#[derive(Debug, Clone)]
enum ActionDestructrice {
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
enum DecisionFermeture {
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
fn garde_fermeture(dirty: bool, confirmation_en_cours: bool) -> DecisionFermeture {
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
#[derive(Debug, Clone)]
struct DemandeEchelle {
    /// Octets d'origine du fichier importé.
    octets: Vec<u8>,
    /// Nombre de triangles du maillage.
    triangles: usize,
    /// Plus grande dimension du maillage, en millimètres.
    dimension: f64,
}

/// Dialogue « + Nouvel alias » (ADR-0019 §6) : nom, entrée ou
/// expression, unité — état de l'app UI, jamais un événement tant
/// que « Poser » n'est pas cliqué.
struct DialogueNouvelAlias {
    /// Nom de l'alias à créer.
    nom: String,
    /// `true` : expression calculée ; `false` : entrée numérique.
    expression: bool,
    /// Valeur de l'entrée.
    valeur: f64,
    /// Texte de l'expression.
    texte: String,
    /// Unité déclarée (`None` = sans unité).
    unite: Option<Unite>,
}

/// Dialogue de suppression d'un alias référencé (ADR-0019 §6,
/// amendement 2026-10-20) : les formules concernées sont listées, la
/// suppression propose cascade, remplacement ou annulation.
struct DialogueSuppression {
    /// Alias demandé en suppression.
    alias: String,
    /// Formules et cotes qui lisent encore l'alias.
    references: Vec<String>,
    /// Alias de remplacement choisi (mode Remplacer).
    remplacement: String,
}

/// `help_id` de la page d'aide du code d'erreur **terminal** du
/// message : les refus de la session enchaînent les codes — un refus
/// de projection enveloppe celui de la cause (OF-SES-0002 puis
/// OF-VAL-0007) —, la page d'aide de la cause terminale, le dernier
/// code du message, s'ouvre (ADR-0019 §6, correctif ADR-0020).
fn aide_du_code(message: &str) -> String {
    let code = message
        .split(' ')
        .rfind(|mot| mot.starts_with("OF-"))
        .unwrap_or_else(|| message.split(' ').next().unwrap_or(""));
    format!("aide-{}", code.to_lowercase())
}

struct DemoApp {
    /// Session de travail : document, projections, undo/redo
    /// (ADR-0008/0011).
    session: Session,
    /// Tampons d'édition des expressions des alias calculés : le
    /// champ texte affiche l'expression du début de frame, un
    /// rechargement du graphe à chaque frame écraserait la saisie
    /// en cours (leçon round 6 egui) — chaque alias garde son
    /// texte et son activité d'une frame à l'autre.
    tampon_expressions: BTreeMap<String, (String, bool)>,
    /// Dernière erreur à afficher dans le viewport.
    error: Option<String>,
    /// Dernier message d'information à afficher dans le viewport.
    info: Option<String>,
    /// Catalogue Fluent embarqué.
    i18n: i18n::Catalogue,
    /// Page d'aide ouverte, si une page est demandée.
    aide: Option<String>,
    /// Écran affiché : accueil ou atelier (ADR-0014).
    ecran: Ecran,
    /// Fichier projet courant, choisi par l'utilisateur : état de
    /// l'app UI, jamais un événement (ADR-0014).
    chemin_projet: Option<PathBuf>,
    /// Cache mémoire non autoritaire des tracés re-parsés (nom du
    /// blob + tracés ou échec de parse), invalidé quand la référence
    /// change (ADR-0011) — un échec est mémorisé, pas re-parsé à
    /// chaque frame (ADR-0016 C1).
    cache: Option<(String, Result<Vec<Path2>, String>)>,
    /// Cache mémoire non autoritaire du maillage re-parsé (nom du
    /// blob + maillage ou échec de parse), invalidé quand la référence
    /// change (ADR-0014, ADR-0016 C1).
    cache_maillage: Option<(String, Result<Mesh, String>)>,
    /// Wireframe 3D du maillage en cache, à clé d'édition (ADR-0027,
    /// patron `camera_cle`) : les segments ne sont reconstruits que
    /// quand la référence du maillage change, jamais à chaque frame
    /// — un STL de centaines de milliers de triangles ne repaye
    /// plus le dédoublonnage à chaque draw (audit n°4 A4-02).
    wireframe_maillage: Option<(Option<String>, Vec<Segment3D>)>,
    /// Caméra du viewport : ajustée automatiquement quand la scêne
    /// change, pilotée ensuite par l'utilisateur (ADR-0015).
    camera: Camera2D,
    /// Clé du dernier ajustement : références des blobs tracés et
    /// maillage — la caméra se réajuste quand elles changent.
    camera_cle: Option<(Option<String>, Option<String>)>,
    /// Caméra orbite de la vue 3D : ajustée automatiquement quand la
    /// scêne change, pilotée ensuite par l'utilisateur (ADR-0015,
    /// phase C2).
    camera3d: Camera3D,
    /// Vue affichée par le viewport : 2D ou 3D (ADR-0015, phase C2).
    vue: Vue,
    /// Pipeline wgpu du viewport créé : le dessin 2D passe par le GPU,
    /// sinon le viewport affiche le message d'indisponibilité.
    viewport_wgpu: bool,
    /// Tampons d'édition des champs de dimensions : le DragValue affiche
    /// la valeur du début de frame, un rechargement du graphe à chaque
    /// frame figeait le chiffre pendant le geste — chaque champ garde
    /// donc sa valeur et son activité d'une frame à l'autre
    /// (validation C1 round 6).
    tampon_dimensions: BTreeMap<String, (f64, bool)>,
    /// Émetteur vers le fil de travail : lectures, parses et exports
    /// hors du fil d'interface (ADR-0016 C1) ; `None` si le fil n'a
    /// pas pu démarrer.
    travaux: Option<mpsc::Sender<Travail>>,
    /// Retours du fil de travail, consommés à chaque frame.
    retours: mpsc::Receiver<Retour>,
    /// Travaux confiés non revenus : le viewport demande un
    /// rafraîchissement tant qu'il y en a (ADR-0016 C1).
    travaux_en_attente: u32,
    /// Génération de l'état : incrémentée à chaque réinitialisation
    /// ou ouverture — les travaux en vol deviennent obsolètes
    /// (ADR-0016 C1).
    generation: u64,
    /// Travail non enregistré : posé par toute commande effective,
    /// levé par la sauvegarde (ADR-0016 C3).
    dirty: bool,
    /// Le titre de la fenêtre reflète-t-il déjà `dirty` ? La commande
    /// de titre n'est émise que quand il change (ADR-0016 C3).
    titre_sale: bool,
    /// Action destructive demandée, en attente de confirmation
    /// (ADR-0016 C3).
    demande: Option<ActionDestructrice>,
    /// Fermeture de la fenêtre confirmée par le dialogue (audit n°3
    /// A3-06) : la garde ne bloque plus la demande de fermeture
    /// qu'elle vient d'émettre.
    fermeture_confirmee: bool,
    /// Choix des unités d'un maillage minuscule, en attente de la
    /// réponse de l'utilisateur (ADR-0017).
    demande_echelle: Option<DemandeEchelle>,
    /// Cache des positions résolues du croquis, clé par (numéro
    /// d’édition, génération des liaisons) (ADR-0018 D3,
    /// ADR-0019 §6) : état dérivé en mémoire, jamais sérialisé ; un
    /// échec est mémorisé à sa clé — pas re-résolu à chaque
    /// frame — et les graines du journal restent la vérité
    /// affichable (ADR-0018 D7). Une cote pilotée re-figée repart
    /// en résolution même sans changement du croquis.
    croquis_resolu: Option<((u64, u64), Result<CroquisResolu, String>)>,
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
    derniere_resolution_bonne: Option<CroquisResolu>,
    /// Clé (édition, liaisons) en cours de résolution au fil de
    /// travail : une seule résolution en vol (ADR-0016 C1).
    resolution_en_attente: Option<(u64, u64)>,
    /// Outil actif du mode esquisse (ADR-0018, lot 3c) : état de
    /// l’app UI, jamais un événement.
    outil: atelier_esquisse::OutilEsquisse,
    /// Geste en cours de l'outil de croquis (ADR-0018, lot 3c-2) :
    /// l'origine posée attend la suite — état de l'app UI,
    /// jamais un événement, abandonné au changement d'outil.
    geste: Option<atelier_esquisse::GesteEsquisse>,
    /// Entités du croquis sélectionnées au clic (ADR-0018 D5,
    /// lot 3c-4) : au plus deux — les contraintes relient au plus
    /// deux entités ; la poignée au glissement garde la sélection
    /// — état de l'app UI, jamais un événement.
    selection: Vec<u32>,
    /// Point saisi par le glissement d'une poignée (ADR-0018 D4) :
    /// l'aperçu local le suit sans écrire, le relâchement pose sa
    /// graine — état de l'app UI, jamais un événement.
    saisie: Option<u32>,
    /// Surcharge d'affichage du point relâché, pendant le vol de sa
    /// résolution (anti-flash du relâchement, re-validation §7) :
    /// le relâchement pose la graine par `MovePoint` et la
    /// résolution repart au fil de travail — sans elle, l'affichage
    /// retomberait sur la résolution d'avant le glissement et la
    /// forme bondirait en arrière le temps du vol. L'édition du
    /// relâchement la caducise dès qu'une autre écriture touche le
    /// journal, le frais la remplace — état dérivé d'affichage,
    /// jamais sérialisé.
    surcharge_relachee: Option<(u64, u32, Length, Length)>,
    /// Aperçu de glissement : dernière résolution d'affichage du
    /// point saisi pendant le glissement (re-test du 2026-10-23) —
    /// la forme liée suit le geste en direct, la sémantique est
    /// exactement celle du relâchement ; l'échec est silencieux.
    /// Depuis le re-test du 24-10, l'aperçu tient AUSSI le vol du
    /// relâchement (purge seulement quand le frais est arrivé) :
    /// le dessin ne montre que des états résolus, jamais une
    /// position brute du curseur. État dérivé d'affichage, jamais
    /// sérialisé.
    croquis_apercu: Option<Result<CroquisResolu, String>>,
    /// Clé du dernier aperçu envoyé au fil de travail : point saisi
    /// et position visée en millimètres — la même position ne
    /// repart pas, un seul aperçu en vol.
    apercu_envoye: Option<(u32, f64, f64)>,
    /// Aperçu en vol au fil de travail.
    apercu_en_attente: bool,
    /// Balayage signé accumulé du geste de l'arc en cours (ADR-0020)
    /// : positif dans le sens trigonométrique, négatif dans le sens
    /// horaire — état de l'app UI, jamais un événement.
    balayage_arc: f64,
    /// Dernier angle du curseur autour du centre de l'arc en cours,
    /// pour accumuler le balayage au fil du mouvement (ADR-0020).
    angle_curseur_arc: f64,
    /// Identifiants des contraintes redondantes ou partiellement
    /// redondantes affichées (ADR-0018 D5, lot 3c-4) : le diagnostic
    /// du solveur est instable (FPA #106) — l'affichage exige le
    /// même diagnostic sur deux résolutions consécutives
    /// (hystérésis). État dérivé d'affichage, jamais sérialisé.
    redondances_affichees: Option<Vec<u32>>,
    /// Redondances de la résolution précédente, pour l'hystérésis
    /// (ADR-0018 D5, lot 3c-4) : jamais affichées telles quelles.
    redondances_precedentes: Option<Vec<u32>>,
    /// Valeur en millimètres de la cote de distance en cours de
    /// saisie (ADR-0018 D5, lot 3c-4) : état de l'app UI, jamais un
    /// événement — elle préremplit le dialogue de cote.
    cote_distance: f64,
    /// Dernière contrainte touchée et instant de pose (ADR-0021
    /// D2) : surbrillance momentanée dans la liste des contraintes
    /// — état d'affichage, jamais sérialisé.
    derniere_contrainte: Option<(u32, f64)>,
    /// Instant de la trame courante, en secondes egui monotones :
    /// sert à estomper la surbrillance momentanée.
    instant: f64,
    /// Génération des liaisons (ADR-0019 §6) : avance à chaque
    /// commande ou annulation qui touche le graphe des valeurs —
    /// les cotes pilotées se re-figent et la résolution repart
    /// même sans changement du croquis (checklist §8).
    generation_liaisons: u64,
    /// Dialogue « + Nouvel alias » ouvert (ADR-0019 §6) : état de
    /// l'app UI, jamais un événement tant que « Poser » n'est pas
    /// cliqué.
    dialogue_nouvel_alias: Option<DialogueNouvelAlias>,
    /// Dialogue de suppression d'un alias référencé (ADR-0019 §6) :
    /// cascade, remplacement ou annulation.
    dialogue_suppression: Option<DialogueSuppression>,
    /// Dialogue de cote de l'atelier esquisse : valeur fixe ou
    /// expression pilotée (ADR-0019 §6).
    dialogue_cote: Option<atelier_esquisse::DialogueCote>,
}

impl DemoApp {
    /// Nouveau démonstrateur, sur le catalogue français embarqué :
    /// session vierge posée, écran d'accueil affiché (ADR-0014) ; le
    /// pipeline wgpu du viewport est créé ici quand le rendu wgpu est
    /// actif (ADR-0015).
    fn new(i18n: i18n::Catalogue, render_state: Option<&RenderState>) -> Self {
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
    fn reinitialiser(&mut self) {
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
    fn effacer_messages(&mut self) {
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
    fn executer(&mut self, commande: &dyn Command) -> Result<(), String> {
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
    fn empreinte_journal(&self) -> (usize, bool, bool) {
        let journal = self.session.document().events().len();
        (journal, self.session.can_undo(), self.session.can_redo())
    }

    /// Annule le dernier groupe d'actions (bouton ou raccourci) : le
    /// tampon d'édition est rafraîchi et le travail non enregistré
    /// suit le journal (ADR-0016 C3).
    fn annuler(&mut self) {
        self.effacer_messages();
        if self.session.undo().is_ok() {
            self.dirty = true;
            self.raz_liaisons();
        }
    }

    /// Rétablit le groupe défait (bouton ou raccourci) : le tampon
    /// d'édition est rafraîchi et le travail non enregistré suit le
    /// journal (ADR-0016 C3).
    fn retablir(&mut self) {
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
    fn raz_liaisons(&mut self) {
        self.tampon_dimensions.clear();
        self.tampon_expressions.clear();
        self.generation_liaisons += 1;
    }

    /// Traduit une clé du catalogue embarqué.
    #[expect(clippy::expect_used)]
    fn texte(&self, cle: &str) -> String {
        self.i18n
            .traduction(cle)
            .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)")
    }

    /// Traduit une clé du catalogue embarqué, avec arguments.
    #[expect(clippy::expect_used)]
    fn texte_avec(&self, cle: &str, arguments: &[(&str, Argument)]) -> String {
        self.i18n
            .traduction_avec(cle, arguments)
            .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)")
    }

    /// Ouvre un projet depuis un chemin choisi (sélecteur natif ou
    /// bibliothèque) : rejeu du journal, mémorisation du chemin —
    /// état d'app UI, pas un événement (ADR-0014).
    fn ouvrir_projet(&mut self, chemin: &Path) {
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
    fn enregistrer_dans(&mut self, chemin: &Path) {
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
    fn enregistrer(&mut self) {
        if let Some(chemin) = self.chemin_projet.clone() {
            self.enregistrer_dans(&chemin);
            return;
        }
        self.enregistrer_sous();
    }

    /// « Enregistrer sous… » : dialogue natif, dossier de la
    /// bibliothèque proposé par défaut (ADR-0014).
    fn enregistrer_sous(&mut self) {
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
    fn demander(&mut self, action: ActionDestructrice) {
        if self.dirty {
            self.demande = Some(action);
        } else {
            self.executer_action(action);
        }
    }

    /// Exécute l'action destructive confirmée (ou immédiate quand
    /// rien n'est à perdre) : nouveau projet, ouverture — chemin
    /// reçu ou sélecteur natif — ou retour à l'atelier (ADR-0016 C3).
    fn executer_action(&mut self, action: ActionDestructrice) {
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
    fn demander_chemin(&self) -> Option<PathBuf> {
        let libelle = self.texte("filtre-projet");
        fichiers::choisir_projet(&libelle)
    }

    /// Fenêtre de confirmation d'une action destructive (ADR-0016 C3) :
    /// « Continuer » exécute l'action, « Annuler » la rejette — du
    /// travail non enregistré serait perdu. Fermeture demandée à la
    /// croix (audit n°3 A3-06) : « Enregistrer » sauve puis ferme,
    /// « Quitter sans enregistrer » ferme, « Annuler » ramène au
    /// travail.
    fn fenetre_confirmation(&mut self, ctx: &egui::Context) {
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
    fn fenetre_echelle(&mut self, ctx: &egui::Context) {
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
            self.importer_maillage(demande.octets, demande.triangles, echelle);
        }
    }

    /// Lit puis importe le fichier de tracés choisi dans le sélecteur
    /// natif : lecture et parse partent au fil de travail, le format
    /// suit le bouton (ADR-0014, ADR-0016 C1).
    fn charger_traces(&mut self, format: &'static str, chemin: &Path) {
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
    fn charger_maillage(&mut self, chemin: &Path) {
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
    fn envoyer_travail(&mut self, travail: Travail) {
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
    fn relancer_fil_de_travail(&mut self) {
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
    fn appliquer_retour(&mut self, retour: Retour) {
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
                octets,
                triangles,
                dimension,
                ..
            } => {
                // Un maillage minuscule ressemble à un export en
                // mètres : l'import attend le choix des unités
                // (ADR-0017) ; rien n'est écrit avant la réponse.
                if propose_echelle_metres(dimension) {
                    self.demande_echelle = Some(DemandeEchelle {
                        octets,
                        triangles,
                        dimension,
                    });
                } else {
                    self.importer_maillage(octets, triangles, 1.0);
                }
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
    fn demande_resolution_esquisse(&mut self) {
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
    fn demande_apercu_glissement(&mut self, point: u32, x: Length, y: Length) {
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
    fn purge_apercu_si_geste_fini(&mut self) {
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
    fn resolution_du_geste(&self) -> Option<&CroquisResolu> {
        if self.saisie.is_some() || atelier_esquisse::surcharge_du_relachement(self).is_some() {
            self.croquis_apercu
                .as_ref()
                .and_then(|resultat| resultat.as_ref().ok())
        } else {
            None
        }
    }

    /// Prépare la scène du mode démo scripté (gouvernance
    /// 2026-10-19) : la galerie de captures lance l'application sous
    /// Xvfb avec le scénario donné — rien n'est ajouté au rendu, la
    /// scène est posée comme par un utilisateur.
    // Graine de démo (décision 4 de l'inventaire-expect) : les 8 expect
    // de preparer_demo sont admis jusqu'au découpage de main.rs
    // (preparation/decoupage-main-rs.md), où la graine redeviendra du
    // vrai code de démonstration testé.
    #[allow(clippy::expect_used)]
    fn preparer_demo(&mut self, scenario: &str) {
        if scenario == "accueil" {
            return;
        }
        self.ecran = Ecran::Atelier;
        if scenario == "atelier-3d" {
            self.vue = Vue::TroisD;
        }
        let traces = svg::import(SVG_DEMO, &svg::SvgBudgets::default()).expect("svg de démo");
        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
        self.executer(&ImportTraces {
            format: "svg".into(),
            octets: SVG_DEMO.to_vec(),
            traces: traces.len(),
            sommets,
        })
        .expect("tracés importés");
        self.importer_maillage(octets_stl_demo(), 4, 1.0);
        let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [
            (0.0, 0.0),
            (pas * 4.0, 0.0),
            (pas * 4.0, pas * 3.0),
            (0.0, pas * 3.0),
        ] {
            self.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("coin posé");
        }
        for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            self.executer(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("côté posé");
        }
        for (ligne, horizontale) in [(4, true), (5, false), (6, true), (7, false)] {
            let contrainte = if horizontale {
                ContrainteEsquisse::Horizontale { ligne }
            } else {
                ContrainteEsquisse::Verticale { ligne }
            };
            self.executer(&AddConstraint { contrainte })
                .expect("contrainte posée");
        }
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 4.0),
                },
            },
        })
        .expect("cote posée");
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 1,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 3.0),
                },
            },
        })
        .expect("cote posée");
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("coin fixé");
        self.demande_resolution_esquisse();
    }

    /// Importe le maillage à l'échelle donnée : commande tout-ou-rien,
    /// le succès n'est affiché que si elle passe (ADR-0016 C4) ; une
    /// demande d'échelle en attente est consommée (ADR-0017).
    fn importer_maillage(&mut self, octets: Vec<u8>, triangles: usize, echelle: f64) {
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
    fn exporter_traces(&mut self, format: &'static str, chemin: &Path) {
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
    fn exporter_maillage(&mut self, chemin: &Path) {
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
    /// Boîte englobante 2D de la scêne importée : union des tracés et
    /// du maillage projeté de dessus — une seule échelle pour tout
    /// (ADR-0014).
    fn boite_scene(traces: &[Path2], maillage: Option<&Mesh>) -> Option<Boite2> {
        let mut boite: Option<Boite2> = None;
        for trace in traces {
            for point in trace.points() {
                let x = point.x().millimeters();
                let y = point.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite2::du_point(x, y),
                });
            }
        }
        if let Some(maillage) = maillage {
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite2::du_point(x, y),
                });
            }
        }
        boite
    }

    /// Boîte de cadrage automatique de la caméra : celle de la scêne
    /// importée quand il y en a une — l'import recentre sur les objets,
    /// même éloignés du rectangle paramétrique — sinon celle du
    /// rectangle paramétrique (ADR-0015).
    fn boite_cadrage(width: f64, height: f64, scene: Option<Boite2>) -> Boite2 {
        match scene {
            Some(scene) => scene,
            None => Boite2::du_point(0.0, 0.0).etendre(width, height),
        }
    }

    /// Boîte englobante 3D de la scêne importée : union des tracés
    /// posés sur le plan z = 0 et des sommets du maillage en volume
    /// (ADR-0015, phase C2).
    fn boite_scene_3d(traces: &[Path2], maillage: Option<&Mesh>) -> Option<Boite3D> {
        let mut boite: Option<Boite3D> = None;
        for trace in traces {
            for point in trace.points() {
                let x = point.x().millimeters();
                let y = point.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y, 0.0),
                    None => Boite3D::du_point(x, y, 0.0),
                });
            }
        }
        if let Some(maillage) = maillage {
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                let z = sommet.z().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y, z),
                    None => Boite3D::du_point(x, y, z),
                });
            }
        }
        boite
    }

    /// Boîte de cadrage automatique de la caméra 3D : celle de la
    /// scêne importée quand il y en a une, sinon le rectangle
    /// paramétrique posé sur le plan z = 0 (ADR-0015, phase C2).
    fn boite_cadrage_3d(width: f64, height: f64, scene: Option<Boite3D>) -> Boite3D {
        match scene {
            Some(scene) => scene,
            None => Boite3D::du_point(0.0, 0.0, 0.0).etendre(width, height, 0.0),
        }
    }

    /// Ajoute à une scêne 2D le wireframe du maillage en projection
    /// de dessus : trois arêtes par triangle, cyan, un pixel —
    /// dessiné par le GPU (ADR-0015, phase C2), plafonné : au-delà
    /// d'un plafond d'arêtes, les triangles sont décimés (audit n°4
    /// A4-02). Le
    /// STL reste visible en vue 2D sans repasser par le painter
    /// (validation C2, round 5).
    fn ajouter_maillage_2d(scene: &mut Scene2D, maillage: &Mesh) {
        let triangles = maillage.triangles();
        // Décimation au-delà du plafond : un maillage démesuré ne
        // déborde plus des tampons GPU (audit n°4 A4-02).
        let pas = pas_decimation(triangles.len());
        let sommets = maillage.vertices();
        for triangle in triangles.iter().step_by(pas) {
            let mut points: Vec<(f64, f64)> = Vec::new();
            for index in triangle.iter() {
                let sommet = sommets[*index as usize];
                points.push((sommet.x().millimeters(), sommet.y().millimeters()));
            }
            let (a, b, c) = (points[0], points[1], points[2]);
            for (depart, arrivee) in [(a, b), (b, c), (c, a)] {
                scene.ajouter(Segment2D {
                    depart,
                    arrivee,
                    couleur: COULEUR_WIREFRAME,
                    largeur_px: 1.0,
                });
            }
        }
    }

    /// Scêne 3D du viewport : grille au sol, tracés posés sur le
    /// plan z = 0, contour du rectangle paramétrique, wireframe du
    /// maillage, donné déjà construit par l'appelant — cache à clé
    /// d'édition (ADR-0027, audit n°4 A4-02). L'ordre d'ajout est
    /// la priorité d'affichage : le
    /// pipeline de lignes ne teste pas la profondeur, le dernier
    /// segment dessiné recouvre les précédents — la grille passe
    /// sous les tracés et le rectangle, qui ne disparaissent plus
    /// quand ils coïncident avec elle (validation C2, round 2).
    fn scene_vue_3d(
        traces: &[Path2],
        wireframe: &[Segment3D],
        camera: &Camera3D,
        width: f64,
        height: f64,
    ) -> Scene3D {
        let echelle = camera.echelle_cible(height);
        let pas = pas_de_grille(echelle, GRILLE_CIBLE_PX);
        let mut scene = Scene3D::vide();
        // La grille au sol couvre la partie du sol visible dans le
        // champ de la caméra : elle atteint les bords de la vue, quel
        // que soit le cadrage du contenu (validation jalon 2 — elle
        // s'arrêtait auparavant au contenu).
        let sol = camera.sol_visible(width, height);
        let boite_grille = Boite3D::du_point(sol.min_x, sol.min_y, 0.0);
        let boite_grille = boite_grille.etendre(sol.max_x, sol.max_y, 0.0);
        scene.ajouter_grille(&boite_grille, pas, COULEUR_GRILLE);
        scene
            .segments
            .extend(Scene3D::depuis_traces(traces, COULEUR_TRACES).segments);
        // Contour du rectangle paramétrique, posé sur le plan z = 0
        // (ADR-0015, phase C2).
        scene.ajouter(Segment3D {
            depart: (0.0, 0.0, 0.0),
            arrivee: (width, 0.0, 0.0),
            couleur: COULEUR_RECTANGLE_3D,
        });
        scene.ajouter(Segment3D {
            depart: (width, 0.0, 0.0),
            arrivee: (width, height, 0.0),
            couleur: COULEUR_RECTANGLE_3D,
        });
        scene.ajouter(Segment3D {
            depart: (width, height, 0.0),
            arrivee: (0.0, height, 0.0),
            couleur: COULEUR_RECTANGLE_3D,
        });
        scene.ajouter(Segment3D {
            depart: (0.0, height, 0.0),
            arrivee: (0.0, 0.0, 0.0),
            couleur: COULEUR_RECTANGLE_3D,
        });
        scene.segments.extend(wireframe.iter().copied());
        scene
    }

    /// Engagement d'un champ de dimension pour cette frame, s'il y en a
    /// un : le DragValue écrit son tampon au relâchement du geste ou à
    /// la fin d'une édition clavier, et un engagement sans changement
    /// effectif n'écrit rien — un faux événement tronquerait les
    /// groupes défaits (ADR-0008). Les bugs des rounds 3 à 6 de la
    /// validation C1 viennent de là : dimensions figées au
    /// glissement, historique détruit au double annuler, valeur
    /// gelée pendant le geste.
    fn engagement_params(
        avant: f64,
        tampon: f64,
        dragged: bool,
        drag_stopped: bool,
        changed: bool,
    ) -> Option<f64> {
        if drag_stopped || (changed && !dragged) {
            return (tampon != avant).then_some(tampon);
        }
        None
    }

    /// Valeur affichée d'un champ de dimension : inactif, le tampon
    /// suit le graphe (undo/redo, ouverture) ; actif, il persiste —
    /// c'est lui qui garde la valeur glissée d'une frame à l'autre
    /// pendant le geste (validation C1 round 6).
    fn tampon_apres_inactivite(actif: bool, tampon: f64, graphe: Option<f64>) -> f64 {
        if actif { tampon } else { graphe.unwrap_or(0.0) }
    }

    /// Projette un point du monde à l'écran : la caméra inverse l'axe
    /// Y (convention SVG/DXF) et donne un décalage par rapport au
    /// centre du viewport (ADR-0015).
    fn point_ecran(camera: &Camera2D, rect: egui::Rect, x: f64, y: f64) -> egui::Pos2 {
        let (px, py) = camera.vers_ecran((x, y));
        egui::pos2(rect.center().x + px as f32, rect.center().y + py as f32)
    }

    /// Re-parse les tracés courants si le blob référencé a changé :
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0011) ;
    /// un échec est mémorisé, pas re-parsé à chaque frame
    /// (ADR-0016 C1).
    fn mettre_a_jour_cache(&mut self) {
        let Some(reference) = self.session.geom().traces() else {
            self.cache = None;
            return;
        };
        let (format, blob) = (reference.format.clone(), reference.blob.clone());
        if self.cache.as_ref().is_some_and(|(nom, _)| *nom == blob) {
            return;
        }
        let Some(octets) = self.session.document().blob(&blob) else {
            return;
        };
        let octets = octets.to_vec();
        let parse = move || {
            let parse = if format == "svg" {
                svg::import(&octets, &svg::SvgBudgets::default())
            } else {
                dxf::import(&octets, &dxf::DxfBudgets::default())
            };
            parse.map_err(|erreur| erreur.to_string())
        };
        met_en_cache(&mut self.cache, blob, parse);
        if let Some((_, Err(erreur))) = &self.cache {
            self.error = Some(erreur.clone());
        }
    }
    /// Re-parse le maillage courant si le blob référencé a changé :
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0014) ;
    /// un échec est mémorisé, pas re-parsé à chaque frame
    /// (ADR-0016 C1).
    fn mettre_a_jour_cache_maillage(&mut self) {
        let Some(reference) = self.session.geom().maillage() else {
            self.cache_maillage = None;
            return;
        };
        let blob = reference.blob.clone();
        if self
            .cache_maillage
            .as_ref()
            .is_some_and(|(nom, _)| *nom == blob)
        {
            return;
        }
        let Some(octets) = self.session.document().blob(&blob) else {
            return;
        };
        let echelle = reference.echelle;
        let octets = octets.to_vec();
        let parse = move || {
            let brut = stl::import(&octets, &stl::StlBudgets::default());
            let maillage = brut.map_err(|erreur| erreur.to_string())?;
            maillage_echelle(maillage, echelle)
        };
        met_en_cache(&mut self.cache_maillage, blob, parse);
        if let Some((_, Err(erreur))) = &self.cache_maillage {
            self.error = Some(erreur.clone());
        }
    }
    /// Reconstruit le wireframe 3D du maillage seulement si la clé
    /// d'édition a changé (ADR-0027, patron `camera_cle`, audit n°4
    /// A4-02) : le dédoublonnage O(triangles) n'est plus payé à
    /// chaque frame, le wireframe décimé vit en cache jusqu'au
    /// prochain import.
    fn rafraichir_wireframe(&mut self, cle: &Option<String>) {
        if self
            .wireframe_maillage
            .as_ref()
            .is_some_and(|(cle_cache, _)| cle_cache == cle)
        {
            return;
        }
        let maillage: Option<&Mesh> = match &self.cache_maillage {
            Some((_, Ok(maillage))) => Some(maillage),
            _ => None,
        };
        let segments = match maillage {
            Some(maillage) => Scene3D::depuis_maillage(maillage, COULEUR_WIREFRAME).segments,
            None => Vec::new(),
        };
        self.wireframe_maillage = Some((cle.clone(), segments));
    }
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.add_enabled(
            self.session.can_undo(),
            egui::Button::new(self.texte("top-annuler")),
        );
        avec_aide_sur_calque(ui, &reponse, "aide-top-annuler", &mut self.aide);
        if reponse.clicked() {
            self.annuler();
        }
        let reponse = ui.add_enabled(
            self.session.can_redo(),
            egui::Button::new(self.texte("top-retablir")),
        );
        avec_aide_sur_calque(ui, &reponse, "aide-top-retablir", &mut self.aide);
        if reponse.clicked() {
            self.retablir();
        }
        ui.separator();
        let reponse = ui.button(self.texte("top-importer-svg"));
        avec_aide(&reponse, "aide-top-importer-svg", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-svg");
            let Some(chemin) = fichiers::choisir_import(&libelle, &["svg", "svgz"]) else {
                return;
            };
            self.charger_traces("svg", &chemin);
        }
        let reponse = ui.button(self.texte("top-importer-dxf"));
        avec_aide(&reponse, "aide-top-importer-dxf", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-dxf");
            let Some(chemin) = fichiers::choisir_import(&libelle, &["dxf"]) else {
                return;
            };
            self.charger_traces("dxf", &chemin);
        }
        let reponse = ui.button(self.texte("top-importer-stl"));
        avec_aide(&reponse, "aide-top-importer-stl", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-stl");
            let Some(chemin) = fichiers::choisir_import(&libelle, &["stl"]) else {
                return;
            };
            self.charger_maillage(&chemin);
        }
        let reponse = ui.add_enabled(
            self.session.geom().traces().is_some(),
            egui::Button::new(self.texte("top-exporter-svg")),
        );
        avec_aide_sur_calque(ui, &reponse, "aide-top-exporter-svg", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-svg");
            let Some(chemin) = fichiers::chemin_export(&libelle, &["svg"]) else {
                return;
            };
            self.exporter_traces("svg", &chemin);
        }
        let reponse = ui.add_enabled(
            self.session.geom().traces().is_some(),
            egui::Button::new(self.texte("top-exporter-dxf")),
        );
        avec_aide_sur_calque(ui, &reponse, "aide-top-exporter-dxf", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-dxf");
            let Some(chemin) = fichiers::chemin_export(&libelle, &["dxf"]) else {
                return;
            };
            self.exporter_traces("dxf", &chemin);
        }
        let reponse = ui.add_enabled(
            self.session.geom().maillage().is_some(),
            egui::Button::new(self.texte("top-exporter-stl")),
        );
        avec_aide_sur_calque(ui, &reponse, "aide-top-exporter-stl", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-stl");
            let Some(chemin) = fichiers::chemin_export(&libelle, &["stl"]) else {
                return;
            };
            self.exporter_maillage(&chemin);
        }
        ui.separator();
        let reponse = ui.button(self.texte("top-recalculer"));
        avec_aide(&reponse, "aide-top-recalculer", &mut self.aide);
        if reponse.clicked() {
            self.effacer_messages();
            match self.session.graph_mut().evaluate_all() {
                Ok(()) => self.info = Some(self.texte("info-recalcul-ok")),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.button(self.texte("top-enregistrer"));
        avec_aide(&reponse, "aide-top-enregistrer", &mut self.aide);
        if reponse.clicked() {
            self.enregistrer();
        }
        let reponse = ui.button(self.texte("top-enregistrer-sous"));
        avec_aide(&reponse, "aide-top-enregistrer-sous", &mut self.aide);
        if reponse.clicked() {
            self.enregistrer_sous();
        }
        let reponse = ui.button(self.texte("top-ouvrir"));
        avec_aide(&reponse, "aide-top-ouvrir", &mut self.aide);
        if reponse.clicked() {
            self.demander(ActionDestructrice::Ouvrir { chemin: None });
        }
        let reponse = ui.button(self.texte("top-nouveau"));
        avec_aide(&reponse, "aide-top-nouveau", &mut self.aide);
        if reponse.clicked() {
            self.demander(ActionDestructrice::Nouveau);
        }
        let reponse = ui.button(self.texte("top-bibliotheque"));
        avec_aide(&reponse, "aide-top-bibliotheque", &mut self.aide);
        if reponse.clicked() {
            self.ecran = Ecran::Accueil;
        }
    }

    /// Ligne descriptive d'un projet de la bibliothèque : âge et
    /// taille, traduits (ADR-0010).
    fn decrire_entree(&self, projet: &EntreeProjet) -> String {
        let maintenant = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duree| duree.as_secs())
            .unwrap_or(0);
        let jours = maintenant.saturating_sub(projet.modifie_le) / 86_400;
        let date = match jours {
            0 => self.texte("accueil-date-aujourdhui"),
            1 => self.texte("accueil-date-hier"),
            _ => {
                let n = jours as usize;
                self.texte_avec("accueil-date-jours", &[("n", Argument::from(n))])
            }
        };
        let taille = if projet.taille < 1024 {
            let n = projet.taille as usize;
            self.texte_avec("accueil-taille-octets", &[("n", Argument::from(n))])
        } else if projet.taille < 1024 * 1024 {
            let n = (projet.taille / 1024) as usize;
            self.texte_avec("accueil-taille-ko", &[("n", Argument::from(n))])
        } else {
            let n = (projet.taille / (1024 * 1024)) as usize;
            self.texte_avec("accueil-taille-mio", &[("n", Argument::from(n))])
        };
        format!("{date} · {taille}")
    }

    /// Écran d'accueil : bibliothèque de projets (ADR-0014) — nouveau,
    /// ouverture, reprise, et liste du dossier.
    fn accueil(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("accueil-titre"));
        avec_aide(&reponse, "aide-accueil-titre", &mut self.aide);
        ui.add_space(8.0);
        let reponse = ui.button(self.texte("accueil-nouveau"));
        avec_aide(&reponse, "aide-accueil-nouveau", &mut self.aide);
        if reponse.clicked() {
            self.demander(ActionDestructrice::Nouveau);
        }
        let reponse = ui.button(self.texte("accueil-ouvrir"));
        avec_aide(&reponse, "aide-accueil-ouvrir", &mut self.aide);
        if reponse.clicked() {
            self.demander(ActionDestructrice::Ouvrir { chemin: None });
        }
        let en_cours = self.session.document().events().len() > EVENEMENTS_VIERGES
            || self.chemin_projet.is_some();
        if en_cours {
            let reponse = ui.button(self.texte("accueil-reprendre"));
            avec_aide(&reponse, "aide-accueil-reprendre", &mut self.aide);
            if reponse.clicked() {
                self.demander(ActionDestructrice::Reprendre);
            }
        }
        ui.add_space(12.0);
        let reponse = ui.heading(self.texte("accueil-liste-titre"));
        avec_aide(&reponse, "aide-accueil-liste", &mut self.aide);
        let Some(dossier) = bibliotheque::dossier() else {
            let texte = self.texte("accueil-indisponible");
            ui.label(egui::RichText::new(texte).color(egui::Color32::YELLOW));
            return;
        };
        let projets = bibliotheque::lister(&dossier);
        if projets.is_empty() {
            ui.label(self.texte("accueil-vide"));
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for projet in &projets {
                ui.horizontal(|ui| {
                    let details = self.decrire_entree(projet);
                    ui.strong(&projet.nom);
                    ui.small(&details);
                    let reponse = ui.button(self.texte("accueil-ouvrir-projet"));
                    avec_aide(&reponse, "aide-accueil-projet", &mut self.aide);
                    if reponse.clicked() {
                        let chemin = projet.chemin.clone();
                        self.demander(ActionDestructrice::Ouvrir {
                            chemin: Some(chemin),
                        });
                    }
                });
            }
        });
    }

    /// Panneau de liaisons (ADR-0019 §6) : le panneau des paramètres
    /// devient la liste des alias du graphe — nom en gras si calculé,
    /// DragValue pour une entrée, champ texte pour une expression,
    /// unité déclarée, erreur du registre en clair — plus la création
    /// d'alias et la suppression. Les cotes système (`esquisse.*`)
    /// sont en lecture seule, jamais supprimables d'ici.
    fn liaisons_panel(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("liaisons-titre"));
        avec_aide(&reponse, "aide-liaisons-titre", &mut self.aide);
        let reponse = ui.button(self.texte("liaisons-nouvel-alias"));
        avec_aide(&reponse, "aide-liaisons-nouvel-alias", &mut self.aide);
        if reponse.clicked() {
            self.dialogue_nouvel_alias = Some(DialogueNouvelAlias {
                nom: String::new(),
                expression: false,
                valeur: 0.0,
                texte: String::new(),
                unite: None,
            });
        }
        ui.separator();
        let libelle_unite_aucune = self.texte("liaisons-unite-aucune");
        let libelle_supprimer = self.texte("liaisons-supprimer");
        // Actions collectées pendant le rendu, exécutées après : le
        // graphe se relit entre deux lignes sans double emprunt.
        let mut entree: Option<(String, f64)> = None;
        let mut expression_modifiee: Option<String> = None;
        let mut unite_modifiee: Option<(String, Option<Unite>)> = None;
        let mut suppression: Option<String> = None;
        let noms = self.session.graph().names();
        for nom in &noms {
            let Some(info) = self.session.graph().noeud(nom) else {
                continue;
            };
            let valeur = self.session.graph_mut().value(nom);
            let valeur_initiale = valeur.as_ref().ok().copied();
            let demande = &mut self.aide;
            ui.horizontal(|ui| {
                if info.expression.is_some() {
                    ui.label(egui::RichText::new(nom.as_str()).strong());
                } else {
                    ui.label(nom.as_str());
                }
                if info.systeme {
                    // Cote système : valeur calculée en lecture seule,
                    // ni édition ni suppression (ADR-0019 §6).
                    if let Ok(calcul) = &valeur {
                        let texte = Length::from_millimeters(*calcul).to_string();
                        ui.strong(texte.as_str());
                    }
                    return;
                }
                if info.expression.is_some() {
                    let (memoire, actif_precedent) = self
                        .tampon_expressions
                        .remove(nom)
                        .unwrap_or((String::new(), false));
                    let courante = info.expression.clone().unwrap_or_default();
                    let mut tampon = if actif_precedent {
                        memoire
                    } else {
                        courante.clone()
                    };
                    let mut actif = false;
                    let reponse =
                        ui.add(egui::TextEdit::singleline(&mut tampon).desired_width(120.0));
                    avec_aide(&reponse, "aide-liaisons-expression", demande);
                    if reponse.has_focus() {
                        actif = true;
                    }
                    // Engagement seulement si le texte a changé : un
                    // faux événement tronquerait les groupes défaits
                    // (ADR-0008, validation C1 round 6).
                    if reponse.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && tampon != courante
                    {
                        expression_modifiee = Some(nom.clone());
                    }
                    if let Ok(calcul) = &valeur {
                        ui.small(format!("{calcul}"));
                    }
                    self.tampon_expressions.insert(nom.clone(), (tampon, actif));
                } else {
                    // Valeur du graphe en début de frame : un
                    // engagement sans changement effectif n'écrit
                    // rien au journal — un faux événement tronquerait
                    // les groupes défaits (ADR-0008).
                    let avant = valeur_initiale.unwrap_or(0.0);
                    // Tampon d'édition persistant du champ : le
                    // DragValue affiche la valeur du début de frame,
                    // un rechargement du graphe à chaque frame
                    // figeait le chiffre pendant le geste (validation
                    // C1 round 6).
                    let (memoire, actif_precedent) =
                        self.tampon_dimensions.remove(nom).unwrap_or((0.0, false));
                    let mut tampon =
                        Self::tampon_apres_inactivite(actif_precedent, memoire, valeur_initiale);
                    let mut actif = false;
                    let reponse = ui.add(egui::DragValue::new(&mut tampon).speed(1.0));
                    avec_aide(&reponse, "aide-liaisons-valeur", demande);
                    // Le clic ouvre le mode édition clavier, qui
                    // garde le focus indéfiniment : un champ focusé ne
                    // réagit plus au glissement. Un appui-glissement
                    // rend le focus pour que le geste déplace la
                    // valeur (piège du glissement, validation C1
                    // round 4).
                    let presse_glisse =
                        ui.input(|i| i.pointer.is_decidedly_dragging() && i.pointer.primary_down());
                    if presse_glisse && reponse.contains_pointer() {
                        ui.memory_mut(|m| m.surrender_focus(reponse.id));
                    }
                    if reponse.dragged() || reponse.has_focus() {
                        actif = true;
                    }
                    if let Some(valeur_engagee) = Self::engagement_params(
                        avant,
                        tampon,
                        reponse.dragged(),
                        reponse.drag_stopped(),
                        reponse.changed(),
                    ) {
                        entree = Some((nom.clone(), valeur_engagee));
                    }
                    self.tampon_dimensions.insert(nom.clone(), (tampon, actif));
                }
                // Unité déclarée (ADR-0019 §6) : sans unité, mm ou
                // deg, redéclarée en réémettant la définition — la
                // cohérence est revérifiée à la définition
                // (OF-VAL-0007).
                let mut choix = info.unite;
                let reponse = egui::ComboBox::from_id_salt(format!("unite-{nom}"))
                    .selected_text(Self::texte_unite(choix, &libelle_unite_aucune))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut choix, None, &libelle_unite_aucune);
                        ui.selectable_value(&mut choix, Some(Unite::Millimetre), "mm");
                        ui.selectable_value(&mut choix, Some(Unite::Degre), "deg");
                    })
                    .response;
                avec_aide(&reponse, "aide-liaisons-unite", demande);
                if choix != info.unite {
                    unite_modifiee = Some((nom.clone(), choix));
                }
                let reponse = ui.small_button(libelle_supprimer.as_str());
                avec_aide(&reponse, "aide-liaisons-supprimer", demande);
                if reponse.clicked() {
                    suppression = Some(nom.clone());
                }
            });
            if let Err(erreur) = &valeur {
                // Badge d'erreur en clair à la ligne concernée : le
                // message du registre, clic droit → page d'aide du
                // code (ADR-0019 §6).
                let message = erreur.to_string();
                let reponse = ui.label(
                    egui::RichText::new(message.as_str())
                        .color(egui::Color32::RED)
                        .small(),
                );
                avec_aide(&reponse, &aide_du_code(&message), &mut self.aide);
            }
        }
        if let Some((nom, valeur_engagee)) = entree {
            let unite = self.session.graph().noeud(&nom).and_then(|info| info.unite);
            let _ = self.executer(&SetInput {
                name: nom,
                value: valeur_engagee,
                unite,
            });
        }
        if let Some(nom) = expression_modifiee
            && let Some(info) = self.session.graph().noeud(&nom)
            && let Some((texte, _)) = self.tampon_expressions.get(&nom)
        {
            let _ = self.executer(&SetExpression {
                name: nom,
                expression: texte.clone(),
                unite: info.unite,
            });
        }
        if let Some((nom, unite)) = unite_modifiee {
            // L'unité se redéclare en réémettant la définition : la
            // cohérence est revérifiée à la définition
            // (OF-VAL-0007, ADR-0019 §6).
            if let Some(info) = self.session.graph().noeud(&nom) {
                if let Some(texte) = &info.expression {
                    let _ = self.executer(&SetExpression {
                        name: nom.clone(),
                        expression: texte.clone(),
                        unite,
                    });
                } else {
                    let valeur = self.session.graph_mut().value(&nom).unwrap_or(0.0);
                    let _ = self.executer(&SetInput {
                        name: nom.clone(),
                        value: valeur,
                        unite,
                    });
                }
            }
        }
        if let Some(nom) = suppression {
            let references = self.session.graph().dependants(&nom).unwrap_or_default();
            if references.is_empty() {
                // Alias libre : suppression directe (ADR-0019 §6).
                let _ = self.executer(&RemoveAlias {
                    name: nom,
                    mode: ModeSuppression::Simple,
                });
            } else {
                // Alias référencé : le dialogue propose cascade,
                // remplacement ou annulation (amendement 2026-10-20).
                self.dialogue_suppression = Some(DialogueSuppression {
                    alias: nom,
                    references,
                    remplacement: String::new(),
                });
            }
        }
        // Outils du mode esquisse (vue 2D) : le croquis contraint se
        // pose dans le viewport (ADR-0018, lot 3c).
        if self.vue == Vue::DeuxD {
            ui.separator();
            atelier_esquisse::panneau(ui, self);
        }
    }

    /// Texte affiché du sélecteur d'unité : « sans unité », ou le
    /// code de l'unité (ADR-0019 §6).
    fn texte_unite(unite: Option<Unite>, sans: &str) -> String {
        match unite {
            None => sans.to_string(),
            Some(u) => u.code().to_string(),
        }
    }

    fn journal_panel(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("journal-titre"));
        avec_aide(&reponse, "aide-journal-titre", &mut self.aide);
        ui.label(self.texte_avec(
            "journal-compteur",
            &[("n", Argument::from(self.session.document().events().len()))],
        ));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for event in self.session.document().events().iter().rev().take(200) {
                ui.label(format!(
                    "#{}  {}  v{}",
                    event.seq, event.event_type, event.schema_version
                ));
                ui.small(format!("{} · {}", event.timestamp, event.actor));
            }
        });
        let inconnus = self.session.unknown_types();
        if !inconnus.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(self.texte("journal-types-ignores"))
                    .color(egui::Color32::YELLOW),
            );
            for event_type in &inconnus {
                ui.small(event_type.clone());
            }
        }
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("viewport-titre"));
        avec_aide(&reponse, "aide-viewport-titre", &mut self.aide);
        if let Some(error) = &self.error {
            let texte = self.texte_avec(
                "viewport-erreur",
                &[("texte", Argument::from(error.as_str()))],
            );
            ui.label(egui::RichText::new(texte).color(egui::Color32::RED));
        }
        if let Some(info) = &self.info {
            ui.label(egui::RichText::new(info.clone()).color(egui::Color32::LIGHT_GREEN));
        }
        ui.separator();
        // Bascule des vues du viewport : 2D (plan) ou 3D (orbite) —
        // phase C2 (ADR-0015).
        ui.horizontal(|ui| {
            let reponse =
                ui.selectable_label(self.vue == Vue::DeuxD, self.texte("viewport-vue-2d"));
            avec_aide(&reponse, "aide-viewport-vue-2d", &mut self.aide);
            if reponse.clicked() {
                self.vue = Vue::DeuxD;
            }
            let reponse =
                ui.selectable_label(self.vue == Vue::TroisD, self.texte("viewport-vue-3d"));
            avec_aide(&reponse, "aide-viewport-vue-3d", &mut self.aide);
            if reponse.clicked() {
                self.vue = Vue::TroisD;
            }
        });
        self.mettre_a_jour_cache();
        self.mettre_a_jour_cache_maillage();
        let width = self
            .session
            .graph_mut()
            .value("largeur")
            .unwrap_or(1.0)
            .max(1.0);
        let height = self
            .session
            .graph_mut()
            .value("hauteur")
            .unwrap_or(1.0)
            .max(1.0);
        // Une ligne est réservée sous le viewport pour chaque compteur.
        let mut lignes_reservees = 0.0;
        if self.session.geom().traces().is_some() {
            lignes_reservees += 24.0;
        }
        if self.session.geom().maillage().is_some() {
            lignes_reservees += 24.0;
        }
        let hauteur = (ui.available_height() - lignes_reservees).max(100.0);
        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width(), hauteur),
            egui::Sense::click_and_drag(),
        );
        avec_aide(&response, "aide-viewport-camera", &mut self.aide);
        // Mode esquisse (vue 2D) : un clic poursuit le geste de
        // l'outil actif à l'endroit visé — les commandes passent par
        // la session, la résolution part au fil de travail
        // (ADR-0018 D3). Le clic s'exécute avant l'emprunt des
        // tracés importés, qui vit jusqu'au dessin des scènes.
        if response.clicked()
            && self.vue == Vue::DeuxD
            && let Some(curseur) = response.interact_pointer_pos()
        {
            let decalage = curseur - response.rect.center();
            let (x, y) = self
                .camera
                .depuis_ecran((decalage.x as f64, decalage.y as f64));
            let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
            atelier_esquisse::traiter_clic(self, pas, x, y);
        }
        // Glissement d'une poignée du croquis (ADR-0018 D4/D5, lot
        // 3c-3) : en vue 2D avec l'outil Sélection, un glissement
        // qui démarre sur un point le saisit — l'aperçu local le
        // suit sans écrire au journal, le relâchement pose sa
        // graine par `MovePoint`, la résolution repart au fil de
        // travail.
        if self.vue == Vue::DeuxD
            && self.outil == atelier_esquisse::OutilEsquisse::Selection
            && let Some(curseur) = response.interact_pointer_pos()
        {
            let decalage = curseur - response.rect.center();
            let (x, y) = self
                .camera
                .depuis_ecran((decalage.x as f64, decalage.y as f64));
            let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
            if response.drag_started() {
                atelier_esquisse::saisir_poignee(self, pas, x, y);
            }
            if response.drag_stopped() {
                atelier_esquisse::relacher_poignee(self, pas, x, y);
            }
        }
        // Références des blobs tracés et maillage : la caméra se
        // réajuste quand l'une d'elles change (import, undo/redo,
        // ouverture — ADR-0015).
        let cle_traces = self
            .session
            .geom()
            .traces()
            .map(|reference| reference.blob.clone());
        let cle_maillage = self
            .session
            .geom()
            .maillage()
            .map(|reference| reference.blob.clone());
        let cle = (cle_traces, cle_maillage);
        // Wireframe 3D : reconstruit seulement si la clé du maillage
        // a changé, jamais à chaque frame (audit n°4 A4-02).
        self.rafraichir_wireframe(&cle.1);
        let traces: &[Path2] = match &self.cache {
            Some((_, Ok(traces))) => traces,
            _ => &[],
        };
        let maillage: Option<&Mesh> = match &self.cache_maillage {
            Some((_, Ok(maillage))) => Some(maillage),
            _ => None,
        };
        let rect = response.rect;
        if self.camera_cle.as_ref() != Some(&cle) {
            // Le cadrage vise la scêne importée quand il y en a une,
            // le rectangle paramétrique sinon : un import éloigné de
            // l'origine reste centré sur ses objets (ADR-0015).
            let scene = Self::boite_scene(traces, maillage);
            let boite = Self::boite_cadrage(width, height, scene);
            self.camera
                .ajuster(rect.width() as f64, rect.height() as f64, boite, 20.0);
            let scene3d = Self::boite_scene_3d(traces, maillage);
            let boite3d = Self::boite_cadrage_3d(width, height, scene3d);
            self.camera3d.ajuster(boite3d);
            self.camera_cle = Some(cle);
        }
        // Vue 2D : la molette zoome au curseur, un glissement déplace
        // la vue. Vue 3D : la molette zoome, un glissement fait orbiter
        // la caméra (ADR-0015, phase C2).
        if response.hovered() {
            let molette: f64 = response
                .ctx
                .input(|i| i.raw_scroll_delta.y as f64)
                .clamp(-600.0, 600.0);
            if molette.abs() > 1e-3 {
                match self.vue {
                    Vue::DeuxD => {
                        let position =
                            response.hover_pos().unwrap_or(rect.center()) - rect.center();
                        self.camera.zoom_au_curseur(
                            (position.x as f64, position.y as f64),
                            (molette / 600.0).exp(),
                        );
                    }
                    Vue::TroisD => self.camera3d.zoomer((molette / 600.0).exp()),
                }
            }
        }
        // Le pan cède à la poignée saisie : seul le glissement du
        // fond déplace la vue (ADR-0018 D4, lot 3c-3).
        if response.dragged() && self.saisie.is_none() {
            let delta = response.drag_delta();
            match self.vue {
                Vue::DeuxD => self.camera.deplacer((delta.x as f64, delta.y as f64)),
                Vue::TroisD => self.camera3d.orbiter((delta.x as f64, delta.y as f64)),
            }
        }
        // Vue 2D : rectangle paramétrique par le painter (remplissage
        // et contour, une seule entité), grille, tracés et wireframe
        // du maillage en projection de dessus par le GPU. Vue 3D :
        // rectangle, tracés et grille au sol posés sur le plan z = 0,
        // wireframe du maillage en volume — arêtes dédupliquées, sans
        // plafond (ADR-0015, phase C2).
        match self.vue {
            Vue::DeuxD => {
                let coin = Self::point_ecran(&self.camera, rect, 0.0, 0.0);
                let oppose = Self::point_ecran(&self.camera, rect, width, height);
                let shape = egui::Rect::from_two_pos(coin, oppose);
                // Priorité d'affichage = ordre de dessin (leçon du
                // round 2 de la validation C2) : la grille GPU au
                // fond, puis le carré bleu — une seule entité dont
                // le contour est rendu juste sur son remplissage
                // (round 5 : les tracés importés ne s'intercalent
                // pas entre les deux) — puis les tracés et le
                // wireframe du maillage par-dessus, la cote en
                // dernier pour rester lisible. Sans wgpu, la
                // scêne 2D n'est pas dessinée et le viewport le
                // dit (ADR-0015).
                let feuille = egui::Color32::from_rgb(38, 96, 152);
                let contour = egui::Stroke::new(1.5_f32, egui::Color32::WHITE);
                if self.viewport_wgpu {
                    let mut scene_grille = Scene2D::vide();
                    let visible = self
                        .camera
                        .boite_visible(rect.width() as f64, rect.height() as f64);
                    scene_grille.ajouter_grille(
                        visible,
                        self.camera.echelle,
                        GRILLE_CIBLE_PX,
                        COULEUR_GRILLE_2D,
                    );
                    dessiner_scene(&painter, rect, scene_grille, self.camera);
                    painter.rect_filled(shape, 4.0, feuille);
                    painter.rect_stroke(shape, 4.0, contour);
                    let mut scene_haut =
                        Scene2D::depuis_traces(traces, COULEUR_TRACES, LARGEUR_TRACES_PX);
                    if let Some(maillage) = maillage {
                        Self::ajouter_maillage_2d(&mut scene_haut, maillage);
                    }
                    dessiner_scene(&painter, rect, scene_haut, self.camera);
                } else {
                    painter.rect_filled(shape, 4.0, feuille);
                    painter.rect_stroke(shape, 4.0, contour);
                    ui.label(
                        egui::RichText::new(self.texte("viewport-wgpu-indisponible"))
                            .color(egui::Color32::YELLOW),
                    );
                }
                // Entités du croquis contraint (ADR-0018, lot 3c) :
                // positions résolues du cache si fraîches, graines du
                // journal sinon — par-dessus la scène, sous la cote.
                // Geste de poignée (re-test du 24-10) : pendant le
                // glissement, la position brute du curseur ne
                // s'affiche plus — le dessin ne montre que des états
                // résolus, une ligne contrainte horizontalement ou
                // verticalement ne devient jamais une diagonale, même
                // le temps d'un aller-retour de résolution. La
                // surcharge ne survit qu'au vol du relâchement
                // (anti-flash §7).
                let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
                let curseur = response.interact_pointer_pos().map(|position| {
                    let decalage = position - response.rect.center();
                    self.camera
                        .depuis_ecran((decalage.x as f64, decalage.y as f64))
                });
                let (surcharge_drag, cible_du_geste) =
                    atelier_esquisse::affichage_du_geste(self, pas, curseur);
                // La forme liée suit le point saisi pendant le
                // glissement (re-test du 2026-10-23) : la
                // résolution d'aperçu part à la position visée
                // — sémantique du relâchement, affichage seul.
                if let Some((id, vise_x, vise_y)) = cible_du_geste {
                    self.demande_apercu_glissement(id, vise_x, vise_y);
                }
                // Résolution d'affichage du geste (re-test du
                // 24-10) : l'aperçu tient le glissement ET le vol
                // du relâchement — purgé quand les deux sont finis.
                self.purge_apercu_si_geste_fini();
                let resolu_apercu = self.resolution_du_geste();
                atelier_esquisse::dessiner(&painter, self, rect, surcharge_drag, resolu_apercu);
                // Aperçu du geste en cours (ADR-0018, lot 3c-2) : le
                // point visé suit le curseur, accroché à la grille
                // et aux points existants — état dérivé d'affichage,
                // jamais écrit au journal.
                if self.outil != atelier_esquisse::OutilEsquisse::Selection
                    && let Some(curseur) = response.hover_pos()
                {
                    let decalage = curseur - response.rect.center();
                    let (x, y) = self
                        .camera
                        .depuis_ecran((decalage.x as f64, decalage.y as f64));
                    // Le geste de l'arc suit le curseur (ADR-0020) :
                    // le balayage signé s'accumule avant l'aperçu.
                    atelier_esquisse::suivre_geste_arc(self, x, y);
                    atelier_esquisse::dessiner_apercu(&painter, self, rect, x, y);
                }
                // Cote du rectangle par le painter, après les
                // callbacks GPU : elle reste lisible par-dessus les
                // tracés (validation C2, round 5).
                painter.text(
                    shape.left_bottom() + egui::vec2(0.0, 16.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "{} × {}",
                        Length::from_millimeters(width),
                        Length::from_millimeters(height)
                    ),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            }
            Vue::TroisD => {
                if self.viewport_wgpu {
                    let wireframe: &[Segment3D] = match &self.wireframe_maillage {
                        Some((_, segments)) => segments,
                        None => &[],
                    };
                    let scene =
                        Self::scene_vue_3d(traces, wireframe, &self.camera3d, width, height);
                    dessiner_wireframe(&painter, rect, scene, self.camera3d);
                } else {
                    ui.label(
                        egui::RichText::new(self.texte("viewport-wgpu-indisponible"))
                            .color(egui::Color32::YELLOW),
                    );
                }
            }
        }
        // Compteurs de géométrie importée, sous le viewport (ADR-0011).
        if let Some(traces) = self.session.geom().traces() {
            let libelle = self.texte_avec(
                "geom-traces-compteur",
                &[
                    ("traces", Argument::from(traces.traces)),
                    ("sommets", Argument::from(traces.sommets)),
                ],
            );
            let reponse = ui.label(libelle);
            avec_aide(&reponse, "aide-geom-compteur", &mut self.aide);
        }
        if let Some(maillage) = self.session.geom().maillage() {
            let libelle = self.texte_avec(
                "geom-maillage-compteur",
                &[("triangles", Argument::from(maillage.triangles))],
            );
            let reponse = ui.label(libelle);
            avec_aide(&reponse, "aide-geom-compteur", &mut self.aide);
        }
    }
}

impl DemoApp {
    /// Fenêtres de dialogue des liaisons (ADR-0019 §6) : création
    /// d'alias et suppression d'un alias référencé — un refus laisse
    /// le dialogue ouvert, la saisie reste corrigeable
    /// (ADR-0016 C4).
    fn fenetre_dialogues_liaisons(&mut self, ctx: &egui::Context) {
        if let Some(mut dialogue) = self.dialogue_nouvel_alias.take() {
            let mut visible = true;
            let mut poser = false;
            let mut fermer = false;
            // Textes traduits avant la fenêtre : la fermeture
            // n'emprunte plus l'app (ADR-0010).
            let titre = self.texte("liaisons-nouveau-titre");
            let libelle_nom = self.texte("liaisons-nouveau-nom");
            let libelle_expression = self.texte("liaisons-nouveau-expression");
            let libelle_texte = self.texte("liaisons-nouveau-texte");
            let libelle_valeur = self.texte("liaisons-nouveau-valeur");
            let libelle_unite = self.texte("liaisons-nouveau-unite");
            let libelle_poser = self.texte("liaisons-nouveau-poser");
            let libelle_annuler = self.texte("liaisons-nouveau-annuler");
            let libelle_unite_aucune = self.texte("liaisons-unite-aucune");
            let demande = &mut self.aide;
            egui::Window::new(titre)
                .open(&mut visible)
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.label(&libelle_nom);
                    let reponse = ui.text_edit_singleline(&mut dialogue.nom);
                    avec_aide(&reponse, "aide-liaisons-nouveau-nom", demande);
                    let reponse =
                        ui.checkbox(&mut dialogue.expression, libelle_expression.as_str());
                    avec_aide(&reponse, "aide-liaisons-nouveau-expression", demande);
                    if dialogue.expression {
                        ui.label(&libelle_texte);
                        let reponse = ui.text_edit_singleline(&mut dialogue.texte);
                        avec_aide(&reponse, "aide-liaisons-nouveau-texte", demande);
                    } else {
                        ui.label(&libelle_valeur);
                        let reponse = ui.add(egui::DragValue::new(&mut dialogue.valeur).speed(1.0));
                        avec_aide(&reponse, "aide-liaisons-nouveau-valeur", demande);
                    }
                    ui.label(&libelle_unite);
                    let mut choix = dialogue.unite;
                    let reponse = egui::ComboBox::from_id_salt("unite-nouvel-alias")
                        .selected_text(Self::texte_unite(choix, &libelle_unite_aucune))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut choix, None, &libelle_unite_aucune);
                            ui.selectable_value(&mut choix, Some(Unite::Millimetre), "mm");
                            ui.selectable_value(&mut choix, Some(Unite::Degre), "deg");
                        })
                        .response;
                    avec_aide(&reponse, "aide-liaisons-nouveau-unite", demande);
                    dialogue.unite = choix;
                    ui.horizontal(|ui| {
                        let reponse = ui.button(&libelle_poser);
                        avec_aide(&reponse, "aide-liaisons-nouveau-poser", demande);
                        if reponse.clicked() {
                            poser = true;
                        }
                        let reponse = ui.button(&libelle_annuler);
                        avec_aide(&reponse, "aide-liaisons-nouveau-annuler", demande);
                        if reponse.clicked() {
                            fermer = true;
                        }
                    });
                });
            if visible && !fermer {
                if poser {
                    let resultat = if dialogue.expression {
                        self.executer(&SetExpression {
                            name: dialogue.nom.clone(),
                            expression: dialogue.texte.clone(),
                            unite: dialogue.unite,
                        })
                    } else {
                        self.executer(&SetInput {
                            name: dialogue.nom.clone(),
                            value: dialogue.valeur,
                            unite: dialogue.unite,
                        })
                    };
                    // Un refus laisse le dialogue ouvert : la saisie
                    // reste corrigeable (ADR-0016 C4).
                    if resultat.is_err() {
                        self.dialogue_nouvel_alias = Some(dialogue);
                    }
                } else {
                    self.dialogue_nouvel_alias = Some(dialogue);
                }
            }
        }
        if let Some(mut dialogue) = self.dialogue_suppression.take() {
            let mut visible = true;
            let mut cascade = false;
            let mut remplacer = false;
            let mut fermer = false;
            let titre = self.texte("liaisons-suppression-titre");
            let n = dialogue.references.len();
            let libelle_texte =
                self.texte_avec("liaisons-suppression-texte", &[("n", Argument::from(n))]);
            let libelle_cascade = self.texte_avec(
                "liaisons-suppression-cascade",
                &[("n", Argument::from(dialogue.references.len()))],
            );
            let libelle_remplacer = self.texte("liaisons-suppression-remplacer");
            let libelle_bouton_remplacer = self.texte("liaisons-suppression-remplacer-bouton");
            let libelle_annuler = self.texte("liaisons-suppression-annuler");
            let libelle_aucun = self.texte("liaisons-suppression-aucun-candidat");
            let candidats = self
                .session
                .graph()
                .names()
                .into_iter()
                .filter(|candidat| {
                    candidat != &dialogue.alias
                        && self
                            .session
                            .graph()
                            .noeud(candidat)
                            .is_some_and(|info| !info.systeme)
                })
                .collect::<Vec<_>>();
            let cascade_possible = !dialogue.references.iter().any(|reference| {
                self.session
                    .graph()
                    .noeud(reference)
                    .is_some_and(|info| info.systeme)
            });
            let demande = &mut self.aide;
            egui::Window::new(titre)
                .open(&mut visible)
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.label(&libelle_texte);
                    for reference in &dialogue.references {
                        ui.label(format!("- {reference}"));
                    }
                    let reponse =
                        ui.add_enabled(cascade_possible, egui::Button::new(&libelle_cascade));
                    avec_aide(&reponse, "aide-liaisons-suppression-cascade", demande);
                    if reponse.clicked() {
                        cascade = true;
                    }
                    ui.separator();
                    ui.label(&libelle_remplacer);
                    if candidats.is_empty() {
                        ui.label(&libelle_aucun);
                    } else {
                        let mut choix = if candidats.contains(&dialogue.remplacement) {
                            dialogue.remplacement.clone()
                        } else {
                            candidats[0].clone()
                        };
                        let reponse = egui::ComboBox::from_id_salt("remplacement-alias")
                            .selected_text(choix.clone())
                            .show_ui(ui, |ui| {
                                for candidat in &candidats {
                                    ui.selectable_value(&mut choix, candidat.clone(), candidat);
                                }
                            })
                            .response;
                        avec_aide(&reponse, "aide-liaisons-suppression-remplacer", demande);
                        dialogue.remplacement = choix;
                        let reponse = ui.button(&libelle_bouton_remplacer);
                        avec_aide(
                            &reponse,
                            "aide-liaisons-suppression-remplacer-bouton",
                            demande,
                        );
                        if reponse.clicked() {
                            remplacer = true;
                        }
                    }
                    let reponse = ui.button(&libelle_annuler);
                    avec_aide(&reponse, "aide-liaisons-suppression-annuler", demande);
                    if reponse.clicked() {
                        fermer = true;
                    }
                });
            if visible && !fermer {
                let mode = if cascade {
                    Some(ModeSuppression::Cascade)
                } else if remplacer {
                    Some(ModeSuppression::Remplacer {
                        par: dialogue.remplacement.clone(),
                    })
                } else {
                    None
                };
                if let Some(mode) = mode {
                    let resultat = self.executer(&RemoveAlias {
                        name: dialogue.alias.clone(),
                        mode,
                    });
                    if resultat.is_err() {
                        self.dialogue_suppression = Some(dialogue);
                    }
                } else {
                    self.dialogue_suppression = Some(dialogue);
                }
            }
        }
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

/// SVG de démonstration de la galerie de captures : sans
/// `width`/`height`, usvg le lit à 25,4 dpi — une unité vaut
/// exactement 1 mm (gouvernance 2026-10-19).
const SVG_DEMO: &[u8] = b"<?xml version=\"1.0\"?>
<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\">
  <circle cx=\"30\" cy=\"40\" r=\"20\" fill=\"none\"/>
  <rect x=\"55\" y=\"10\" width=\"40\" height=\"30\" fill=\"none\"/>
  <path d=\"M10,80 C40,60 60,95 90,70\" fill=\"none\"/>
</svg>
";

/// Maillage de démonstration de la galerie de captures : un
/// tétraèdre STL binaire, quatre triangles écrits octet par
/// octet en petit-boutiste (gouvernance 2026-10-19).
fn octets_stl_demo() -> Vec<u8> {
    let sommets = [
        (0.0f32, 0.0f32, 0.0f32),
        (30.0, 0.0, 0.0),
        (0.0, 40.0, 0.0),
        (0.0, 0.0, 25.0),
    ];
    let faces = [(0, 1, 2), (0, 1, 3), (1, 2, 3), (0, 2, 3)];
    let mut octets = vec![0u8; 80];
    octets.extend_from_slice(&4u32.to_le_bytes());
    let normale_nulle = 0.0f32.to_le_bytes();
    let attribut_nul = 0u16.to_le_bytes();
    for (a, b, c) in faces {
        octets.extend_from_slice(&normale_nulle);
        octets.extend_from_slice(&normale_nulle);
        octets.extend_from_slice(&normale_nulle);
        for indice in [a, b, c] {
            let (x, y, z) = sommets[indice];
            octets.extend_from_slice(&x.to_le_bytes());
            octets.extend_from_slice(&y.to_le_bytes());
            octets.extend_from_slice(&z.to_le_bytes());
        }
        octets.extend_from_slice(&attribut_nul);
    }
    octets
}

#[expect(clippy::expect_used)]
fn main() -> eframe::Result<()> {
    let scenario = std::env::var("OMNIFAB_DEMO").ok();
    let catalogue = i18n::catalogue();
    let titre = catalogue
        .traduction("titre-fenetre")
        .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        &titre,
        options,
        Box::new(move |cc| {
            let etat = cc.wgpu_render_state.as_ref();
            let mut app = DemoApp::new(catalogue, etat);
            if let Some(scenario) = scenario.as_deref() {
                app.preparer_demo(scenario);
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Point2, Point3};
    use forge_session::{AddConstraint, AddEntity, ContrainteEsquisse, EntiteEsquisse, MovePoint};
    use travail::{ecris_fichier, lire_borne, plus_grande_dimension};

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
        // Travail non enregistré, rien d'ouvert : bloquer et
        // demander.
        assert_eq!(
            garde_fermeture(true, false),
            DecisionFermeture::BloquerEtConfirmer,
            "la confirmation de perte doit s'ouvrir avant de fermer"
        );
        // Rien à perdre : la fenêtre se ferme.
        assert_eq!(
            garde_fermeture(false, false),
            DecisionFermeture::LaisserFermer,
            "sans travail non enregistré, la fermeture passe"
        );
    }

    // --- Lecture bornée avant allocation (audit n°3 A3-07) ---

    /// La lecture d'un fichier d'import est bornée AVANT lecture
    /// (audit n°3 A3-07, ADR-0009) : un fichier plus gros que le
    /// budget est refusé sans être lu — le refus porte le code
    /// OF-IO-0001 du registre.
    #[test]
    fn lire_borne_refuse_avant_lecture() {
        let chemin = std::env::temp_dir().join("omnifab-audit3-import.bin");
        std::fs::write(&chemin, [0u8; 32]).expect("écriture du fichier test");
        // 32 octets sous un budget de 64 : lecture entière.
        assert_eq!(
            lire_borne(&chemin, 64).expect("lecture sous budget"),
            vec![0u8; 32]
        );
        // 32 octets au-dessus d'un budget de 16 : refus avant
        // lecture, code OF-IO-0001 (ADR-0009).
        let erreur = lire_borne(&chemin, 16).expect_err("refus attendu");
        assert!(
            erreur.contains("OF-IO-0001"),
            "le refus doit porter le code du registre : {erreur}"
        );
        assert!(
            erreur.contains("32") && erreur.contains("16"),
            "le refus cite la taille et le budget : {erreur}"
        );
        let _ = std::fs::remove_file(&chemin);
    }

    // --- Fil de travail résilient (audit n°3 N8) ---

    /// Pompe les retours du fil de travail jusqu'à en recevoir le
    /// nombre attendu, 10 secondes au plus (audit n°3 N8).
    fn pomper_retours(recepteur: &mpsc::Receiver<Retour>, attendus: usize) -> Vec<Retour> {
        let mut retours: Vec<Retour> = Vec::new();
        for _ in 0..2_000 {
            retours.extend(recepteur.try_iter());
            if retours.len() >= attendus {
                return retours;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        retours
    }

    /// Le fil survit à une panique du travail (audit n°3 N8) : le
    /// travail qui panique revient en erreur typée OF-WORK-0001 —
    /// code à inscrire au registre central — et le travail suivant
    /// est traité normalement.
    #[test]
    fn le_fil_survit_a_une_panique_de_travail() {
        let (emetteur_travaux, recepteur_travaux) = mpsc::channel::<Travail>();
        let (emetteur_retours, recepteur_retours) = mpsc::channel::<Retour>();
        let fil = std::thread::spawn(move || fil_de_travail(recepteur_travaux, emetteur_retours));
        emetteur_travaux
            .send(Travail::Bogue { generation: 1 })
            .expect("envoi du travail bogué");
        emetteur_travaux
            .send(Travail::ResoudreApercu {
                generation: 2,
                croquis: EsquisseProjection::default(),
            })
            .expect("envoi du travail sain");
        let retours = pomper_retours(&recepteur_retours, 2);
        drop(emetteur_travaux);
        fil.join()
            .expect("le fil se termine proprement à la déconnexion");
        assert_eq!(retours.len(), 2, "un retour par travail, même bogué");
        match &retours[0] {
            Retour::Erreur {
                generation,
                message,
            } => {
                assert_eq!(
                    *generation, 1,
                    "l'erreur porte la génération du travail bogué"
                );
                assert!(
                    message.contains("OF-WORK-0001"),
                    "l'échec inopiné doit porter le code du registre : {message}"
                );
            }
            _ => panic!("le travail bogué doit revenir en erreur typée"),
        }
        assert_eq!(retours[1].generation(), 2, "le travail suivant est traité");
    }

    /// La déconnexion de l'émetteur éteint le fil comme un état, pas
    /// un crash (audit n°3 N8) : le fil se termine proprement.
    #[test]
    fn la_deconnexion_eteint_le_fil_proprement() {
        let (emetteur_travaux, recepteur_travaux) = mpsc::channel::<Travail>();
        let (emetteur_retours, _recepteur_retours) = mpsc::channel::<Retour>();
        let fil = std::thread::spawn(move || fil_de_travail(recepteur_travaux, emetteur_retours));
        drop(emetteur_travaux);
        fil.join()
            .expect("le fil s'éteint proprement à la déconnexion");
    }

    /// Le fil mort ne laisse pas un « travail en cours » permanent
    /// (audit n°3 N8) : l'envoi suivant relance un fil neuf, le
    /// compteur des travaux repart de zéro et le travail est traité.
    #[test]
    fn le_fil_mort_est_relance_au_prochain_envoi() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        // Simule un fil mort : l'émetteur en place n'a plus de fil.
        let (emetteur_mort, recepteur_mort) = mpsc::channel::<Travail>();
        drop(recepteur_mort);
        app.travaux = Some(emetteur_mort);
        app.travaux_en_attente = 7;
        app.envoyer_travail(Travail::ResoudreApercu {
            generation: app.generation,
            croquis: EsquisseProjection::default(),
        });
        assert_eq!(
            app.travaux_en_attente, 1,
            "le compteur repart de zéro puis compte le travail relancé"
        );
        // Le travail relancé revient : le fil neuf le traite.
        let mut recus: Vec<Retour> = Vec::new();
        for _ in 0..2_000 {
            recus.extend(app.retours.try_iter());
            if !recus.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            matches!(recus.first(), Some(Retour::EsquisseApercue { .. })),
            "le travail relancé est traité par le fil neuf"
        );
    }

    /// Le code du fil de travail vit au registre central des codes
    /// d'erreur (ADR-0003, audit n°3 N8) : OF-WORK-0001 doit y
    /// figurer, comme les codes des crates du noyau.
    #[test]
    fn of_work_0001_est_au_registre() {
        const REGISTRE: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/error-codes.md"
        ));
        assert!(
            REGISTRE.contains("OF-WORK-0001"),
            "OF-WORK-0001 absent du registre docs/error-codes.md"
        );
    }

    /// L'import est une action annulable : undo vide la géométrie de
    /// l'état (ADR-0011).
    #[test]
    fn undo_d_import_disparait_de_l_etat() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        importer_svg_de_test(&mut app);
        assert!(app.session.geom().traces().is_some(), "géométrie importée");
        app.session.undo().expect("undo");
        assert!(app.session.geom().traces().is_none(), "l'import est défait");
    }

    /// Export puis re-import forment un aller-retour : mêmes compteurs.
    #[test]
    fn export_puis_reimport_est_equivalent() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        importer_svg_de_test(&mut app);
        let reference = app.session.geom().traces().expect("traces importées");
        let blob = reference.blob.clone();
        let octets = app.session.document().blob(&blob).expect("blob déposé");
        let traces = svg::import(octets, &svg::SvgBudgets::default()).expect("re-parse");
        let exporte = svg::export(&traces).expect("export");
        let reimporte = svg::import(&exporte, &svg::SvgBudgets::default()).expect("re-import");
        assert_eq!(reimporte.len(), traces.len(), "même nombre de tracés");
        let attendus: usize = traces.iter().map(|trace| trace.len()).sum();
        let obtenus: usize = reimporte.iter().map(|trace| trace.len()).sum();
        assert_eq!(obtenus, attendus, "même nombre de sommets");
    }

    /// Un nouvel import après un undo ne laisse fuir aucun blob
    /// orphelin (ADR-0011) : l'ancien contenu est remplacé, et une
    /// sauvegarde après undo n'emporte aucun blob.
    #[test]
    fn nouvel_import_apres_undo_ne_fuit_pas_de_blob() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        importer_svg_de_test(&mut app);
        let noms = app.session.document().blob_names();
        assert_eq!(noms.len(), 1, "le blob de l'import est déposé");
        assert!(noms[0].starts_with("geometry/traces"), "nom du blob");
        app.session.undo().expect("undo");
        importer_svg_de_test(&mut app);
        assert_eq!(
            app.session.document().blob_names().len(),
            1,
            "un seul blob : l'ancien est purgé, pas dupliqué"
        );
        app.session.undo().expect("undo");
        let octets = app.session.save_to_bytes().expect("sauvegarde");
        let document = Document::from_bytes(&octets).expect("conteneur");
        assert!(
            document.blob_names().is_empty(),
            "aucun blob orphelin dans le fichier"
        );
    }

    /// Un projet enregistré vers un chemin choisi se rouvre tel quel :
    /// même état rejoué, chemin mémorisé (ADR-0014).
    #[test]
    fn enregistrer_puis_ouvrir_par_chemin() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 1234.0,
            unite: None,
        })
        .expect("largeur");
        let dossier = std::env::temp_dir().join("omnifab-phase-b-enregistrer");
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("dossier de test");
        let chemin = dossier.join("projet.omnifab");
        app.enregistrer_dans(&chemin);
        assert!(chemin.is_file(), "le projet est écrit");
        assert_eq!(app.chemin_projet, Some(chemin.clone()), "chemin mémorisé");
        let mut relue = DemoApp::new(i18n::catalogue(), None);
        relue.ouvrir_projet(&chemin);
        assert_eq!(
            relue.chemin_projet,
            Some(chemin),
            "chemin repris à l'ouverture"
        );
        assert_eq!(relue.ecran, Ecran::Atelier, "l'atelier s'ouvre");
        let largeur = relue.session.graph_mut().value("largeur").expect("valeur");
        assert_eq!(largeur, 1234.0, "l'état est rejoué");
    }

    /// La boîte de scêne englobe tracés et maillage : union des boîtes
    /// englobantes, une seule échelle (ADR-0014).
    #[test]
    fn boite_scene_union_traces_maillage() {
        let un = Length::from_millimeters(1.0);
        let a = Point2::new(Length::ZERO, Length::ZERO).expect("fini");
        let b = Point2::new(un, Length::ZERO).expect("fini");
        let trace = Path2::from_points(vec![a, b], false);
        let s0 = Point3::new(Length::ZERO, un, Length::ZERO).expect("fini");
        let s1 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s2 = Point3::new(un, un, un).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        let boite = DemoApp::boite_scene(&[trace], Some(&maillage)).expect("boîte");
        assert_eq!(boite.min_x, 0.0, "origine x commune");
        assert_eq!(boite.min_y, 0.0, "origine y commune");
        assert_eq!(boite.max_x, 1.0, "union en x");
        assert_eq!(boite.max_y, 1.0, "union en y");
    }

    /// Le cadrage automatique vise la scêne importée, même éloignée de
    /// l'origine : un import positionné loin du rectangle paramétrique
    /// reste centré sur ses objets (bug de recentrage DXF signalé à la
    /// validation C1 — l'union avec le rectangle déplaçait le centre).
    #[test]
    fn le_cadrage_vise_la_scene_meme_eloignee_du_rectangle() {
        let a = Point2::new(
            Length::from_millimeters(100.0),
            Length::from_millimeters(200.0),
        )
        .expect("fini");
        let b = Point2::new(
            Length::from_millimeters(120.0),
            Length::from_millimeters(220.0),
        )
        .expect("fini");
        let trace = Path2::from_points(vec![a, b], false);
        let scene = DemoApp::boite_scene(&[trace], None).expect("boîte de scêne");
        let boite = DemoApp::boite_cadrage(50.0, 50.0, Some(scene));
        assert_eq!(boite.min_x, 100.0, "origine x de la scêne");
        assert_eq!(boite.min_y, 200.0, "origine y de la scêne");
        assert_eq!(boite.max_x, 120.0, "portée x de la scêne");
        assert_eq!(boite.max_y, 220.0, "portée y de la scêne");
    }

    /// Sans scêne importée, le cadrage vise le rectangle paramétrique
    /// seul (ADR-0015).
    #[test]
    fn le_cadrage_sans_scene_vise_le_rectangle() {
        let boite = DemoApp::boite_cadrage(800.0, 600.0, None);
        assert_eq!(boite.min_x, 0.0, "origine x du rectangle");
        assert_eq!(boite.min_y, 0.0, "origine y du rectangle");
        assert_eq!(boite.max_x, 800.0, "portée x du rectangle");
        assert_eq!(boite.max_y, 600.0, "portée y du rectangle");
    }

    /// La boîte de scêne 3D englobe tracés et maillage en volume :
    /// les tracés posés sur le plan z = 0, les sommets du maillage
    /// avec leur altitude (ADR-0015, phase C2).
    #[test]
    fn boite_scene_3d_union_traces_maillage() {
        let un = Length::from_millimeters(1.0);
        let a = Point2::new(Length::ZERO, Length::ZERO).expect("fini");
        let b = Point2::new(un, Length::ZERO).expect("fini");
        let trace = Path2::from_points(vec![a, b], false);
        let s0 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s1 = Point3::new(un, un, un).expect("fini");
        let s2 = Point3::new(un, un, Length::ZERO).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        let boite = DemoApp::boite_scene_3d(&[trace], Some(&maillage)).expect("boîte");
        assert_eq!(boite.min_x, 0.0, "origine x commune");
        assert_eq!(boite.min_y, 0.0, "origine y commune");
        assert_eq!(boite.min_z, 0.0, "tracé posé sur le plan z = 0");
        assert_eq!(boite.max_x, 1.0, "union en x");
        assert_eq!(boite.max_y, 1.0, "union en y");
        assert_eq!(boite.max_z, 1.0, "altitude du maillage");
    }

    /// Sans scêne importée, le cadrage 3D vise le rectangle
    /// paramétrique posé sur le plan z = 0 (ADR-0015, phase C2).
    #[test]
    fn le_cadrage_3d_sans_scene_vise_le_rectangle() {
        let boite = DemoApp::boite_cadrage_3d(800.0, 600.0, None);
        assert_eq!(boite.min_x, 0.0, "origine x du rectangle");
        assert_eq!(boite.min_y, 0.0, "origine y du rectangle");
        assert_eq!(boite.max_x, 800.0, "portée x du rectangle");
        assert_eq!(boite.max_y, 600.0, "portée y du rectangle");
        assert_eq!(boite.min_z, 0.0, "plan z = 0");
        assert_eq!(boite.max_z, 0.0, "rectangle posé sur le plan");
    }

    /// Position du premier segment portant la couleur donnée :
    /// sert à vérifier la priorité d'affichage de la vue 3D.
    fn premiere_couleur(scene: &Scene3D, couleur: Couleur) -> usize {
        scene
            .segments
            .iter()
            .position(|s| s.couleur == couleur)
            .expect("couleur présente")
    }

    /// Priorité d'affichage de la vue 3D : le pipeline de lignes ne
    /// teste pas la profondeur, le dernier segment ajouté recouvre
    /// les précédents. La grille est donc posée sous les tracés et
    /// le rectangle, qui ne scintillent plus quand ils coïncident
    /// avec elle (validation C2, round 2).
    #[test]
    fn la_grille_3d_passe_sous_les_traces_et_le_rectangle() {
        let un = Length::from_millimeters(1.0);
        let a = Point2::new(Length::ZERO, Length::ZERO).expect("fini");
        let b = Point2::new(un, Length::ZERO).expect("fini");
        let trace = Path2::from_points(vec![a, b], false);
        let camera = Camera3D::nouvelle();
        let scene = DemoApp::scene_vue_3d(&[trace], &[], &camera, 800.0, 600.0);
        let grille = premiere_couleur(&scene, COULEUR_GRILLE);
        let trace_pos = premiere_couleur(&scene, COULEUR_TRACES);
        let rectangle = premiere_couleur(&scene, COULEUR_RECTANGLE_3D);
        assert!(
            grille < trace_pos && trace_pos < rectangle,
            "grille sous les tracés sous le rectangle"
        );
    }

    /// La valeur glissée s'engage depuis le tampon persistant : au
    /// relâchement, le champ est déjà revenu à la valeur du graphe —
    /// un engagement sur la seule valeur du champ écrivait l'ancienne
    /// valeur (dimensions figées au glissement, validations C1
    /// rounds 3 à 6). Sans changement effectif, rien ne va au
    /// journal.
    #[test]
    fn le_glissement_s_engage_sur_le_tampon_persistant() {
        // Relâchement après un glissement : le tampon gagne.
        assert_eq!(
            DemoApp::engagement_params(800.0, 500.0, false, true, false),
            Some(500.0)
        );
        // Glissement en cours : aucun événement par frame.
        assert_eq!(
            DemoApp::engagement_params(800.0, 500.0, true, false, true),
            None
        );
        // Édition clavier : le tampon final s'engage.
        assert_eq!(
            DemoApp::engagement_params(800.0, 500.0, false, false, true),
            Some(500.0)
        );
        // Geste revenu à la valeur de départ : rien au journal.
        assert_eq!(
            DemoApp::engagement_params(800.0, 800.0, false, true, true),
            None
        );
    }

    /// Tampon d'édition d'un champ de dimension : inactif, il suit le
    /// graphe (undo/redo, ouverture) ; actif, il persiste — c'est lui
    /// qui garde la valeur glissée d'une frame à l'autre pendant le
    /// geste (validation C1 round 6).
    #[test]
    fn le_tampon_inactif_suit_le_graphe_l_actif_persiste() {
        assert_eq!(
            DemoApp::tampon_apres_inactivite(false, 123.0, Some(800.0)),
            800.0,
            "inactif : reprise du graphe"
        );
        assert_eq!(
            DemoApp::tampon_apres_inactivite(true, 123.0, Some(800.0)),
            123.0,
            "actif : le tampon persiste"
        );
        assert_eq!(
            DemoApp::tampon_apres_inactivite(false, 123.0, None),
            0.0,
            "entrée absente : affichage zéro"
        );
    }

    /// Les trois événements vierges (largeur, hauteur, diagonale)
    /// forment trois groupes d'undo : annuler puis rétablir les
    /// parcourt sans perdre le journal — après un undo des
    /// dimensions, le clamp d'affichage d'un champ comptait comme
    /// un changement et détruisait les groupes défaits (validation
    /// C1 round 3).
    #[test]
    fn annuler_puis_retablir_les_groupes_vierges() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        assert!(app.session.can_undo(), "session vierge annulable");
        app.session.undo().expect("premier undo");
        assert!(app.session.can_redo(), "rétablissable après un undo");
        app.session.undo().expect("second undo");
        assert!(app.session.can_redo(), "rétablissable après deux undos");
        app.session.redo().expect("premier rétablissement");
        app.session.redo().expect("second rétablissement");
        assert_eq!(
            app.session.document().events().len(),
            3,
            "journal intact : trois événements"
        );
        let largeur = app.session.graph_mut().value("largeur").expect("valeur");
        let hauteur = app.session.graph_mut().value("hauteur").expect("valeur");
        assert_eq!(largeur, 240.0, "largeur rétablie");
        assert_eq!(hauteur, 160.0, "hauteur rétablie");
    }

    /// Le projet vierge porte les trois liaisons de référence :
    /// largeur et hauteur en entrées mm, diagonale en expression mm
    /// (ADR-0019 §6).
    #[test]
    fn le_projet_vierge_porte_les_trois_liaisons() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let noms = app.session.graph().names();
        assert!(noms.contains(&"largeur".to_string()), "entrée largeur");
        assert!(noms.contains(&"hauteur".to_string()), "entrée hauteur");
        assert!(
            noms.contains(&"diagonale".to_string()),
            "expression diagonale"
        );
        let info = app.session.graph().noeud("largeur").expect("nœud");
        assert_eq!(info.unite, Some(Unite::Millimetre), "unité mm déclarée");
        assert!(info.expression.is_none(), "largeur est une entrée");
        let diagonale = app.session.graph_mut().value("diagonale").expect("valeur");
        assert!(
            (diagonale - 288.44410203711914).abs() < 1e-9,
            "sqrt(240² + 160²)"
        );
    }

    /// La génération des liaisons avance sur les commandes de
    /// valeurs et l'annulation, pas sur les autres commandes : les
    /// cotes pilotées savent quand se re-figer (ADR-0019 §6).
    #[test]
    fn la_generation_des_liaisons_avance() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let avant = app.generation_liaisons;
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 900.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
        assert_eq!(
            app.generation_liaisons,
            avant + 1,
            "une commande de valeur avance la génération"
        );
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(1.0),
                y: Length::from_millimeters(2.0),
            },
        })
        .expect("point");
        assert_eq!(
            app.generation_liaisons,
            avant + 1,
            "une commande hors liaisons ne l'avance pas"
        );
        app.annuler();
        assert_eq!(
            app.generation_liaisons,
            avant + 2,
            "l'annulation avance la génération"
        );
    }

    /// Une cote pilotée se fige depuis le graphe et suit les
    /// liaisons : changer la largeur re-fige la cote à la prochaine
    /// résolution (ADR-0019 §4/§6, checklist §8).
    #[test]
    fn la_cote_pilotee_se_gele_depuis_le_graphe() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        for (x, y) in [(0.0, 0.0), (100.0, 0.0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point");
        }
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Expression {
                    texte: "largeur / 2".into(),
                    alias: "esquisse.demi".into(),
                },
            },
        })
        .expect("cote pilotée");
        let figee = app.session.croquis_pour_resolution().expect("croquis figé");
        // La cote figée ne porte plus son alias (Expression →
        // Fixe) : la contrainte Distance se cherche directement.
        let valeur = figee
            .contraintes()
            .iter()
            .find_map(|(_, contrainte)| match contrainte {
                ContrainteEsquisse::Distance { valeur, .. } => Some(valeur.clone()),
                _ => None,
            })
            .expect("cote présente");
        let ValeurCote::Fixe { mm } = valeur else {
            panic!("cote figée pour le solveur");
        };
        assert_eq!(mm.millimeters(), 120.0, "largeur 240 → cote 120");
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 1000.0,
            unite: Some(Unite::Millimetre),
        })
        .expect("largeur");
        let figee = app.session.croquis_pour_resolution().expect("croquis figé");
        // La cote figée ne porte plus son alias (Expression →
        // Fixe) : la contrainte Distance se cherche directement.
        let valeur = figee
            .contraintes()
            .iter()
            .find_map(|(_, contrainte)| match contrainte {
                ContrainteEsquisse::Distance { valeur, .. } => Some(valeur.clone()),
                _ => None,
            })
            .expect("cote présente");
        let ValeurCote::Fixe { mm } = valeur else {
            panic!("cote figée pour le solveur");
        };
        assert_eq!(mm.millimeters(), 500.0, "largeur 1000 → cote 500");
    }

    /// L'axe Y monte dans la scêne et descend à l'écran : un point
    /// plus haut apparaît plus haut — sans l'inversion, les fichiers
    /// asymétriques se reflétaient en miroir (DXF réel importé à la
    /// validation de la phase B).
    #[test]
    fn projection_inverse_l_axe_y() {
        let mut camera = Camera2D::nouvelle();
        camera.ajuster(
            100.0,
            100.0,
            Boite2::du_point(0.0, 0.0).etendre(10.0, 10.0),
            0.0,
        );
        let bas = camera.vers_ecran((5.0, 0.0));
        let haut = camera.vers_ecran((5.0, 10.0));
        assert!(
            haut.1 < bas.1,
            "plus haut dans la scêne, plus haut à l'écran"
        );
    }

    /// La caméra vise le centre du rect, pas son coin : le centre de
    /// la boîte d'ajustement atterrit au centre du viewport (bug de
    /// recentrage signalé à la validation C1).
    #[test]
    fn le_centre_de_la_camera_atterrit_au_centre_du_rect() {
        let mut camera = Camera2D::nouvelle();
        camera.ajuster(
            1000.0,
            600.0,
            Boite2::du_point(0.0, 0.0).etendre(800.0, 600.0),
            20.0,
        );
        let rect = egui::Rect::from_min_size(egui::pos2(40.0, 30.0), egui::vec2(1000.0, 600.0));
        let centre = camera.centre_mm;
        let affiche = DemoApp::point_ecran(&camera, rect, centre.0, centre.1);
        assert_eq!(affiche, rect.center(), "centré, pas vers le coin");
    }

    /// Le wireframe 2D du maillage : trois arêtes par triangle en
    /// projection de dessus, cyan, un pixel — le STL revient en vue
    /// 2D, dessiné par le même pipeline GPU que les tracés, sans
    /// plafond (validation C2, round 5).
    #[test]
    fn le_wireframe_2d_pose_trois_aretes_par_triangle() {
        let un = Length::from_millimeters(1.0);
        let s0 = Point3::new(Length::ZERO, Length::ZERO, un).expect("fini");
        let s1 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s2 = Point3::new(un, Length::ZERO, un).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        let mut scene = Scene2D::vide();
        DemoApp::ajouter_maillage_2d(&mut scene, &maillage);
        assert_eq!(scene.segments.len(), 3, "trois arêtes par triangle");
        assert_eq!(scene.segments[0].depart, (0.0, 0.0), "vue de dessus");
        assert!(
            scene
                .segments
                .iter()
                .all(|segment| segment.couleur == COULEUR_WIREFRAME),
            "arêtes cyan"
        );
    }

    /// Plafond d'arêtes attendu pour le wireframe 2D (audit n°4
    /// A4-02).
    const MAX_ARETES_WIREFRAME: usize = 2_097_152;

    /// Le wireframe 2D d'un maillage démesuré est plafonné : jamais
    /// plus de `MAX_ARETES_WIREFRAME` segments par frame, les
    /// tampons GPU restent sous la limite de 256 Mio (audit n°4
    /// A4-02).
    #[test]
    fn le_wireframe_2d_est_plafonne() {
        let un = Length::from_millimeters(1.0);
        let s0 = Point3::new(Length::ZERO, Length::ZERO, un).expect("fini");
        let s1 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s2 = Point3::new(un, Length::ZERO, un).expect("fini");
        let triangles = vec![[0, 1, 2]; 800_000];
        let maillage = Mesh::from_parts(vec![s0, s1, s2], triangles).expect("valide");
        let mut scene = Scene2D::vide();
        DemoApp::ajouter_maillage_2d(&mut scene, &maillage);
        assert!(
            scene.segments.len() <= MAX_ARETES_WIREFRAME,
            "le wireframe 2D est plafonné à {} arêtes, pas {}",
            MAX_ARETES_WIREFRAME,
            scene.segments.len()
        );
    }

    /// Le wireframe 3D vit en cache à clé d'édition (ADR-0027,
    /// patron `camera_cle`, audit n°4 A4-02) : deux frames de
    /// suite ne reconstruisent pas le wireframe, une clé changée
    /// le reconstruit.
    #[test]
    fn le_wireframe_3d_vit_en_cache_a_cle_edition() {
        let un = Length::from_millimeters(1.0);
        let s0 = Point3::new(Length::ZERO, Length::ZERO, un).expect("fini");
        let s1 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s2 = Point3::new(un, Length::ZERO, un).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.cache_maillage = Some(("blob-stl".to_string(), Ok(maillage)));
        app.rafraichir_wireframe(&Some("blob-stl".to_string()));
        let segments = app.wireframe_maillage.as_ref().expect("cache posé").1.len();
        assert!(segments > 0, "le wireframe est construit une première fois");
        // Sabotage du cache : s'il survit à la frame suivante, le
        // wireframe n'est pas reconstruit à chaque frame.
        app.wireframe_maillage
            .as_mut()
            .expect("cache posé")
            .1
            .clear();
        app.rafraichir_wireframe(&Some("blob-stl".to_string()));
        assert!(
            app.wireframe_maillage
                .as_ref()
                .expect("cache posé")
                .1
                .is_empty(),
            "même clé : le sabotage survit, pas de reconstruction"
        );
        // Clé d'édition changée : le wireframe est reconstruit.
        app.rafraichir_wireframe(&None);
        assert!(
            !app.wireframe_maillage
                .as_ref()
                .expect("cache posé")
                .1
                .is_empty(),
            "une clé d'édition changée reconstruit le wireframe"
        );
    }

    /// Parse factice : échoue en comptant ses appels.
    fn echec_de_parse(parses: &mut u32) -> impl FnOnce() -> Result<Vec<u8>, String> + '_ {
        move || {
            *parses += 1;
            Err("fichier invalide".to_string())
        }
    }

    /// Un échec de parse reste en cache : la même clé n'est plus
    /// re-parsée à chaque frame (ADR-0016 C1).
    #[test]
    fn un_echec_de_parse_est_en_cache_pas_reparse() {
        let mut parses: u32 = 0;
        let mut cache: Option<(String, Result<Vec<u8>, String>)> = None;
        assert!(met_en_cache(
            &mut cache,
            "a".into(),
            echec_de_parse(&mut parses)
        ));
        assert!(!met_en_cache(
            &mut cache,
            "a".into(),
            echec_de_parse(&mut parses)
        ));
        assert!(met_en_cache(
            &mut cache,
            "b".into(),
            echec_de_parse(&mut parses)
        ));
        assert_eq!(parses, 2, "deux parses pour deux clés distinctes");
        let (_, resultat) = cache.expect("entrée en cache");
        assert!(resultat.is_err(), "l'échec est mémorisé");
    }

    /// Les retours d'une génération obsolète sont ignorés : un
    /// travail parti avant une réinitialisation ou une ouverture ne
    /// touche plus l'état (ADR-0016 C1).
    #[test]
    fn les_retours_obsoletes_sont_ignores() {
        let file = vec![
            Retour::Erreur {
                generation: 7,
                message: "obsolète".to_string(),
            },
            Retour::Erreur {
                generation: 8,
                message: "actuel".to_string(),
            },
        ];
        let gardes = garde_actuels(file, 8);
        assert_eq!(gardes.len(), 1, "seul le retour courant est gardé");
        assert_eq!(gardes[0].generation(), 8, "le retour gardé est courant");
    }

    /// La sauvegarde après un undo ne casse plus le redo : le fichier
    /// contient le journal appliqué, la pile redo survit en mémoire
    /// (ADR-0016 C3).
    #[test]
    fn enregistrer_apres_undo_preserve_le_redo() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 900.0,
            unite: None,
        })
        .expect("largeur");
        app.annuler();
        assert!(app.session.can_redo(), "le redo est en place");
        let dossier = std::env::temp_dir().join("omnifab-c3-enregistrer-undo");
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("dossier de test");
        let chemin = dossier.join("projet.omnifab");
        app.enregistrer_dans(&chemin);
        assert!(chemin.is_file(), "le projet est écrit");
        let document = Document::open_from_path(&chemin).expect("ouverture");
        assert_eq!(
            document.events().len(),
            EVENEMENTS_VIERGES,
            "le fichier contient le journal appliqué"
        );
        assert!(app.session.can_redo(), "enregistrer garde le redo");
        app.retablir();
        let largeur = app.session.graph_mut().value("largeur").expect("valeur");
        assert_eq!(largeur, 900.0, "le redo restaure la largeur");
    }

    /// L'indicateur de travail non enregistré suit les commandes,
    /// l'annulation et la sauvegarde (ADR-0016 C3).
    #[test]
    fn dirty_suit_les_commandes_et_la_sauvegarde() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        assert!(!app.dirty, "session vierge : rien à enregistrer");
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 900.0,
            unite: None,
        })
        .expect("largeur");
        assert!(app.dirty, "une commande effective pose l'indicateur");
        let dossier = std::env::temp_dir().join("omnifab-c3-dirty");
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).expect("dossier de test");
        let chemin = dossier.join("projet.omnifab");
        app.enregistrer_dans(&chemin);
        assert!(!app.dirty, "la sauvegarde lève l'indicateur");
        app.annuler();
        assert!(app.dirty, "l'annulation repose l'indicateur");
    }

    /// Un import au format inconnu est refusé par la session : aucun
    /// succès n'est affiché, et l'import valide suivant efface le
    /// refus (ADR-0016 C4).
    #[test]
    fn succes_d_import_conditionne_a_la_commande() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.appliquer_retour(Retour::ImportTraces {
            generation: app.generation,
            format: "step",
            octets: vec![],
            traces: 1,
            sommets: 3,
        });
        assert!(app.error.is_some(), "le refus est affiché");
        assert!(app.info.is_none(), "aucun succès après un refus");
        app.appliquer_retour(Retour::ImportTraces {
            generation: app.generation,
            format: "svg",
            octets: SVG_TEST.to_vec(),
            traces: 1,
            sommets: 3,
        });
        assert!(app.info.is_some(), "l'import réussi affiche son succès");
        assert!(app.error.is_none(), "le refus précédent est effacé");
    }

    /// L'action suivante efface l'erreur de la précédente : plus de
    /// rouge persistant après correction (ADR-0016 C4).
    #[test]
    fn messages_d_etat_fideles() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.appliquer_retour(Retour::Erreur {
            generation: app.generation,
            message: "fichier invalide".to_string(),
        });
        assert!(app.error.is_some(), "l'échec est affiché");
        assert!(app.info.is_none(), "aucun succès affiché");
        app.executer(&SetInput {
            name: "largeur".into(),
            value: 900.0,
            unite: None,
        })
        .expect("commande valide");
        assert!(app.error.is_none(), "la correction efface l'erreur");
    }

    /// Modificateurs pour un test de raccourcis (ADR-0016 C2).
    fn modificateurs(ctrl: bool, command: bool, maj: bool) -> egui::Modifiers {
        egui::Modifiers {
            alt: false,
            ctrl,
            shift: maj,
            mac_cmd: false,
            command,
        }
    }

    /// Raccourcis globaux : Cmd annule, Cmd+Y ou Cmd+Maj+Z rétablit,
    /// rien ne se déclenche quand un champ de texte a le focus, et
    /// Ctrl nu ne fait rien — Cmd macOS est « command », pas « ctrl »
    /// (ADR-0016 C2).
    #[test]
    fn les_raccourcis_ignorent_la_saisie_en_cours() {
        assert_eq!(
            raccourcis_annulation(&modificateurs(false, true, false), true, false, false),
            (true, false)
        );
        assert_eq!(
            raccourcis_annulation(&modificateurs(false, true, false), false, true, false),
            (false, true)
        );
        assert_eq!(
            raccourcis_annulation(&modificateurs(false, true, true), true, false, false),
            (false, true)
        );
        // Un champ de texte a le focus : la saisie n'est pas écrasée.
        assert_eq!(
            raccourcis_annulation(&modificateurs(false, true, false), true, false, true),
            (false, false)
        );
        // Ctrl seul (sans Cmd) ne déclenche rien sous macOS.
        assert_eq!(
            raccourcis_annulation(&modificateurs(true, false, false), true, false, false),
            (false, false)
        );
    }
    /// L'échelle mètres est proposée pour un maillage dont la plus
    /// grande dimension reste dans la bande des exports en mètres :
    /// un objet Blender de 3 m de diamètre exporté en mètres mesure
    /// 3,0 « mm » après lecture et déclenche le dialogue (retour
    /// checklist jalon 2) ; une dimension nulle (maillage vide) ou
    /// franchement millimétrique ne déclenche rien (ADR-0017).
    #[test]
    fn echelle_metres_proposee_sous_le_seuil() {
        assert!(propose_echelle_metres(0.05));
        assert!(propose_echelle_metres(1.0));
        assert!(
            propose_echelle_metres(3.0),
            "objet Blender de 3 m (retour jalon 2)"
        );
        assert!(!propose_echelle_metres(0.0));
        assert!(!propose_echelle_metres(10.0));
        assert!(!propose_echelle_metres(50.0));
    }

    /// La plus grande dimension est le plus grand côté de la boîte
    /// englobante ; un maillage vide mesure 0 (ADR-0017).
    #[test]
    fn plus_grande_dimension_est_le_plus_grand_cote() {
        let un = Length::from_millimeters(1.0);
        let s0 = Point3::new(Length::ZERO, un, un).expect("fini");
        let s1 = Point3::new(un, un, Length::ZERO).expect("fini");
        let s2 = Point3::new(un, Length::ZERO, un).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        assert_eq!(plus_grande_dimension(&maillage), 1.0);
        assert_eq!(plus_grande_dimension(&Mesh::new()), 0.0);
    }

    /// L'échelle d'import multiplie les coordonnées : × 1000 pour un
    /// export en mètres, l'identité ne touche à rien (ADR-0017).
    #[test]
    fn maillage_echelle_multiplie_les_coordonnees() {
        let un = Length::from_millimeters(1.0);
        let s0 = Point3::new(Length::ZERO, Length::ZERO, un).expect("fini");
        let s1 = Point3::new(un, Length::ZERO, un).expect("fini");
        let s2 = Point3::new(un, un, un).expect("fini");
        let maillage = Mesh::from_parts(vec![s0, s1, s2], vec![[0, 1, 2]]).expect("valide");
        let grossi = maillage_echelle(maillage, 1000.0).expect("échelle appliquée");
        assert_eq!(grossi.vertices()[0].x().millimeters(), 0.0);
        assert_eq!(grossi.vertices()[0].z().millimeters(), 1000.0);
        let intact = maillage_echelle(grossi, 1.0).expect("identité");
        assert_eq!(intact.vertices()[0].z().millimeters(), 1000.0);
    }

    /// Un import minuscule attend le choix des unités : rien n'est
    /// écrit avant la réponse, et « Mètres » importe à l'échelle
    /// × 1000 (ADR-0017).
    #[test]
    fn import_minuscule_attend_le_choix_des_unites() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.appliquer_retour(Retour::ImportMaillage {
            generation: app.generation,
            octets: b"solid minuscule".to_vec(),
            triangles: 1,
            dimension: 0.05,
        });
        assert!(app.demande_echelle.is_some(), "le choix des unités attend");
        assert!(app.session.geom().maillage().is_none(), "rien au journal");
        app.importer_maillage(b"solid minuscule".to_vec(), 1, 1000.0);
        assert!(app.demande_echelle.is_none(), "demande consommée");
        let maillage = app.session.geom().maillage().expect("maillage importé");
        assert_eq!(maillage.echelle, 1000.0, "échelle mètres retenue");
    }

    /// Compare une position résolue aux coordonnées attendues,
    /// à la tolérance du domaine près (1e-6 mm).
    fn proche(position: Option<(Length, Length)>, x: f64, y: f64) -> bool {
        let (px, py) = match position {
            Some((px, py)) => (px, py),
            None => return false,
        };
        px.approx_eq(Length::from_millimeters(x)) && py.approx_eq(Length::from_millimeters(y))
    }

    /// Pompe les retours du fil de travail jusqu’à ce que la
    /// résolution en vol revienne : délai maximal 10 secondes
    /// (ADR-0016 C1).
    fn pomper_les_retours(app: &mut DemoApp) {
        for _ in 0..2_000 {
            if app.resolution_en_attente.is_none() {
                return;
            }
            let recus: Vec<Retour> = app.retours.try_iter().collect();
            app.travaux_en_attente = app.travaux_en_attente.saturating_sub(recus.len() as u32);
            for retour in garde_actuels(recus, app.generation) {
                app.appliquer_retour(retour);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("la résolution du croquis ne revient jamais");
    }

    /// Pompe le retour de l'aperçu de glissement : la boucle ne
    /// tient que l'aperçu en vol (`apercu_en_attente`), jamais la
    /// résolution — même canal, deux attentes distinctes.
    fn pomper_l_apercu(app: &mut DemoApp) {
        for _ in 0..2_000 {
            if !app.apercu_en_attente {
                return;
            }
            let recus: Vec<Retour> = app.retours.try_iter().collect();
            app.travaux_en_attente = app.travaux_en_attente.saturating_sub(recus.len() as u32);
            for retour in garde_actuels(recus, app.generation) {
                app.appliquer_retour(retour);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("l'aperçu de glissement ne revient jamais");
    }

    /// Le croquis contraint se résout au fil de travail : une entité
    /// posée, une demande, le cache reçoit les positions résolues
    /// (ADR-0018 D3).
    #[test]
    fn le_croquis_se_resout_au_fil_de_travail() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(10.0),
                y: Length::from_millimeters(20.0),
            },
        })
        .expect("entité posée");
        app.demande_resolution_esquisse();
        assert_eq!(
            app.resolution_en_attente,
            Some((1, app.generation_liaisons)),
            "résolution demandée"
        );
        pomper_les_retours(&mut app);
        let (cle, resultat) = app.croquis_resolu.expect("croquis résolu");
        assert_eq!(
            cle,
            (1, app.generation_liaisons),
            "clé du cache : édition et liaisons"
        );
        let resolu = resultat.expect("résolution réussie");
        assert!(proche(resolu.position(0), 10.0, 20.0), "position résolue");
    }

    /// Le croquis résolu n’est pas renvoyé : une seule résolution par
    /// édition, aucune nouvelle demande quand le cache est à jour —
    /// et le croquis vide ne demande rien (ADR-0016 C1,
    /// ADR-0018 D3).
    #[test]
    fn le_croquis_resolu_n_est_pas_reenvoye() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.demande_resolution_esquisse();
        assert_eq!(app.travaux_en_attente, 0, "croquis vide : aucun envoi");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(10.0),
                y: Length::from_millimeters(20.0),
            },
        })
        .expect("entité posée");
        app.demande_resolution_esquisse();
        assert_eq!(app.travaux_en_attente, 1, "une résolution envoyée");
        app.demande_resolution_esquisse();
        assert_eq!(app.travaux_en_attente, 1, "en vol : pas de redemande");
        pomper_les_retours(&mut app);
        app.demande_resolution_esquisse();
        assert_eq!(app.travaux_en_attente, 0, "cache à jour : aucun renvoi");
        assert!(app.croquis_resolu.is_some(), "le cache est rempli");
        app.session.undo().expect("undo");
        app.demande_resolution_esquisse();
        assert!(app.croquis_resolu.is_none(), "croquis vide : cache vidé");
        assert_eq!(app.travaux_en_attente, 0, "aucun nouvel envoi");
    }

    /// Pendant le vol d'une résolution fraîche, l'affichage garde la
    /// dernière résolution connue : le croquis contraint ne saute
    /// jamais aux graines au moment d'un clic — la « déformation »
    /// rapportée à la re-validation §7 disparaît. L'accroche, elle,
    /// attend la résolution fraîche.
    #[test]
    fn l_affichage_garde_la_derniere_resolution_pendant_le_vol() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("origine");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(80.0),
                y: Length::from_millimeters(-3.0),
            },
        })
        .expect("mobile");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("ancrage");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("cote");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Cache à jour : l'affichage montre la position résolue (à
        // 40 mm de l'ancré), pas la graine (80, -3) — l'ancrage
        // épingle la position résolue du point (ADR-0021, révision
        // du 2026-10-22), la distance se lit autour de l'ancré.
        let resolu = atelier_esquisse::resolu_affichage(&app).expect("résolution affichable");
        let (x0, y0) = resolu.position(0).expect("point ancré résolu");
        let (x, y) = resolu.position(1).expect("point mobile résolu");
        let dx = x.millimeters() - x0.millimeters();
        let dy = y.millimeters() - y0.millimeters();
        assert!(
            (dx.hypot(dy) - 40.0).abs() < 1e-6,
            "position résolue à 40 mm de l'ancré, pas à la graine"
        );
        // Un point de plus : l'édition avance, la résolution
        // repart — l'affichage garde la dernière connue pendant le
        // vol au lieu de flasher les graines.
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(120.0),
                y: Length::from_millimeters(90.0),
            },
        })
        .expect("point posé");
        // Trame du clic elle-même : la demande n'est pas encore
        // partie — l'affichage garde déjà la dernière résolution,
        // il ne saute pas aux graines.
        let resolu = atelier_esquisse::resolu_affichage(&app)
            .expect("l'ancienne résolution est montrée dès le clic");
        let (x0, y0) = resolu.position(0).expect("point ancré affiché");
        let (x, y) = resolu.position(1).expect("point mobile affiché");
        let dx = x.millimeters() - x0.millimeters();
        let dy = y.millimeters() - y0.millimeters();
        assert!(
            (dx.hypot(dy) - 40.0).abs() < 1e-6,
            "l'affichage ne flashe pas les graines dès le clic"
        );
        app.demande_resolution_esquisse();
        assert_eq!(app.travaux_en_attente, 1, "résolution fraîche en vol");
        assert!(
            !app.croquis_resolu
                .as_ref()
                .is_some_and(|(cle, _)| cle.0 == app.session.esquisse().edition()),
            "le cache est bien périmé"
        );
        let resolu = atelier_esquisse::resolu_affichage(&app)
            .expect("dernière résolution tenue pendant le vol");
        let (x0, y0) = resolu.position(0).expect("point ancré affiché");
        let (x, y) = resolu.position(1).expect("point mobile affiché");
        let dx = x.millimeters() - x0.millimeters();
        let dy = y.millimeters() - y0.millimeters();
        assert!(
            (dx.hypot(dy) - 40.0).abs() < 1e-6,
            "l'affichage tient la résolution, pas la graine"
        );
    }

    /// L'affichage garde la dernière résolution RÉUSSIE quand la
    /// fraîche ÉCHOUE (re-test du 24-10) : une résolution qui
    /// échoue ne doit jamais effacer la dernière forme saine —
    /// sinon le croquis saute aux graines du journal et paraît se
    /// déformer au moment même de la pose fautive. L'erreur est
    /// signalée, le DoF du panneau disparaît avec la résolution
    /// fraîche, mais la forme saine reste dessinée.
    #[test]
    fn l_affichage_garde_la_derniere_resolution_bonne_quand_le_frais_echoue() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("origine posée");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(80.0),
                y: Length::from_millimeters(-3.0),
            },
        })
        .expect("mobile posé");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("ancrage posé");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("cote posée");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Une cote impossible : à 40 mm de l'origine ET à 200 mm du
        // point déjà à 40 mm de l'origine — le triangle ne peut pas
        // exister, la résolution fraîche échoue.
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(120.0),
                y: Length::from_millimeters(90.0),
            },
        })
        .expect("troisième point posé");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("cote posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 1,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(200.0),
                },
            },
        })
        .expect("cote posée");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let frais = app.croquis_resolu.as_ref().expect("résolution fraîche");
        assert!(frais.1.is_err(), "la résolution fraîche échoue");
        assert!(app.error.is_some(), "l'échec est signalé");
        // La dernière résolution réussie tient l'affichage : la
        // forme saine d'avant la cote impossible reste dessinée,
        // le croquis ne saute pas aux graines.
        let resolu = atelier_esquisse::resolu_affichage(&app)
            .expect("la dernière résolution réussie reste affichable");
        let (x0, y0) = resolu.position(0).expect("point ancré affiché");
        let (x, y) = resolu.position(1).expect("point mobile affiché");
        let dx = x.millimeters() - x0.millimeters();
        let dy = y.millimeters() - y0.millimeters();
        assert!(
            (dx.hypot(dy) - 40.0).abs() < 1e-6,
            "l'affichage tient la forme saine, pas les graines"
        );
    }

    /// La coïncidence traverse le fil de travail : deux points
    /// distincts se superposent après résolution (ADR-0018 D3).
    #[test]
    fn la_coincidence_traverse_le_fil_de_travail() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("premier point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(30.0),
                y: Length::from_millimeters(40.0),
            },
        })
        .expect("second point");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Coincidence { p1: 0, p2: 1 },
        })
        .expect("contrainte posée");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let (_, resultat) = app.croquis_resolu.expect("croquis résolu");
        let resolu = resultat.expect("résolution réussie");
        let (x1, y1) = resolu.position(0).expect("premier point résolu");
        let (x2, y2) = resolu.position(1).expect("second point résolu");
        assert!(x1.approx_eq(x2) && y1.approx_eq(y2), "points superposés");
    }

    /// Un clic de l'outil Point se colle au pas de grille le plus
    /// proche (ADR-0018, lot 3c-2).
    #[test]
    fn le_clic_de_point_vise_la_grille() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Point;
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 1.24, pas * 0.67);
        let esquisse = app.session.esquisse();
        assert_eq!(esquisse.entites().len(), 1, "un point posé");
        let EntiteEsquisse::Point { x, y } = esquisse.entites()[0].1 else {
            panic!("l'entité posée est un point");
        };
        assert_eq!(x.millimeters(), pas, "abscisse collée au pas");
        assert_eq!(y.millimeters(), pas, "ordonnée collée au pas");
    }

    /// Le clic reprend le point existant le plus proche sous le
    /// seuil d'un quart de pas, et pose un point neuf collé à la
    /// grille au-delà (ADR-0018, lot 3c-2).
    #[test]
    fn le_clic_s_accroche_aux_points_existants() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Point;
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 0.24, pas * 0.1);
        assert_eq!(
            app.session.esquisse().entites().len(),
            1,
            "premier point posé"
        );
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 0.1, pas * 0.05);
        assert_eq!(
            app.session.esquisse().entites().len(),
            1,
            "point proche repris"
        );
        atelier_esquisse::traiter_clic(&mut app, pas, pas, 0.0);
        let esquisse = app.session.esquisse();
        assert_eq!(esquisse.entites().len(), 2, "un point neuf posé");
        let EntiteEsquisse::Point { x, y } = esquisse.entites()[1].1 else {
            panic!("l'entité posée est un point");
        };
        assert_eq!(x.millimeters(), pas, "abscisse au pas");
        assert_eq!(y.millimeters(), 0.0, "ordonnée nulle");
    }

    /// L'outil Ligne pose l'origine au premier clic, l'extrémité
    /// au second, et l'entité ligne les relie (ADR-0018,
    /// lot 3c-2).
    #[test]
    fn l_outil_ligne_pose_deux_points_et_une_ligne() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Ligne;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        assert!(app.geste.is_some(), "geste ouvert à l'origine");
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 2.0, pas);
        assert!(app.geste.is_none(), "geste clos à l'extrémité");
        let esquisse = app.session.esquisse();
        assert_eq!(esquisse.entites().len(), 3, "deux points et une ligne");
        let EntiteEsquisse::Ligne { p1, p2 } = esquisse.entites()[2].1 else {
            panic!("l'entité posée est une ligne");
        };
        assert_eq!(p1, 0, "origine au premier clic");
        assert_eq!(p2, 1, "extrémité au second clic");
    }

    /// L'outil Cercle pose le centre au premier clic, le rayon au
    /// second — sans point d'extrémité (ADR-0018, lot 3c-2).
    #[test]
    fn l_outil_cercle_pose_le_centre_puis_le_rayon() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Cercle;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, pas * 2.0);
        let esquisse = app.session.esquisse();
        assert_eq!(esquisse.entites().len(), 2, "le centre et le cercle");
        let EntiteEsquisse::Cercle { centre, rayon } = esquisse.entites()[1].1 else {
            panic!("l'entité posée est un cercle");
        };
        assert_eq!(centre, 0, "centre au premier clic");
        assert_eq!(rayon.millimeters(), pas * 2.0, "rayon du centre au clic");
    }

    /// Le balayage est signé (ADR-0020) : une fin « avant » le début
    /// dans le sens trigonométrique balaie dans le sens horaire — le
    /// sens suit le geste au lieu d'enrouler le cercle.
    #[test]
    fn le_balayage_de_l_arc_est_signe() {
        let angles = atelier_esquisse::angles_arc(
            forge_units::Angle::from_radians(0.5),
            forge_units::Angle::from_radians(0.2),
            8,
        );
        assert_eq!(angles.len(), 9, "segments + bornes");
        assert!((angles[0] - 0.5).abs() < 1e-12, "le départ est conservé");
        assert!(
            (angles[8] - 0.2).abs() < 1e-12,
            "arrivée au sens horaire à l'angle de fin"
        );
    }

    /// Une fin confondue avec le début dessine le tour entier, dans
    /// le sens trigonométrique (ADR-0020).
    #[test]
    fn l_arc_confondu_balaie_le_tour_entier() {
        let angles = atelier_esquisse::angles_arc(
            forge_units::Angle::from_radians(0.5),
            forge_units::Angle::from_radians(0.5),
            8,
        );
        assert!(
            (angles[8] - (0.5 + std::f64::consts::TAU)).abs() < 1e-12,
            "le tour entier depuis le départ"
        );
    }

    /// Le sens de l'arc suit le geste (ADR-0020) : un troisième clic
    /// « avant » le début dans le sens trigonométrique balaie dans
    /// le sens horaire.
    #[test]
    fn l_arc_balaie_dans_le_sens_du_geste() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Arc;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 2.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, pas * -2.0);
        let esquisse = app.session.esquisse();
        let EntiteEsquisse::Arc { angle_fin, .. } = esquisse.entites()[3].1 else {
            panic!("l'entité posée est un arc");
        };
        assert!(
            (angle_fin.radians() + std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "fin « avant » le début : balayage horaire"
        );
    }

    /// La sélection atteint les cercles (ADR-0020) : un clic sur la
    /// courbe sélectionne l'entité, pas seulement les points et les
    /// lignes.
    #[test]
    fn la_selection_atteint_les_cercles() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Cercle;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 2.0, 0.0);
        app.outil = atelier_esquisse::OutilEsquisse::Selection;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, pas * 2.0);
        assert_eq!(app.selection, vec![1], "le cercle visé est sélectionné");
    }

    /// La page d'aide suit le code terminal du message (ADR-0020) :
    /// le refus de la session enveloppe le code de la cause, le
    /// dernier code du message est la page à ouvrir.
    #[test]
    fn la_page_d_aide_suivit_le_code_terminal() {
        let mut message =
            String::from("OF-SES-0002 : la projection « values » rejette l'événement : ");
        message.push_str("OF-VAL-0007 : « angle » en deg lit « largeur » en mm");
        assert_eq!(aide_du_code(&message), "aide-of-val-0007");
        assert_eq!(
            aide_du_code("OF-VAL-0001 : cycle dans le graphe de liaisons : b → a → b"),
            "aide-of-val-0001"
        );
    }

    /// L'outil Arc pose le centre, le début puis la fin : trois
    /// points et l'arc, angles au centre aux extrémités
    /// (ADR-0018, lot 3c-2).
    #[test]
    fn l_outil_arc_pose_trois_points_et_un_arc() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Arc;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 2.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, pas * 2.0);
        assert!(app.geste.is_none(), "geste clos à la fin de l'arc");
        let esquisse = app.session.esquisse();
        assert_eq!(esquisse.entites().len(), 4, "trois points et un arc");
        let EntiteEsquisse::Arc {
            centre,
            debut,
            fin,
            rayon,
            angle_debut,
            angle_fin,
        } = esquisse.entites()[3].1
        else {
            panic!("l'entité posée est un arc");
        };
        assert_eq!(centre, 0, "centre au premier clic");
        assert_eq!(debut, 1, "début au second clic");
        assert_eq!(fin, 2, "fin au troisième clic");
        assert_eq!(rayon.millimeters(), pas * 2.0, "rayon du début");
        assert_eq!(angle_debut.radians(), 0.0, "angle de début nul");
        assert!(
            (angle_fin.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "angle de fin droit"
        );
    }

    /// L'outil Sélection ne pose rien : aucun geste n'ouvre, le
    /// journal reste vierge (ADR-0018, lot 3c-2).
    #[test]
    fn l_outil_selection_ne_pose_rien() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.outil = atelier_esquisse::OutilEsquisse::Selection;
        atelier_esquisse::traiter_clic(&mut app, pas, 0.0, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, pas, pas);
        assert_eq!(app.session.esquisse().entites().len(), 0, "aucune entité");
        assert!(app.geste.is_none(), "aucun geste");
    }

    /// L'affichage du croquis préfère les positions résolues du
    /// cache et retombe sur les graines du journal sans résolution
    /// (ADR-0018 D7, lot 3c).
    #[test]
    fn l_affichage_prefere_les_positions_resolues() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("premier point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(30.0),
                y: Length::from_millimeters(40.0),
            },
        })
        .expect("second point");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Coincidence { p1: 0, p2: 1 },
        })
        .expect("contrainte posée");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let (_, resultat) = app.croquis_resolu.expect("croquis résolu");
        let resolu = resultat.expect("résolution réussie");
        let esquisse = app.session.esquisse();
        // Sans résolution : les graines distinctes du journal.
        let (x0, y0) = atelier_esquisse::position_point(esquisse, None, 0).expect("graine 0");
        let (x1, y1) = atelier_esquisse::position_point(esquisse, None, 1).expect("graine 1");
        assert!(
            !x0.approx_eq(x1) || !y0.approx_eq(y1),
            "les graines du journal restent distinctes"
        );
        // Avec la résolution : les points coïncident.
        let (x0, y0) = atelier_esquisse::position_point(esquisse, Some(&resolu), 0)
            .expect("position résolue du premier point");
        let (x1, y1) = atelier_esquisse::position_point(esquisse, Some(&resolu), 1)
            .expect("position résolue du second point");
        assert!(
            x0.approx_eq(x1) && y0.approx_eq(y1),
            "les positions résolues coïncident"
        );
    }

    /// L'outil Sélection vise le point le plus proche au clic et
    /// vide la sélection ailleurs (ADR-0018 D5, lot 3c-3) : aucun
    /// geste n'ouvre, le journal ne grandit pas.
    #[test]
    fn la_selection_vise_le_point_le_plus_proche() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.outil = atelier_esquisse::OutilEsquisse::Selection;
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 0.1, 0.0);
        assert_eq!(app.selection, vec![0], "le point proche est sélectionné");
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 4.0, 0.0);
        assert!(app.selection.is_empty(), "un clic loin vide la sélection");
    }

    /// Le relâchement d'une poignée pose la graine par `MovePoint`
    /// (ADR-0018 D4, lot 3c-3) : l'aperçu du glissement déplace le
    /// point saisi sans écrire au journal, le relâchement écrit un
    /// événement et la projection porte la nouvelle graine.
    #[test]
    fn le_relachement_deplace_le_point_par_move_point() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.outil = atelier_esquisse::OutilEsquisse::Selection;
        app.saisie = Some(0);
        let empreinte_avant = app.empreinte_journal();
        // Aperçu du glissement : le point saisi suit le curseur,
        // la graine du journal ne bouge pas.
        let (id, x, y) = atelier_esquisse::cible_du_geste(&app, pas, pas * 2.0, pas)
            .expect("visée du point saisi");
        assert_eq!(id, 0, "le point saisi");
        assert_eq!(x.millimeters(), pas * 2.0, "abscisse visée en aperçu");
        assert_eq!(y.millimeters(), pas, "ordonnée visée en aperçu");
        assert_eq!(
            app.empreinte_journal(),
            empreinte_avant,
            "l'aperçu n'écrit rien au journal"
        );
        // Relâchement : la graine part à la position visée, un
        // événement de plus au journal.
        atelier_esquisse::relacher_poignee(&mut app, pas, pas * 2.0, pas);
        assert_eq!(
            app.empreinte_journal().0,
            empreinte_avant.0 + 1,
            "un événement écrit au relâchement"
        );
        let (x, y) = app
            .session
            .esquisse()
            .position_graine(0)
            .expect("graine du point déplacé");
        assert_eq!(x.millimeters(), pas * 2.0, "graine à l'abscisse visée");
        assert_eq!(y.millimeters(), pas, "graine à l'ordonnée visée");
    }

    /// L'aperçu de glissement suit le geste en direct (re-test du
    /// 2026-10-23) : la résolution d'aperçu part au fil de travail
    /// à la position visée — une seule par position — le point
    /// saisi y est résolu et le journal ne bouge pas ; l'échec est
    /// silencieux, il ne remonte jamais comme erreur.
    #[test]
    fn l_apercu_de_glissement_suite_le_geste_en_direct() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.saisie = Some(0);
        let empreinte_avant = app.empreinte_journal();
        app.demande_apercu_glissement(
            0,
            Length::from_millimeters(20.0),
            Length::from_millimeters(10.0),
        );
        assert!(
            app.apercu_en_attente,
            "l'aperçu est parti au fil de travail"
        );
        // La même position ne repart pas : un seul aperçu en vol.
        app.demande_apercu_glissement(
            0,
            Length::from_millimeters(20.0),
            Length::from_millimeters(10.0),
        );
        assert_eq!(
            app.travaux_en_attente, 1,
            "une seule résolution d'aperçu en vol"
        );
        // Pompage du retour : la résolution d'aperçu revient.
        for _ in 0..2_000 {
            if !app.apercu_en_attente {
                break;
            }
            let recus: Vec<Retour> = app.retours.try_iter().collect();
            app.travaux_en_attente = app.travaux_en_attente.saturating_sub(recus.len() as u32);
            for retour in garde_actuels(recus, app.generation) {
                app.appliquer_retour(retour);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            !app.apercu_en_attente,
            "l'aperçu est revenu du fil de travail"
        );
        let resolu = app
            .croquis_apercu
            .as_ref()
            .and_then(|resultat| resultat.as_ref().ok())
            .expect("aperçu résolu");
        assert!(
            proche(resolu.position(0), 20.0, 10.0),
            "le point saisi est à la visée"
        );
        assert_eq!(
            app.empreinte_journal(),
            empreinte_avant,
            "l'aperçu n'écrit rien au journal"
        );
    }

    /// Pendant le glissement, aucune position brute du curseur ne
    /// s'affiche (re-test du 24-10) : la visée reste la graine de
    /// l'aperçu, jamais une position d'affichage — le dessin ne
    /// montre que des états résolus, une ligne contrainte
    /// horizontalement ou verticalement ne devient jamais une
    /// diagonale, même le temps d'un aller-retour de résolution.
    #[test]
    fn pendant_le_glissement_aucune_position_brute_ne_s_affiche() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("premier point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(pas * 4.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("second point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 2 },
        })
        .expect("contrainte horizontale");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Glissement diagonal du second point : la visée part à
        // l'aperçu, aucune surcharge d'affichage ne la montre.
        app.saisie = Some(1);
        let (surcharge, cible) =
            atelier_esquisse::affichage_du_geste(&app, pas, Some((pas * 5.0, pas * 2.0)));
        assert!(
            surcharge.is_none(),
            "aucune position brute du curseur ne s'affiche pendant le glissement"
        );
        let (id, vise_x, vise_y) = cible.expect("visée du geste à l'aperçu");
        assert_eq!(id, 1, "le point saisi");
        assert_eq!(vise_x.millimeters(), pas * 5.0, "abscisse visée");
        assert_eq!(vise_y.millimeters(), pas * 2.0, "ordonnée visée");
    }

    /// L'aperçu d'un geste sur une ligne contrainte horizontale
    /// garde la ligne horizontale (re-test du 24-10) : la
    /// résolution d'aperçu épingle le point saisi à la visée et la
    /// contrainte entraîne l'autre extrémité — la forme affichée
    /// ne montre jamais une diagonale.
    #[test]
    fn l_apercu_du_geste_sur_une_ligne_horizontale_reste_horizontale() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("premier point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(pas * 4.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("second point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Horizontale { ligne: 2 },
        })
        .expect("contrainte horizontale");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Geste diagonal : l'aperçu épingle le point saisi à la
        // visée, la contrainte tient l'ordonnée des deux bouts.
        app.saisie = Some(1);
        app.demande_apercu_glissement(
            1,
            Length::from_millimeters(pas * 5.0),
            Length::from_millimeters(pas * 2.0),
        );
        pomper_l_apercu(&mut app);
        let resolu = app
            .croquis_apercu
            .as_ref()
            .and_then(|resultat| resultat.as_ref().ok())
            .expect("aperçu résolu");
        assert!(
            proche(resolu.position(1), pas * 5.0, pas * 2.0),
            "le point saisi est épinglé à la visée"
        );
        let (_, y0) = resolu.position(0).expect("premier point résolu");
        let (_, y1) = resolu.position(1).expect("second point résolu");
        assert!(
            y0.approx_eq(y1),
            "la contrainte horizontale tient : même ordonnée aux deux bouts"
        );
    }

    /// Le vol du relâchement garde l'aperçu du geste comme
    /// résolution d'affichage (re-test du 24-10) : l'aperçu n'est
    /// purgé qu'une fois le frais du relâchement arrivé —
    /// l'affichage ne retombe jamais sur la résolution d'avant le
    /// glissement, même le temps du vol.
    #[test]
    fn le_vol_du_relachement_affiche_l_apercu_du_geste() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Geste : l'aperçu résout le croquis à la visée.
        app.saisie = Some(0);
        app.demande_apercu_glissement(
            0,
            Length::from_millimeters(pas * 2.0),
            Length::from_millimeters(pas),
        );
        pomper_l_apercu(&mut app);
        // Relâchement : la graine part au journal, le frais vole —
        // l'aperçu du geste reste l'affichage du vol.
        atelier_esquisse::relacher_poignee(&mut app, pas, pas * 2.0, pas);
        assert!(
            atelier_esquisse::surcharge_du_relachement(&app).is_some(),
            "le vol du relâchement tient la surcharge anti-flash"
        );
        assert!(
            app.resolution_du_geste().is_some(),
            "l'aperçu du geste reste la résolution d'affichage pendant le vol"
        );
        // Fin du vol : le frais remplace l'aperçu, la purge suit.
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        assert!(
            atelier_esquisse::surcharge_du_relachement(&app).is_none(),
            "le frais remplace la surcharge du relâchement"
        );
        assert!(
            app.resolution_du_geste().is_none(),
            "l'aperçu se purge une fois le frais arrivé"
        );
    }

    /// Le relâchement s'accroche aux autres points, jamais au
    /// point saisi lui-même (ADR-0018 D4, lot 3c-3) : la graine du
    /// point déplacé colle au point visé.
    #[test]
    fn le_relachement_s_accroche_aux_autres_points() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("premier point");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(pas),
                y: Length::from_millimeters(pas),
            },
        })
        .expect("second point");
        app.saisie = Some(0);
        // Relâchement près du second point : la graine du point
        // saisi s'accroche à lui, jamais à elle-même.
        atelier_esquisse::relacher_poignee(&mut app, pas, pas * 1.1, pas);
        let (x, y) = app
            .session
            .esquisse()
            .position_graine(0)
            .expect("graine du point déplacé");
        assert_eq!(x.millimeters(), pas, "graine collée au second point");
        assert_eq!(y.millimeters(), pas, "graine collée au second point");
    }

    /// Le relâchement ne fait pas flasher le croquis en arrière
    /// pendant le vol de sa résolution (anti-flash du relâchement,
    /// re-validation §7) : le point lâché reste affiché à sa
    /// position visée jusqu'à l'arrivée du frais — l'affichage ne
    /// retombe pas sur la résolution d'avant le glissement.
    #[test]
    fn le_relachement_ne_fait_pas_flasher_le_croquis_en_arriere() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Glissement puis relâchement sur (2·pas, pas) : la graine
        // part au journal, la résolution repart au fil de travail.
        app.saisie = Some(0);
        atelier_esquisse::relacher_poignee(&mut app, pas, pas * 2.0, pas);
        // Trame du relâchement : le frais n'est pas encore là — le
        // point lâché reste affiché à sa position visée.
        let (id, x, y) = atelier_esquisse::surcharge_du_relachement(&app)
            .expect("surcharge du point relâché pendant le vol");
        assert_eq!(id, 0, "le point relâché");
        assert_eq!(x.millimeters(), pas * 2.0, "abscisse visée au relâchement");
        assert_eq!(y.millimeters(), pas, "ordonnée visée au relâchement");
        // Fin du vol : la résolution fraîche remplace la surcharge.
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        assert!(
            atelier_esquisse::surcharge_du_relachement(&app).is_none(),
            "le frais remplace la surcharge du relâchement"
        );
    }

    /// La sélection garde au plus deux entités et retire celle
    /// re-cliquée (ADR-0018 D5, lot 3c-4) : les contraintes
    /// relient au plus deux entités, la plus ancienne cède sa
    /// place à la nouvelle.
    #[test]
    fn la_selection_borne_deux_entites() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [(0.0, 0.0), (pas, 0.0), (pas * 2.0, 0.0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point posé");
        }
        app.outil = atelier_esquisse::OutilEsquisse::Selection;
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 0.1, 0.0);
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 1.1, 0.0);
        assert_eq!(app.selection, vec![0, 1], "deux entités sélectionnées");
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 2.1, 0.0);
        assert_eq!(app.selection, vec![1, 2], "la plus ancienne cède sa place");
        atelier_esquisse::traiter_clic(&mut app, pas, pas * 1.1, 0.0);
        assert_eq!(app.selection, vec![2], "le re-clic retire de la sélection");
    }

    /// Une contrainte posée depuis l'atelier passe par la session
    /// (ADR-0018 D2, lot 3c-4) : un événement de plus au journal,
    /// la contrainte visible ; un refus n'écrit rien et le message
    /// d'état porte l'erreur (ADR-0016 C4).
    #[test]
    fn la_contrainte_de_l_atelier_passe_par_la_session() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [(0.0, 0.0), (pas, 0.0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point posé");
        }
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne posée");
        app.selection = vec![2];
        let empreinte_avant = app.empreinte_journal();
        atelier_esquisse::ajouter_contrainte(
            &mut app,
            ContrainteEsquisse::Horizontale { ligne: 2 },
        );
        assert_eq!(
            app.empreinte_journal().0,
            empreinte_avant.0 + 1,
            "un événement écrit"
        );
        assert_eq!(
            app.session.esquisse().contraintes().len(),
            1,
            "la contrainte est posée"
        );
        atelier_esquisse::ajouter_contrainte(
            &mut app,
            ContrainteEsquisse::Horizontale { ligne: 99 },
        );
        assert_eq!(
            app.empreinte_journal().0,
            empreinte_avant.0 + 1,
            "le refus n'écrit rien au journal"
        );
        assert!(app.error.is_some(), "le refus s'affiche en message d'état");
    }

    /// L'orientation opposée depuis l'atelier est refusée, comme
    /// dans FreeCAD (ADR-0021 D1 révisée le 2026-10-22) : une ligne
    /// déjà orientée refuse toute pose d'orientation — même genre
    /// (redondance) ou genre opposé — sans rien écrire ; seule la
    /// pose effective surligne dans la liste (ADR-0021 D2).
    #[test]
    fn l_orientation_opposee_depuis_l_atelier_est_refusee() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        for (x, y) in [(0.0, 0.0), (40.0, 25.0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("point posé");
        }
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Ligne { p1: 0, p2: 1 },
        })
        .expect("ligne posée");
        app.selection = vec![2];
        atelier_esquisse::ajouter_contrainte(
            &mut app,
            ContrainteEsquisse::Horizontale { ligne: 2 },
        );
        assert_eq!(
            app.derniere_contrainte,
            Some((0, app.instant)),
            "la pose surligne la contrainte"
        );
        // Genre opposé : refusé (alignement FreeCAD), la
        // contrainte posée tient, rien d'autre au journal.
        atelier_esquisse::ajouter_contrainte(&mut app, ContrainteEsquisse::Verticale { ligne: 2 });
        let contraintes = app.session.esquisse().contraintes();
        assert_eq!(contraintes.len(), 1, "refus : rien d'ajouté");
        assert!(matches!(
            contraintes[0].1,
            ContrainteEsquisse::Horizontale { ligne: 2 }
        ));
        assert!(app.error.is_some(), "le refus s'affiche en message d'état");
        assert_eq!(
            app.derniere_contrainte,
            Some((0, app.instant)),
            "rien de neuf à surligner"
        );
        // Reposer le même genre : refusé, aucune nouvelle
        // surbrillance.
        atelier_esquisse::ajouter_contrainte(&mut app, ContrainteEsquisse::Verticale { ligne: 2 });
        assert!(app.error.is_some(), "le refus s'affiche en message d'état");
        assert_eq!(
            app.derniere_contrainte,
            Some((0, app.instant)),
            "rien de neuf à surligner"
        );
    }

    /// Un point ancré ne se saisit plus (ADR-0021 D3) : le grab est
    /// refusé et un déplacement scripté par `MovePoint` laisse la
    /// graine en place.
    #[test]
    fn un_point_ancre_ne_se_saisit_plus() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.selection = vec![0];
        atelier_esquisse::ajouter_contrainte(&mut app, ContrainteEsquisse::PointFixe { point: 0 });
        atelier_esquisse::saisir_poignee(&mut app, pas, 0.0, 0.0);
        assert!(app.saisie.is_none(), "le point ancré n'est pas saisi");
        // Un déplacement scripté est refusé : la graine tient.
        let _ = app.executer(&MovePoint {
            point: 0,
            x: Length::from_millimeters(pas),
            y: Length::from_millimeters(pas),
        });
        assert!(
            app.error.is_some(),
            "le déplacement d'un point ancré est refusé"
        );
        let (x, y) = app
            .session
            .esquisse()
            .position_graine(0)
            .expect("graine du point");
        assert_eq!(x.millimeters(), 0.0, "la graine tient");
        assert_eq!(y.millimeters(), 0.0, "la graine tient");
    }

    /// Un croquis entièrement contraint ne saisit pas ses poignées
    /// (re-test du 24-10) : zéro degré de liberté ne donne rien à
    /// absorber — FreeCAD épingle le point tiré par une contrainte
    /// temporaire et la forme ne suit jamais un geste impossible.
    /// Le glissement cède au pan de vue, comme pour un point ancré.
    #[test]
    fn un_croquis_entierement_contraint_ne_saisit_pas_ses_poignees() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [
            (0.0, 0.0),
            (pas * 4.0, 0.0),
            (pas * 4.0, pas * 3.0),
            (0.0, pas * 3.0),
        ] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("coin posé");
        }
        for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("côté posé");
        }
        for (ligne, horizontale) in [(4, true), (5, false), (6, true), (7, false)] {
            let contrainte = if horizontale {
                ContrainteEsquisse::Horizontale { ligne }
            } else {
                ContrainteEsquisse::Verticale { ligne }
            };
            app.executer(&AddConstraint { contrainte })
                .expect("contrainte posée");
        }
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 4.0),
                },
            },
        })
        .expect("cote posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 1,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 3.0),
                },
            },
        })
        .expect("cote posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("coin fixé");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Emprunt, pas déplacement : le test saisit une poignée
        // APRÈS la lecture du cache — `app` ne doit être que
        // partiellement emprunté, jamais déplacé (E0382).
        let resolu = app
            .croquis_resolu
            .as_ref()
            .and_then(|(_, resultat)| resultat.as_ref().ok())
            .expect("résolution réussie");
        assert_eq!(resolu.dof(), 0, "le rectangle est entièrement contraint");
        // Le coin 2 n'est ni ancré ni absent : seule l'absence de
        // degré de liberté doit refuser la saisie.
        atelier_esquisse::saisir_poignee(&mut app, pas, pas * 4.0, pas * 3.0);
        assert!(
            app.saisie.is_none(),
            "un croquis entièrement contraint ne saisit pas ses poignées"
        );
    }

    /// Fixer un point glissé ne le fait pas sauter (ADR-0021 D2,
    /// révisée le 2026-10-23 — re-test jalon 2, point 8) : le bouton
    /// « Point fixe » capture la position DESSINÉE — la résolution
    /// épinglée du cache, pas la passe de base — et l'ancre la fige :
    /// après re-résolution, le point reste exactement là où
    /// l'utilisateur le voyait.
    #[test]
    fn fixer_un_point_glisse_ne_le_fait_pas_sauter() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(0.0),
                y: Length::from_millimeters(0.0),
            },
        })
        .expect("point posé");
        app.executer(&AddEntity {
            entite: EntiteEsquisse::Point {
                x: Length::from_millimeters(80.0),
                y: Length::from_millimeters(-3.0),
            },
        })
        .expect("point posé");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(40.0),
                },
            },
        })
        .expect("distance posée");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        // Le geste : le point 0 part à (10, 20) — l'affichage
        // l'épingle là où on le lâche.
        app.executer(&MovePoint {
            point: 0,
            x: Length::from_millimeters(10.0),
            y: Length::from_millimeters(20.0),
        })
        .expect("déplacement");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let resolu = atelier_esquisse::resolu_affichage(&app).expect("résolution affichable");
        assert!(
            proche(resolu.position(0), 10.0, 20.0),
            "l'affichage épingle"
        );
        // Fixer : la position DESSINÉE est capturée et figée.
        atelier_esquisse::ancrer_point(&mut app, 0);
        assert_eq!(
            app.derniere_contrainte,
            Some((1, app.instant)),
            "la contrainte d'ancrage posée se surligne"
        );
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let ancre = atelier_esquisse::resolu_affichage(&app).expect("résolution affichable");
        assert!(
            proche(ancre.position(0), 10.0, 20.0),
            "le point fixé ne saute pas : la position affichée tient"
        );
    }

    /// Le rectangle contraint tombe à zéro degré de liberté
    /// (ADR-0018 D5, lot 3c-4) : quatre points partagés, quatre
    /// lignes, quatre contraintes horizontale et verticale, deux
    /// cotes de distance, un coin fixé — le DoF lu sur le cache
    /// résolu tombe à zéro.
    #[test]
    fn le_rectangle_contraint_tombe_a_zero_dof() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        let pas = pas_de_grille(app.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [
            (0.0, 0.0),
            (pas * 4.0, 0.0),
            (pas * 4.0, pas * 3.0),
            (0.0, pas * 3.0),
        ] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .expect("coin posé");
        }
        for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            app.executer(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .expect("côté posé");
        }
        for (ligne, horizontale) in [(4, true), (5, false), (6, true), (7, false)] {
            let contrainte = if horizontale {
                ContrainteEsquisse::Horizontale { ligne }
            } else {
                ContrainteEsquisse::Verticale { ligne }
            };
            app.executer(&AddConstraint { contrainte })
                .expect("contrainte posée");
        }
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 4.0),
                },
            },
        })
        .expect("cote posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 1,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 3.0),
                },
            },
        })
        .expect("cote posée");
        app.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .expect("coin fixé");
        app.demande_resolution_esquisse();
        pomper_les_retours(&mut app);
        let (_, resultat) = app.croquis_resolu.expect("croquis résolu");
        let resolu = resultat.expect("résolution réussie");
        assert_eq!(resolu.dof(), 0, "le rectangle est entièrement contraint");
    }

    /// L'export écrit au chemin cible via un temporaire synchronisé
    /// puis renommé : le fichier de l'utilisateur n'est jamais
    /// tronqué par un échec en cours d'écriture, aucun temporaire
    /// ne survit (audit n°2, N7).
    #[test]
    fn l_ecriture_d_export_est_atomique() {
        let cible = std::env::temp_dir().join("omnifab-audit2-export.svg");
        let _ = std::fs::remove_file(&cible);
        assert!(
            matches!(ecris_fichier(0, &cible, b"<svg/>"), Retour::Export { .. }),
            "l'export doit réussir"
        );
        assert_eq!(
            std::fs::read(&cible).expect("fichier écrit"),
            b"<svg/>".to_vec()
        );
        let mut tmp = cible.as_os_str().to_owned();
        tmp.push(".tmp");
        assert!(!PathBuf::from(tmp).exists(), "aucun temporaire résiduel");
        let _ = std::fs::remove_file(&cible);
        // un dossier parent absent : échec propre, retour d'erreur
        let absent = std::env::temp_dir().join("omnifab-audit2-absent/sortie.svg");
        assert!(
            matches!(ecris_fichier(0, &absent, b"x"), Retour::Erreur { .. }),
            "l'échec remonte comme erreur"
        );
    }

    /// L'hystérésis des redondances (ADR-0018 D5, lot 3c-4) : le
    /// diagnostic ne s'affiche que stable sur deux résolutions
    /// consécutives — jamais de faux conflit clignotant
    /// (FPA #106).
    #[test]
    fn les_redondances_demandent_deux_resolutions() {
        assert!(
            atelier_esquisse::redondances_stables(None, &[1]).is_none(),
            "une première résolution ne s'affiche pas"
        );
        assert_eq!(
            atelier_esquisse::redondances_stables(Some(&[1]), &[1]),
            Some(vec![1]),
            "le diagnostic stable s'affiche"
        );
        assert!(
            atelier_esquisse::redondances_stables(Some(&[1]), &[2]).is_none(),
            "un diagnostic qui change ne s'affiche pas"
        );
    }
}
