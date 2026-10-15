//! # omnifab-desktop — démonstrateur du jalon 1
//!
//! Conforme à `docs/architecture.md` et à l'ADR-0011 : la fenêtre egui
//! met en scène le noyau complet, branché de bout en bout :
//!
//! - l'application porte une **session** forge-session (ADR-0008) :
//!   commandes, projections, undo/redo par curseur — les méthodes de
//!   projection dupliquées du jalon 0 ont disparu, la session fait ce
//!   travail, mieux ;
//! - **annuler / rétablir** visibles : boutons et raccourcis Ctrl+Z,
//!   Ctrl+Y, Ctrl+Maj+Z (une annulation impossible par raccourci est
//!   ignorée silencieusement, ADR-0011) ;
//! - **import/export SVG/DXF/STL** par forge-io (ADR-0009) : les octets
//!   d'origine partent dans des blobs, le journal ne porte que
//!   références et compteurs (ADR-0001) ; l'export ne change pas l'état
//!   — rien au journal ; chemins de démonstration fixes, pas de
//!   sélecteur de fichiers au jalon 1 (ADR-0011) ;
//! - les dimensions manipulées restent des `Length` (forge-units, jamais
//!   de `f64` nu dans l'UI) ;
//! - toute l'interface passe par le catalogue Fluent embarqué
//!   (forge-i18n, ADR-0010) : aucune chaîne d'interface dans le code ;
//! - chaque widget interactif porte un `help_id` : un clic droit ouvre sa
//!   page d'aide (convention jalon 1, ADR-0010).

mod aide;
mod i18n;

use aide::avec_aide;
use eframe::egui;
use forge_doc::Document;
use forge_geom::Path2;
use forge_i18n::Argument;
use forge_io::{dxf, stl, svg};
use forge_session::{
    Command, ImportMesh, ImportTraces, Session, SetExpression, SetInput, EVENT_EXPRESSION_SET,
};
use forge_units::Length;

/// Fichier de projet de démonstration, dans le répertoire courant.
const DEFAULT_PROJECT: &str = "demo-jalon0.omnifab";
/// Fichier SVG importé par « Importer SVG », dans le répertoire courant.
const TRACES_SVG: &str = "demo-import.svg";
/// Fichier DXF importé par « Importer DXF », dans le répertoire courant.
const TRACES_DXF: &str = "demo-import.dxf";
/// Fichier STL importé par « Importer STL », dans le répertoire courant.
const MAILLAGE_STL: &str = "demo-import.stl";
/// Fichier SVG écrit par « Exporter SVG », dans le répertoire courant.
const EXPORT_SVG: &str = "demo-export.svg";
/// Fichier DXF écrit par « Exporter DXF », dans le répertoire courant.
const EXPORT_DXF: &str = "demo-export.dxf";
/// Fichier STL écrit par « Exporter STL », dans le répertoire courant.
const EXPORT_STL: &str = "demo-export.stl";
/// Blob des octets d'origine des tracés importés (ADR-0011).
const BLOB_TRACES: &str = "geometry/traces";
/// Blob des octets d'origine du maillage importé (ADR-0011).
const BLOB_MAILLAGE: &str = "geometry/maillage";
/// Acteur des événements écrits par le démonstrateur (ADR-0011).
const ACTEUR: &str = "desktop.demo";
/// Expression initiale de la diagonale du rectangle.
const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(width_mm * width_mm + height_mm * height_mm)";

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
    /// Cache mémoire non autoritaire des tracés re-parsés (nom du blob +
    /// tracés), invalidé quand la référence change (ADR-0011).
    cache: Option<(String, Vec<Path2>)>,
}

impl DemoApp {
    /// Nouveau démonstrateur, sur le catalogue français embarqué.
    fn new(i18n: i18n::Catalogue) -> Self {
        let mut app = Self {
            session: Session::new(ACTEUR),
            expr_buf: String::new(),
            error: None,
            info: None,
            i18n,
            aide: None,
            cache: None,
        };
        app.reinitialiser();
        app
    }

    /// Remet le démonstrateur à l'état initial (bouton « Nouveau ») :
    /// nouvelle session, dimensions et expression par défaut — trois
    /// groupes d'undo séparés (ADR-0011).
    fn reinitialiser(&mut self) {
        self.session = Session::new(ACTEUR);
        self.expr_buf = DEFAULT_DIAGONAL_EXPR.to_string();
        self.error = None;
        self.info = None;
        self.cache = None;
        self.aide = None;
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
        self.info = Some(self.texte_avec(
            "info-import-maillage",
            &[("triangles", Argument::from(triangles))],
        ));
    }

    /// Exporte les tracés courants au format demandé : re-parse du blob
    /// d'origine via forge-io puis sérialisation. L'export ne change
    /// pas l'état du projet : rien au journal (ADR-0011).
    fn exporter_traces(&mut self, format: &'static str, chemin: &str) {
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
        self.info = Some(self.texte_avec(
            "info-export",
            &[("chemin", Argument::from(chemin))],
        ));
    }

