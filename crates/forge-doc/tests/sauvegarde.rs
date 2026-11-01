// Convention expect v2 (2026-11-01, lot E) : échafaudage de test —
// `allow-expect-in-tests` (clippy.toml) ne couvre que les fonctions
// `#[test]` et les modules `#[cfg(test)]`, pas les helpers de ce
// fichier d'intégration : expect admis dans tout le fichier.
#![allow(clippy::expect_used)]

use std::path::PathBuf;

use forge_doc::{Document, NewEvent};
use serde_json::json;

fn dossier_temporaire() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COMPTEUR: AtomicUsize = AtomicUsize::new(0);
    let dossier = std::env::temp_dir().join(format!(
        "forge-doc-sauvegarde-{}-{}",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dossier).expect("dossier temporaire créé");
    dossier
}

fn document(n_evenements: usize) -> Document {
    let mut document = Document::new();
    for i in 0..n_evenements {
        document.append(NewEvent::new("demo.test", 1, "tests", json!({ "i": i })));
    }
    document
}

/// Sauvegardes répétées : le projet existe à chaque étape, le `.bak`
/// contient la version précédente, et le projet relu est complet
/// (ADR-0016 A2).
#[test]
fn sauvegardes_repetees_rotent_le_backup() {
    let dossier = dossier_temporaire();
    let projet = dossier.join("projet.omnifab");
    let backup = dossier.join("projet.omnifab.bak");

    document(1)
        .save_to_path(&projet)
        .expect("première sauvegarde");
    assert!(
        projet.exists(),
        "le projet existe après la première sauvegarde"
    );
    assert!(!backup.exists(), "aucun `.bak` sans projet précédent");
    assert_eq!(Document::open_from_path(&projet).unwrap().events().len(), 1);

    document(2)
        .save_to_path(&projet)
        .expect("deuxième sauvegarde");
    assert_eq!(Document::open_from_path(&projet).unwrap().events().len(), 2);
    assert_eq!(
        Document::open_from_path(&backup).unwrap().events().len(),
        1,
        "le `.bak` contient la version précédente"
    );

    document(3)
        .save_to_path(&projet)
        .expect("troisième sauvegarde");
    assert_eq!(Document::open_from_path(&projet).unwrap().events().len(), 3);
    assert_eq!(
        Document::open_from_path(&backup).unwrap().events().len(),
        2,
        "le `.bak` est recomposé à chaque sauvegarde"
    );

    let _ = std::fs::remove_dir_all(&dossier);
}

/// Un échec d'écriture ne doit jamais retirer le projet de son
/// chemin : le temporaire est écrit et synchronisé AVANT de toucher
/// à l'existant (ADR-0016 A2). Un dossier en travers du `.tmp`
/// simule l'échec de création du fichier.
#[test]
fn echec_ecriture_preserve_le_projet() {
    let dossier = dossier_temporaire();
    let projet = dossier.join("projet.omnifab");

    document(1)
        .save_to_path(&projet)
        .expect("sauvegarde initiale");
    let original = std::fs::read(&projet).expect("projet initial lisible");

    // Bloqueur : `File::create` échoue sur un dossier.
    let bloqueur = dossier.join("projet.omnifab.tmp");
    std::fs::create_dir(&bloqueur).expect("bloqueur en place");

    let resultat = document(2).save_to_path(&projet);
    assert!(resultat.is_err(), "l'écriture doit échouer");
    assert!(
        projet.exists(),
        "le projet reste à son chemin après l'échec"
    );
    assert_eq!(
        std::fs::read(&projet).unwrap(),
        original,
        "le projet est inchangé"
    );
    assert!(
        !dossier.join("projet.omnifab.bak").exists(),
        "aucun `.bak` créé sur un échec"
    );

    // Bloqueur levé : la sauvegarde repart.
    std::fs::remove_dir(&bloqueur).expect("bloqueur retiré");
    document(2)
        .save_to_path(&projet)
        .expect("sauvegarde après retrait du bloqueur");
    assert_eq!(Document::open_from_path(&projet).unwrap().events().len(), 2);

    let _ = std::fs::remove_dir_all(&dossier);
}
