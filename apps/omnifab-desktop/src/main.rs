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
//!   fichier invalide n'est plus re-parsé à chaque frame ;
//! - **protection du travail** (ADR-0016 C3) : indicateur « non
//!   enregistré » posé par toute commande effective et levé par la
//!   sauvegarde — le titre de la fenêtre porte `*`, « Nouveau »,
//!   « Ouvrir » et « Reprendre » demandent confirmation, et
//!   l'enregistrement ne casse plus la pile redo ;
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
mod bibliotheque;
mod fichiers;
mod i18n;

use aide::avec_aide;
use bibliotheque::EntreeProjet;
use eframe::egui;
use forge_doc::Document;
use forge_geom::{Mesh, Path2, Point3};
use forge_i18n::Argument;
use forge_io::{dxf, stl, svg};
use forge_render::{
    Boite2, Boite3D, Camera2D, Camera3D, Couleur, Renderer2D, Scene2D, Scene3D, Segment2D,
    Segment3D, dessiner_scene, dessiner_wireframe, pas_de_grille,
};
use forge_session::{
    Command, EVENT_EXPRESSION_SET, ImportMesh, ImportTraces, Session, SetExpression, SetInput,
};
use forge_units::Length;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// État de rendu wgpu d'eframe : alias pour alléger la signature du
/// constructeur (ADR-0015).
type RenderState = eframe::egui_wgpu::RenderState;

/// Acteur des événements écrits par le démonstrateur (ADR-0011).
const ACTEUR: &str = "desktop.demo";
/// Expression initiale de la diagonale du rectangle.
const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(width_mm * width_mm + height_mm * height_mm)";
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
const GRILLE_CIBLE_PX: f64 = 80.0;
/// Seuil de détection d'un export en mètres : un maillage dont la
/// plus grande dimension est sous ce seuil (en millimètres) propose
/// l'échelle × 1000 à l'import (ADR-0017).
const SEUIL_METRES: f64 = 1.0;
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

/// Travail confié au fil de travail : lecture et parse d'un
/// import, parse et écriture d'un export — jamais sur le fil
/// d'interface (ADR-0016 C1).
enum Travail {
    /// Lecture puis parse d'un fichier de tracés (SVG ou DXF).
    ImportTraces {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Format attendu : « svg » ou « dxf ».
        format: &'static str,
        /// Fichier à lire.
        chemin: PathBuf,
    },
    /// Lecture puis parse d'un maillage STL.
    ImportMaillage {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Fichier à lire.
        chemin: PathBuf,
    },
    /// Re-parse du blob d'origine puis export des tracés.
    ExportTraces {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Format du blob d'origine : « svg » ou « dxf ».
        format_origine: &'static str,
        /// Format d'export demandé : « svg » ou « dxf ».
        format: &'static str,
        /// Octets du blob d'origine.
        octets: Vec<u8>,
        /// Fichier de destination.
        chemin: PathBuf,
    },
    /// Re-parse du blob d'origine puis export du maillage en STL.
    ExportMaillage {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Octets du blob d'origine.
        octets: Vec<u8>,
        /// Échelle d'import appliquée avant la sérialisation : l'export
        /// écrit toujours des millimètres (ADR-0017).
        echelle: f64,
        /// Fichier de destination.
        chemin: PathBuf,
    },
}

