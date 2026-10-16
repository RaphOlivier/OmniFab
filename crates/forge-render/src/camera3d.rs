//! Caméra orbite 3D du viewport (ADR-0015, phase C2).
//!
//! Convention CAO du projet : l'axe Z monte, la cible est le centre
//! de la scène. Les grandeurs de la caméra sont en millimètres et en
//! pixels logiques ; le GPU reçoit des f32 (leçon egui du jalon 1).
//! L'élévation est bornée juste sous la verticale : la base caméra
//! (droite, haut, avant) ne dégénère jamais.

use crate::scene3d::Boite3D;

/// Distance minimale à la cible, en millimètres.
const DISTANCE_MIN: f64 = 1e-3;
/// Distance maximale à la cible, en millimètres.
const DISTANCE_MAX: f64 = 1e9;
/// Élévation maximale, en radians (juste sous la verticale).
const ELEVATION_MAX: f64 = 1.55;
/// Champ de vision vertical, en radians (45 degrés).
const FOVY: f64 = std::f64::consts::FRAC_PI_4;
/// Marge du cadrage automatique : 10 pour cent au-delà de la sphère
/// englobante.
const MARGE_CADRAGE: f64 = 1.1;
/// Sensibilité de l'orbite, en radians par pixel glissé.
const SENSIBILITE: f64 = 0.01;

