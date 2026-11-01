//! Barre du haut de l'atelier (découpage étape 5 de
//! `preparation/decoupage-main-rs.md`, déplacement mécanique).

use crate::aide::{avec_aide, avec_aide_sur_calque};
use crate::fichiers;
use crate::{ActionDestructrice, DemoApp, Ecran};
use eframe::egui;
use forge_io::{dxf, stl, svg};

impl DemoApp {
    pub(crate) fn top_bar(&mut self, ui: &mut egui::Ui) {
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
}
