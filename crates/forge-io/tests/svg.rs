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
fn reprend_apres_z_depuis_le_depart() {
    // Sémantique SVG : après `Z`, le point courant revient au
    // départ du sous-tracé — les commandes suivantes reprennent
    // depuis ce sommet, aucun segment n'est perdu.
    let source = b"<svg><path d=\"M 0 0 L 10 0 L 10 10 Z L 5 5 L 7 7\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 2);
    assert!(traces[0].is_closed());
    assert_eq!(traces[0].len(), 3);
    let fermes = traces[0].points();
    assert_eq!(fermes[0].x().millimeters(), 0.0);
    assert_eq!(fermes[0].y().millimeters(), 0.0);
    assert_eq!(fermes[2].x().millimeters(), 10.0);
    assert_eq!(fermes[2].y().millimeters(), -10.0);
    assert!(!traces[1].is_closed());
    assert_eq!(traces[1].len(), 3);
    let ouverts = traces[1].points();
    assert_eq!(ouverts[0].x().millimeters(), 0.0);
    assert_eq!(ouverts[0].y().millimeters(), 0.0);
    assert_eq!(ouverts[1].x().millimeters(), 5.0);
    assert_eq!(ouverts[1].y().millimeters(), -5.0);
    assert_eq!(ouverts[2].x().millimeters(), 7.0);
    assert_eq!(ouverts[2].y().millimeters(), -7.0);
}

#[test]
fn courbe_apres_z_part_du_depart() {
    // Une courbe après `Z` part du sommet de départ du
    // sous-tracé, pas du dernier point avant la fermeture.
    let source = b"<svg><path d=\"M 0 0 L 10 0 Z Q 0 5 0 10\"/></svg>";
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 2);
    let points = traces[1].points();
    assert_eq!(points.len(), 1 + svg::SEGMENTS_COURBE);
    assert_eq!(points[0].x().millimeters(), 0.0);
    assert_eq!(points[0].y().millimeters(), 0.0);
    let dernier = points.last().expect("au moins un sommet");
    assert_eq!(dernier.x().millimeters(), 0.0);
    assert_eq!(dernier.y().millimeters(), -10.0);
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
fn importe_les_tailles_physiques_en_mm() {
    // Fichiers Inkscape/FreeCAD : `width`/`height` en mm + viewBox
    // — un dessin de 10 mm importe en 10 mm. Régression du bug
    // « beaucoup plus grande que prévue » : usvg convertissait les
    // tailles physiques en pixels à 96 dpi (10 mm → 38 px).
    let source = br#"<svg width="10mm" height="10mm" viewBox="0 0 10 10">
<path d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
</svg>"#;
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    assert!(traces[0].is_closed());
    let points = traces[0].points();
    let trouve = |x: f64, y: f64| {
        points.iter().any(|p| {
            (p.x().millimeters() - x).abs() < 1e-3 && (p.y().millimeters() - y).abs() < 1e-3
        })
    };
    assert!(trouve(0.0, 0.0), "sommet origine absent");
    assert!(trouve(10.0, 0.0), "sommet (10, 0) absent");
    assert!(trouve(10.0, -10.0), "sommet (10, -10) absent");
    assert!(trouve(0.0, -10.0), "sommet (0, -10) absent");
}

