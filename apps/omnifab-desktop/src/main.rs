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
//! - **wireframe du maillage STL** dans le viewport, même échelle que
//!   les tracés (ADR-0014) : projection orthographique de dessus,
//!   plafond d'affichage au-delà duquel seule la boîte englobante est
//!   dessinée — aucune perte de données ;
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
use forge_session::{
    Command, EVENT_EXPRESSION_SET, ImportMesh, ImportTraces, Session, SetExpression, SetInput,
};
use forge_units::Length;
use std::path::{Path, PathBuf};

/// Blob des octets d'origine des tracés importés (ADR-0011).
const BLOB_TRACES: &str = "geometry/traces";
/// Blob des octets d'origine du maillage importé (ADR-0011).
const BLOB_MAILLAGE: &str = "geometry/maillage";
/// Acteur des événements écrits par le démonstrateur (ADR-0011).
const ACTEUR: &str = "desktop.demo";
/// Expression initiale de la diagonale du rectangle.
const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(width_mm * width_mm + height_mm * height_mm)";
/// Plafond d'affichage du wireframe : au-delà, seule la boîte
/// englobante du maillage est dessinée. Réglage du démonstrateur,
/// relevable sans ADR ; supprimé par le viewport wgpu de la phase C
/// (ADR-0014).
const LIMITE_WIREFRAME_TRIANGLES: usize = 20_000;
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

/// Vue commune aux tracés et au maillage : projection orthographique
/// de dessus, ajustée sur une boîte englobante en millimètres — une
/// seule échelle pour toute la scène importée (ADR-0014).
struct Vue {
    /// Échelle, en pixels par millimètre.
    echelle: f64,
    /// Centre de la scène, en millimètres.
    centre_mm: (f64, f64),
    /// Centre de la zone d'affichage, à l'écran.
    centre_ecran: egui::Pos2,
}

impl Vue {
    /// Ajuste la vue sur `boite` dans `rect`, avec une marge.
    fn nouvelle(rect: egui::Rect, boite: Boite) -> Self {
        let largeur_mm = (boite.max_x - boite.min_x).max(1e-9);
        let hauteur_mm = (boite.max_y - boite.min_y).max(1e-9);
        let marge = 20.0;
        let echelle = ((rect.width() - 2.0 * marge) as f64 / largeur_mm)
            .min((rect.height() - 2.0 * marge) as f64 / hauteur_mm)
            .max(1e-9);
        let centre_x = (boite.min_x + boite.max_x) / 2.0;
        let centre_y = (boite.min_y + boite.max_y) / 2.0;
        Self {
            echelle,
            centre_mm: (centre_x, centre_y),
            centre_ecran: rect.center(),
        }
    }

    /// Projette un point de la scène, en millimètres, à l'écran.
    fn projetter(&self, x: f64, y: f64) -> egui::Pos2 {
        let (centre_x, centre_y) = self.centre_mm;
        let ecran_x = self.centre_ecran.x as f64 + (x - centre_x) * self.echelle;
        let ecran_y = self.centre_ecran.y as f64 + (y - centre_y) * self.echelle;
        egui::pos2(ecran_x as f32, ecran_y as f32)
    }
}

/// Boîte englobante 2D, en millimètres.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Boite {
    /// Borne inférieure en x.
    min_x: f64,
    /// Borne inférieure en y.
    min_y: f64,
    /// Borne supérieure en x.
    max_x: f64,
    /// Borne supérieure en y.
    max_y: f64,
}

