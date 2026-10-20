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
    let calque = ui.interact(
        reponse.rect,
        reponse.id.with(help_id),
        egui::Sense::click(),
    );
    if calque.clicked_by(egui::PointerButton::Secondary) {
        *ouverte = Some(help_id.to_string());
    }
}

/// Affiche la fenêtre d'aide, à appeler à chaque trame.
///
/// `ouverte` à `Some(help_id)` ouvre la page du widget, `None` ne
/// dessine rien ; fermer la fenêtre remet `ouverte` à `None`.
pub fn fenetre(ctx: &egui::Context, catalogue: &Catalogue, ouverte: &mut Option<String>) {
    let Some(help_id) = ouverte.clone() else {
        return;
    };
    let mut visible = true;
    match catalogue.page_aide(&help_id) {
        Ok(page) => {
            let titre = catalogue
                .traduction_avec(CLE_TITRE, &[("titre", page.titre.as_str().into())])
                .expect("clé présente dans le FTL (test de la CI)");
            egui::Window::new(titre)
                .open(&mut visible)
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.heading(&page.titre);
                    ui.label(&page.description);
                    ui.separator();
                    let exemple = catalogue
                        .traduction(CLE_EXEMPLE)
                        .expect("clé présente dans le FTL (test de la CI)");
                    ui.label(exemple);
                    ui.label(&page.exemple);
                });
        }
        Err(erreur) => {
            let texte = match erreur {
                I18nError::CleAbsente { .. } => catalogue
                    .traduction_avec(CLE_INTROUVABLE, &[("identifiant", help_id.as_str().into())])
                    .expect("clé présente dans le FTL (test de la CI)"),
                autre => autre.to_string(),
            };
            egui::Window::new(&help_id)
                .open(&mut visible)
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.label(texte);
                });
        }
    }
    if !visible {
        *ouverte = None;
    }
}
