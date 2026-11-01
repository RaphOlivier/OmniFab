//! Écran d'accueil : bibliothèque de projets (ADR-0014,
//! découpage étape 5 de `preparation/decoupage-main-rs.md`,
//! déplacement mécanique).

use crate::aide::avec_aide;
use crate::app::{ActionDestructrice, DemoApp};
use crate::bibliotheque;
use crate::bibliotheque::EntreeProjet;
use crate::theme::EVENEMENTS_VIERGES;
use eframe::egui;
use forge_i18n::Argument;

impl DemoApp {
    /// Ligne descriptive d'un projet de la bibliothèque : âge et
    /// taille, traduits (ADR-0010).
    pub(crate) fn decrire_entree(&self, projet: &EntreeProjet) -> String {
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
    pub(crate) fn accueil(&mut self, ui: &mut egui::Ui) {
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
}