/// Produit vectoriel de deux vecteurs de l'espace.
fn croise(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

/// Produit scalaire de deux vecteurs de l'espace.
fn scalaire(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

/// Caméra orbite du viewport 3D : cible, distance, azimut et
/// élévation (ADR-0015, phase C2).
///
/// La caméra est persistante dans l'application : ajustée
/// automatiquement quand la scène change, puis pilotée par
/// l'utilisateur — molette (zoom) et glisser (orbite).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// Point visé, en millimètres.
    pub cible_mm: (f64, f64, f64),
    /// Distance de la caméra à la cible, en millimètres.
    pub distance: f64,
    /// Azimut autour de l'axe Z, en radians.
    pub azimut: f64,
    /// Élévation au-dessus du plan XY, en radians.
    pub elevation: f64,
}

impl Camera3D {
    /// Caméra en diagonale au-dessus de l'origine, à 1000 mm.
    pub fn nouvelle() -> Self {
        Self {
            cible_mm: (0.0, 0.0, 0.0),
            distance: 1000.0,
            azimut: -std::f64::consts::FRAC_PI_4,
            elevation: std::f64::consts::FRAC_PI_4,
        }
    }

    /// Position de la caméra dans le monde.
    fn position(&self) -> (f64, f64, f64) {
        let (c, d) = (self.cible_mm, self.distance);
        let plan = d * self.elevation.cos();
        (
            c.0 + plan * self.azimut.cos(),
            c.1 + plan * self.azimut.sin(),
            c.2 + d * self.elevation.sin(),
        )
    }

    /// Base caméra : vecteurs unitaires droite, haut, avant.
    fn base(&self) -> ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) {
        let position = self.position();
        let cible = self.cible_mm;
        let avant = (
            cible.0 - position.0,
            cible.1 - position.1,
            cible.2 - position.2,
        );
        let norme = scalaire(avant, avant).sqrt();
        let avant = (avant.0 / norme, avant.1 / norme, avant.2 / norme);
        // L'élévation est bornée sous la verticale : le produit
        // vectoriel avec l'axe Z ne s'annule jamais.
        let haut_monde = (0.0, 0.0, 1.0);
        let droite = croise(avant, haut_monde);
        let norme = scalaire(droite, droite).sqrt();
        let droite = (droite.0 / norme, droite.1 / norme, droite.2 / norme);
        let haut = croise(droite, avant);
        (droite, haut, avant)
    }

    /// Ajuste la caméra sur une boîte 3D : la sphère englobante tient
    /// dans le champ vertical, marge comprise.
    pub fn ajuster(&mut self, boite: Boite3D) {
        self.cible_mm = boite.centre();
        let rayon = boite.rayon();
        let demi_angle = (FOVY / 2.0).sin();
        let distance = (rayon * MARGE_CADRAGE / demi_angle).clamp(DISTANCE_MIN, DISTANCE_MAX);
        self.distance = distance;
    }

    /// Orbite : l'azimut suit le glissement horizontal, l'élévation
    /// le glissement vertical — bornée juste sous la verticale.
    pub fn orbiter(&mut self, delta_px: (f64, f64)) {
        self.azimut += delta_px.0 * SENSIBILITE;
        let cible = self.elevation + delta_px.1 * SENSIBILITE;
        self.elevation = cible.clamp(-ELEVATION_MAX, ELEVATION_MAX);
    }

    /// Zoome : la distance à la cible change du facteur inverse.
    pub fn zoomer(&mut self, facteur: f64) {
        self.distance = (self.distance / facteur).clamp(DISTANCE_MIN, DISTANCE_MAX);
    }

    /// Échelle apparente au plan de la cible, en pixels logiques par
    /// millimètre — sert au pas de la grille au sol.
    pub fn echelle_cible(&self, hauteur_px: f64) -> f64 {
        hauteur_px / (2.0 * self.distance * (FOVY / 2.0).tan())
    }

    /// Projette un point du monde : position en pixels logiques
    /// relative au coin du viewport, None derrière la caméra.
    pub fn vers_ecran(
        &self,
        point_mm: (f64, f64, f64),
        largeur_px: f64,
        hauteur_px: f64,
    ) -> Option<(f64, f64)> {
        let position = self.position();
        let (droite, haut, avant) = self.base();
        let vecteur = (
            point_mm.0 - position.0,
            point_mm.1 - position.1,
            point_mm.2 - position.2,
        );
        let profondeur = scalaire(vecteur, avant);
        if profondeur <= 0.0 {
            return None;
        }
        let x_cam = scalaire(vecteur, droite);
        let y_cam = scalaire(vecteur, haut);
        let demi = (FOVY / 2.0).tan();
        let aspect = largeur_px.max(1.0) / hauteur_px.max(1.0);
        let x_ndc = x_cam / (profondeur * demi * aspect);
        let y_ndc = y_cam / (profondeur * demi);
        Some((
            largeur_px / 2.0 + x_ndc * largeur_px / 2.0,
            hauteur_px / 2.0 - y_ndc * hauteur_px / 2.0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le point visé s'affiche au centre du viewport.
    #[test]
    fn le_point_vise_atterrit_au_centre_du_viewport() {
        let camera = Camera3D {
            cible_mm: (12.0, -7.0, 4.0),
            distance: 500.0,
            azimut: 0.7,
            elevation: 0.3,
        };
        let affiche = camera
            .vers_ecran(camera.cible_mm, 800.0, 600.0)
            .expect("devant la caméra");
        assert!((affiche.0 - 400.0).abs() < 1e-6, "x centré");
        assert!((affiche.1 - 300.0).abs() < 1e-6, "y centré");
    }

    /// L'axe Z monte à l'écran : un point plus haut s'affiche plus haut.
    #[test]
    fn l_axe_z_monte_a_l_ecran() {
        let camera = Camera3D::nouvelle();
        let bas = camera
            .vers_ecran((0.0, 0.0, 0.0), 800.0, 600.0)
            .expect("origine devant la caméra");
        let haut = camera
            .vers_ecran((0.0, 0.0, 10.0), 800.0, 600.0)
            .expect("point haut devant la caméra");
        assert!(
            haut.1 < bas.1,
            "plus haut dans le monde, plus haut à l'écran"
        );
    }

    /// L'orbite suit le glissement ; l'élévation est bornée sous la
    /// verticale.
    #[test]
    fn l_orbite_suit_le_glissement_et_borne_l_elevation() {
        let mut camera = Camera3D::nouvelle();
        let azimut = camera.azimut;
        camera.orbiter((20.0, -30.0));
        assert!((camera.azimut - azimut - 0.2).abs() < 1e-12, "azimut suivi");
        let attendue = std::f64::consts::FRAC_PI_4 - 0.3;
        assert!(
            (camera.elevation - attendue).abs() < 1e-12,
            "élévation suivie"
        );
        for _ in 0..1000 {
            camera.orbiter((0.0, -100.0));
        }
        assert!(
            camera.elevation <= 1.55,
            "élévation bornée sous la verticale"
        );
    }

    /// Le zoom divise la distance ; la distance minimale tient.
    #[test]
    fn le_zoom_divise_la_distance() {
        let mut camera = Camera3D::nouvelle();
        camera.zoomer(2.0);
        assert_eq!(camera.distance, 500.0, "distance divisée par deux");
        camera.distance = 1e-3;
        camera.zoomer(100.0);
        assert_eq!(camera.distance, 1e-3, "distance minimale conservée");
    }

    /// Le cadrage vise le centre de la boîte, à distance de sa sphère
    /// englobante, marge comprise.
    #[test]
    fn le_cadrage_vise_le_centre_de_la_boite() {
        let mut camera = Camera3D::nouvelle();
        camera.ajuster(Boite3D::du_point(0.0, 0.0, 0.0).etendre(100.0, 50.0, 20.0));
        assert_eq!(camera.cible_mm, (50.0, 25.0, 10.0), "centre de la boîte");
        let rayon = (50.0 * 50.0 + 25.0 * 25.0 + 10.0 * 10.0).sqrt();
        let attendue = rayon * 1.1 / std::f64::consts::FRAC_PI_8.sin();
        assert!(
            (camera.distance - attendue).abs() < 1e-9,
            "distance de cadrage"
        );
    }

    /// Un point derrière la caméra n'est pas projeté.
    #[test]
    fn le_point_derriere_la_camera_n_est_pas_projete() {
        let mut camera = Camera3D::nouvelle();
        camera.cible_mm = (0.0, 0.0, 0.0);
        camera.azimut = 0.0;
        camera.elevation = 0.0;
        // La caméra est à x positif, regardant vers moins x : un
        // point au-delà d'elle est derrière.
        let derriere = (camera.distance * 2.0, 0.0, 0.0);
        assert!(
            camera.vers_ecran(derriere, 800.0, 600.0).is_none(),
            "point derrière la caméra ignoré"
        );
    }
}
