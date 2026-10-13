//! # omnifab-desktop — démonstrateur du jalon 0
//!
//! Conforme à `docs/architecture.md` (« Noyau minimal livré au jalon 0 ») :
//! fenêtre egui + viewport (vide : rectangle de démonstration) + journal
//! d'événements affiché.
//!
//! Le démonstrateur met en scène les trois crates du noyau :
//!
//! - chaque action utilisateur est un **événement** enregistré dans le
//!   journal (forge-doc, event sourcing) ;
//! - l'état (graphe de liaisons) est **reconstruit par rejeu** du journal
//!   (forge-values, évaluation paresseuse) ;
//! - les dimensions manipulées sont des `Length` (forge-units, jamais de
//!   `f64` nu dans l'UI) ;
//! - le bouton « Recalculer tout » implémente la commande d'invalidation
//!   globale requise par l'ADR-0002 (futurement obligatoire avant usinage) ;
//! - « Enregistrer » / « Ouvrir » font l'aller-retour complet conteneur ZIP
//!   → rejeu, preuve du verrou nº1 (ADR-0000/0001).

use eframe::egui;
use forge_doc::{Document, Event, NewEvent};
use forge_units::Length;
use forge_values::ValueGraph;

/// Événement : valeur d'entrée fixée.
const EVENT_INPUT_SET: &str = "demo.value.input_set";
/// Événement : expression de liaison fixée.
const EVENT_EXPRESSION_SET: &str = "demo.value.expression_set";
/// Fichier de démonstration, dans le répertoire courant.
const DEFAULT_PROJECT: &str = "demo-jalon0.omnifab";
/// Expression initiale de la diagonale du rectangle.
const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(width_mm * width_mm + height_mm * height_mm)";

struct DemoApp {
    doc: Document,
    graph: ValueGraph,
    expr_buf: String,
    error: Option<String>,
    info: Option<String>,
    unknown_types: Vec<String>,
}

impl DemoApp {
    fn new() -> Self {
        let mut app = Self {
            doc: Document::new(),
            graph: ValueGraph::new(),
            expr_buf: DEFAULT_DIAGONAL_EXPR.to_string(),
            error: None,
            info: None,
            unknown_types: Vec::new(),
        };
        app.record(NewEvent::new(
            EVENT_INPUT_SET,
            1,
            "desktop.demo",
            serde_json::json!({"name": "width_mm", "value": 800.0}),
        ));
        app.record(NewEvent::new(
            EVENT_INPUT_SET,
            1,
            "desktop.demo",
            serde_json::json!({"name": "height_mm", "value": 600.0}),
        ));
        app.record(NewEvent::new(
            EVENT_EXPRESSION_SET,
            1,
            "desktop.demo",
            serde_json::json!({"name": "diagonal", "expression": DEFAULT_DIAGONAL_EXPR}),
        ));
        app
    }

    /// Applique l'événement au graphe et ne l'écrit au journal qu'en cas
    /// de succès : un événement invalide ne pollue jamais le journal, donc
    /// le rejeu d'un projet de ce document ne peut plus échouer.
    fn record(&mut self, event: NewEvent) {
        if let Err(error) = self.apply(&event) {
            self.error = Some(error.to_string());
            return;
        }
        self.doc.append(event);
    }

    /// Applique un événement au graphe de liaisons (projection).
    fn apply(&mut self, event: &NewEvent) -> Result<(), forge_values::ValuesError> {
        match event.event_type.as_str() {
            EVENT_INPUT_SET => {
                let name = event
                    .data
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let value = event
                    .data
                    .get("value")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                self.graph.set_input(name, value);
                Ok(())
            }
            EVENT_EXPRESSION_SET => {
                let name = event
                    .data
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let expression = event
                    .data
                    .get("expression")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if name == "diagonal" {
                    self.expr_buf = expression.to_string();
                }
                self.graph.set_expression(name, expression)
            }
            _ => {
                if !self.unknown_types.iter().any(|t| t == &event.event_type) {
                    self.unknown_types.push(event.event_type.clone());
                }
                Ok(())
            }
        }
    }