    /// Exporte le maillage courant en STL binaire, depuis les octets
    /// d'origine re-parsés (ADR-0011).
    fn exporter_maillage(&mut self, chemin: &str) {
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
        self.info = Some(self.texte_avec(
            "info-export",
            &[("chemin", Argument::from(chemin))],
        ));
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
            match std::fs::read(TRACES_SVG) {
                Ok(octets) => self.importer_traces("svg", octets),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.button(self.texte("top-importer-dxf"));
        avec_aide(&reponse, "aide-top-importer-dxf", &mut self.aide);
        if reponse.clicked() {
            match std::fs::read(TRACES_DXF) {
                Ok(octets) => self.importer_traces("dxf", octets),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.button(self.texte("top-importer-stl"));
        avec_aide(&reponse, "aide-top-importer-stl", &mut self.aide);
        if reponse.clicked() {
            match std::fs::read(MAILLAGE_STL) {
                Ok(octets) => self.importer_maillage(octets),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.add_enabled(
            self.session.geom().traces().is_some(),
            egui::Button::new(self.texte("top-exporter-svg")),
        );
        avec_aide(&reponse, "aide-top-exporter-svg", &mut self.aide);
        if reponse.clicked() {
            self.exporter_traces("svg", EXPORT_SVG);
        }
        let reponse = ui.add_enabled(
            self.session.geom().traces().is_some(),
            egui::Button::new(self.texte("top-exporter-dxf")),
        );
        avec_aide(&reponse, "aide-top-exporter-dxf", &mut self.aide);
        if reponse.clicked() {
            self.exporter_traces("dxf", EXPORT_DXF);
        }
        let reponse = ui.add_enabled(
            self.session.geom().maillage().is_some(),
            egui::Button::new(self.texte("top-exporter-stl")),
        );
        avec_aide(&reponse, "aide-top-exporter-stl", &mut self.aide);
        if reponse.clicked() {
            self.exporter_maillage(EXPORT_STL);
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
            match self.session.save_to_path(std::path::Path::new(DEFAULT_PROJECT)) {
                Ok(()) => {
                    self.info = Some(self.texte_avec(
                        "info-enregistre",
                        &[("chemin", Argument::from(DEFAULT_PROJECT))],
                    ));
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.button(self.texte("top-ouvrir"));
        avec_aide(&reponse, "aide-top-ouvrir", &mut self.aide);
        if reponse.clicked() {
            match Document::open_from_path(std::path::Path::new(DEFAULT_PROJECT)) {
                Ok(doc) => {
                    let nombre = doc.events().len();
                    match Session::from_document(ACTEUR, doc) {
                        Ok(session) => {
                            self.session = session;
                            self.cache = None;
                            self.error = None;
                            self.expr_buf = self.expression_courante();
                            self.info = Some(self.texte_avec(
                                "info-recharge",
                                &[("n", Argument::from(nombre))],
                            ));
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let reponse = ui.button(self.texte("top-nouveau"));
        avec_aide(&reponse, "aide-top-nouveau", &mut self.aide);
        if reponse.clicked() {
            self.reinitialiser();
        }
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
                ui.label(self.texte_avec(
                    "params-diagonale",
                    &[(
                        "valeur",
                        Argument::from(Length::from_millimeters(value).to_string()),
                    )],
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

    /// Dessine les tracés dans le viewport : mise à l'échelle sur la
    /// boîte englobante, centrés, un tracé fermé refermé sur son
    /// premier sommet (ADR-0011).
    fn dessiner_traces(&self, painter: &egui::Painter, rect: egui::Rect, traces: &[Path2]) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for trace in traces {
            for point in trace.points() {
                min_x = min_x.min(point.x().millimeters());
                min_y = min_y.min(point.y().millimeters());
                max_x = max_x.max(point.x().millimeters());
                max_y = max_y.max(point.y().millimeters());
            }
        }
        let largeur_mm = (max_x - min_x).max(1e-9);
        let hauteur_mm = (max_y - min_y).max(1e-9);
        let marge = 20.0;
        let echelle = ((rect.width() - 2.0 * marge) as f64 / largeur_mm)
            .min((rect.height() - 2.0 * marge) as f64 / hauteur_mm)
            .max(1e-9);
        let centre_x = (min_x + max_x) / 2.0;
        let centre_y = (min_y + max_y) / 2.0;
        let centre = rect.center();
        let couleur = egui::Color32::from_rgb(240, 180, 20);
        for trace in traces {
            let mut points: Vec<egui::Pos2> = trace
                .points()
                .iter()
                .map(|point| {
                    egui::pos2(
                        (centre.x + (point.x().millimeters() - centre_x) * echelle) as f32,
                        (centre.y + (point.y().millimeters() - centre_y) * echelle) as f32,
                    )
                })
                .collect();
            if points.len() < 2 {
                continue;
            }
            if trace.is_closed() {
                points.push(points[0]);
            }
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, couleur)));
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
        let width = self.session.graph_mut().value("width_mm").unwrap_or(1.0).max(1.0);
        let height = self.session.graph_mut().value("height_mm").unwrap_or(1.0).max(1.0);
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
        // Tracés importés, par-dessus le rectangle, couleur distincte.
        if let Some((_, traces)) = &self.cache {
            self.dessiner_traces(&painter, rect, traces);
        }
        // Compteurs de géométrie importée, sous le rectangle (ADR-0011) ;
        // le maillage n'est pas dessiné au jalon 1 (rendu wgpu, jalon 2).
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
        egui::CentralPanel::default().show(ctx, |ui| {
            self.viewport(ui);
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
}
