//! # omnifab-android — preuve de concept egui sur Android (jalon 0)
//!
//! Objectif (ADR-0005) : valider egui sur Android sur son point faible
//! connu — **tactile et clavier virtuel** — avant de clore le jalon 0.
//! Le verdict (egui validé, sinon plan B Slint) est tranché après
//! installation de l'APK sur un téléphone.
//!
//! L'application met à l'épreuve :
//!
//! - le **tactile** : bouton géant et zone de glisser ;
//! - le **clavier virtuel** : champ une ligne + champ multi-lignes ;
//! - le **diagnostic** : journal des événements d'entrée bruts reçus par
//!   egui (touch, texte, touche, zoom), pour localiser précisément une
//!   défaillance (côté winit ou côté egui).
//!
//! Renderer : **glow** — eframe 0.29 n'active pas le backend GLES de
//! wgpu sur Android (cf. ADR-0005). Le renderer est indépendant du
//! chemin d'entrée tactile/IME, seul objet de ce PoC.

use eframe::egui;

/// Taille du journal de diagnostic (événements conservés).
const LOG_LEN: usize = 12;

/// Renvoie `true` si l'événement d'entrée doit apparaître dans le journal
/// de diagnostic (tactile, texte, touche, zoom).
fn is_traced_event(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Touch { .. }
            | egui::Event::Text(_)
            | egui::Event::Key { .. }
            | egui::Event::Zoom(_)
    )
}

/// Application du PoC : état minimal, aucun domaine du noyau mobilisé.
pub struct PocApp {
    /// Pressions sur le bouton de test tactile.
    tap_count: u32,
    /// Gestes de glisser détectés dans la zone viewport.
    drag_count: u32,
    /// Dernière position du pointeur dans la zone de glisser.
    circle_pos: Option<egui::Pos2>,
    /// Champ de saisie sur une ligne (test du clavier virtuel).
    text: String,
    /// Champ de saisie multi-lignes (test du clavier virtuel).
    text_multi: String,
    /// Journal borné des derniers événements d'entrée bruts.
    log: Vec<String>,
}

impl PocApp {
    /// Crée l'application avec un état initial visible et non vide.
    pub fn new() -> Self {
        Self {
            tap_count: 0,
            drag_count: 0,
            circle_pos: None,
            text: "Touchez ce champ : le clavier doit s'ouvrir".to_string(),
            text_multi: String::new(),
            log: Vec::new(),
        }
    }

    /// Ajoute une entrée au journal de diagnostic (borné à `LOG_LEN`).
    fn log_push(&mut self, entry: String) {
        self.log.push(entry);
        let excess = self.log.len().saturating_sub(LOG_LEN);
        self.log.drain(..excess);
    }

    /// Capture les événements d'entrée bruts utiles au diagnostic tactile/IME.
    fn capture_events(&mut self, ctx: &egui::Context) {
        let new_events: Vec<String> = ctx.input(|i| {
            i.events
                .iter()
                .filter(|event| is_traced_event(event))
                .map(|event| format!("{event:?}"))
                .collect()
        });
        for entry in new_events {
            self.log_push(entry);
        }
    }
}

impl Default for PocApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for PocApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.capture_events(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("OmniFab — PoC Android (jalon 0)");
                ui.label("Épreuves : tactile, glisser, clavier virtuel.");
                ui.separator();

                ui.heading("1. Tactile — bouton");
                ui.label(format!("Appuis comptés : {}", self.tap_count));
                let button = egui::Button::new("Appuyez ici")
                    .min_size(egui::vec2(ui.available_width(), 70.0));
                if ui.add(button).clicked() {
                    self.tap_count += 1;
                }
                ui.separator();

                ui.heading("2. Tactile — glisser");
                let (response, painter) = ui
                    .allocate_painter(egui::vec2(ui.available_width(), 220.0), egui::Sense::drag());
                painter.rect_filled(response.rect, 6.0, egui::Color32::from_rgb(28, 28, 34));
                if response.drag_started() {
                    self.drag_count += 1;
                }
                if response.dragged()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    self.circle_pos = Some(pos);
                }
                let center = self.circle_pos.unwrap_or(response.rect.center());
                painter.circle_filled(center, 18.0, egui::Color32::from_rgb(90, 160, 255));
                ui.label(format!("Gestes de glisser : {}", self.drag_count));
                ui.separator();

                ui.heading("3. Clavier virtuel");
                ui.label("Champ sur une ligne :");
                ui.text_edit_singleline(&mut self.text);
                ui.label("Champ multi-lignes :");
                ui.text_edit_multiline(&mut self.text_multi);
                ui.separator();

                ui.heading("4. Journal des événements bruts");
                ui.label(format!(
                    "{} derniers événements touch/texte/touche/zoom :",
                    LOG_LEN
                ));
                if self.log.is_empty() {
                    ui.label("(aucun événement pour l'instant)");
                }
                for entry in &self.log {
                    ui.monospace(entry.clone());
                }
            });
        });
    }
}

/// Point d'entrée Android : le `AndroidApp` doit être injecté dans le
/// `EventLoopBuilder` de winit avant `eframe::run_native` (cf. ADR-0005).
#[cfg(target_os = "android")]
mod android {
    use super::PocApp;

    #[unsafe(no_mangle)]
    fn android_main(app: winit::platform::android::activity::AndroidApp) {
        use winit::platform::android::EventLoopBuilderExtAndroid;

        let options = eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            event_loop_builder: Some(Box::new(move |builder| {
                builder.with_android_app(app);
            })),
            ..Default::default()
        };
        if let Err(error) = eframe::run_native(
            "OmniFab PoC",
            options,
            Box::new(|_cc| Ok(Box::new(PocApp::new()))),
        ) {
            eprintln!("eframe a échoué : {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nouvel_etat_coherent() {
        let app = PocApp::new();
        assert_eq!(app.tap_count, 0);
        assert_eq!(app.drag_count, 0);
        assert!(app.circle_pos.is_none());
        assert!(!app.text.is_empty());
        assert!(app.log.is_empty());
    }

    #[test]
    fn journal_borne() {
        let mut app = PocApp::new();
        for i in 0..(LOG_LEN + 5) {
            app.log_push(format!("evenement {i}"));
        }
        assert_eq!(app.log.len(), LOG_LEN);
    }
}
