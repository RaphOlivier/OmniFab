//! Aide intégrée : chaque widget interactif porte un `help_id` et un
//! clic droit ouvre sa page d'aide (ADR-0010).
//!
//! La page d'un widget est le message Fluent du même nom que son
//! `help_id`, avec les attributs `titre`, `description` et `exemple`
//! (cf. `locales/fr/main.ftl`). Une page incomplète est une erreur
//! typée (OF-I18N-0003), attrapée par les tests du démonstrateur.

use eframe::egui;
use forge_i18n::{Catalogue, I18nError};

/// Clé FTL du titre de la fenêtre d'aide (« Aide — X »).
pub const CLE_TITRE: &str = "aide-titre-fenetre";
/// Clé FTL de l'étiquette précédant l'exemple concret.
pub const CLE_EXEMPLE: &str = "aide-section-exemple";
/// Clé FTL du message « page d'aide introuvable ».
pub const CLE_INTROUVABLE: &str = "aide-page-introuvable";

/// Attache une page d'aide à un widget : un clic droit (bouton
/// secondaire) ouvre la fenêtre d'aide portée par `help_id`.
///
/// `ouverte` retient le `help_id` de la page demandée (`None` quand la
/// fenêtre est fermée) ; il est partagé par tous les widgets de
/// l'application.
pub fn avec_aide(reponse: &egui::Response, help_id: &str, ouverte: &mut Option<String>) {
    if reponse.clicked_by(egui::PointerButton::Secondary) {
        *ouverte = Some(help_id.to_string());
    }
}

/// Attache une page d'aide à un widget, y compris désactivé : egui
/// ne délivre pas les clics aux widgets désactivés, un calque
/// interactif posé sur le même rectangle reçoit le clic droit à
/// leur place (ADR-0010, validation jalon 2 — l'aide des boutons
/// grisés doit s'ouvrir aussi).
///
/// Le calque n'est posé que sur un widget **désactivé** : egui
/// délivre le clic au dernier widget interactif posé sur le
/// rectangle — sur un widget actif, le calque lui volerait le clic
/// gauche (correctif de la validation jalon 2 : Annuler, Rétablir,
/// les contraintes et les exports ne répondaient plus au clic,
/// ADR-0020).
pub fn avec_aide_sur_calque(
    ui: &egui::Ui,
    reponse: &egui::Response,
    help_id: &str,
    ouverte: &mut Option<String>,
) {
    if reponse.clicked_by(egui::PointerButton::Secondary) {
        *ouverte = Some(help_id.to_string());
        return;
    }
    if !reponse.enabled() {
        let calque = ui.interact(reponse.rect, reponse.id.with(help_id), egui::Sense::click());
        if calque.clicked_by(egui::PointerButton::Secondary) {
            *ouverte = Some(help_id.to_string());
        }
    }
}

/// Affiche la fenêtre d'aide, à appeler à chaque trame.
///
/// `ouverte` à `Some(help_id)` ouvre la page du widget, `None` ne
/// dessine rien ; fermer la fenêtre remet `ouverte` à `None`. Chaque
/// nouvelle demande ouvre la fenêtre près du curseur — le
/// demandeur vient de cliquer là (ADR-0020 : la validation jalon 2
/// la voyait s'ouvrir au milieu de l'écran).
pub fn fenetre(ctx: &egui::Context, catalogue: &Catalogue, ouverte: &mut Option<String>) {
    let Some(help_id) = ouverte.clone() else {
        // Fermée : la prochaine demande compte pour neuve.
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("aide-page"), String::new()));
        return;
    };
    let nouvelle = ctx.data_mut(|d| {
        let precedente = d.get_temp::<String>(egui::Id::new("aide-page"));
        d.insert_temp(egui::Id::new("aide-page"), help_id.clone());
        precedente.as_deref() != Some(help_id.as_str())
    });
    let pos = ctx
        .pointer_latest_pos()
        .map(|pos| pos + egui::vec2(16.0, 16.0));
    let mut visible = true;
    match catalogue.page_aide(&help_id) {
        Ok(page) => {
            let titre = catalogue
                .traduction_avec(CLE_TITRE, &[("titre", page.titre.as_str().into())])
                .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)");
            let mut fenetre = egui::Window::new(titre)
                .open(&mut visible)
                .default_width(360.0);
            if nouvelle && let Some(pos) = pos {
                fenetre = fenetre.current_pos(pos);
            }
            fenetre.show(ctx, |ui| {
                ui.heading(&page.titre);
                ui.label(&page.description);
                ui.separator();
                let exemple = catalogue
                    .traduction(CLE_EXEMPLE)
                    .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)");
                ui.label(exemple);
                ui.label(&page.exemple);
            });
        }
        Err(erreur) => {
            let texte = match erreur {
                I18nError::CleAbsente { .. } => catalogue
                    .traduction_avec(CLE_INTROUVABLE, &[("identifiant", help_id.as_str().into())])
                    .expect("invariant : clé FTL vérifiée par toutes_les_cles_existent (CI)"),
                autre => autre.to_string(),
            };
            let mut fenetre = egui::Window::new(&help_id)
                .open(&mut visible)
                .default_width(360.0);
            if nouvelle && let Some(pos) = pos {
                fenetre = fenetre.current_pos(pos);
            }
            fenetre.show(ctx, |ui| {
                ui.label(texte);
            });
        }
    }
    if !visible {
        *ouverte = None;
    }
}
