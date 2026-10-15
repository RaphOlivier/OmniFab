#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-i18n — moteur Fluent embarqué et pages d'aide
//!
//! (ADR-0010.) Frontière unique entre les clés d'interface et le
//! texte affiché : les applications embarquent leurs fichiers FTL à
//! la compilation (`include_str!`), cette crate fournit le catalogue
//! (traductions à clés, [`Catalogue`]) et les pages d'aide
//! ([`PageAide`]).
//!
//! Conventions :
//!
//! - toute chaîne d'interface vit derrière une clé Fluent dès le
//!   jalon 1 ;
//! - chaque widget porte un `help_id` : la page d'aide est le message
//!   Fluent du même nom, avec les attributs `titre`, `description` et
//!   `exemple` — une page incomplète est une erreur typée
//!   (OF-I18N-0003), attrapée par les tests des applications ;
//! - langue de référence : le français ; une autre langue se fait par
//!   fichiers FTL additionnels, sans toucher au code ;
//! - cette crate est **pure** : ni egui, ni I/O disque — les apps
//!   embarquent la source FTL, elle ne se charge jamais à l'exécution.

mod catalogue;
mod erreur;

pub use catalogue::{Argument, Catalogue, PageAide};
pub use erreur::I18nError;

#[cfg(test)]
mod tests {
    use super::{Argument, Catalogue};

    /// Un FTL minimal couvrant tous les cas du moteur.
    const FTL: &str = "salut-monde = Bonjour, le monde !\navec-argument = { $n } événements rejoués.\npage-aide-demo =\n    .titre = Démo\n    .description = Un widget de démonstration.\n    .exemple = Cliquez, puis observez le journal.\npage-incomplete =\n    .titre = Démo\n";

    /// Catalogue de test (fr).
    fn catalogue() -> Catalogue {
        Catalogue::depuis_ftl("fr", FTL).expect("FTL de test valide")
    }

    #[test]
    fn traduit_une_cle() {
        let texte = catalogue().traduction("salut-monde").expect("traduction");
        assert_eq!(texte, "Bonjour, le monde !");
    }

    #[test]
    fn cle_absente_est_une_erreur_typee() {
        let erreur = catalogue().traduction("inconnu").unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0001");
    }

    #[test]
    fn traduit_avec_argument_entier() {
        let texte = catalogue()
            .traduction_avec("avec-argument", &[("n", Argument::Entier(12))])
            .expect("traduction");
        assert!(texte.contains("12"), "le nombre apparaît : {texte}");
        assert!(texte.contains("événements"), "le texte apparaît : {texte}");
    }

    #[test]
    fn argument_manquant_est_une_erreur_typee() {
        let erreur = catalogue().traduction("avec-argument").unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0004");
    }

    #[test]
    fn ftl_invalide_est_une_erreur_typee() {
        let erreur = Catalogue::depuis_ftl("fr", "pas un ftl valide\n").unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0002");
    }

    #[test]
    fn langue_invalide_est_une_erreur_typee() {
        let erreur = Catalogue::depuis_ftl("123", FTL).unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0002");
    }

    #[test]
    fn page_d_aide_complete() {
        let page = catalogue().page_aide("page-aide-demo").expect("page complète");
        assert_eq!(page.titre, "Démo");
        assert!(page.description.contains("démonstration"));
        assert!(page.exemple.contains("journal"));
    }

    #[test]
    fn page_d_aide_incomplete_est_une_erreur_typee() {
        let erreur = catalogue().page_aide("page-incomplete").unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0003");
    }

    #[test]
    fn page_inexistante_est_une_cle_absente() {
        let erreur = catalogue().page_aide("inconnu").unwrap_err();
        assert_eq!(erreur.code(), "OF-I18N-0001");
    }

    #[test]
    fn contient_repond_pour_les_cles_connues() {
        let catalogue = catalogue();
        assert!(catalogue.contient("salut-monde"));
        assert!(!catalogue.contient("inconnu"));
    }
}
