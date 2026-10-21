//! Scêne 2D du viewport : segments en millimètres, alimentés par
//! l'application (ADR-0015).

use forge_geom::Path2;

/// Couleur RGBA, composantes normalisées entre 0 et 1.
pub type Couleur = [f32; 4];

/// Boîte englobante 2D, en millimètres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boite2 {
    /// Borne inférieure en x, en millimètres.
    pub min_x: f64,
    /// Borne inférieure en y, en millimètres.
    pub min_y: f64,
    /// Borne supérieure en x, en millimètres.
    pub max_x: f64,
    /// Borne supérieure en y, en millimètres.
    pub max_y: f64,
}

impl Boite2 {
    /// Première boîte d'une scêne : réduite au point donné.
    pub fn du_point(x: f64, y: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    /// Étend la boîte pour englober le point donné.
    pub fn etendre(self, x: f64, y: f64) -> Self {
        Self {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
        }
    }

    /// Centre de la boîte, en millimètres.
    pub fn centre(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
}

/// Segment dessiné par le viewport : extrémités en millimètres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment2D {
    /// Départ, en millimètres.
    pub depart: (f64, f64),
    /// Arrivée, en millimètres.
    pub arrivee: (f64, f64),
    /// Couleur RGBA normalisée.
    pub couleur: Couleur,
    /// Largeur du trait, en pixels logiques.
    pub largeur_px: f32,
}

/// Scêne 2D du viewport : une liste de segments en millimètres
/// (ADR-0015). Alimentée par l'application, jamais par la crate.
#[derive(Debug, Default)]
pub struct Scene2D {
    /// Segments de la scêne.
    pub segments: Vec<Segment2D>,
}

impl Scene2D {
    /// Scêne vide.
    pub fn vide() -> Self {
        Self::default()
    }

    /// Ajoute un segment.
    pub fn ajouter(&mut self, segment: Segment2D) {
        self.segments.push(segment);
    }

    /// Scêne des tracés importés : un segment par arête, les tracés
    /// fermés refermés sur leur premier sommet.
    pub fn depuis_traces(traces: &[Path2], couleur: Couleur, largeur_px: f32) -> Self {
        let mut scene = Self::vide();
        for trace in traces {
            let points = trace.points();
            if points.len() < 2 {
                continue;
            }
            for paire in points.windows(2) {
                scene.ajouter(Segment2D {
                    depart: (paire[0].x().millimeters(), paire[0].y().millimeters()),
                    arrivee: (paire[1].x().millimeters(), paire[1].y().millimeters()),
                    couleur,
                    largeur_px,
                });
            }
            if trace.is_closed() {
                // fermeture : dernier sommet vers premier
                let premier = points.first().expect("longueur vérifiée");
                let dernier = points.last().expect("longueur vérifiée");
                scene.ajouter(Segment2D {
                    depart: (dernier.x().millimeters(), dernier.y().millimeters()),
                    arrivee: (premier.x().millimeters(), premier.y().millimeters()),
                    couleur,
                    largeur_px,
                });
            }
        }
        scene
    }

    /// Ajoute la grille de fond : lignes aux multiples d'un pas
    /// adaptatif, couvrant toute la zone visible.
    pub fn ajouter_grille(
        &mut self,
        visible: Boite2,
        echelle: f64,
        cible_px: f64,
        couleur: Couleur,
    ) {
        let pas = pas_de_grille(echelle, cible_px);
        let mut x = (visible.min_x / pas).ceil() * pas;
        while x <= visible.max_x {
            self.ajouter(Segment2D {
                depart: (x, visible.min_y),
                arrivee: (x, visible.max_y),
                couleur,
                largeur_px: 1.0,
            });
            x += pas;
        }
        let mut y = (visible.min_y / pas).ceil() * pas;
        while y <= visible.max_y {
            self.ajouter(Segment2D {
                depart: (visible.min_x, y),
                arrivee: (visible.max_x, y),
                couleur,
                largeur_px: 1.0,
            });
            y += pas;
        }
    }
}

/// Pas de grille lisible : la plus petite puissance de deux dont la
/// représentation atteint `cible_px` pixels (ADR-0020). La série
/// dyadique garde chaque ligne alignée sur les lignes du zoom
/// précédent — chaque pas est le double du précédent, l'aimantation
/// ne saute jamais d'une grille à une autre décalée.
pub fn pas_de_grille(echelle: f64, cible_px: f64) -> f64 {
    let brut = (cible_px / echelle).max(1e-9);
    2.0_f64.powf(brut.log2().ceil())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::Point2;
    use forge_units::Length;

    /// Point de test aux coordonnées données (mm).
    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(Length::from_millimeters(x), Length::from_millimeters(y))
            .expect("coordonnées finies")
    }

    /// Le tracé fermé est refermé sur son premier sommet.
    #[test]
    fn trace_ferme_referme_sur_premier_sommet() {
        let trace = Path2::from_points(
            vec![point(0.0, 0.0), point(10.0, 0.0), point(0.0, 10.0)],
            true,
        );
        let scene = Scene2D::depuis_traces(&[trace], [1.0, 1.0, 1.0, 1.0], 1.5);
        assert_eq!(scene.segments.len(), 3, "deux arêtes et fermeture");
    }

    /// Le pas de grille est une puissance de deux : chaque ligne du
    /// zoom précédent reste sur une ligne du pas suivant (ADR-0020).
    #[test]
    fn pas_de_grille_est_dyadique() {
        assert!(
            (pas_de_grille(1.0, 80.0) - 128.0).abs() < 1e-9,
            "80 px à 1 px/mm"
        );
        assert!(
            (pas_de_grille(2.0, 80.0) - 64.0).abs() < 1e-9,
            "40 px à 2 px/mm"
        );
        assert!(
            (pas_de_grille(10.0, 60.0) - 8.0).abs() < 1e-9,
            "6 px à 10 px/mm"
        );
        for echelle in [0.3, 1.0, 2.7, 10.0, 63.0] {
            let pas = pas_de_grille(echelle, 80.0);
            assert!(
                (pas.log2().round() - pas.log2()).abs() < 1e-9,
                "le pas reste une puissance de deux à l'échelle {echelle}"
            );
            let grossier = pas_de_grille(echelle / 2.0, 80.0);
            assert!(
                (grossier - 2.0 * pas).abs() < 1e-9,
                "un cran de zoom double le pas"
            );
        }
    }

    /// La grille couvre la zone visible au pas calculé.
    #[test]
    fn grille_couvre_la_zone_visible() {
        let mut scene = Scene2D::vide();
        let visible = Boite2::du_point(0.0, 0.0).etendre(250.0, 100.0);
        scene.ajouter_grille(visible, 2.0, 100.0, [1.0, 1.0, 1.0, 0.1]);
        assert_eq!(
            scene.segments.len(),
            6,
            "quatre verticales et deux horizontales"
        );
        let verticales = scene
            .segments
            .iter()
            .filter(|segment| (segment.depart.0 - segment.arrivee.0).abs() < 1e-9)
            .count();
        assert_eq!(verticales, 4, "x = 0, 64, 128, 192 mm");
    }
}
