//! Rejeu d'un journal de croquis sans déplacement (lot C de
//! l'audit « expect ») : un point ajouté sans jamais être glissé —
//! aucun \`sketch.point.moved\` au journal — rejoue et se résout
//! à sa position d'origine, sans panique. La boucle des graines de
//! la passe de résolution en pose une pour tout point du croquis
//! (repli sur les coordonnées d'origine en l'absence de
//! déplacement) ; le site qu'un \`expect\` gardait est inatteignable
//! par journal — les identifiants séquentiels sont vérifiés au
//! rejeu — et le refus typé \`OF-GCS-0001\` le remplace en défense
//! en profondeur.

// Convention expect v2 (2026-11-01, lot E) : échafaudage de test —
// `allow-expect-in-tests` (clippy.toml) ne couvre que les fonctions
// `#[test]` et les modules `#[cfg(test)]`, pas les helpers de ce
// fichier d'intégration : expect admis dans tout le fichier.
#![allow(clippy::expect_used)]

use forge_doc::{Document, NewEvent};
use forge_gcs::Systeme2D;
use forge_session::Session;
use forge_units::Length;

/// Position résolue en millimètres, pour les asserts.
fn mm(position: Option<(Length, Length)>) -> (f64, f64) {
    let (x, y) = position.expect("position résolue");
    (x.millimeters(), y.millimeters())
}

/// Journal forgé d'un seul point posé, jamais déplacé : le journal
/// « hostile » de la fiche du lot — pas de \`sketch.point.moved\`.
/// La fiche présumait la graine absente au rejeu et une panique ;
/// le repli du code réel pose la position d'origine — le rejeu doit
/// résoudre sans panique, à l'origine.
#[test]
fn point_jamais_deplace_rejoue_a_son_origine() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        "sketch.entity.added",
        2,
        "tests",
        serde_json::json!({"genre": "point", "id": 0, "x": 10.0, "y": 20.0}),
    ));
    let session = Session::from_document("tests", document).expect("rejeu");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session
        .esquisse()
        .resoudre(&mut solveur)
        .expect("résolu sans panique");
    assert_eq!(resolu.dof(), 2, "un point libre : deux degrés de liberté");
    let (x, y) = mm(resolu.position(0));
    assert!(
        (x - 10.0).abs() < 1e-9 && (y - 20.0).abs() < 1e-9,
        "la graine d'un point jamais déplacé est sa position d'origine"
    );
}

/// Journal légitime témoin : le même point suivi d'un déplacement
/// rejoue à sa dernière graine — le comportement ordinaire tient,
/// le rejeu rejoue à l'identique.
#[test]
fn point_deplace_rejoue_a_sa_derniere_graine() {
    let mut document = Document::new();
    document.append(NewEvent::new(
        "sketch.entity.added",
        2,
        "tests",
        serde_json::json!({"genre": "point", "id": 0, "x": 10.0, "y": 20.0}),
    ));
    document.append(NewEvent::new(
        "sketch.point.moved",
        1,
        "tests",
        serde_json::json!({"point": 0, "x": 30.0, "y": 40.0}),
    ));
    let session = Session::from_document("tests", document).expect("rejeu");
    let mut solveur = Systeme2D::new().expect("solveur");
    let resolu = session.esquisse().resoudre(&mut solveur).expect("résolu");
    let (x, y) = mm(resolu.position(0));
    assert!(
        (x - 30.0).abs() < 1e-9 && (y - 40.0).abs() < 1e-9,
        "le rejeu tient la dernière graine posée au journal"
    );
}
