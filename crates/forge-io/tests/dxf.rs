//! Tests du parseur DXF : entités R12 (LINE, LWPOLYLINE, CIRCLE,
//! ARC), entités ignorées, budgets, erreurs typées, aller-retour
//! export/import.

// Convention expect v2 (2026-11-01, lot E) : échafaudage de test —
// `allow-expect-in-tests` (clippy.toml) ne couvre que les fonctions
// `#[test]` et les modules `#[cfg(test)]`, pas les helpers de ce
// fichier d'intégration : expect admis dans tout le fichier.
#![allow(clippy::expect_used)]

use forge_geom::{Path2, Point2, Segment2};
use forge_io::dxf::{self, DxfBudgets};
use forge_units::Length;

/// SECTION ENTITIES minimale (R12, sans HEADER) contenant une
/// `LINE` de (10, 20) à (30, 40).
const LIGNE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n10.0\n20\n20.0\n11\n30.0\n21\n40.0\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant un `CIRCLE` de centre (1, 2), rayon 1.
const CERCLE: &[u8] =
    b"0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\n0\n10\n1.0\n20\n2.0\n40\n1.0\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant un `ARC` de centre (0, 0), rayon 1,
/// de 0 à 90 degrés.
const ARC: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nARC\n8\n0\n10\n0.0\n20\n0.0\n40\n1.0\n50\n0.0\n51\n90.0\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant une `LWPOLYLINE` fermée (90 : 2
/// sommets, 70 : 1) de (0, 0) à (10, 0).
const LWPOLYLIGNE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\n0\n90\n2\n70\n1\n10\n0.0\n20\n0.0\n10\n10.0\n20\n0.0\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant une `POLYLINE` fermée (R12 : 66
/// suivi des sommets, 70 bit 1) de (0, 0) à (10, 0).
const POLYLIGNE_FERMEE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nPOLYLINE\n8\n0\n66\n1\n70\n1\n0\nVERTEX\n8\n0\n10\n0.0\n20\n0.0\n0\nVERTEX\n8\n0\n10\n10.0\n20\n0.0\n0\nSEQEND\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant une `POLYLINE` ouverte (70 : 0) de
/// (0, 0) à (10, 0).
const POLYLIGNE_OUVERTE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nPOLYLINE\n8\n0\n66\n1\n70\n0\n0\nVERTEX\n8\n0\n10\n0.0\n20\n0.0\n0\nVERTEX\n8\n0\n10\n10.0\n20\n0.0\n0\nSEQEND\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant un `TEXT` (ignoré) puis une `LINE`.
const TEXT_ET_LIGNE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\n0\n10\n0.0\n20\n0.0\n40\n5.0\n1\nbonjour\n0\nLINE\n8\n0\n10\n10.0\n20\n20.0\n11\n30.0\n21\n40.0\n0\nENDSEC\n0\nEOF\n";

/// Construit un point de test aux coordonnées données (mm).
fn point(x: f64, y: f64) -> Point2 {
    let abscisse = Length::from_millimeters(x);
    let ordonnee = Length::from_millimeters(y);
    Point2::new(abscisse, ordonnee).expect("coordonnées finies")
}

#[test]
fn importe_une_ligne() {
    let traces = dxf::import(LIGNE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(traces.len(), 1);
    assert!(!traces[0].is_closed());
    assert_eq!(traces[0].len(), 2);
    assert_eq!(traces[0].points()[0].x().millimeters(), 10.0);
}

#[test]
fn cercle_et_arc_devient_des_arcs_natifs() {
    // Cercle et arc ne sont plus discrétisés à l'import (ADR-0024,
    // phase B) : chacun devient un arc natif — un balayage nul
    // dénote le cercle complet.
    let cercles = dxf::import(CERCLE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(cercles.len(), 1);
    assert!(cercles[0].is_closed(), "le cercle est fermé");
    assert_eq!(cercles[0].segments().len(), 1, "un cercle = un arc");
    match cercles[0].segments()[0] {
        Segment2::Arc { centre, rayon, .. } => {
            assert_eq!(centre.x().millimeters(), 1.0);
            assert_eq!(centre.y().millimeters(), 2.0);
            assert_eq!(rayon.millimeters(), 1.0);
        }
        _ => panic!("le cercle doit devenir un arc natif"),
    }
    // La discrétisation bornée reste sur le cercle.
    let points = cercles[0].points();
    assert!(points.len() > 8, "cercle discrétisé : {}", points.len());
    for point in &points {
        let x = point.x().millimeters() - 1.0;
        let y = point.y().millimeters() - 2.0;
        assert!(((x * x + y * y).sqrt() - 1.0).abs() < 1e-9);
    }

    let arcs = dxf::import(ARC, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(arcs.len(), 1);
    assert!(!arcs[0].is_closed());
    assert_eq!(arcs[0].segments().len(), 1, "un arc = un arc natif");
    let points = arcs[0].points();
    assert_eq!(points[0].x().millimeters(), 1.0);
    assert_eq!(points[0].y().millimeters(), 0.0);
    let dernier = points.last().expect("au moins un sommet");
    assert_eq!(dernier.y().millimeters(), 1.0);
    assert!(dernier.x().millimeters().abs() < 1e-9);
}

#[test]
fn importe_une_lwpolyline_fermee() {
    let traces = dxf::import(LWPOLYLIGNE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    assert_eq!(traces[0].len(), 2);
    assert_eq!(traces[0].points()[1].x().millimeters(), 10.0);
}

#[test]
fn importe_une_polyline_ferme_r12() {
    let traces = dxf::import(POLYLIGNE_FERMEE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    assert_eq!(traces[0].len(), 2);
}

#[test]
fn polyline_ouverte_reste_ouverte() {
    let traces = dxf::import(POLYLIGNE_OUVERTE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(traces.len(), 1);
    assert!(!traces[0].is_closed());
    assert_eq!(traces[0].len(), 2);
}

#[test]
fn parite_polyline_lwpolyline() {
    let polylignes = dxf::import(POLYLIGNE_FERMEE, &DxfBudgets::default()).expect("dxf valide");
    let lwpolylignes = dxf::import(LWPOLYLIGNE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(polylignes.len(), lwpolylignes.len());
    assert_eq!(polylignes[0].is_closed(), lwpolylignes[0].is_closed());
    assert_eq!(polylignes[0].len(), lwpolylignes[0].len());
    assert_eq!(
        polylignes[0].points()[1].x().millimeters(),
        lwpolylignes[0].points()[1].x().millimeters()
    );
}

#[test]
fn ignore_les_entites_inconnues() {
    let traces = dxf::import(TEXT_ET_LIGNE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].len(), 2);
}

#[test]
fn dxf_invalide() {
    let erreur = dxf::import(b"abc\ndef\n", &DxfBudgets::default())
        .expect_err("paires code/valeur invalides refusées");
    assert_eq!(erreur.code(), "OF-IO-0009");
}

#[test]
fn budget_de_sommets_depasse() {
    let budgets = DxfBudgets {
        max_vertices: 1,
        ..DxfBudgets::default()
    };
    let erreur = dxf::import(LIGNE, &budgets).expect_err("deuxième sommet refusé");
    assert_eq!(erreur.code(), "OF-IO-0008");
}

#[test]
fn export_puis_import() {
    let trace = Path2::from_points(vec![point(10.0, 20.0), point(30.0, 40.0)], false);
    let octets = dxf::export(&[trace]).expect("export dxf");
    let traces = dxf::import(&octets, &DxfBudgets::default()).expect("relecture dxf");
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].len(), 2);
    assert_eq!(traces[0].points()[0].x().millimeters(), 10.0);
}
/// SECTION ENTITIES contenant une `LINE` en espace papier
/// (code 67) — la mise en page, pas le dessin.
const LIGNE_ESPACE_PAPIER: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n67\n1\n10\n10.0\n20\n20.0\n11\n30.0\n21\n40.0\n0\nENDSEC\n0\nEOF\n";

/// SECTION ENTITIES contenant une `LINE` invisible (code 60).
const LIGNE_INVISIBLE: &[u8] = b"0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n60\n1\n10\n10.0\n20\n20.0\n11\n30.0\n21\n40.0\n0\nENDSEC\n0\nEOF\n";

/// Table `LAYER` déclarant le calque `ETEINT` (couleur négative
/// au code 62 : calque éteint) puis SECTION ENTITIES avec une
/// `LINE` sur ce calque.
const LIGNE_CALQUE_ETEINT: &[u8] = b"0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nETEINT\n70\n0\n62\n-7\n6\nCONTINUOUS\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nETEINT\n10\n10.0\n20\n20.0\n11\n30.0\n21\n40.0\n0\nENDSEC\n0\nEOF\n";

#[test]
fn espace_papier_ignore() {
    // L'espace papier (code 67) est la mise en page, pas le
    // dessin : ses entités ne deviennent pas des tracés
    // (audit n°3, A3-05).
    let traces = dxf::import(LIGNE_ESPACE_PAPIER, &DxfBudgets::default()).expect("dxf valide");
    assert!(traces.is_empty(), "l'espace papier est ignoré");
}

#[test]
fn entite_invisible_ignoree() {
    // Une entité invisible (code 60) n'est pas dessinée : elle ne
    // devient pas un tracé (audit n°3, A3-05).
    let traces = dxf::import(LIGNE_INVISIBLE, &DxfBudgets::default()).expect("dxf valide");
    assert!(traces.is_empty(), "l'entité invisible est ignorée");
}

#[test]
fn calque_eteint_ignore() {
    // Les entités d'un calque éteint (couleur négative dans la
    // table LAYER) ne sont pas dessinées : elles ne deviennent
    // pas des tracés (audit n°3, A3-05).
    let traces = dxf::import(LIGNE_CALQUE_ETEINT, &DxfBudgets::default()).expect("dxf valide");
    assert!(traces.is_empty(), "le calque éteint est ignoré");
}

/// Table `LAYER` déclarant le calque éteint `ETEINT` (couleur
/// négative au code 62) puis SECTION ENTITIES : une `LINE` en
/// espace papier (67), une invisible (60), une sur le calque
/// éteint, une normale.
const MIXTE: &[u8] = b"0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nETEINT\n70\n0\n62\n-7\n6\nCONTINUOUS\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n67\n1\n10\n0.0\n20\n0.0\n11\n1.0\n21\n1.0\n0\nLINE\n8\n0\n60\n1\n10\n2.0\n20\n2.0\n11\n3.0\n21\n3.0\n0\nLINE\n8\nETEINT\n10\n4.0\n20\n4.0\n11\n5.0\n21\n5.0\n0\nLINE\n8\n0\n10\n6.0\n20\n6.0\n11\n7.0\n21\n7.0\n0\nENDSEC\n0\nEOF\n";

#[test]
fn rapport_compte_les_entites_ecartees() {
    // Chaque sorte d'entité écartée est comptée dans le rapport
    // (audit n°3, A3-05) ; seule l'entité dessinée devient un
    // tracé.
    let resultat = dxf::import_avec_rapport(MIXTE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(resultat.traces.len(), 1);
    assert_eq!(resultat.rapport.espace_papier, 1);
    assert_eq!(resultat.rapport.invisibles, 1);
    assert_eq!(resultat.rapport.calques_eteints, 1);
    assert_eq!(resultat.rapport.total_ignorees(), 3);
}
