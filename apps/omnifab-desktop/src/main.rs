//! # omnifab-desktop — démonstrateur des jalons 1 et 2
//!
//! Conforme à `docs/architecture.md` et aux ADR-0011/0014 : la
//! fenêtre egui met en scène le noyau complet, branché de bout en
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
//! - **bibliothèque de projets** (ADR-0014) : écran d'accueil listant
//!   les `.omnifab` du dossier `OmniFab/Projets` du dossier personnel,
//!   « Enregistrer sous… » y propose par défaut ; le chemin courant
//!   est un état de l'app UI, jamais un événement ;
//! - **viewport wgpu** (ADR-0015, phases C1 et C2) : vue 2D — grille
//!   de fond et tracés importés dessinés par le GPU via forge-render,
//!   caméra à la molette (zoom au curseur) et au glisser (pan) ;
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
use forge_geom::{Mesh, Path2};
use forge_i18n::Argument;
use forge_io::{dxf, stl, svg};
use forge_render::{
    Boite2, Boite3D, Camera2D, Camera3D, Couleur, Renderer2D, Scene2D, Scene3D, Segment3D,
    dessiner_scene, dessiner_wireframe, pas_de_grille,
};
use forge_session::{
    Command, EVENT_EXPRESSION_SET, ImportMesh, ImportTraces, Session, SetExpression, SetInput,
};
use forge_units::Length;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Cache mémoire non autoritaire des tracés re-parsés (nom du blob
    /// + tracés), invalidé quand la référence change (ADR-0011).
    cache: Option<(String, Vec<Path2>)>,
    /// Cache mémoire non autoritaire du maillage re-parsé (nom du blob
    /// + maillage), invalidé quand la référence change (ADR-0014).
    cache_maillage: Option<(String, Mesh)>,
    /// Caméra du viewport : ajustée automatiquement quand la scène
    /// change, pilotée ensuite par l'utilisateur (ADR-0015).
    camera: Camera2D,
    /// Clé du dernier ajustement : références des blobs tracés et
    /// maillage — la caméra se réajuste quand elles changent.
    camera_cle: Option<(Option<String>, Option<String>)>,
    /// Caméra orbite de la vue 3D : ajustée automatiquement quand la
    /// scène change, pilotée ensuite par l'utilisateur (ADR-0015,
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
        };
        app.reinitialiser();
        app
    }

    /// Remet le démonstrateur à l'état initial (nouveau projet) :
    /// nouvelle session, dimensions et expression par défaut — trois
    /// groupes d'undo séparés (ADR-0011).
    fn reinitialiser(&mut self) {
        self.session = Session::new(ACTEUR);
        self.expr_buf = DEFAULT_DIAGONAL_EXPR.to_string();
        self.error = None;
        self.info = None;
        self.cache = None;
        self.cache_maillage = None;
        self.camera = Camera2D::nouvelle();
        self.camera_cle = None;
        self.camera3d = Camera3D::nouvelle();
        self.vue = Vue::DeuxD;
        self.aide = None;
        self.chemin_projet = None;
        self.executer(&SetInput {
            name: "width_mm".into(),
            value: 800.0,
        });
        self.executer(&SetInput {
            name: "height_mm".into(),
            value: 600.0,
        });
        self.executer(&SetExpression {
            name: "diagonal".into(),
            expression: DEFAULT_DIAGONAL_EXPR.into(),
        });
    }

    /// Exécute une commande de session ; un refus remonte au canal
    /// d'erreur du viewport sans interrompre l'interface.
    fn executer(&mut self, commande: &dyn Command) {
        if let Err(error) = self.session.execute(commande) {
            self.error = Some(error.to_string());
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
        let document = match Document::open_from_path(chemin) {
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
        self.cache = None;
        self.cache_maillage = None;
        self.camera = Camera2D::nouvelle();
        self.camera_cle = None;
        self.camera3d = Camera3D::nouvelle();
        self.chemin_projet = Some(chemin.to_path_buf());
        self.ecran = Ecran::Atelier;
        self.error = None;
        self.expr_buf = self.expression_courante();
        self.info = Some(self.texte_avec("info-recharge", &[("n", Argument::from(nombre))]));
    }

    /// Enregistre le projet dans `chemin` : conteneur ZIP, les
    /// actions annulées sont abandonnées (ADR-0008) ; le chemin
    /// devient le projet courant (ADR-0014).
    fn enregistrer_dans(&mut self, chemin: &Path) {
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

    /// Importe des tracés SVG ou DXF déjà lus en octets : parse forge-io
    /// sous budgets (la validation du contenu précède la commande,
    /// ADR-0011), puis commande `ImportTraces` — une transaction = un
    /// groupe d'undo, octets d'origine dans un blob.
    fn importer_traces(&mut self, format: &'static str, octets: Vec<u8>) {
        let parse = if format == "svg" {
            svg::import(&octets, &svg::SvgBudgets::default())
        } else {
            dxf::import(&octets, &dxf::DxfBudgets::default())
        };
        let traces = match parse {
            Ok(traces) => traces,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let nombre_traces = traces.len();
        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
        self.executer(&ImportTraces {
            format: format.to_string(),
            octets,
            traces: nombre_traces,
            sommets,
        });
        self.cache = None;
        self.info = Some(self.texte_avec(
            "info-import-traces",
            &[
                ("traces", Argument::from(nombre_traces)),
                ("sommets", Argument::from(sommets)),
            ],
        ));
    }

    /// Lit puis importe le fichier de tracés choisi dans le sélecteur
    /// natif : le format suit le bouton, l'erreur de lecture remonte
    /// au canal d'erreur (ADR-0014).
    fn charger_traces(&mut self, format: &'static str, chemin: &Path) {
        let octets = match std::fs::read(chemin) {
            Ok(octets) => octets,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        self.importer_traces(format, octets);
    }

    /// Importe un maillage STL déjà lu en octets : parse forge-io sous
    /// budgets, puis commande `ImportMesh`.
    fn importer_maillage(&mut self, octets: Vec<u8>) {
        let maillage = match stl::import(&octets, &stl::StlBudgets::default()) {
            Ok(maillage) => maillage,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let triangles = maillage.triangle_count();
        self.executer(&ImportMesh {
            format: "stl".to_string(),
            octets,
            triangles,
        });
        self.cache_maillage = None;
        self.info = Some(self.texte_avec(
            "info-import-maillage",
            &[("triangles", Argument::from(triangles))],
        ));
    }

    /// Lit puis importe le maillage STL choisi dans le sélecteur natif
    /// (ADR-0014).
    fn charger_maillage(&mut self, chemin: &Path) {
        let octets = match std::fs::read(chemin) {
            Ok(octets) => octets,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        self.importer_maillage(octets);
    }

    /// Exporte les tracés courants au format demandé, vers le chemin
    /// choisi : re-parse du blob d'origine via forge-io puis
    /// sérialisation. L'export ne change pas l'état du projet : rien
    /// au journal (ADR-0011).
    fn exporter_traces(&mut self, format: &'static str, chemin: &Path) {
        let Some(reference) = self.session.geom().traces() else {
            return;
        };
        let (format_origine, blob) = (reference.format.clone(), reference.blob.clone());
        let Some(octets) = self.session.document().blob(&blob) else {
            self.error = Some(format!("blob absent du document : {blob}"));
            return;
        };
        let octets = octets.to_vec();
        let parse = if format_origine == "svg" {
            svg::import(&octets, &svg::SvgBudgets::default())
        } else {
            dxf::import(&octets, &dxf::DxfBudgets::default())
        };
        let traces = match parse {
            Ok(traces) => traces,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let sortie = if format == "svg" {
            svg::export(&traces)
        } else {
            dxf::export(&traces)
        };
        let octets = match sortie {
            Ok(octets) => octets,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        if let Err(error) = std::fs::write(chemin, &octets) {
            self.error = Some(error.to_string());
            return;
        }
        let affiche = chemin.display().to_string();
        self.info = Some(self.texte_avec(
            "info-export",
            &[("chemin", Argument::from(affiche.as_str()))],
        ));
    }

    /// Exporte le maillage courant en STL binaire, vers le chemin
    /// choisi, depuis les octets d'origine re-parsés (ADR-0011).
    fn exporter_maillage(&mut self, chemin: &Path) {
        let Some(reference) = self.session.geom().maillage() else {
            return;
        };
        let blob = reference.blob.clone();
        let Some(octets) = self.session.document().blob(&blob) else {
            self.error = Some(format!("blob absent du document : {blob}"));
            return;
        };
        let octets = octets.to_vec();
        let maillage = match stl::import(&octets, &stl::StlBudgets::default()) {
            Ok(maillage) => maillage,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let octets = match stl::export(&maillage) {
            Ok(octets) => octets,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        if let Err(error) = std::fs::write(chemin, &octets) {
            self.error = Some(error.to_string());
            return;
        }
        let affiche = chemin.display().to_string();
        self.info = Some(self.texte_avec(
            "info-export",
            &[("chemin", Argument::from(affiche.as_str()))],
        ));
    }

    /// Boîte englobante 2D de la scène importée : union des tracés et
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

    /// Boîte de cadrage automatique de la caméra : celle de la scène
    /// importée quand il y en a une — l'import recentre sur les objets,
    /// même éloignés du rectangle paramétrique — sinon celle du
    /// rectangle paramétrique (ADR-0015).
    fn boite_cadrage(width: f64, height: f64, scene: Option<Boite2>) -> Boite2 {
        match scene {
            Some(scene) => scene,
            None => Boite2::du_point(0.0, 0.0).etendre(width, height),
        }
    }

    /// Boîte englobante 3D de la scène importée : union des tracés
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
    /// scène importée quand il y en a une, sinon le rectangle
    /// paramétrique posé sur le plan z = 0 (ADR-0015, phase C2).
    fn boite_cadrage_3d(width: f64, height: f64, scene: Option<Boite3D>) -> Boite3D {
        match scene {
            Some(scene) => scene,
            None => Boite3D::du_point(0.0, 0.0, 0.0).etendre(width, height, 0.0),
        }
    }

    /// Scène 3D du viewport : grille au sol, tracés posés sur le
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
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0011).
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
        let parse = if format == "svg" {
            svg::import(&octets, &svg::SvgBudgets::default())
        } else {
            dxf::import(&octets, &dxf::DxfBudgets::default())
        };
        match parse {
            Ok(traces) => self.cache = Some((blob, traces)),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Re-parse le maillage courant si le blob référencé a changé :
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0014).
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
        let octets = octets.to_vec();
        match stl::import(&octets, &stl::StlBudgets::default()) {
            Ok(maillage) => self.cache_maillage = Some((blob, maillage)),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.add_enabled(
            self.session.can_undo(),
            egui::Button::new(self.texte("top-annuler")),
        );
        avec_aide(&reponse, "aide-top-annuler", &mut self.aide);
        if reponse.clicked() {
            let _ = self.session.undo();
            self.expr_buf = self.expression_courante();
        }
        let reponse = ui.add_enabled(
            self.session.can_redo(),
            egui::Button::new(self.texte("top-retablir")),
        );
        avec_aide(&reponse, "aide-top-retablir", &mut self.aide);
        if reponse.clicked() {
            let _ = self.session.redo();
            self.expr_buf = self.expression_courante();
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
            let libelle = self.texte("filtre-projet");
            let Some(chemin) = fichiers::choisir_projet(&libelle) else {
                return;
            };
            self.ouvrir_projet(&chemin);
        }
        let reponse = ui.button(self.texte("top-nouveau"));
        avec_aide(&reponse, "aide-top-nouveau", &mut self.aide);
        if reponse.clicked() {
            self.reinitialiser();
            self.ecran = Ecran::Atelier;
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
            self.reinitialiser();
            self.ecran = Ecran::Atelier;
        }
        let reponse = ui.button(self.texte("accueil-ouvrir"));
        avec_aide(&reponse, "aide-accueil-ouvrir", &mut self.aide);
        if reponse.clicked() {
            let libelle = self.texte("filtre-projet");
            if let Some(chemin) = fichiers::choisir_projet(&libelle) {
                self.ouvrir_projet(&chemin);
            }
        }
        let en_cours = self.session.document().events().len() > EVENEMENTS_VIERGES
            || self.chemin_projet.is_some();
        if en_cours {
            let reponse = ui.button(self.texte("accueil-reprendre"));
            avec_aide(&reponse, "aide-accueil-reprendre", &mut self.aide);
            if reponse.clicked() {
                self.ecran = Ecran::Atelier;
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
                        self.ouvrir_projet(&chemin);
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
            self.executer(&SetInput {
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
            self.executer(&SetExpression {
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
            Some((_, traces)) => traces,
            None => &[],
        };
        let maillage: Option<&Mesh> = self.cache_maillage.as_ref().map(|(_, m)| m);
        let rect = response.rect;
        if self.camera_cle.as_ref() != Some(&cle) {
            // Le cadrage vise la scène importée quand il y en a une,
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
        // Vue 2D : rectangle paramétrique par le painter, grille et
        // tracés par le GPU. Vue 3D : rectangle, tracés et grille au
        // sol posés sur le plan z = 0, wireframe du maillage en
        // volume — arêtes dédupliquées, sans plafond (ADR-0015,
        // phase C2).
        match self.vue {
            Vue::DeuxD => {
                let coin = Self::point_ecran(&self.camera, rect, 0.0, 0.0);
                let oppose = Self::point_ecran(&self.camera, rect, width, height);
                let shape = egui::Rect::from_two_pos(coin, oppose);
                painter.rect_filled(shape, 4.0, egui::Color32::from_rgb(38, 96, 152));
                // Priorité d'affichage = ordre de dessin (leçon du
                // round 2 de la validation C2) : le fond bleu par le
                // painter, puis la grille et les tracés par le GPU,
                // puis le contour et la cote du rectangle par-dessus.
                // Grille et tracés par le GPU (ADR-0015) ; sans wgpu,
                // la scène 2D n'est pas dessinée et le viewport le dit.
                if self.viewport_wgpu {
                    let mut scene = Scene2D::vide();
                    let visible = self
                        .camera
                        .boite_visible(rect.width() as f64, rect.height() as f64);
                    scene.ajouter_grille(
                        visible,
                        self.camera.echelle,
                        GRILLE_CIBLE_PX,
                        COULEUR_GRILLE_2D,
                    );
                    let traces_scene =
                        Scene2D::depuis_traces(traces, COULEUR_TRACES, LARGEUR_TRACES_PX);
                    scene.segments.extend(traces_scene.segments);
                    dessiner_scene(&painter, rect, scene, self.camera);
                } else {
                    ui.label(
                        egui::RichText::new(self.texte("viewport-wgpu-indisponible"))
                            .color(egui::Color32::YELLOW),
                    );
                }
                // Contour et cote du rectangle par le painter, après
                // le callback GPU : la grille ne les recouvre plus quand
                // elle coïncide avec eux (priorité d'affichage,
                // validation C2, round 3).
                painter.rect_stroke(shape, 4.0, egui::Stroke::new(1.5_f32, egui::Color32::WHITE));
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
        // Raccourcis : Ctrl+Z annule, Ctrl+Y ou Ctrl+Maj+Z rétablit ;
        // un raccourci impossible est ignoré silencieusement (ADR-0011).
        let (annuler, retablir) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            let maj = i.modifiers.shift;
            (
                ctrl && !maj && i.key_pressed(egui::Key::Z),
                (ctrl && i.key_pressed(egui::Key::Y))
                    || (ctrl && maj && i.key_pressed(egui::Key::Z)),
            )
        });
        if annuler && self.session.can_undo() {
            let _ = self.session.undo();
            self.expr_buf = self.expression_courante();
        }
        if retablir && self.session.can_redo() {
            let _ = self.session.redo();
            self.expr_buf = self.expression_courante();
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
        });
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
        });
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

    /// La boîte de scène englobe tracés et maillage : union des boîtes
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

    /// Le cadrage automatique vise la scène importée, même éloignée de
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
        let scene = DemoApp::boite_scene(&[trace], None).expect("boîte de scène");
        let boite = DemoApp::boite_cadrage(50.0, 50.0, Some(scene));
        assert_eq!(boite.min_x, 100.0, "origine x de la scène");
        assert_eq!(boite.min_y, 200.0, "origine y de la scène");
        assert_eq!(boite.max_x, 120.0, "portée x de la scène");
        assert_eq!(boite.max_y, 220.0, "portée y de la scène");
    }

    /// Sans scène importée, le cadrage vise le rectangle paramétrique
    /// seul (ADR-0015).
    #[test]
    fn le_cadrage_sans_scene_vise_le_rectangle() {
        let boite = DemoApp::boite_cadrage(800.0, 600.0, None);
        assert_eq!(boite.min_x, 0.0, "origine x du rectangle");
        assert_eq!(boite.min_y, 0.0, "origine y du rectangle");
        assert_eq!(boite.max_x, 800.0, "portée x du rectangle");
        assert_eq!(boite.max_y, 600.0, "portée y du rectangle");
    }

    /// La boîte de scène 3D englobe tracés et maillage en volume :
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

    /// Sans scène importée, le cadrage 3D vise le rectangle
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

    /// L'axe Y monte dans la scène et descend à l'écran : un point
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
            "plus haut dans la scène, plus haut à l'écran"
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
}
