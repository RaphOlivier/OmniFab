//! Panneaux de l'atelier : liaisons (ADR-0019) et journal
//! (découpage étape 5 de `preparation/decoupage-main-rs.md`,
//! déplacement mécanique).

use crate::aide::avec_aide;
use crate::aide_du_code;
use crate::atelier_esquisse;
use crate::dialogues::{DialogueNouvelAlias, DialogueSuppression};
use crate::{DemoApp, Vue};
use eframe::egui;
use forge_i18n::Argument;
use forge_session::{ModeSuppression, RemoveAlias, SetExpression, SetInput, Unite};
use forge_units::Length;

impl DemoApp {
    /// Panneau de liaisons (ADR-0019 §6) : le panneau des paramètres
    /// devient la liste des alias du graphe — nom en gras si calculé,
    /// DragValue pour une entrée, champ texte pour une expression,
    /// unité déclarée, erreur du registre en clair — plus la création
    /// d'alias et la suppression. Les cotes système (`esquisse.*`)
    /// sont en lecture seule, jamais supprimables d'ici.
    pub(crate) fn liaisons_panel(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("liaisons-titre"));
        avec_aide(&reponse, "aide-liaisons-titre", &mut self.aide);
        let reponse = ui.button(self.texte("liaisons-nouvel-alias"));
        avec_aide(&reponse, "aide-liaisons-nouvel-alias", &mut self.aide);
        if reponse.clicked() {
            self.dialogue_nouvel_alias = Some(DialogueNouvelAlias {
                nom: String::new(),
                expression: false,
                valeur: 0.0,
                texte: String::new(),
                unite: None,
            });
        }
        ui.separator();
        let libelle_unite_aucune = self.texte("liaisons-unite-aucune");
        let libelle_supprimer = self.texte("liaisons-supprimer");
        // Actions collectées pendant le rendu, exécutées après : le
        // graphe se relit entre deux lignes sans double emprunt.
        let mut entree: Option<(String, f64)> = None;
        let mut expression_modifiee: Option<String> = None;
        let mut unite_modifiee: Option<(String, Option<Unite>)> = None;
        let mut suppression: Option<String> = None;
        let noms = self.session.graph().names();
        for nom in &noms {
            let Some(info) = self.session.graph().noeud(nom) else {
                continue;
            };
            let valeur = self.session.graph_mut().value(nom);
            let valeur_initiale = valeur.as_ref().ok().copied();
            let demande = &mut self.aide;
            ui.horizontal(|ui| {
                if info.expression.is_some() {
                    ui.label(egui::RichText::new(nom.as_str()).strong());
                } else {
                    ui.label(nom.as_str());
                }
                if info.systeme {
                    // Cote système : valeur calculée en lecture seule,
                    // ni édition ni suppression (ADR-0019 §6).
                    if let Ok(calcul) = &valeur {
                        let texte = Length::from_millimeters(*calcul).to_string();
                        ui.strong(texte.as_str());
                    }
                    return;
                }
                if info.expression.is_some() {
                    let (memoire, actif_precedent) = self
                        .tampon_expressions
                        .remove(nom)
                        .unwrap_or((String::new(), false));
                    let courante = info.expression.clone().unwrap_or_default();
                    let mut tampon = if actif_precedent {
                        memoire
                    } else {
                        courante.clone()
                    };
                    let mut actif = false;
                    let reponse =
                        ui.add(egui::TextEdit::singleline(&mut tampon).desired_width(120.0));
                    avec_aide(&reponse, "aide-liaisons-expression", demande);
                    if reponse.has_focus() {
                        actif = true;
                    }
                    // Engagement seulement si le texte a changé : un
                    // faux événement tronquerait les groupes défaits
                    // (ADR-0008, validation C1 round 6).
                    if reponse.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && tampon != courante
                    {
                        expression_modifiee = Some(nom.clone());
                    }
                    if let Ok(calcul) = &valeur {
                        ui.small(format!("{calcul}"));
                    }
                    self.tampon_expressions.insert(nom.clone(), (tampon, actif));
                } else {
                    // Valeur du graphe en début de frame : un
                    // engagement sans changement effectif n'écrit
                    // rien au journal — un faux événement tronquerait
                    // les groupes défaits (ADR-0008).
                    let avant = valeur_initiale.unwrap_or(0.0);
                    // Tampon d'édition persistant du champ : le
                    // DragValue affiche la valeur du début de frame,
                    // un rechargement du graphe à chaque frame
                    // figeait le chiffre pendant le geste (validation
                    // C1 round 6).
                    let (memoire, actif_precedent) =
                        self.tampon_dimensions.remove(nom).unwrap_or((0.0, false));
                    let mut tampon =
                        Self::tampon_apres_inactivite(actif_precedent, memoire, valeur_initiale);
                    let mut actif = false;
                    let reponse = ui.add(egui::DragValue::new(&mut tampon).speed(1.0));
                    avec_aide(&reponse, "aide-liaisons-valeur", demande);
                    // Le clic ouvre le mode édition clavier, qui
                    // garde le focus indéfiniment : un champ focusé ne
                    // réagit plus au glissement. Un appui-glissement
                    // rend le focus pour que le geste déplace la
                    // valeur (piège du glissement, validation C1
                    // round 4).
                    let presse_glisse =
                        ui.input(|i| i.pointer.is_decidedly_dragging() && i.pointer.primary_down());
                    if presse_glisse && reponse.contains_pointer() {
                        ui.memory_mut(|m| m.surrender_focus(reponse.id));
                    }
                    if reponse.dragged() || reponse.has_focus() {
                        actif = true;
                    }
                    if let Some(valeur_engagee) = Self::engagement_params(
                        avant,
                        tampon,
                        reponse.dragged(),
                        reponse.drag_stopped(),
                        reponse.changed(),
                    ) {
                        entree = Some((nom.clone(), valeur_engagee));
                    }
                    self.tampon_dimensions.insert(nom.clone(), (tampon, actif));
                }
                // Unité déclarée (ADR-0019 §6) : sans unité, mm ou
                // deg, redéclarée en réémettant la définition — la
                // cohérence est revérifiée à la définition
                // (OF-VAL-0007).
                let mut choix = info.unite;
                let reponse = egui::ComboBox::from_id_salt(format!("unite-{nom}"))
                    .selected_text(Self::texte_unite(choix, &libelle_unite_aucune))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut choix, None, &libelle_unite_aucune);
                        ui.selectable_value(&mut choix, Some(Unite::Millimetre), "mm");
                        ui.selectable_value(&mut choix, Some(Unite::Degre), "deg");
                    })
                    .response;
                avec_aide(&reponse, "aide-liaisons-unite", demande);
                if choix != info.unite {
                    unite_modifiee = Some((nom.clone(), choix));
                }
                let reponse = ui.small_button(libelle_supprimer.as_str());
                avec_aide(&reponse, "aide-liaisons-supprimer", demande);
                if reponse.clicked() {
                    suppression = Some(nom.clone());
                }
            });
            if let Err(erreur) = &valeur {
                // Badge d'erreur en clair à la ligne concernée : le
                // message du registre, clic droit → page d'aide du
                // code (ADR-0019 §6).
                let message = erreur.to_string();
                let reponse = ui.label(
                    egui::RichText::new(message.as_str())
                        .color(egui::Color32::RED)
                        .small(),
                );
                avec_aide(&reponse, &aide_du_code(&message), &mut self.aide);
            }
        }
        if let Some((nom, valeur_engagee)) = entree {
            let unite = self.session.graph().noeud(&nom).and_then(|info| info.unite);
            let _ = self.executer(&SetInput {
                name: nom,
                value: valeur_engagee,
                unite,
            });
        }
        if let Some(nom) = expression_modifiee
            && let Some(info) = self.session.graph().noeud(&nom)
            && let Some((texte, _)) = self.tampon_expressions.get(&nom)
        {
            let _ = self.executer(&SetExpression {
                name: nom,
                expression: texte.clone(),
                unite: info.unite,
            });
        }
        if let Some((nom, unite)) = unite_modifiee {
            // L'unité se redéclare en réémettant la définition : la
            // cohérence est revérifiée à la définition
            // (OF-VAL-0007, ADR-0019 §6).
            if let Some(info) = self.session.graph().noeud(&nom) {
                if let Some(texte) = &info.expression {
                    let _ = self.executer(&SetExpression {
                        name: nom.clone(),
                        expression: texte.clone(),
                        unite,
                    });
                } else {
                    let valeur = self.session.graph_mut().value(&nom).unwrap_or(0.0);
                    let _ = self.executer(&SetInput {
                        name: nom.clone(),
                        value: valeur,
                        unite,
                    });
                }
            }
        }
        if let Some(nom) = suppression {
            let references = self.session.graph().dependants(&nom).unwrap_or_default();
            if references.is_empty() {
                // Alias libre : suppression directe (ADR-0019 §6).
                let _ = self.executer(&RemoveAlias {
                    name: nom,
                    mode: ModeSuppression::Simple,
                });
            } else {
                // Alias référencé : le dialogue propose cascade,
                // remplacement ou annulation (amendement 2026-10-20).
                self.dialogue_suppression = Some(DialogueSuppression {
                    alias: nom,
                    references,
                    remplacement: String::new(),
                });
            }
        }
        // Outils du mode esquisse (vue 2D) : le croquis contraint se
        // pose dans le viewport (ADR-0018, lot 3c).
        if self.vue == Vue::DeuxD {
            ui.separator();
            atelier_esquisse::panneau(ui, self);
        }
    }

    /// Texte affiché du sélecteur d'unité : « sans unité », ou le
    /// code de l'unité (ADR-0019 §6).
    pub(crate) fn texte_unite(unite: Option<Unite>, sans: &str) -> String {
        match unite {
            None => sans.to_string(),
            Some(u) => u.code().to_string(),
        }
    }

    pub(crate) fn journal_panel(&mut self, ui: &mut egui::Ui) {
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
}
