//! Frontière OCCT (ADR-0023 D1) : tout accès au noyau OCCT vit dans ce
//! module — aucun type d'`opencascade` ne fuit hors de la crate (même
//! politique de re-encapsulation que le shim planegcs, ADR-0013).
//!
//! OCCT 7.8 BUILTIN (occt-sys 7.8.1) depuis le checkpoint \[J3] (2026-10-26) :
//! OCCT 7.9 casse opencascade-rs 0.3.0 (class→namespace TopoDS), 8.x refusé
//! par la gate — la version d'OCCT est épinglée par le lock, pas par le système.

use std::path::Path;

use opencascade::mesh::Mesh;
use opencascade::primitives::{IntoShape, Shape, Solid};

/// Forme topologique opaque, enveloppée par le
/// [`Solide`](crate::Solide) public.
pub(crate) struct Forme {
    /// La forme OCCT encapsulée — jamais exposée hors de ce module.
    forme: Shape,
}

impl Forme {
    /// Boîte alignée sur les axes, coin à l'origine.
    pub(crate) fn boite(largeur: f64, profondeur: f64, hauteur: f64) -> Self {
        Self {
            forme: Shape::box_with_dimensions(largeur, profondeur, hauteur),
        }
    }

    /// Cylindre vertical de rayon `rayon`, base centrée en (x, y, 0),
    /// de hauteur `hauteur` vers +Z.
    pub(crate) fn cylindre_centre(rayon: f64, x: f64, y: f64, hauteur: f64) -> Self {
        let base = glam::dvec3(x, y, 0.0);
        Self {
            forme: Shape::cylinder(base, rayon, glam::DVec3::Z, hauteur),
        }
    }

    /// Prisme : face construite sur le contour fermé, extrudée vers +Z
    /// (vraies courbes OCCT, pas de polyligne — ADR-0024 D2).
    pub(crate) fn prisme(
        contour: Vec<glam::DVec3>,
        hauteur: f64,
    ) -> Result<Self, opencascade::Error> {
        let solide = Solid::extrude_polygon(contour, hauteur)?;
        Ok(Self {
            forme: Shape::from(&solide),
        })
    }

    /// Soustraction booléenne `self - autre`.
    pub(crate) fn soustraire(&self, autre: &Self) -> Self {
        Self {
            forme: self.forme.subtract(&autre.forme).into_shape(),
        }
    }

    /// Union booléenne `self + autre`.
    pub(crate) fn unir(&self, autre: &Self) -> Self {
        Self {
            forme: self.forme.union(&autre.forme).into_shape(),
        }
    }

    /// Tessellation (coordonnées en millimètres).
    pub(crate) fn tesseller(&self, tolerance: f64) -> Result<Mesh, opencascade::Error> {
        self.forme.mesh_with_tolerance(tolerance)
    }

    /// Écrit toutes les formes dans un fichier STEP.
    pub(crate) fn ecrire_step_tous(
        formes: &[&Self],
        chemin: &Path,
    ) -> Result<(), opencascade::Error> {
        let references: Vec<&Shape> = formes.iter().map(|f| &f.forme).collect();
        Shape::write_all_step(references, chemin)
    }

    /// Lit un fichier STEP (une forme par fichier au lot 1).
    pub(crate) fn lire_step(chemin: &Path) -> Result<Self, opencascade::Error> {
        Ok(Self {
            forme: Shape::read_step(chemin)?,
        })
    }

    /// Écrit la forme dans un fichier BREP textuel.
    pub(crate) fn ecrire_brep(&self, chemin: &Path) -> Result<(), opencascade::Error> {
        self.forme.write_brep_text(chemin)
    }

    /// Lit un fichier BREP textuel.
    pub(crate) fn lire_brep(chemin: &Path) -> Result<Self, opencascade::Error> {
        Ok(Self {
            forme: Shape::read_brep_text(chemin)?,
        })
    }
}

// `Shape` OCCT n'implémente pas `Debug` (enveloppe C++) : `Forme` en
// fournit un minimal, sans divulguer son intérieur (frontière ADR-0023).
impl std::fmt::Debug for Forme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Forme").finish_non_exhaustive()
    }
}

// ----------------------------------------------------------------------------
// SONDE TEMPORAIRE DU SPIKE PHASE A (ADR-0023) — À RETIRER après le spike.
// Journal du comportement de `Shape::empty()`/`as_solid()` sur un booléen
// vide (consigne checkpoint-pc §« Consignes du spike phase A », point 4).
// Ce test ne peut pas vivre dans `tests/` : le module `occt` est privé
// (frontière ADR-0023). Exécuter : cargo test -p forge-cad -- --nocapture.
// ----------------------------------------------------------------------------

#[cfg(test)]
mod sonde_spike_phase_a {
    use super::Forme;

    /// ADR-0023 spike : journal du comportement de `Shape::empty()` et
    /// `as_solid()` sur un booléen vide (self − self).
    #[test]
    fn sonde_boolen_vide() {
        let a = Forme::boite(10.0, 10.0, 10.0);
        let b = Forme::boite(10.0, 10.0, 10.0);
        let vide = Forme::soustraire(&a, &b);
        // Journal : Ok(triangles=N) ou Err(...) — noter également toute panique.
        match vide.tesseller(0.01) {
            Ok(m) => println!(
                "[SONDE] self - self : tessellation Ok, sommets={}, triangles={}",
                m.vertices.len(),
                m.indices.len() / 3
            ),
            Err(e) => println!("[SONDE] self - self : tessellation Err({e:?})"),
        }
        println!("[SONDE] self - self : as_solid = {}", vide.forme.as_solid().is_some());

        // Shape::empty() : « modèle d'espace vide » du binding (composé vide).
        let vide_occt = opencascade::primitives::Shape::empty();
        println!("[SONDE] Shape::empty() : as_solid = {}", vide_occt.as_solid().is_some());
        match vide_occt.mesh_with_tolerance(0.01) {
            Ok(m) => {
                println!("[SONDE] Shape::empty() : tessellation Ok, sommets={}", m.vertices.len())
            }
            Err(e) => println!("[SONDE] Shape::empty() : tessellation Err({e:?})"),
        }
    }
}
