//! Sélecteurs de fichiers natifs du démonstrateur (ADR-0014).
//!
//! Toute demande de fichier à l'utilisateur passe par ce module, isolé
//! pour le jalon 7 (Android : intent système en remplacement) et le
//! jalon 9 (web : API async de `rfd`). Sous Linux, `rfd` passe par
//! xdg-desktop-portal (ADR-0014) : sans portail installé, le dialogue
//! ne s'ouvre pas et les fonctions retournent `None` — l'application
//! continue de fonctionner.
//!
//! Les libellés des filtres sont traduits par l'appelant (ADR-0010) :
//! ce module ne connaît aucune chaîne d'interface.

use std::path::PathBuf;

/// Demande un fichier à importer, filtré par extension.
/// `None` : annulé, ou sélecteur indisponible.
pub fn choisir_import(libelle: &str, extensions: &[&str]) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(libelle, extensions)
        .pick_file()
}

/// Demande un chemin d'export pour le format donné : le suffixe est
/// ajouté si l'utilisateur l'a omis. `None` : annulé, ou sélecteur
/// indisponible.
pub fn chemin_export(libelle: &str, extensions: &[&str]) -> Option<PathBuf> {
    let chemin = rfd::FileDialog::new()
        .add_filter(libelle, extensions)
        .save_file()?;
    Some(ajouter_suffixe(chemin, extensions))
}

/// Demande un chemin pour un nouveau projet `.omnifab` : la
/// bibliothèque est proposée comme dossier par défaut quand elle est
/// disponible (ADR-0014). L'extension `.omnifab` est ajoutée si
/// l'utilisateur l'a omise (ADR-0016 C5).
pub fn chemin_projet(libelle: &str, dossier_defaut: Option<&std::path::Path>) -> Option<PathBuf> {
    let dialogue = rfd::FileDialog::new()
        .add_filter(libelle, &["omnifab"])
        .set_file_name("projet.omnifab");
    let dialogue = match dossier_defaut {
        Some(dossier) => dialogue.set_directory(dossier),
        None => dialogue,
    };
    let chemin = dialogue.save_file()?;
    Some(ajouter_suffixe(chemin, &["omnifab"]))
}

/// Demande un projet existant à ouvrir, n'importe où sur le disque.
pub fn choisir_projet(libelle: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(libelle, &["omnifab"])
        .pick_file()
}

/// Ajoute le premier suffixe quand l'extension actuelle n'est pas
/// déjà parmi les extensions attendues (comparaison sans casse) :
/// « plan v1.5 » devient « plan v1.5.svg » — son extension est « 5 »,
/// pas « svg » —, « dessin.SVG » est conservé tel quel (ADR-0016 C5).
fn ajouter_suffixe(chemin: PathBuf, extensions: &[&str]) -> PathBuf {
    let extension = chemin
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    let extension_attendue = extension.as_deref().is_some_and(|extension| {
        extensions
            .iter()
            .any(|attendue| attendue.to_ascii_lowercase() == extension)
    });
    if extension_attendue {
        return chemin;
    }
    let mut nom = chemin.into_os_string();
    nom.push(".");
    nom.push(extensions.first().copied().unwrap_or_default());
    PathBuf::from(nom)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le suffixe est ajouté seulement quand il manque.
    #[test]
    fn ajouter_suffixe_quand_il_manque() {
        let chemin = ajouter_suffixe(PathBuf::from("/tmp/dessin"), &["svg"]);
        assert_eq!(chemin.extension().and_then(|e| e.to_str()), Some("svg"));
        let chemin = ajouter_suffixe(PathBuf::from("/tmp/dessin.svg"), &["svg"]);
        assert_eq!(chemin.to_str(), Some("/tmp/dessin.svg"), "suffixe conservé");
    }

    /// « plan v1.5 » a pour extension « 5 » : le suffixe est ajouté ;
    /// une extension attendue en casse différente est conservée
    /// (ADR-0016 C5).
    #[test]
    fn ajouter_suffixe_v1_5_et_casse() {
        let chemin = ajouter_suffixe(PathBuf::from("/tmp/plan v1.5"), &["svg"]);
        assert_eq!(
            chemin.to_str(),
            Some("/tmp/plan v1.5.svg"),
            "l'extension « 5 » n'est pas « svg » : le suffixe est ajouté"
        );
        let chemin = ajouter_suffixe(PathBuf::from("/tmp/dessin.SVG"), &["svg"]);
        assert_eq!(
            chemin.to_str(),
            Some("/tmp/dessin.SVG"),
            "extension attendue, casse différente : conservée"
        );
        let chemin = ajouter_suffixe(PathBuf::from("/tmp/projet"), &["omnifab"]);
        assert_eq!(
            chemin.to_str(),
            Some("/tmp/projet.omnifab"),
            "projet sans extension complété"
        );
    }
}
