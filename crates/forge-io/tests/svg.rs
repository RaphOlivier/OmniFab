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
    assert_eq!(points[0].y().millimeters(), -20.0);
    assert_eq!(points[2].x().millimeters(), 30.0);
    assert_eq!(points[3].y().millimeters(), -40.0);
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
fn importe_une_courbe_cubique() {
    // Bézier cubique `C` (outil plume d'Inkscape) : aplatie en
    // SEGMENTS_COURBE sommets, sommet milieu exact.
    let source = b"<svg><path d=\"M 0 0 C 0 10 20 10 20 0\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(!traces[0].is_closed());
    assert_eq!(traces[0].len(), 1 + svg::SEGMENTS_COURBE);
    let points = traces[0].points();
    assert_eq!(points[0].x().millimeters(), 0.0);
    let milieu = points.get(svg::SEGMENTS_COURBE / 2).expect("sommet milieu");
    assert_eq!(milieu.x().millimeters(), 10.0);
    assert_eq!(milieu.y().millimeters(), -7.5);
    let dernier = points.last().expect("au moins un sommet");
    assert_eq!(dernier.x().millimeters(), 20.0);
    assert_eq!(dernier.y().millimeters(), 0.0);
}

#[test]
fn importe_un_cercle_primitif() {
    // Cercle gardé en primitive `<circle>` (comme Inkscape) :
    // usvg le normalise en Béziers cubiques, aplaties par
    // l'import. Les quatre extrémités du cercle doivent être
    // présentes (axe Y inversé : le monde monte).
    let source = b"<svg><circle cx=\"10\" cy=\"10\" r=\"5\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    assert!(traces[0].len() > 4, "cercle aplati : {}", traces[0].len());
    let points = traces[0].points();
    let trouve = |x: f64, y: f64| {
        points.iter().any(|p| {
            (p.x().millimeters() - x).abs() < 1e-3 && (p.y().millimeters() - y).abs() < 1e-3
        })
    };
    assert!(trouve(15.0, -10.0), "extrémité droite du cercle absente");
    assert!(trouve(10.0, -15.0), "extrémité haute du cercle absente");
    assert!(trouve(5.0, -10.0), "extrémité gauche du cercle absente");
    assert!(trouve(10.0, -5.0), "extrémité basse du cercle absente");
}

#[test]
fn importe_un_arc_svg() {
    // Arc en commande `A` (les primitives arc d'Inkscape sont
    // stockées en commandes a/A) : converti en Béziers puis aplatit.
    let source = b"<svg><path d=\"M 0 0 A 10 10 0 0 1 20 0\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(!traces[0].is_closed());
    assert!(traces[0].len() > 2, "arc aplati : {}", traces[0].len());
    let points = traces[0].points();
    let traverse = points.iter().any(|p| {
        (p.x().millimeters() - 10.0).abs() < 5e-3 && (p.y().millimeters().abs() - 10.0).abs() < 5e-3
    });
    assert!(traverse, "l'arc doit passer par un sommet à 10 mm");
}

#[test]
fn importe_les_groupes_imbriques() {
    // Inkscape encapsule les formes dans des groupes (calques) :
    // la transformation absolue doit s'appliquer aux primitives.
    let source = b"<svg><g transform=\"translate(5,0)\"><g><circle cx=\"10\" cy=\"10\" r=\"5\"/></g></g></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    let points = traces[0].points();
    let trouve = |x: f64, y: f64| {
        points.iter().any(|p| {
            (p.x().millimeters() - x).abs() < 1e-3 && (p.y().millimeters() - y).abs() < 1e-3
        })
    };
    assert!(trouve(20.0, -10.0), "extrémité droite translatée absente");
}

#[test]
fn budget_de_traces_depasse() {
    let budgets = SvgBudgets {
        max_paths: 2,
        ..SvgBudgets::default()
    };
    let source = b"<svg><path d=\"M 0 0 L 1 0 M 2 0 L 3 0 M 4 0 L 5 0\"/></svg>";
    let erreur = svg::import(source, &budgets).expect_err("troisième tracé refusé");
    assert_eq!(erreur.code(), "OF-IO-0008");
}

#[test]
fn entree_trop_volumineuse() {
    let budgets = SvgBudgets {
        max_bytes: 8,
        ..SvgBudgets::default()
    };
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

#[test]
fn export_inverse_l_axe_y() {
    // Le monde monte, le SVG descend : l'export négative le Y.
    let trace = Path2::from_points(vec![point(0.0, 5.0), point(10.0, 5.0)], false);
    let octets = svg::export(&[trace]).expect("export svg");
    let texte = String::from_utf8(octets).expect("svg en utf-8");
    assert!(texte.contains("M 0 -5 L 10 -5"), "y inversée : {texte}");
}
