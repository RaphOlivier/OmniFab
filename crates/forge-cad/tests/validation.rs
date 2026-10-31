//! Tests de validation des entrées de forge-cad — audit n°3 A3-08,
//! enrichis du spike phase A et du signalement A4-14 : le noyau OCCT
//! n'apporte AUCUNE garde (points confondus → abort C++, deux points →
//! segfault, hauteur nulle ou négative → prisme Ok). Toute entrée est
//! refusée côté OmniFab AVANT le pont cxx — jamais d'entrée brute
//! confiée au noyau (convention « entrées bornées », v12).

use forge_cad::{CadBudgets, Solide, brep};
use forge_units::Length;
use opencascade::primitives::Shape;

/// Millimètres — jamais de f64 nu (verrou n°5).
fn mm(valeur: f64) -> Length {
    Length::from_millimeters(valeur)
}

/// Carré de côté `cote`, antihoraire, origine au coin.
fn carre(cote: f64) -> Vec<(Length, Length)> {
    vec![
        (mm(0.0), mm(0.0)),
        (mm(cote), mm(0.0)),
        (mm(cote), mm(cote)),
        (mm(0.0), mm(cote)),
    ]
}

#[test]
fn boite_invalide_rejetee() {
    // A3-08.a : toute dimension est finie et strictement positive —
    // OCCT n'apporte aucune garde (spike phase A, cas 4).
    let cas = [
        (0.0, 10.0, 5.0),
        (10.0, 0.0, 5.0),
        (10.0, 10.0, 0.0),
        (-1.0, 10.0, 5.0),
        (f64::NAN, 10.0, 5.0),
        (f64::INFINITY, 10.0, 5.0),
    ];
    for (largeur, profondeur, hauteur) in cas {
        let erreur = Solide::boite(mm(largeur), mm(profondeur), mm(hauteur))
            .unwrap_err()
            .code();
        assert_eq!(
            erreur, "OF-CAD-0002",
            "cas {largeur}×{profondeur}×{hauteur}"
        );
    }
}

#[test]
fn cylindre_invalide_rejete() {
    let cas = [
        (0.0, 0.0, 0.0, 10.0),
        (-5.0, 0.0, 0.0, 10.0),
        (f64::NAN, 0.0, 0.0, 10.0),
        (5.0, 0.0, 0.0, 0.0),
        (5.0, 0.0, 0.0, -10.0),
        (5.0, f64::NAN, 0.0, 10.0),
        (5.0, 0.0, f64::INFINITY, 10.0),
    ];
    for (rayon, x, y, hauteur) in cas {
        let erreur = Solide::cylindre_centre(mm(rayon), mm(x), mm(y), mm(hauteur))
            .unwrap_err()
            .code();
        assert_eq!(erreur, "OF-CAD-0002", "cas r={rayon}, h={hauteur}");
    }
}

