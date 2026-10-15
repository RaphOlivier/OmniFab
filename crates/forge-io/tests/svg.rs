//! Tests du parseur SVG : rectangles, aplatissement des courbes,
//! budgets, erreurs typées, aller-retour export/import.

use forge_geom::{Path2, Point2};
use forge_io::svg::{self, SvgBudgets};
use forge_units::Length;

/// Construit un point de test aux coordonnées données (mm).
fn point(x: f64, y: f64) -> Point2 {
    let abscisse = Length::from_millimeters(x);
    let ordonnee = Length::from_millimeters(y);
    Point2::new(abscisse, ordonnee).expect("coordonnées finies")
}

#[test]
fn importe_un_rectangle() {
    let source = b"<svg><path d=\"M 10 20 L 30 20 L 30 40 L 10 40 Z\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    assert_eq!(traces[0].len(), 4);
    let points = traces[0].points();
    assert_eq!(points[0].x().millimeters(), 10.0);
    assert_eq!(points[0].y().millimeters(), 20.0);
    assert_eq!(points[2].x().millimeters(), 30.0);
    assert_eq!(points[3].y().millimeters(), 40.0);
}

#[test]
fn aplatit_les_courbes() {
    let source = b"<svg><path d=\"M 0 0 Q 10 10 20 0\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(!traces[0].is_closed());
    assert_eq!(traces[0].len(), 1 + svg::SEGMENTS_COURBE);
    let dernier = traces[0].points().last().expect("au moins un sommet");
    assert_eq!(dernier.x().millimeters(), 20.0);
    assert_eq!(dernier.y().millimeters(), 0.0);
}

#[test]
fn budget_de_traces_depasse() {
    let budgets = SvgBudgets { max_paths: 2, ..SvgBudgets::default() };
    let source = b"<svg><path d=\"M 0 0 L 1 0 M 2 0 L 3 0 M 4 0 L 5 0\"/></svg>";
    let erreur = svg::import(source, &budgets).expect_err("troisième tracé refusé");
    assert_eq!(erreur.code(), "OF-IO-0008");
}

#[test]
fn entree_trop_volumineuse() {
    let budgets = SvgBudgets { max_bytes: 8, ..SvgBudgets::default() };
    let erreur = svg::import(b"<svg></svg>", &budgets).expect_err("budget d'octets dépassé");
    assert_eq!(erreur.code(), "OF-IO-0001");
}

#[test]
fn svg_invalide() {
    let erreur = svg::import(b"<svg><path", &SvgBudgets::default()).expect_err("xml malformé");
    assert_eq!(erreur.code(), "OF-IO-0007");
}

#[test]
fn export_puis_import() {
    let ouvert = Path2::from_points(vec![point(0.0, 0.0), point(10.0, 0.0)], false);
    let ferme = Path2::from_points(
        vec![point(0.0, 0.0), point(10.0, 0.0), point(10.0, 10.0)],
        true,
    );
    let octets = svg::export(&[ouvert, ferme]).expect("export svg");
    let traces = svg::import(&octets, &SvgBudgets::default()).expect("relecture svg");
    assert_eq!(traces.len(), 2);
    assert!(!traces[0].is_closed());
    assert!(traces[1].is_closed());
    assert_eq!(traces[1].len(), 3);
    let dernier = traces[1].points().last().expect("au moins un sommet");
    assert_eq!(dernier.y().millimeters(), 10.0);
}
