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
        "top-annuler",
        "top-retablir",
        "top-importer-svg",
        "top-importer-dxf",
        "top-importer-stl",
        "top-exporter-svg",
        "top-exporter-dxf",
        "top-exporter-stl",
        "top-recalculer",
        "top-enregistrer",
        "top-enregistrer-sous",
        "top-ouvrir",
        "top-nouveau",
        "top-bibliotheque",
        "filtre-svg",
        "filtre-dxf",
        "filtre-stl",
        "filtre-projet",
        "accueil-titre",
        "accueil-nouveau",
        "accueil-ouvrir",
        "accueil-reprendre",
        "accueil-liste-titre",
        "accueil-vide",
        "accueil-indisponible",
        "accueil-ouvrir-projet",
        "accueil-date-aujourdhui",
        "accueil-date-hier",
        "accueil-date-jours",
        "accueil-taille-octets",
        "accueil-taille-ko",
        "accueil-taille-mio",
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
        "geom-traces-compteur",
        "geom-maillage-compteur",
        "viewport-vue-2d",
        "viewport-vue-3d",
        "viewport-wgpu-indisponible",
        "esquisse-titre",
        "esquisse-outil-selection",
        "esquisse-outil-point",
        "esquisse-outil-ligne",
        "esquisse-outil-cercle",
        "esquisse-outil-arc",
        "esquisse-contraintes",
        "esquisse-contrainte-horizontale",
        "esquisse-contrainte-verticale",
        "esquisse-contrainte-point-fixe",
        "esquisse-contrainte-distance",
        "esquisse-cote-distance",
        "esquisse-dof",
        "esquisse-redondances",
        "info-recalcul-ok",
        "info-enregistre",
        "info-recharge",
        "info-import-traces",
        "info-import-maillage",
        "info-export",
        "info-travail-en-cours",
        "titre-fenetre-modifie",
        "confirmation-titre",
        "confirmation-texte",
        "confirmation-confirmer",
        "confirmation-annuler",
        "echelle-titre",
        "echelle-texte",
        "echelle-metres",
        "echelle-millimetres",
        "echelle-annuler",
        aide::CLE_TITRE,
        aide::CLE_EXEMPLE,
        aide::CLE_INTROUVABLE,
    ];

    /// Pages d'aide attendues : une par widget interactif et par
    /// titre de panneau (convention `help_id`, ADR-0010).
    const HELP_IDS: &[&str] = &[
        "aide-top-annuler",
        "aide-top-retablir",
        "aide-top-importer-svg",
        "aide-top-importer-dxf",
        "aide-top-importer-stl",
        "aide-top-exporter-svg",
        "aide-top-exporter-dxf",
        "aide-top-exporter-stl",
        "aide-top-recalculer",
        "aide-top-enregistrer",
        "aide-top-enregistrer-sous",
        "aide-top-ouvrir",
        "aide-top-nouveau",
        "aide-top-bibliotheque",
        "aide-accueil-titre",
        "aide-accueil-nouveau",
        "aide-accueil-ouvrir",
        "aide-accueil-reprendre",
        "aide-accueil-liste",
        "aide-accueil-projet",
        "aide-params-titre",
        "aide-params-largeur",
        "aide-params-hauteur",
        "aide-params-expression",
        "aide-journal-titre",
        "aide-viewport-titre",
        "aide-viewport-camera",
        "aide-viewport-vue-2d",
        "aide-viewport-vue-3d",
        "aide-esquisse-titre",
        "aide-esquisse-outil-selection",
        "aide-esquisse-outil-point",
        "aide-esquisse-outil-ligne",
        "aide-esquisse-outil-cercle",
        "aide-esquisse-outil-arc",
        "aide-esquisse-contraintes",
        "aide-esquisse-contrainte-horizontale",
        "aide-esquisse-contrainte-verticale",
        "aide-esquisse-contrainte-point-fixe",
        "aide-esquisse-contrainte-distance",
        "aide-esquisse-cote-distance",
        "aide-geom-compteur",
        "aide-confirmation-perte",
        "aide-echelle-metres",
        "aide-echelle-millimetres",
        "aide-echelle-annuler",
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