#[test]
fn respecte_la_taille_physique_du_document() {
    // Le viewBox est cadré sur la taille physique déclarée : un
    // document de 20 mm dont le viewBox fait 10 unités importe
    // en 20 mm (l'échelle viewBox→taille vit dans usvg).
    let source = br#"<svg width="20mm" height="20mm" viewBox="0 0 10 10">
<path d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
</svg>"#;
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    let points = traces[0].points();
    let trouve = |x: f64, y: f64| {
        points.iter().any(|p| {
            (p.x().millimeters() - x).abs() < 1e-3 && (p.y().millimeters() - y).abs() < 1e-3
        })
    };
    assert!(trouve(0.0, 0.0), "sommet origine absent");
    assert!(trouve(20.0, 0.0), "sommet (20, 0) absent");
    assert!(trouve(20.0, -20.0), "sommet (20, -20) absent");
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

#[test]
fn export_declare_la_taille_physique() {
    // En-tête : boîte englobante du document — width/height en
    // mm, viewBox exact, Y négatifs couverts, aucune translation.
    let trace = Path2::from_points(vec![point(0.0, 0.0), point(20.0, 10.0)], false);
    let octets = svg::export(&[trace]).expect("export svg");
    let texte = String::from_utf8(octets).expect("svg en utf-8");
    assert!(
        texte.contains("width=\"20mm\""),
        "largeur physique : {texte}"
    );
    assert!(
        texte.contains("height=\"10mm\""),
        "hauteur physique : {texte}"
    );
    assert!(
        texte.contains("viewBox=\"0 -10 20 10\""),
        "viewBox : {texte}"
    );
    assert!(
        texte.contains("fill=\"none\""),
        "sans remplissage : {texte}"
    );
    assert!(
        texte.contains("stroke-width=\"0.1\""),
        "trait fin : {texte}"
    );
}

#[test]
fn export_d_un_trace_plat_garde_une_boite_visible() {
    // Ligne horizontale : sans marge, height="0mm" et un viewBox de
    // hauteur nulle — que les navigateurs refusent de rendre
    // (audit n°2, N9).
    let trace = Path2::from_points(vec![point(0.0, 5.0), point(20.0, 5.0)], false);
    let octets = svg::export(&[trace]).expect("export svg");
    let texte = String::from_utf8(octets).expect("svg en utf-8");
    assert!(
        texte.contains("height=\"0.1mm\""),
        "hauteur au plancher de marge : {texte}"
    );
    assert!(
        texte.contains("viewBox=\"0 -5 20 0.1\""),
        "viewBox jamais de hauteur nulle : {texte}"
    );
}

#[test]
fn export_relit_aux_memes_dimensions() {
    // Un lecteur extérieur (ici : notre import) interprète le
    // document à la bonne taille physique — mêmes coordonnées.
    let trace = Path2::from_points(vec![point(0.0, 0.0), point(20.0, 10.0)], false);
    let octets = svg::export(&[trace]).expect("export svg");
    let traces = svg::import(&octets, &SvgBudgets::default()).expect("relecture svg");
    assert_eq!(traces.len(), 1);
    let points = traces[0].points();
    assert_eq!(points[0].x().millimeters(), 0.0);
    assert_eq!(points[0].y().millimeters(), 0.0);
    assert_eq!(points[1].x().millimeters(), 20.0);
    assert_eq!(points[1].y().millimeters(), 10.0);
}
#[test]
fn viewbox_d_un_symbol_ne_s_applique_pas() {
    // Seule la `viewBox` de l'élément racine compte (ADR-0016 B3) :
    // celle d'un `<symbol>` ne doit pas décaler tout l'import
    // (audit n°3, A3-04) — le tracé racine garde ses coordonnées.
    let source = br#"<svg width="100mm" height="100mm">
<symbol viewBox="10 10 20 20"><path d="M 0 0 L 10 0"/></symbol>
<path d="M 5 5 L 15 5"/>
</svg>"#;
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    let mut racine_presente = false;
    for trace in &traces {
        for point in trace.points() {
            let x = point.x().millimeters();
            let y = point.y().millimeters();
            if (x - 5.0).abs() < 1e-3 && (y + 5.0).abs() < 1e-3 {
                racine_presente = true;
            }
            assert!(
                (x - 55.0).abs() > 1e-3 || (y + 55.0).abs() > 1e-3,
                "la viewBox du symbol ne doit pas s'appliquer : sommet ({x}, {y})"
            );
        }
    }
    assert!(
        racine_presente,
        "le tracé racine doit rester à ses coordonnées"
    );
}

#[test]
fn viewbox_avec_virgules_est_compensee() {
    // Une `viewBox` est une liste de nombres au format SVG :
    // espaces **et** virgules sont des séparateurs valides —
    // `viewBox="-50,-50,100,100"` doit être compensée exactement
    // comme `-50 -50 100 100` (audit n°3, A3-04).
    let source = br#"<svg viewBox="-50,-50,100,100"><path d="M 0 0 L 10 0"/></svg>"#;
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    let points = traces[0].points();
    assert_eq!(points[0].x().millimeters(), -50.0);
    assert_eq!(points[0].y().millimeters(), 50.0);
    assert_eq!(points[1].x().millimeters(), -40.0);
    assert_eq!(points[1].y().millimeters(), 50.0);
}

#[test]
fn viewbox_dans_un_commentaire_est_ignoree() {
    // Un `viewBox` cité dans un commentaire n'est pas celui du
    // document : la compensation ne se lit jamais dans le texte
    // brut (audit n°3, A3-04).
    let source = br#"<!-- viewBox='1 1 2 2' --><svg width="10mm" height="10mm"><path d="M 1 2 L 3 4"/></svg>"#;
    let traces = svg::import(source, &SvgBudgets::default()).expect("svg valide");
    assert_eq!(traces.len(), 1);
    let points = traces[0].points();
    assert_eq!(points[0].x().millimeters(), 1.0);
    assert_eq!(points[0].y().millimeters(), -2.0);
    assert_eq!(points[1].x().millimeters(), 3.0);
    assert_eq!(points[1].y().millimeters(), -4.0);
}
