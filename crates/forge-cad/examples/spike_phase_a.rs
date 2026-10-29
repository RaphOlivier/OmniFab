//! Spike phase A (ADR-0023) — wire → prisme → STEP → relecture.
//!
//! Journal du spike à remplir à chaque exécution (consignes checkpoint-pc
//! §« Consignes du spike phase A ») :
//! - paniques OCCT (formes dégénérées, dimensions ≤ 0 — cf. A3-08.a) ;
//! - comportement de `Shape::empty()`/`as_solid()` sur booléens vides
//!   (sonde temporaire dans `src/occt.rs`, `cargo test -p forge-cad`) ;
//! - temps de build froid ;
//! - va-et-vient `volume_mm3` avant/après relecture.
//!
//! Exécuter : `cargo run -p forge-cad --example spike_phase_a`
//! (après un `cargo clean` pour mesurer le temps de build froid).
//! Écrit les témoins `spike_phase_a.step` / `spike_phase_a.brep` dans le
//! répertoire courant (oracle externe : suite step-conformance de camcraft).
//!
//! Un abort C++ (cf. StdFail_NotDone sur les cas dégénérés) abat le
//! process : chaque cas dégénéré se lance individuellement via
//! `cargo run -p forge-cad --example spike_phase_a -- <n° 1-4>`.

use forge_cad::{brep, step, CadBudgets, Solide};
use forge_units::Length;

fn main() {
    // ---- 1. Wire rectangulaire (4 coins) → prisme par extrusion ----
    // Hauteur paramétrée : c'est LE paramètre du spike.
    let hauteur = Length::from_millimeters(12.0);
    let contour = [
        (Length::from_millimeters(0.0), Length::from_millimeters(0.0)),
        (Length::from_millimeters(40.0), Length::from_millimeters(0.0)),
        (Length::from_millimeters(40.0), Length::from_millimeters(25.0)),
        (Length::from_millimeters(0.0), Length::from_millimeters(25.0)),
    ];
    let debut = std::time::Instant::now();
    let prisme = match Solide::prisme(&contour, hauteur) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[SPIKE] construction du prisme : ERREUR {e:?}");
            return;
        }
    };
    println!("[SPIKE] prisme construit en {:?}", debut.elapsed());

    // Volume attendu : 40 × 25 × 12 = 12 000 mm³ (faces planes → exact).
    let attendu = 40.0 * 25.0 * hauteur.millimeters();
    let tolerance = Length::from_millimeters(0.01);
    let volume_avant = prisme.volume_mm3(tolerance).expect("volume avant");
    println!(
        "[SPIKE] volume avant export : {volume_avant} mm³ (attendu {attendu}, écart {:.6})",
        (volume_avant - attendu).abs()
    );

    // ---- 2. Export STEP + BREP ----
    // `Solide` n'implémente pas `Clone` : export par référence unique.
    let octets_step = step::exporter(std::slice::from_ref(&prisme)).expect("export STEP");
    let octets_brep = brep::exporter(&prisme).expect("export BREP");
    println!(
        "[SPIKE] export STEP {} octets, BREP {} octets",
        octets_step.len(),
        octets_brep.len()
    );

    // ---- 2b. Fichiers témoins (oracle externe : step-conformance camcraft) ----
    std::fs::write("spike_phase_a.step", &octets_step).expect("écriture spike_phase_a.step");
    std::fs::write("spike_phase_a.brep", &octets_brep).expect("écriture spike_phase_a.brep");
    println!("[SPIKE] témoins écrits : spike_phase_a.step / spike_phase_a.brep");

    // ---- 3. Relecture et va-et-vient volume_mm3 ----
    let relus = step::importer(&octets_step, &CadBudgets::default()).expect("import STEP");
    let relu = relus.first().expect("au moins un solide relu");
    let volume_apres = relu.volume_mm3(tolerance).expect("volume après");
    println!(
        "[SPIKE] volume après relecture : {volume_apres} mm³ (écart {:.6} — cf. A3-08.d : abs() du volume signé)",
        (volume_apres - volume_avant).abs()
    );

    let relus_brep = brep::importer(&octets_brep, &CadBudgets::default()).expect("import BREP");
    let relu_brep = relus_brep.first().expect("au moins un solide BREP relu");
    let volume_brep = relu_brep.volume_mm3(tolerance).expect("volume BREP");
    println!(
        "[SPIKE] volume via BREP : {volume_brep} mm³ (écart {:.6})",
        (volume_brep - volume_avant).abs()
    );

    // ---- 4. Journal des formes dégénérées (A3-08.a) ----
    // Ces appels sont volontairement hors garde : le spike JOURNALISE le
    // comportement d'OCCT (panique, erreur propre, forme vide), il ne le corrige pas.
    // Un abort C++ abat le process (constaté cas 1 : StdFail_NotDone) : chaque
    // cas se lance individuellement — `-- <n° 1-4>` ; sans argument, tous.
    let cas_demande = std::env::args().nth(1).and_then(|s| s.parse::<usize>().ok());
    for (i, (nom, contour, h)) in [
        ("contour dégénéré (points confondus)", vec![(0.0, 0.0); 4], 12.0),
        ("contour à 2 points", vec![(0.0, 0.0), (10.0, 0.0)], 12.0),
        ("hauteur nulle", vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], 0.0),
        ("hauteur négative", vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)], -5.0),
    ]
    .iter()
    .enumerate()
    {
        let numero = i + 1;
        if cas_demande.is_some_and(|c| c != numero) {
            continue;
        }
        let contour: Vec<(Length, Length)> = contour
            .iter()
            .map(|(x, y)| (Length::from_millimeters(*x), Length::from_millimeters(*y)))
            .collect();
        let debut = std::time::Instant::now();
        // On attrape les paniques OCCT potentielles via le résultat ;
        // une vraie panique C++ abattrait le process — le noter au journal.
        let resultat = Solide::prisme(&contour, Length::from_millimeters(*h));
        match resultat {
            Ok(s) => {
                let v = s.volume_mm3(tolerance);
                println!("[SPIKE] cas {numero} — {nom} : Ok, volume={v:?}, {:?}", debut.elapsed());
            }
            Err(e) => {
                println!("[SPIKE] cas {numero} — {nom} : Err({e:?}) en {:?}", debut.elapsed())
            }
        }
    }

    // ---- 5. Booléen vide (journal Shape::empty()/as_solid()) ----
    // self - self : le résultat devrait être vide. Comportement à
    // journaliser : panique, forme vide, volume nul ?
    let s1 = Solide::boite(
        Length::from_millimeters(10.0),
        Length::from_millimeters(10.0),
        Length::from_millimeters(10.0),
    )
    .expect("boîte 1");
    let s2 = Solide::boite(
        Length::from_millimeters(10.0),
        Length::from_millimeters(10.0),
        Length::from_millimeters(10.0),
    )
    .expect("boîte 2");
    let vide = s1.soustraire(&s2).expect("soustraction");
    let v = vide.volume_mm3(tolerance);
    println!("[SPIKE] booléen vide (self - self) : volume={v:?}");

    println!("[SPIKE] terminé — compléter le journal (checkpoint-pc §spike).");
}
