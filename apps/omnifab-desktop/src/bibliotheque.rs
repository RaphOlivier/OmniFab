//! Bibliothèque de projets : dossier dédié + écran d'accueil
//! (ADR-0014). Un projet de la bibliothèque est un fichier
//! `.omnifab` du dossier `OmniFab/Projets` du dossier personnel —
//! visible et manipulable à la main, aucune configuration au jalon 2.

use std::path::{Path, PathBuf};

/// Sous-dossier de la bibliothèque dans le dossier personnel.
const SOUS_DOSSIER: &str = "OmniFab/Projets";
/// Extension d'un fichier projet OmniFab (conteneur ZIP, jalon 0).
pub const EXTENSION: &str = "omnifab";

/// Un projet de la bibliothèque : un fichier `.omnifab` du dossier.
#[derive(Debug, Clone)]
pub struct EntreeProjet {
    /// Chemin complet du fichier.
    pub chemin: PathBuf,
    /// Nom du fichier, sans extension.
    pub nom: String,
    /// Dernière modification, en secondes Unix.
    pub modifie_le: u64,
    /// Taille du fichier, en octets.
    pub taille: u64,
}

/// Crée (si besoin) le sous-dossier de la bibliothèque sous `racine`.
fn creer_dossier_dans(racine: &Path) -> std::io::Result<PathBuf> {
    let dossier = racine.join(SOUS_DOSSIER);
    std::fs::create_dir_all(&dossier)?;
    Ok(dossier)
}

/// Dossier de la bibliothèque de l'utilisateur courant
/// (`<dossier personnel>/OmniFab/Projets`), créé s'il manque
/// (ADR-0014). `None` : dossier personnel introuvable ou création
/// impossible — l'écran d'accueil s'affiche alors sans la liste.
pub fn dossier() -> Option<PathBuf> {
    creer_dossier_dans(&std::env::home_dir()?).ok()
}

/// Liste les projets d'un dossier : fichiers `.omnifab` uniquement,
/// du plus récemment modifié au plus ancien. Un fichier illisible
/// est ignoré — la bibliothèque ne doit jamais empêcher le
/// démarrage de l'application.
pub fn lister(dossier: &Path) -> Vec<EntreeProjet> {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return Vec::new();
    };
    let mut projets: Vec<EntreeProjet> = entrees
        .filter_map(|entree| {
            let chemin = entree.ok()?.path();
            let extension = chemin.extension()?.to_str()?;
            if extension != EXTENSION {
                return None;
            }
            let nom = chemin.file_stem()?.to_str()?.to_string();
            let metadonnees = std::fs::metadata(&chemin).ok()?;
            let modifie = metadonnees.modified().ok()?;
            let secondes = modifie
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(EntreeProjet {
                chemin,
                nom,
                modifie_le: secondes,
                taille: metadonnees.len(),
            })
        })
        .collect();
    trier(&mut projets);
    projets
}

/// Tri du plus récent au plus ancien (stable : deux projets modifiés
/// la même seconde gardent l'ordre du dossier).
fn trier(projets: &mut [EntreeProjet]) {
    projets.sort_by_key(|projet| std::cmp::Reverse(projet.modifie_le));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier de test jetable, unique par appel.
    fn dossier_temp(nom: &str) -> PathBuf {
        let racine = std::env::temp_dir().join(format!("omnifab-bibliotheque-{nom}"));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(&racine).expect("dossier de test");
        racine
    }

    /// Le sous-dossier de la bibliothèque est créé, puis réutilisé.
    #[test]
    fn creer_le_dossier_bibliotheque() {
        let racine = dossier_temp("creer");
        let dossier = creer_dossier_dans(&racine).expect("création");
        assert!(dossier.starts_with(&racine), "sous la racine");
        assert!(dossier.is_dir(), "le dossier existe");
        creer_dossier_dans(&racine).expect("création idempotente");
    }

    /// Seuls les fichiers `.omnifab` sont listés, avec leurs
    /// métadonnées.
    #[test]
    fn lister_ne_garde_que_les_projets() {
        let racine = dossier_temp("lister");
        let dossier = creer_dossier_dans(&racine).expect("création");
        std::fs::write(dossier.join("a.omnifab"), b"a").expect("écriture");
        std::fs::write(dossier.join("b.omnifab"), b"bb").expect("écriture");
        std::fs::write(dossier.join("c.txt"), b"ignore").expect("écriture");
        let projets = lister(&dossier);
        assert_eq!(projets.len(), 2, "seuls les .omnifab sont listés");
        let noms: Vec<&str> = projets.iter().map(|p| p.nom.as_str()).collect();
        assert!(noms.contains(&"a"), "projet a listé");
        assert!(noms.contains(&"b"), "projet b listé");
        let tailles: Vec<u64> = projets.iter().map(|p| p.taille).collect();
        assert!(tailles.contains(&1), "taille de a lue");
    }

    /// Le tri va du plus récent au plus ancien.
    #[test]
    fn trier_range_du_plus_recent_au_plus_ancien() {
        let racine = dossier_temp("trier");
        let chemin = racine.join("x.omnifab");
        let vieux = EntreeProjet {
            chemin: chemin.clone(),
            nom: "x".into(),
            modifie_le: 100,
            taille: 1,
        };
        let recent = EntreeProjet {
            chemin,
            nom: "x".into(),
            modifie_le: 200,
            taille: 1,
        };
        let mut projets = vec![vieux, recent];
        trier(&mut projets);
        assert_eq!(projets[0].modifie_le, 200, "le plus récent d'abord");
    }
}