    /// Reconstruit l'état complet par rejeu du journal (event sourcing).
    fn rebuild_from_journal(&mut self, doc: Document) {
        self.graph = ValueGraph::new();
        self.unknown_types.clear();
        self.doc = doc;
        self.error = None;
        let events: Vec<Event> = self.doc.events().to_vec();
        for event in &events {
            let new = NewEvent::new(
                &event.event_type,
                event.schema_version,
                &event.actor,
                event.data.clone(),
            );
            if let Err(error) = self.apply(&new) {
                self.error = Some(error.to_string());
                return;
            }
        }
        self.info = Some(format!(
            "Projet rechargé : {} événements rejoués.",
            events.len()
        ));
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        if ui.button("Recalculer tout").clicked() {
            match self.graph.evaluate_all() {
                Ok(()) => self.info = Some("Toutes les valeurs sont à jour.".to_string()),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if ui.button("Enregistrer").clicked() {
            match self.doc.save_to_path(std::path::Path::new(DEFAULT_PROJECT)) {
                Ok(()) => {
                    self.info = Some(format!("Projet enregistré dans « {DEFAULT_PROJECT} »."));
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if ui.button("Ouvrir").clicked() {
            match Document::open_from_path(std::path::Path::new(DEFAULT_PROJECT)) {
                Ok(doc) => self.rebuild_from_journal(doc),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if ui.button("Nouveau").clicked() {
            *self = Self::new();
        }
    }

    fn params_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Paramètres");
        let mut edited: Option<(&str, f64)> = None;
        for (name, label) in [("width_mm", "Largeur"), ("height_mm", "Hauteur")] {
            let mut mm = self.graph.value(name).unwrap_or(0.0);
            ui.horizontal(|ui| {
                ui.label(label);
                let response = ui.add(
                    egui::DragValue::new(&mut mm)
                        .speed(1.0)
                        .range(1.0..=5000.0)
                        .suffix(" mm"),
                );
                // Un événement à la fin du geste (drag_stopped) ou pour une
                // édition clavier — pas un événement par frame de glissement.
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    edited = Some((name, mm));
                }
            });
        }
        if let Some((name, value)) = edited {
            self.record(NewEvent::new(
                EVENT_INPUT_SET,
                1,
                "desktop.demo.params",
                serde_json::json!({"name": name, "value": value}),
            ));
        }
        ui.separator();
        ui.label("Expression de la diagonale :");
        let response = ui.add(egui::TextEdit::singleline(&mut self.expr_buf).desired_width(240.0));
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let expression = self.expr_buf.clone();
            self.record(NewEvent::new(
                EVENT_EXPRESSION_SET,
                1,
                "desktop.demo.params",
                serde_json::json!({"name": "diagonal", "expression": expression}),
            ));
        }
        match self.graph.value("diagonal") {
            Ok(value) => {
                ui.label(format!("Diagonale : {}", Length::from_millimeters(value)));
            }
            Err(error) => {
                ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
            }
        }
    }

    fn journal_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Journal d'événements");
        ui.label(format!(
            "{} événements · conteneur ZIP · JSON Lines",
            self.doc.events().len()
        ));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for event in self.doc.events().iter().rev().take(200) {
                ui.label(format!(
                    "#{}  {}  v{}",
                    event.seq, event.event_type, event.schema_version
                ));
                ui.small(format!("{} · {}", event.timestamp, event.actor));
            }
        });
        if !self.unknown_types.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Types ignorés (journal d'une version plus récente ?) :")
                    .color(egui::Color32::YELLOW),
            );
            for event_type in &self.unknown_types {
                ui.small(event_type.clone());
            }
        }
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        ui.heading("Viewport (vide au jalon 0 — rectangle de démonstration)");
        if let Some(error) = &self.error {
            ui.label(egui::RichText::new(format!("Erreur : {error}")).color(egui::Color32::RED));
        }
        if let Some(info) = &self.info {
            ui.label(egui::RichText::new(info.clone()).color(egui::Color32::LIGHT_GREEN));
        }
        ui.separator();
        let width = self.graph.value("width_mm").unwrap_or(1.0).max(1.0);
        let height = self.graph.value("height_mm").unwrap_or(1.0).max(1.0);
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
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
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OmniFab — démonstrateur jalon 0",
        options,
        Box::new(|_cc| Ok(Box::new(DemoApp::new()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chemin du projet temporaire pour l'aller-retour de test.
    fn projet_temporaire() -> std::path::PathBuf {
        std::env::temp_dir().join("omnifab-tests-jalon0.omnifab")
    }

    /// Une expression invalide ne laisse aucun événement au journal.
    #[test]
    fn expression_invalide_n_ecrit_rien_au_journal() {
        let mut app = DemoApp::new();
        let avant = app.doc.events().len();
        app.record(NewEvent::new(
            EVENT_EXPRESSION_SET,
            1,
            "tests",
            serde_json::json!({"name": "diagonal", "expression": "inconnu + 1"}),
        ));
        assert_eq!(
            app.doc.events().len(),
            avant,
            "aucun événement ne doit être ajouté"
        );
        assert!(app.error.is_some(), "l'erreur doit être signalée");
    }

    /// Un journal valide se rejoue en entier à la réouverture.
    #[test]
    fn journal_valide_se_rejoue_entierement() {
        let mut app = DemoApp::new();
        let chemin = projet_temporaire();
        app.doc.save_to_path(&chemin).expect("sauvegarde impossible");
        let doc = Document::open_from_path(&chemin).expect("ouverture impossible");
        let attendu = doc.events().len();
        app.rebuild_from_journal(doc);
        assert!(app.error.is_none(), "rejeu sans erreur");
        assert_eq!(app.doc.events().len(), attendu, "journal rejoué en entier");
        assert!(
            app.graph.value("diagonal").is_ok(),
            "la projection est complète"
        );
        let _ = std::fs::remove_file(&chemin);
    }
}
