//! Fenêtres de dialogue des liaisons (ADR-0019 §6,
//! découpage étape 5 de `preparation/decoupage-main-rs.md`,
//! déplacement mécanique).

use crate::aide::avec_aide;
use crate::app::DemoApp;
use eframe::egui;
use forge_i18n::Argument;
use forge_session::{ModeSuppression, RemoveAlias, SetExpression, SetInput, Unite};

impl DemoApp {
    /// Fenêtres de dialogue des liaisons (ADR-0019 §6) : création
    /// d'alias et suppression d'un alias référencé — un refus laisse
    /// le dialogue ouvert, la saisie reste corrigeable
    /// (ADR-0016 C4).
    pub(crate) fn fenetre_dialogues_liaisons(&mut self, ctx: &egui::Context) {
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