impl Travail {
    /// Exécute le travail hors du fil d'interface : tout échec
    /// devient un retour d'erreur à message (ADR-0016 C1).
    fn executer(self) -> Retour {
        match self {
            Travail::ImportTraces {
                generation,
                format,
                chemin,
            } => {
                let octets = match std::fs::read(&chemin) {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                let parse = if format == "svg" {
                    svg::import(&octets, &svg::SvgBudgets::default())
                } else {
                    dxf::import(&octets, &dxf::DxfBudgets::default())
                };
                match parse {
                    Ok(traces) => {
                        let nombre_traces = traces.len();
                        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
                        Retour::ImportTraces {
                            generation,
                            format,
                            octets,
                            traces: nombre_traces,
                            sommets,
                        }
                    }
                    Err(error) => erreur_de_travail(generation, error),
                }
            }
            Travail::ImportMaillage { generation, chemin } => {
                let octets = match std::fs::read(&chemin) {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                match stl::import(&octets, &stl::StlBudgets::default()) {
                    Ok(maillage) => {
                        let triangles = maillage.triangle_count();
                        let dimension = plus_grande_dimension(&maillage);
                        Retour::ImportMaillage {
                            generation,
                            octets,
                            triangles,
                            dimension,
                        }
                    }
                    Err(error) => erreur_de_travail(generation, error),
                }
            }
            Travail::ExportTraces {
                generation,
                format_origine,
                format,
                octets,
                chemin,
            } => {
                let parse = if format_origine == "svg" {
                    svg::import(&octets, &svg::SvgBudgets::default())
                } else {
                    dxf::import(&octets, &dxf::DxfBudgets::default())
                };
                let traces = match parse {
                    Ok(traces) => traces,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                let sortie = if format == "svg" {
                    svg::export(&traces)
                } else {
                    dxf::export(&traces)
                };
                let octets = match sortie {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                ecris_fichier(generation, &chemin, &octets)
            }
            Travail::ExportMaillage {
                generation,
                octets,
                echelle,
                chemin,
            } => {
                let maillage = match stl::import(&octets, &stl::StlBudgets::default()) {
                    Ok(maillage) => maillage,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                let maillage = match maillage_echelle(maillage, echelle) {
                    Ok(maillage) => maillage,
                    Err(erreur) => return erreur_de_travail(generation, erreur),
                };
                let octets = match stl::export(&maillage) {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                ecris_fichier(generation, &chemin, &octets)
            }
        }
    }
}

/// Retour du fil de travail : import réussi (octets d'origine et
/// compteurs), export écrit, ou échec à message (ADR-0016 C1).
enum Retour {
    /// Tracés lus et parsés : octets d'origine pour le blob,
    /// compteurs pour le journal.
    ImportTraces {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Format du fichier importé : « svg » ou « dxf ».
        format: &'static str,
        /// Octets d'origine.
        octets: Vec<u8>,
        /// Nombre de tracés.
        traces: usize,
        /// Nombre de sommets.
        sommets: usize,
    },
    /// Maillage lu et parsé : octets d'origine, compte de triangles
    /// et plus grande dimension (ADR-0017).
    ImportMaillage {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Octets d'origine.
        octets: Vec<u8>,
        /// Nombre de triangles.
        triangles: usize,
        /// Plus grande dimension du maillage, en millimètres
        /// (ADR-0017).
        dimension: f64,
    },
    /// Export écrit au chemin donné.
    Export {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Chemin du fichier écrit.
        chemin: String,
    },
    /// Échec du travail : message affiché au canal d'erreur.
    Erreur {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Message d'échec.
        message: String,
    },
}

impl Retour {
    /// Génération de l'état au moment de l'envoi du travail.
    fn generation(&self) -> u64 {
        match self {
            Retour::ImportTraces { generation, .. }
            | Retour::ImportMaillage { generation, .. }
            | Retour::Export { generation, .. }
            | Retour::Erreur { generation, .. } => *generation,
        }
    }
}

/// Boucle du fil de travail : chaque travail reçu est exécuté et son
/// retour émis ; le fil s'éteint quand l'émetteur des travaux est
/// abandonné (ADR-0016 C1).
fn fil_de_travail(recepteur: mpsc::Receiver<Travail>, emetteur: mpsc::Sender<Retour>) {
    for travail in recepteur {
        let _ = emetteur.send(travail.executer());
    }
}

/// Plus grande dimension du maillage, en millimètres : le plus grand
/// côté de la boîte englobante (ADR-0017). Un maillage vide mesure 0.
fn plus_grande_dimension(maillage: &Mesh) -> f64 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in maillage.vertices() {
        let coordonnees = [
            point.x().millimeters(),
            point.y().millimeters(),
            point.z().millimeters(),
        ];
        for axe in 0..3 {
            min[axe] = min[axe].min(coordonnees[axe]);
            max[axe] = max[axe].max(coordonnees[axe]);
        }
    }
    let mut dimension = 0.0;
    for axe in 0..3 {
        dimension = dimension.max(max[axe] - min[axe]);
    }
    dimension
}

/// Un maillage dont la plus grande dimension est sous le seuil
/// ressemble à un export en mètres (Blender exporte en mètres) :
/// l'import propose l'échelle × 1000 (ADR-0017). Un maillage vide ou
/// déjà grand ne déclenche rien.
fn propose_echelle_metres(dimension: f64) -> bool {
    (0.0..SEUIL_METRES).contains(&dimension)
}

/// Applique l'échelle d'import aux coordonnées du maillage (ADR-0017) :
/// l'événement porte le facteur, le blob garde les octets bruts — le
/// rendu et l'export travaillent sur le maillage mis à l'échelle.
fn maillage_echelle(maillage: Mesh, echelle: f64) -> Result<Mesh, String> {
    if echelle == 1.0 {
        return Ok(maillage);
    }
    let triangles = maillage.triangles().to_vec();
    let mut sommets = Vec::with_capacity(maillage.vertex_count());
    for point in maillage.vertices() {
        let x = Length::from_millimeters(point.x().millimeters() * echelle);
        let y = Length::from_millimeters(point.y().millimeters() * echelle);
        let z = Length::from_millimeters(point.z().millimeters() * echelle);
        sommets.push(Point3::new(x, y, z).map_err(|erreur| erreur.to_string())?);
    }
    Mesh::from_parts(sommets, triangles).map_err(|erreur| erreur.to_string())
}

/// Ne garde que les retours de la génération courante : les travaux
/// lancés avant une réinitialisation ou une ouverture sont
/// obsolètes, leurs résultats ne touchent plus l'état (ADR-0016 C1).
fn garde_actuels(retours: Vec<Retour>, generation: u64) -> Vec<Retour> {
    retours
        .into_iter()
        .filter(|retour| retour.generation() == generation)
        .collect()
}

/// Mémorise le résultat d'un parse dans le cache, clé à l'appui :
/// la même clé n'est plus re-parsée, même en échec (ADR-0016 C1) —
/// un fichier invalide ne revient pas à chaque frame. Retourne
/// « true » si un parse a eu lieu.
fn met_en_cache<T>(
    cache: &mut Option<(String, Result<T, String>)>,
    cle: String,
    parse: impl FnOnce() -> Result<T, String>,
) -> bool {
    if cache.as_ref().is_some_and(|(nom, _)| *nom == cle) {
        return false;
    }
    *cache = Some((cle, parse()));
    true
}

/// Raccourcis globaux d'annulation et de rétablissement
/// (ADR-0016 C2) : `command` (Ctrl sous Windows/Linux, Cmd sous
/// macOS) avec Z annule, avec Y ou Maj+Z rétablit. Aucun raccourci
/// quand un champ de texte a le focus (`clavier_pris`) : la saisie
/// en cours n'est pas écrasée par un Ctrl+Z global.
///
/// Retourne `(annuler, retablir)`.
fn raccourcis_annulation(
    modifiers: &egui::Modifiers,
    z: bool,
    y: bool,
    clavier_pris: bool,
) -> (bool, bool) {
    if clavier_pris {
        return (false, false);
    }
    let cmd = modifiers.command;
    let maj = modifiers.shift;
    (cmd && !maj && z, cmd && (y || maj && z))
}

/// Retour d'erreur du fil de travail : message à afficher au canal
/// d'erreur (ADR-0016 C1).
fn erreur_de_travail(generation: u64, erreur: impl std::fmt::Display) -> Retour {
    Retour::Erreur {
        generation,
        message: erreur.to_string(),
    }
}

/// Écrit les octets exportés au chemin choisi : retour d'export ou
/// d'erreur (ADR-0016 C1).
fn ecris_fichier(generation: u64, chemin: &Path, octets: &[u8]) -> Retour {
    if let Err(error) = std::fs::write(chemin, octets) {
        return erreur_de_travail(generation, error);
    }
    Retour::Export {
        generation,
        chemin: chemin.display().to_string(),
    }
}

struct DemoApp {
    /// Session de travail : document, projections, undo/redo
    /// (ADR-0008/0011).
    session: Session,
    /// Tampon d'édition de l'expression de la diagonale.
    expr_buf: String,
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
    tampon_dimensions: BTreeMap<&'static str, (f64, bool)>,
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
    /// Choix des unités d'un maillage minuscule, en attente de la
    /// réponse de l'utilisateur (ADR-0017).
    demande_echelle: Option<DemandeEchelle>,
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
            expr_buf: String::new(),
            error: None,
            info: None,
            i18n,
            aide: None,
            ecran: Ecran::Accueil,
            chemin_projet: None,
            cache: None,
            cache_maillage: None,
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
            demande_echelle: None,
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
        self.expr_buf = DEFAULT_DIAGONAL_EXPR.to_string();
        self.effacer_messages();
        self.cache = None;
        self.cache_maillage = None;
        self.camera = Camera2D::nouvelle();
        self.camera_cle = None;
        self.camera3d = Camera3D::nouvelle();
        self.vue = Vue::DeuxD;
        self.aide = None;
        self.demande_echelle = None;
        self.chemin_projet = None;
        let _ = self.executer(&SetInput {
            name: "width_mm".into(),
            value: 800.0,
        });
        let _ = self.executer(&SetInput {
            name: "height_mm".into(),
            value: 600.0,
        });
        let _ = self.executer(&SetExpression {
            name: "diagonal".into(),
            expression: DEFAULT_DIAGONAL_EXPR.into(),
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
    /// affiche un succès, jamais après un refus (ADR-0016 C4).
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
            self.expr_buf = self.expression_courante();
        }
    }

    /// Rétablit le groupe défait (bouton ou raccourci) : le tampon
    /// d'édition est rafraîchi et le travail non enregistré suit le
    /// journal (ADR-0016 C3).
    fn retablir(&mut self) {
        self.effacer_messages();
        if self.session.redo().is_ok() {
            self.dirty = true;
            self.expr_buf = self.expression_courante();
        }
    }

    /// Traduit une clé du catalogue embarqué.
    fn texte(&self, cle: &str) -> String {
        self.i18n
            .traduction(cle)
            .expect("clé présente dans le FTL (test de la CI)")
    }

    /// Traduit une clé du catalogue embarqué, avec arguments.
    fn texte_avec(&self, cle: &str, arguments: &[(&str, Argument)]) -> String {
        self.i18n
            .traduction_avec(cle, arguments)
            .expect("clé présente dans le FTL (test de la CI)")
    }

    /// Dernière expression de la diagonale écrite au journal, ou
    /// l'expression par défaut (tampon d'édition après undo/redo ou
    /// ouverture).
    fn expression_courante(&self) -> String {
        self.session
            .document()
            .events()
            .iter()
            .rev()
            .filter(|event| event.event_type == EVENT_EXPRESSION_SET)
            .find_map(|event| {
                let nom = event.data.get("name")?.as_str()?;
                let expression = event.data.get("expression")?.as_str()?;
                (nom == "diagonal").then_some(expression.to_string())
            })
            .unwrap_or_else(|| DEFAULT_DIAGONAL_EXPR.to_string())
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
        self.chemin_projet = Some(chemin.to_path_buf());
        self.ecran = Ecran::Atelier;
        self.expr_buf = self.expression_courante();
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
    /// travail non enregistré serait perdu.
    fn fenetre_confirmation(&mut self, ctx: &egui::Context) {
        let Some(action) = self.demande.clone() else {
            return;
        };
        let mut decision: Option<bool> = None;
        egui::Window::new(self.texte("confirmation-titre"))
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(self.texte("confirmation-texte"));
                ui.horizontal(|ui| {
                    let reponse = ui.button(self.texte("confirmation-confirmer"));
                    avec_aide(&reponse, "aide-confirmation-perte", &mut self.aide);
                    if reponse.clicked() {
                        decision = Some(true);
                    }
                    let reponse = ui.button(self.texte("confirmation-annuler"));
                    if reponse.clicked() {
                        decision = Some(false);
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
    /// disponible, la demande est abandonnée en silence.
    fn envoyer_travail(&mut self, travail: Travail) {
        let Some(emetteur) = &self.travaux else {
            return;
        };
        if emetteur.send(travail).is_ok() {
            self.travaux_en_attente += 1;
            self.info = Some(self.texte("info-travail-en-cours"));
        }
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
        }
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
    /// dessiné par le GPU (ADR-0015, phase C2), sans plafond. Le
    /// STL reste visible en vue 2D sans repasser par le painter
    /// (validation C2, round 5).
    fn ajouter_maillage_2d(scene: &mut Scene2D, maillage: &Mesh) {
        let sommets = maillage.vertices();
        for triangle in maillage.triangles() {
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
    /// maillage. L'ordre d'ajout est la priorité d'affichage : le
    /// pipeline de lignes ne teste pas la profondeur, le dernier
    /// segment dessiné recouvre les précédents — la grille passe
    /// sous les tracés et le rectangle, qui ne disparaissent plus
    /// quand ils coïncident avec elle (validation C2, round 2).
    fn scene_vue_3d(
        traces: &[Path2],
        maillage: Option<&Mesh>,
        width: f64,
        height: f64,
        echelle: f64,
    ) -> Scene3D {
        let pas = pas_de_grille(echelle, GRILLE_CIBLE_PX);
        let mut scene = Scene3D::vide();
        // La grille au sol couvre la boîte de cadrage, élargie d'un
        // pas de chaque côté.
        let boite = Self::boite_cadrage_3d(width, height, Self::boite_scene_3d(traces, maillage));
        let boite_grille = boite
            .etendre(boite.min_x - pas, boite.min_y - pas, 0.0)
            .etendre(boite.max_x + pas, boite.max_y + pas, 0.0);
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
        if let Some(maillage) = maillage {
            scene
                .segments
                .extend(Scene3D::depuis_maillage(maillage, COULEUR_WIREFRAME).segments);
        }
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
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.add_enabled(
            self.session.can_undo(),
            egui::Button::new(self.texte("top-annuler")),
        );
        avec_aide(&reponse, "aide-top-annuler", &mut self.aide);
        if reponse.clicked() {
            self.annuler();
        }
        let reponse = ui.add_enabled(
            self.session.can_redo(),
            egui::Button::new(self.texte("top-retablir")),
        );
        avec_aide(&reponse, "aide-top-retablir", &mut self.aide);
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
        avec_aide(&reponse, "aide-top-exporter-svg", &mut self.aide);
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
        avec_aide(&reponse, "aide-top-exporter-dxf", &mut self.aide);
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
        avec_aide(&reponse, "aide-top-exporter-stl", &mut self.aide);
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

    fn params_panel(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("params-titre"));
        avec_aide(&reponse, "aide-params-titre", &mut self.aide);
        let mut edited: Option<(&str, f64)> = None;
        for (name, cle, help_id) in [
            ("width_mm", "params-largeur", "aide-params-largeur"),
            ("height_mm", "params-hauteur", "aide-params-hauteur"),
        ] {
            let libelle = self.texte(cle);
            let graphe = self.session.graph_mut().value(name).ok();
            // Valeur du graphe en début de frame : un engagement sans
            // changement effectif n'écrit rien au journal — un faux
            // événement tronquerait les groupes défaits (ADR-0008).
            let avant = graphe.unwrap_or(0.0);
            // Tampon d'édition persistant du champ : le DragValue affiche
            // la valeur du début de frame, un rechargement du graphe à
            // chaque frame figeait le chiffre pendant le geste
            // (validation C1 round 6).
            let (memoire, actif_precedent) =
                self.tampon_dimensions.remove(name).unwrap_or((0.0, false));
            let mut tampon = Self::tampon_apres_inactivite(actif_precedent, memoire, graphe);
            let mut actif = false;
            let demande = &mut self.aide;
            ui.horizontal(|ui| {
                ui.label(&libelle);
                let reponse = ui.add(
                    egui::DragValue::new(&mut tampon)
                        .speed(1.0)
                        .range(1.0..=5000.0)
                        // Une valeur existante hors plage (entrée absente
                        // après un undo) n'est jamais « corrigée » : le
                        // clamp par défaut compte comme un changement et
                        // poussait un faux événement qui détruisait
                        // l'historique défait (validation C1 round 3).
                        .clamp_existing_to_range(false)
                        .suffix(" mm"),
                );
                // Le clic ouvre le mode édition clavier, qui garde le
                // focus indéfiniment : un champ focusé ne réagit plus
                // au glissement (le geste sélectionne le texte du
                // tampon). Un appui-glissement rend le focus pour que
                // le geste déplace la valeur (piège du glissement,
                // validation C1 round 4).
                let presse_glisse =
                    ui.input(|i| i.pointer.is_decidedly_dragging() && i.pointer.primary_down());
                if presse_glisse && reponse.contains_pointer() {
                    ui.memory_mut(|m| m.surrender_focus(reponse.id));
                }
                if reponse.dragged() || reponse.has_focus() {
                    actif = true;
                }
                if let Some(valeur) = Self::engagement_params(
                    avant,
                    tampon,
                    reponse.dragged(),
                    reponse.drag_stopped(),
                    reponse.changed(),
                ) {
                    edited = Some((name, valeur));
                }
                avec_aide(&reponse, help_id, demande);
            });
            self.tampon_dimensions.insert(name, (tampon, actif));
        }
        if let Some((name, value)) = edited {
            let _ = self.executer(&SetInput {
                name: name.into(),
                value,
            });
        }
        ui.separator();
        ui.label(self.texte("params-expression"));
        let reponse = ui.add(egui::TextEdit::singleline(&mut self.expr_buf).desired_width(240.0));
        avec_aide(&reponse, "aide-params-expression", &mut self.aide);
        if reponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let expression = self.expr_buf.clone();
            let _ = self.executer(&SetExpression {
                name: "diagonal".into(),
                expression,
            });
        }
        match self.session.graph_mut().value("diagonal") {
            Ok(value) => {
                let valeur = Length::from_millimeters(value).to_string();
                ui.label(self.texte_avec(
                    "params-diagonale",
                    &[("valeur", Argument::from(valeur.as_str()))],
                ));
            }
            Err(error) => {
                ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
            }
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
            .value("width_mm")
            .unwrap_or(1.0)
            .max(1.0);
        let height = self
            .session
            .graph_mut()
            .value("height_mm")
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
        if response.dragged() {
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
                    let echelle = self.camera3d.echelle_cible(rect.height() as f64);
                    let scene = Self::scene_vue_3d(traces, maillage, width, height, echelle);
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

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            egui::SidePanel::left("params")
                .resizable(false)
                .default_width(280.0)
                .show(ctx, |ui| {
                    self.params_panel(ui);
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
        // Titre de la fenêtre : « * » signale le travail non
        // enregistré, levé par la sauvegarde (ADR-0016 C3). La
        // fermeture de la fenêtre n'est pas interceptable dans
        // eframe 0.29 (pas de `on_request_exit`) : limite documentée
        // du démonstrateur.
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
        aide::fenetre(ctx, &self.i18n, &mut self.aide);
    }
}

fn main() -> eframe::Result<()> {
    let catalogue = i18n::catalogue();
    let titre = catalogue
        .traduction("titre-fenetre")
        .expect("clé présente dans le FTL (test de la CI)");
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
            Ok(Box::new(DemoApp::new(catalogue, etat)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Point2, Point3};

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
            name: "width_mm".into(),
            value: 1234.0,
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
        let largeur = relue.session.graph_mut().value("width_mm").expect("valeur");
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
        let scene = DemoApp::scene_vue_3d(&[trace], None, 800.0, 600.0, 1.0);
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
        let largeur = app.session.graph_mut().value("width_mm").expect("valeur");
        let hauteur = app.session.graph_mut().value("height_mm").expect("valeur");
        assert_eq!(largeur, 800.0, "largeur rétablie");
        assert_eq!(hauteur, 600.0, "hauteur rétablie");
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
            name: "width_mm".into(),
            value: 900.0,
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
        let largeur = app.session.graph_mut().value("width_mm").expect("valeur");
        assert_eq!(largeur, 900.0, "le redo restaure la largeur");
    }

    /// L'indicateur de travail non enregistré suit les commandes,
    /// l'annulation et la sauvegarde (ADR-0016 C3).
    #[test]
    fn dirty_suit_les_commandes_et_la_sauvegarde() {
        let mut app = DemoApp::new(i18n::catalogue(), None);
        assert!(!app.dirty, "session vierge : rien à enregistrer");
        app.executer(&SetInput {
            name: "width_mm".into(),
            value: 900.0,
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
            name: "width_mm".into(),
            value: 900.0,
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
    /// L'échelle mètres n'est proposée que pour un maillage
    /// minuscule : une dimension nulle (maillage vide) ou déjà
    /// millimétrique ne déclenche rien (ADR-0017).
    #[test]
    fn echelle_metres_proposee_sous_le_seuil() {
        assert!(propose_echelle_metres(0.05));
        assert!(!propose_echelle_metres(0.0));
        assert!(!propose_echelle_metres(1.0));
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
        assert_eq!(grossi.vertices()[0].y().millimeters(), 1000.0);
        let intact = maillage_echelle(grossi, 1.0).expect("identité");
        assert_eq!(intact.vertices()[0].y().millimeters(), 1000.0);
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
}
