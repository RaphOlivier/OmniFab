//! Scénarios de démonstration scriptée de la galerie de captures
//! (gouvernance 2026-10-19) : `OMNIFAB_DEMO` lance l'application sous
//! Xvfb avec le scénario donné — la scène est posée comme par un
//! utilisateur, rien n'est ajouté au rendu. Décision n°4 de
//! `preparation/inventaire-expect.md` (actée 2026-10-26, découpage
//! étape 4 de `preparation/decoupage-main-rs.md`) : la graine ne
//! panique plus — chaque refus garde son message d'origine et
//! remonte à l'appelant, l'application retombe sur l'écran
//! d'accueil (repli dégradé).

use crate::{DemoApp, Ecran, Vue};
use forge_io::svg;
use forge_render::pas_de_grille;
use forge_session::{
    AddConstraint, AddEntity, ContrainteEsquisse, EntiteEsquisse, ImportTraces, ValeurCote,
};
use forge_units::Length;
use crate::theme::GRILLE_CIBLE_PX;

/// SVG de démonstration de la galerie de captures : sans
/// `width`/`height`, usvg le lit à 25,4 dpi — une unité vaut
/// exactement 1 mm (gouvernance 2026-10-19).
const SVG_DEMO: &[u8] = b"<?xml version=\"1.0\"?>
<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\">
  <circle cx=\"30\" cy=\"40\" r=\"20\" fill=\"none\"/>
  <rect x=\"55\" y=\"10\" width=\"40\" height=\"30\" fill=\"none\"/>
  <path d=\"M10,80 C40,60 60,95 90,70\" fill=\"none\"/>
</svg>
";

/// Maillage de démonstration de la galerie de captures : un
/// tétraèdre STL binaire, quatre triangles écrits octet par
/// octet en petit-boutiste (gouvernance 2026-10-19).
fn octets_stl_demo() -> Vec<u8> {
    let sommets = [
        (0.0f32, 0.0f32, 0.0f32),
        (30.0, 0.0, 0.0),
        (0.0, 40.0, 0.0),
        (0.0, 0.0, 25.0),
    ];
    let faces = [(0, 1, 2), (0, 1, 3), (1, 2, 3), (0, 2, 3)];
    let mut octets = vec![0u8; 80];
    octets.extend_from_slice(&4u32.to_le_bytes());
    let normale_nulle = 0.0f32.to_le_bytes();
    let attribut_nul = 0u16.to_le_bytes();
    for (a, b, c) in faces {
        octets.extend_from_slice(&normale_nulle);
        octets.extend_from_slice(&normale_nulle);
        octets.extend_from_slice(&normale_nulle);
        for indice in [a, b, c] {
            let (x, y, z) = sommets[indice];
            octets.extend_from_slice(&x.to_le_bytes());
            octets.extend_from_slice(&y.to_le_bytes());
            octets.extend_from_slice(&z.to_le_bytes());
        }
        octets.extend_from_slice(&attribut_nul);
    }
    octets
}

impl DemoApp {
    /// Prépare la scène du mode démo scripté (gouvernance
    /// 2026-10-19) : la galerie de captures lance l'application sous
    /// Xvfb avec le scénario donné — rien n'est ajouté au rendu, la
    /// scène est posée comme par un utilisateur.
    ///
    /// Décision n°4 de `preparation/inventaire-expect.md` : un échec
    /// de la graine est un bug du scénario, pas d'un utilisateur —
    /// l'erreur remonte à l'appelant, l'application retombe sur
    /// l'écran d'accueil (repli dégradé), plus aucune panique.
    pub(crate) fn preparer_demo(&mut self, scenario: &str) -> Result<(), String> {
        if scenario == "accueil" {
            return Ok(());
        }
        self.ecran = Ecran::Atelier;
        if scenario == "atelier-3d" {
            self.vue = Vue::TroisD;
        }
        if let Err(raison) = self.poser_la_graine() {
            self.ecran = Ecran::Accueil;
            return Err(raison);
        }
        Ok(())
    }

    /// Pose la graine du scénario : tracés SVG importés, tétraèdre
    /// STL, croquis contraint du rectangle paramétrique — chaque
    /// refus garde le message de l'ancien `expect` (« svg de démo »,
    /// « coin posé »…), plus aucun `expect` (décision n°4).
    fn poser_la_graine(&mut self) -> Result<(), String> {
        let traces = svg::import(SVG_DEMO, &svg::SvgBudgets::default())
            .map_err(|erreur| format!("svg de démo : {erreur}"))?;
        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
        self.executer(&ImportTraces {
            format: "svg".into(),
            octets: SVG_DEMO.to_vec(),
            traces: traces.len(),
            sommets,
        })
        .map_err(|refus| format!("tracés importés : {refus}"))?;
        self.importer_maillage(octets_stl_demo(), 4, 1.0);
        let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
        for (x, y) in [
            (0.0, 0.0),
            (pas * 4.0, 0.0),
            (pas * 4.0, pas * 3.0),
            (0.0, pas * 3.0),
        ] {
            self.executer(&AddEntity {
                entite: EntiteEsquisse::Point {
                    x: Length::from_millimeters(x),
                    y: Length::from_millimeters(y),
                },
            })
            .map_err(|refus| format!("coin posé : {refus}"))?;
        }
        for (p1, p2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            self.executer(&AddEntity {
                entite: EntiteEsquisse::Ligne { p1, p2 },
            })
            .map_err(|refus| format!("côté posé : {refus}"))?;
        }
        for (ligne, horizontale) in [(4, true), (5, false), (6, true), (7, false)] {
            let contrainte = if horizontale {
                ContrainteEsquisse::Horizontale { ligne }
            } else {
                ContrainteEsquisse::Verticale { ligne }
            };
            self.executer(&AddConstraint { contrainte })
                .map_err(|refus| format!("contrainte posée : {refus}"))?;
        }
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 0,
                p2: 1,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 4.0),
                },
            },
        })
        .map_err(|refus| format!("cote posée : {refus}"))?;
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::Distance {
                p1: 1,
                p2: 2,
                valeur: ValeurCote::Fixe {
                    mm: Length::from_millimeters(pas * 3.0),
                },
            },
        })
        .map_err(|refus| format!("cote posée : {refus}"))?;
        self.executer(&AddConstraint {
            contrainte: ContrainteEsquisse::PointFixe { point: 0 },
        })
        .map_err(|refus| format!("coin fixé : {refus}"))?;
        self.demande_resolution_esquisse();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La graine se pose sans refus et mène à l'atelier — c'est du
    /// code de démonstration testé (décision n°4 de
    /// `preparation/inventaire-expect.md`).
    #[test]
    fn la_graine_se_pose_sans_refus() {
        let mut app = DemoApp::new(crate::i18n::catalogue(), None);
        app.preparer_demo("atelier-3d").expect("graine posée");
        assert_eq!(app.ecran, Ecran::Atelier, "l'atelier s'ouvre");
        assert_eq!(app.vue, Vue::TroisD, "le scénario 3D est respecté");
    }
}
