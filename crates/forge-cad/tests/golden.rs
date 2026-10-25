//! Golden tests de forge-cad (ADR-0023 D3) : les volumes des primitives
//! et des booléens figent le comportement du binding opencascade-rs à
//! chaque montée de version (OCCT comme du binding). Les faces planes
//! sont exactes au f64 près ; les faces courbes portent l'erreur de la
//! tessellation, d'autant plus fine que la tolérance est basse.

use forge_cad::{brep, step, CadBudgets, CadError, Solide};
use forge_units::Length;

/// Millimètres — jamais de f64 nu (verrou n°5).
fn mm(valeur: f64) -> Length {
    Length::from_millimeters(valeur)
}

/// Tolérance fine : faces courbes (erreur relative ~1e-4).
fn tolerance_fine() -> Length {
    mm(0.001)
}

/// Tolérance standard : faces planes (volume exact au f64 près).
fn tolerance_standard() -> Length {
    mm(0.01)
}

/// Assertion relative partagée.
fn proche(obtenu: f64, attendu: f64, tolerance_relative: f64) {
    assert!(
        (obtenu - attendu).abs() <= attendu * tolerance_relative,
        "volume obtenu {obtenu}, attendu {attendu}"
    );
}

#[test]
fn boite_10_10_5_volume_exact() {
    let boite = Solide::boite(mm(10.0), mm(10.0), mm(5.0)).unwrap();
    let volume = boite.volume_mm3(tolerance_standard()).unwrap();
    proche(volume, 500.0, 1e-9);
}

#[test]
fn cylindre_diametre_20_hauteur_10() {
    let cylindre = Solide::cylindre_centre(mm(10.0), mm(0.0), mm(0.0), mm(10.0)).unwrap();
    let attendu = std::f64::consts::PI * 10.0 * 10.0 * 10.0;
    let volume = cylindre.volume_mm3(tolerance_fine()).unwrap();
    proche(volume, attendu, 1e-3);
}

#[test]
fn prisme_rectangle_100_50_hauteur_4() {
    let contour = [
        (mm(0.0), mm(0.0)),
        (mm(100.0), mm(0.0)),
        (mm(100.0), mm(50.0)),
        (mm(0.0), mm(50.0)),
    ];
    let prisme = Solide::prisme(&contour, mm(4.0)).unwrap();
    let volume = prisme.volume_mm3(tolerance_standard()).unwrap();
    proche(volume, 20_000.0, 1e-9);
}

#[test]
fn boite_moins_cylindre_perco() {
    let cube = Solide::boite(mm(20.0), mm(20.0), mm(10.0)).unwrap();
    let perco = Solide::cylindre_centre(mm(5.0), mm(10.0), mm(10.0), mm(10.0)).unwrap();
    let troue = cube.soustraire(&perco).unwrap();
    let attendu = 4000.0 - std::f64::consts::PI * 25.0 * 10.0;
    let volume = troue.volume_mm3(tolerance_fine()).unwrap();
    proche(volume, attendu, 1e-3);
}

#[test]
fn step_aller_retour_volume_conserve() {
    let boite = Solide::boite(mm(10.0), mm(10.0), mm(5.0)).unwrap();
    let octets = step::exporter(&[boite]).unwrap();
    let budgets = CadBudgets::default();
    let relus = step::importer(&octets, &budgets).unwrap();
    assert_eq!(relus.len(), 1);
    let volume = relus[0].volume_mm3(tolerance_standard()).unwrap();
    proche(volume, 500.0, 1e-9);
}

#[test]
fn brep_aller_retour_volume_conserve() {
    let boite = Solide::boite(mm(10.0), mm(10.0), mm(5.0)).unwrap();
    let octets = brep::exporter(&boite).unwrap();
    let budgets = CadBudgets::default();
    let relus = brep::importer(&octets, &budgets).unwrap();
    assert_eq!(relus.len(), 1);
    let volume = relus[0].volume_mm3(tolerance_standard()).unwrap();
    proche(volume, 500.0, 1e-9);
}

#[test]
fn budget_octets_depasse_refuse_avant_occt() {
    let budgets = CadBudgets {
        octets_max: 4,
        formes_max: 100,
    };
    let erreur = step::importer(b"STEP!", &budgets).unwrap_err();
    assert!(matches!(erreur, CadError::Budget { .. }));
    assert_eq!(erreur.code(), "OF-CAD-0004");
}

#[test]
fn step_invalide_refuse() {
    let erreur = step::importer(b"ceci n'est pas du STEP", &CadBudgets::default()).unwrap_err();
    assert!(matches!(erreur, CadError::Illisible { .. }));
    assert_eq!(erreur.code(), "OF-CAD-0003");
}

#[test]
fn codes_stables_par_variante() {
    let occt = CadError::Occt {
        operation: "test".into(),
    };
    assert_eq!(occt.code(), "OF-CAD-0001");
    let invalide = CadError::FormeInvalide {
        operation: "test".into(),
    };
    assert_eq!(invalide.code(), "OF-CAD-0002");
    let illisible = CadError::Illisible {
        formate: "STEP".into(),
        raison: "test".into(),
    };
    assert_eq!(illisible.code(), "OF-CAD-0003");
    let budget = CadError::Budget {
        limite: "test".into(),
    };
    assert_eq!(budget.code(), "OF-CAD-0004");
}

#[test]
fn codes_presents_au_registre() {
    let chemin = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/error-codes.md"
    );
    let registre = std::fs::read_to_string(chemin).unwrap();
    for code in ["OF-CAD-0001", "OF-CAD-0002", "OF-CAD-0003", "OF-CAD-0004"] {
        assert!(
            registre.contains(code),
            "code {code} absent du registre docs/error-codes.md"
        );
    }
}
