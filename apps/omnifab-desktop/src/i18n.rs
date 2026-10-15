//! Catalogue français embarqué du démonstrateur (ADR-0010).
//!
//! Le fichier FTL vit dans `locales/fr/main.ftl` et est embarqué à la
//! compilation (`include_str!`) : aucune lecture disque à l'exécution,
//! et la CI vérifie sa complétude (clés et pages d'aide, cf. les tests).

pub use forge_i18n::Catalogue;

/// Source FTL française — langue de référence du projet (ADR-0010).
const FR: &str = include_str!("../locales/fr/main.ftl");

/// Catalogue français du démonstrateur.
pub fn catalogue() -> Catalogue {
    Catalogue::depuis_ftl("fr", FR).expect("FTL embarqué valide (test de la CI)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aide;

    /// Toutes les clés d'interface attendues dans le FTL embarqué :
    /// celles des panneaux (main.rs) et celles de la fenêtre d'aide
    /// (aide.rs). Une clé absente = CI en échec.
    const CLES: &[&str] = &[
        "titre-fenetre",
        "top-recalculer",
        "top-enregistrer",
        "top-ouvrir",
        "top-nouveau",
        "params-titre",
        "params-largeur",
        "params-hauteur",
        "params-expression",
        "params-diagonale",
        "journal-titre",
        "journal-compteur",
        "journal-types-ignores",
        "viewport-titre",
        "viewport-erreur",
        "info-recalcul-ok",
        "info-enregistre",
        "info-recharge",
        aide::CLE_TITRE,
        aide::CLE_EXEMPLE,
        aide::CLE_INTROUVABLE,
    ];

    /// Pages d'aide attendues : une par widget interactif et par
    /// titre de panneau (convention `help_id`, ADR-0010).
    const HELP_IDS: &[&str] = &[
        "aide-top-recalculer",
        "aide-top-enregistrer",
        "aide-top-ouvrir",
        "aide-top-nouveau",
        "aide-params-titre",
        "aide-params-largeur",
        "aide-params-hauteur",
        "aide-params-expression",
        "aide-journal-titre",
        "aide-viewport-titre",
    ];

    /// Chaque clé utilisée par l'interface existe dans le FTL.
    #[test]
    fn toutes_les_cles_existent() {
        let catalogue = catalogue();
        for cle in CLES {
            assert!(catalogue.contient(cle), "clé absente du FTL : {cle}");
        }
    }

    /// Chaque page d'aide est complète : titre, description et
    /// exemple non vide — convention « une page sans exemple = CI en
    /// échec » (ADR-0010).
    #[test]
    fn toutes_les_pages_d_aide_sont_completes() {
        let catalogue = catalogue();
        for help_id in HELP_IDS {
            let page = catalogue.page_aide(help_id).expect("page complète");
            assert!(!page.exemple.is_empty(), "page sans exemple : {help_id}");
        }
    }
}
