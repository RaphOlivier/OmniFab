#![forbid(unsafe_code)]

//! # omnifab-desktop — démonstrateur des jalons 1 et 2
//!
//! Lanceur seul — découpage 6/6 de `main.rs` (fiche
//! `preparation/decoupage-main-rs.md`) : l'application egui vit dans
//! le module `app` (`DemoApp` et son impl `eframe::App`), le fil de
//! travail dans `travail.rs` ; ce fichier ne porte que `main`.

mod aide;
mod app;
mod atelier_esquisse;
mod bibliotheque;
mod demo;
mod dialogues;
mod esquisse;
mod fichiers;
mod i18n;
mod theme;
mod travail;
mod ui;

use app::DemoApp;
use eframe::egui;

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
            if let Some(scenario) = scenario.as_deref()
                && let Err(raison) = app.preparer_demo(scenario)
            {
                eprintln!("omnifab : scénario de démo refusé : {raison}");
            }
            Ok(Box::new(app))
        }),
    )
}
