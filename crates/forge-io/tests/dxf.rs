//! Tests du parseur DXF : entités R12 (LINE, LWPOLYLINE, CIRCLE,
//! ARC), entités ignorées, budgets, erreurs typées, aller-retour
//! export/import.

use forge_geom::{Path2, Point2};
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
fn discretise_cercle_et_arc() {
    let cercles = dxf::import(CERCLE, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(cercles.len(), 1);
    assert!(cercles[0].is_closed());
    assert_eq!(cercles[0].len(), dxf::POINTS_CERCLE);

    let arcs = dxf::import(ARC, &DxfBudgets::default()).expect("dxf valide");
    assert_eq!(arcs.len(), 1);
    assert!(!arcs[0].is_closed());
    assert_eq!(arcs[0].len(), dxf::SEGMENTS_ARC + 1);
    assert_eq!(arcs[0].points()[0].x().millimeters(), 1.0);
    assert_eq!(arcs[0].points()[0].y().millimeters(), 0.0);
    let dernier = arcs[0].points().last().expect("au moins un sommet");
    assert_eq!(dernier.y().millimeters(), 1.0);
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