#[test]
fn prisme_moins_de_trois_points_rejete() {
    // spike phase A, cas 2 : deux points → segfault C++ — refus typé
    // avant le pont.
    let deux_points = [(mm(0.0), mm(0.0)), (mm(10.0), mm(0.0))];
    let erreur = Solide::prisme(&deux_points, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");

    let vide: [(Length, Length); 0] = [];
    let erreur = Solide::prisme(&vide, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn prisme_points_confondus_rejete() {
    // spike phase A, cas 1 : points confondus → abort C++ StdFail_NotDone.
    let confondus = [(mm(5.0), mm(5.0)), (mm(5.0), mm(5.0)), (mm(5.0), mm(5.0))];
    let erreur = Solide::prisme(&confondus, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");

    // doublon consécutif : arête dégénérée.
    let doublon = [
        (mm(0.0), mm(0.0)),
        (mm(0.0), mm(0.0)),
        (mm(10.0), mm(0.0)),
        (mm(10.0), mm(10.0)),
    ];
    let erreur = Solide::prisme(&doublon, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");

    // le contour est fermé implicitement : répéter le premier point en
    // dernière position crée une arête nulle.
    let fermee = [
        (mm(0.0), mm(0.0)),
        (mm(10.0), mm(0.0)),
        (mm(10.0), mm(10.0)),
        (mm(0.0), mm(0.0)),
    ];
    let erreur = Solide::prisme(&fermee, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn prisme_points_alignes_rejete() {
    // trois points alignés : aire nulle (formule du lacet).
    let alignes = [(mm(0.0), mm(0.0)), (mm(10.0), mm(0.0)), (mm(20.0), mm(0.0))];
    let erreur = Solide::prisme(&alignes, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn prisme_coordonnee_non_finie_rejetee() {
    let non_fini = [
        (mm(0.0), mm(0.0)),
        (mm(f64::NAN), mm(0.0)),
        (mm(10.0), mm(10.0)),
    ];
    let erreur = Solide::prisme(&non_fini, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn prisme_hauteur_invalide_rejetee() {
    // spike phase A, cas 3-4 : hauteur nulle → prisme Ok (!), hauteur
    // négative → prisme Ok extrudé en |h| — aucune garde côté OCCT.
    let contour = carre(10.0);
    for hauteur in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        let erreur = Solide::prisme(&contour, mm(hauteur)).unwrap_err().code();
        assert_eq!(erreur, "OF-CAD-0002", "cas hauteur = {hauteur}");
    }
}

#[test]
fn prisme_contour_trop_long_rejete() {
    // convention « entrées bornées » (v12) : le contour est plafonné
    // avant le contrôle de distinctness O(n²).
    let contour: Vec<(Length, Length)> = (0..5001).map(|i| (mm(i as f64), mm(0.0))).collect();
    let erreur = Solide::prisme(&contour, mm(5.0)).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn soustraction_identique_explicite() {
    // A3-08.b : self − self rend aujourd'hui un Ok silencieux au
    // volume nul — le refus doit être explicite (jamais un Ok vide).
    let cube = Solide::boite(mm(10.0), mm(10.0), mm(10.0)).unwrap();
    let identique = Solide::boite(mm(10.0), mm(10.0), mm(10.0)).unwrap();
    let erreur = cube.soustraire(&identique).unwrap_err().code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn import_non_solide_rejete() {
    // A3-08.c : un composé vide est une forme non solide — l'import
    // doit la refuser, pas l'envelopper en Ok(Solide) sans sens.
    let vide = Shape::empty();
    let nom = format!("omnifab-test-vide-{}.brep", std::process::id());
    let chemin = std::env::temp_dir().join(nom);
    vide.write_brep_text(&chemin).unwrap();
    let octets = std::fs::read(&chemin).unwrap();
    let _ = std::fs::remove_file(&chemin);
    let erreur = brep::importer(&octets, &CadBudgets::default())
        .unwrap_err()
        .code();
    assert_eq!(erreur, "OF-CAD-0002");
}

#[test]
fn tesseller_tolerance_invalide_rejetee() {
    // A3-08.d : la tolérance de tessellation est finie et strictement
    // positive — jamais confiée telle quelle à OCCT.
    let boite = Solide::boite(mm(10.0), mm(10.0), mm(10.0)).unwrap();
    for tolerance in [0.0, -0.01, f64::NAN, f64::INFINITY] {
        let erreur = boite.tesseller(mm(tolerance)).unwrap_err().code();
        assert_eq!(erreur, "OF-CAD-0002", "cas tolérance = {tolerance}");
    }
}

#[test]
fn volume_toujours_positif() {
    // A3-08.d : le volume signé de la tessellation dépend de
    // l'orientation des triangles — l'API rend la valeur absolue.
    let prisme = Solide::prisme(&carre(10.0), mm(5.0)).unwrap();
    let volume = prisme.volume_mm3(mm(0.01)).unwrap();
    assert!(volume > 0.0, "volume antihoraire = {volume}");

    let horaire = [
        (mm(0.0), mm(10.0)),
        (mm(10.0), mm(10.0)),
        (mm(10.0), mm(0.0)),
        (mm(0.0), mm(0.0)),
    ];
    let prisme_inverse = Solide::prisme(&horaire, mm(5.0)).unwrap();
    let volume = prisme_inverse.volume_mm3(mm(0.01)).unwrap();
    assert!(volume > 0.0, "volume horaire = {volume}");
}