impl Boite {
    /// Première boîte d'une scène : réduite au point donné.
    fn du_point(x: f64, y: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    /// Étend la boîte pour englober le point donné.
    fn etendre(self, x: f64, y: f64) -> Self {
        Self {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
        }
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
    /// Cache mémoire non autoritaire des tracés re-parsés (nom du blob
    /// + tracés), invalidé quand la référence change (ADR-0011).
    cache: Option<(String, Vec<Path2>)>,
    /// Cache mémoire non autoritaire du maillage re-parsé (nom du blob
    /// + maillage), invalidé quand la référence change (ADR-0014).
    cache_maillage: Option<(String, Mesh)>,
}

impl DemoApp {
    /// Nouveau démonstrateur, sur le catalogue français embarqué :
    /// session vierge posée, écran d'accueil affiché (ADR-0014).
    fn new(i18n: i18n::Catalogue) -> Self {
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
            blob: BLOB_TRACES.to_string(),
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
            blob: BLOB_MAILLAGE.to_string(),
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
    fn boite_scene(traces: &[Path2], maillage: Option<&Mesh>) -> Option<Boite> {
        let mut boite: Option<Boite> = None;
        for trace in traces {
            for point in trace.points() {
                let x = point.x().millimeters();
                let y = point.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite::du_point(x, y),
                });
            }
        }
        if let Some(maillage) = maillage {
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite::du_point(x, y),
                });
            }
        }
        boite
    }

    /// Dessine les tracés dans la vue commune : couleur jaune, un
    /// tracé fermé refermé sur son premier sommet (ADR-0011).
    fn dessiner_traces(painter: &egui::Painter, vue: &Vue, traces: &[Path2]) {
        let couleur = egui::Color32::from_rgb(240, 180, 20);
        for trace in traces {
            let mut points: Vec<egui::Pos2> = trace
                .points()
                .iter()
                .map(|point| vue.projetter(point.x().millimeters(), point.y().millimeters()))
                .collect();
            if points.len() < 2 {
                continue;
            }
            if trace.is_closed() {
                points.push(points[0]);
            }
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5_f32, couleur),
            ));
        }
    }

    /// Dessine le wireframe du maillage : projection de dessus, cyan,
    /// plafond d'affichage au-delà duquel seule la boîte englobante
    /// est dessinée — le maillage reste importé et compté, aucune
    /// perte de données (ADR-0014).
    fn dessiner_maillage(painter: &egui::Painter, vue: &Vue, maillage: &Mesh) {
        let couleur = egui::Color32::from_rgb(40, 220, 220);
        if maillage.triangle_count() > LIMITE_WIREFRAME_TRIANGLES {
            let mut boite: Option<Boite> = None;
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite::du_point(x, y),
                });
            }
            if let Some(boite) = boite {
                let coin = vue.projetter(boite.min_x, boite.min_y);
                let oppose = vue.projetter(boite.max_x, boite.max_y);
                let rect = egui::Rect::from_two_pos(coin, oppose);
                painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, couleur));
            }
            return;
        }
        let sommets = maillage.vertices();
        for triangle in maillage.triangles() {
            let points: Vec<egui::Pos2> = triangle
                .iter()
                .map(|index| {
                    let sommet = sommets[*index as usize];
                    vue.projetter(sommet.x().millimeters(), sommet.y().millimeters())
                })
                .collect();
            painter.add(egui::Shape::line(
                vec![points[0], points[1], points[2], points[0]],
                egui::Stroke::new(1.0_f32, couleur),
            ));
        }
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
        if self.cache_maillage.as_ref().is_some_and(|(nom, _)| *nom == blob) {
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
            let mut mm = self.session.graph_mut().value(name).unwrap_or(0.0);
            let demande = &mut self.aide;
            ui.horizontal(|ui| {
                ui.label(&libelle);
                let reponse = ui.add(
                    egui::DragValue::new(&mut mm)
                        .speed(1.0)
                        .range(1.0..=5000.0)
                        .suffix(" mm"),
                );
                // Un événement à la fin du geste (drag_stopped) ou pour une
                // édition clavier — pas un événement par frame de glissement.
                if reponse.drag_stopped() || (reponse.changed() && !reponse.dragged()) {
                    edited = Some((name, mm));
                }
                avec_aide(&reponse, help_id, demande);
            });
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
            egui::Sense::hover(),
        );
        let rect = response.rect;
        let margin = 30.0;
        let scale = ((rect.width() - 2.0 * margin) as f64 / width)
            .min((rect.height() - 2.0 * margin) as f64 / height)
            .max(1.0);
        let shape = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2((width * scale) as f32, (height * scale) as f32),
        );
        painter.rect_filled(shape, 4.0, egui::Color32::from_rgb(38, 96, 152));
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
        // Tracés et maillage importés par-dessus le rectangle, dans une
        // vue commune : union des boîtes englobantes, une seule
        // échelle (ADR-0014).
        let traces: &[Path2] = match &self.cache {
            Some((_, traces)) => traces,
            None => &[],
        };
        let maillage: Option<&Mesh> = self.cache_maillage.as_ref().map(|(_, m)| m);
        if let Some(boite) = Self::boite_scene(traces, maillage) {
            let vue = Vue::nouvelle(rect, boite);
            if !traces.is_empty() {
                Self::dessiner_traces(&painter, &vue, traces);
            }
            if let Some(maillage) = maillage {
                Self::dessiner_maillage(&painter, &vue, maillage);
            }
        }
        // Compteurs de géométrie importée, sous le rectangle (ADR-0011) ;
        // le wireframe plafonne : au-delà, boîte englobante seule
        // (ADR-0014).
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
            if maillage.triangles > LIMITE_WIREFRAME_TRIANGLES {
                let limite = self.texte_avec(
                    "viewport-wireframe-limite",
                    &[
                        ("triangles", Argument::from(maillage.triangles)),
                        ("limite", Argument::from(LIMITE_WIREFRAME_TRIANGLES)),
                    ],
                );
                ui.label(egui::RichText::new(limite).color(egui::Color32::YELLOW));
            }
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
        ..Default::default()
    };
    eframe::run_native(
        &titre,
        options,
        Box::new(move |_cc| Ok(Box::new(DemoApp::new(catalogue)))),
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
            blob: BLOB_TRACES.into(),
            octets: SVG_TEST.to_vec(),
            traces: traces.len(),
            sommets,
        });
    }

    /// L'import est une action annulable : undo vide la géométrie de
    /// l'état (ADR-0011).
    #[test]
    fn undo_d_import_disparait_de_l_etat() {
        let mut app = DemoApp::new(i18n::catalogue());
        importer_svg_de_test(&mut app);
        assert!(app.session.geom().traces().is_some(), "géométrie importée");
        app.session.undo().expect("undo");
        assert!(app.session.geom().traces().is_none(), "l'import est défait");
    }

    /// Export puis re-import forment un aller-retour : mêmes compteurs.
    #[test]
    fn export_puis_reimport_est_equivalent() {
        let mut app = DemoApp::new(i18n::catalogue());
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
        let mut app = DemoApp::new(i18n::catalogue());
        importer_svg_de_test(&mut app);
        assert_eq!(
            app.session.document().blob_names(),
            vec![BLOB_TRACES.to_string()],
            "le blob de l'import est déposé"
        );
        app.session.undo().expect("undo");
        importer_svg_de_test(&mut app);
        assert_eq!(
            app.session.document().blob_names(),
            vec![BLOB_TRACES.to_string()],
            "un seul blob : l'ancien est remplacé, pas dupliqué"
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
        let mut app = DemoApp::new(i18n::catalogue());
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
        let mut relue = DemoApp::new(i18n::catalogue());
        relue.ouvrir_projet(&chemin);
        assert_eq!(relue.chemin_projet, Some(chemin), "chemin repris à l'ouverture");
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
}
