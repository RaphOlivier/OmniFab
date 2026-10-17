//! Caméra orthographique 2D du viewport (ADR-0015).
//!
//! Le monde monte, l'écran descend : la projection inverse l'axe Y
//! (convention SVG/DXF du projet). Les grandeurs de la caméra sont en
//! millimètres et en pixels logiques (« points ») ; le GPU reçoit des
//! f32 (leçon egui du jalon 1).

use crate::scene::Boite2;

/// Échelle minimale, en pixels par millimètre : évite les débordements
/// numériques aux zooms les plus poussés.
const ECHELLE_MIN: f64 = 1e-5;
/// Échelle maximale, en pixels par millimètre.
const ECHELLE_MAX: f64 = 1e6;

/// Caméra orthographique 2D : centre visé et échelle (ADR-0015).
///
/// La caméra est persistante dans l'application : ajustée
/// automatiquement quand la scêne change, puis pilotée par
/// l'utilisateur — molette (zoom au curseur) et glisser (pan).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    /// Centre visé, en millimètres.
    pub centre_mm: (f64, f64),
    /// Échelle, en pixels par millimètre.
    pub echelle: f64,
}

impl Camera2D {
    /// Caméra centrée sur l'origine, un pixel par millimètre.
    pub fn nouvelle() -> Self {
        Self {
            centre_mm: (0.0, 0.0),
            echelle: 1.0,
        }
    }

    /// Ajuste la caméra sur une boîte du monde : la boîte entière
    /// tient dans le viewport, marge comprise.
    pub fn ajuster(&mut self, largeur_px: f64, hauteur_px: f64, boite: Boite2, marge_px: f64) {
        let largeur_mm = (boite.max_x - boite.min_x).max(1e-9);
        let hauteur_mm = (boite.max_y - boite.min_y).max(1e-9);
        let echelle = ((largeur_px - 2.0 * marge_px).max(1.0) / largeur_mm)
            .min((hauteur_px - 2.0 * marge_px).max(1.0) / hauteur_mm)
            .clamp(ECHELLE_MIN, ECHELLE_MAX);
        self.echelle = echelle;
        self.centre_mm = boite.centre();
    }

    /// Zoome d'un facteur donné : le point du monde sous le curseur
    /// reste sous le curseur. Le décalage du curseur est donné par
    /// rapport au centre du viewport.
    pub fn zoom_au_curseur(&mut self, curseur_px: (f64, f64), facteur: f64) {
        let avant = self.depuis_ecran(curseur_px);
        self.echelle = (self.echelle * facteur).clamp(ECHELLE_MIN, ECHELLE_MAX);
        let apres = self.depuis_ecran(curseur_px);
        self.centre_mm.0 += avant.0 - apres.0;
        self.centre_mm.1 += avant.1 - apres.1;
    }

    /// Déplace la caméra d'un glissement : la scêne suit le pointeur.
    pub fn deplacer(&mut self, delta_px: (f64, f64)) {
        self.centre_mm.0 -= delta_px.0 / self.echelle;
        self.centre_mm.1 += delta_px.1 / self.echelle;
    }

    /// Point du monde affiché à un décalage du centre du viewport,
    /// en pixels logiques, l'axe Y vers le bas.
    pub fn depuis_ecran(&self, position_px: (f64, f64)) -> (f64, f64) {
        let (cx, cy) = self.centre_mm;
        (
            cx + position_px.0 / self.echelle,
            cy - position_px.1 / self.echelle,
        )
    }

    /// Décalage écran, par rapport au centre du viewport, d'un point
    /// du monde : en pixels logiques, l'axe Y vers le bas.
    pub fn vers_ecran(&self, point_mm: (f64, f64)) -> (f64, f64) {
        let (cx, cy) = self.centre_mm;
        (
            (point_mm.0 - cx) * self.echelle,
            (cy - point_mm.1) * self.echelle,
        )
    }

    /// Partie du monde visible dans un viewport de la taille donnée.
    pub fn boite_visible(&self, largeur_px: f64, hauteur_px: f64) -> Boite2 {
        let demi_x = largeur_px / 2.0 / self.echelle;
        let demi_y = hauteur_px / 2.0 / self.echelle;
        Boite2 {
            min_x: self.centre_mm.0 - demi_x,
            min_y: self.centre_mm.1 - demi_y,
            max_x: self.centre_mm.0 + demi_x,
            max_y: self.centre_mm.1 + demi_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'ajustement centre la boîte et prend l'échelle la plus grande
    /// qui la fait tenir avec sa marge.
    #[test]
    fn ajuster_englobe_et_centrer() {
        let mut camera = Camera2D::nouvelle();
        camera.ajuster(
            100.0,
            50.0,
            Boite2::du_point(0.0, 0.0).etendre(20.0, 10.0),
            10.0,
        );
        assert_eq!(camera.centre_mm, (10.0, 5.0), "centre de la boîte");
        assert_eq!(
            camera.echelle, 3.0,
            "limitée par la hauteur : 30 px / 10 mm"
        );
    }

    /// Le zoom garde le point du monde sous le curseur immobile.
    #[test]
    fn zoom_garde_le_point_sous_le_curseur() {
        let mut camera = Camera2D::nouvelle();
        camera.ajuster(
            200.0,
            100.0,
            Boite2::du_point(0.0, 0.0).etendre(100.0, 50.0),
            0.0,
        );
        let sous_curseur = camera.depuis_ecran((60.0, 40.0));
        camera.zoom_au_curseur((60.0, 40.0), 2.5);
        let apres = camera.depuis_ecran((60.0, 40.0));
        assert!(
            (apres.0 - sous_curseur.0).abs() < 1e-9,
            "x immobile sous le curseur"
        );
        assert!(
            (apres.1 - sous_curseur.1).abs() < 1e-9,
            "y immobile sous le curseur"
        );
    }

    /// Le glissement déplace la scêne dans le sens du pointeur.
    #[test]
    fn deplacer_suit_le_pointeur() {
        let mut camera = Camera2D {
            centre_mm: (0.0, 0.0),
            echelle: 2.0,
        };
        camera.deplacer((10.0, 6.0));
        assert_eq!(
            camera.centre_mm,
            (-5.0, 3.0),
            "x recule, y monte : la scêne suit"
        );
    }

    /// L'axe Y du monde monte, l'écran descend : un point plus haut
    /// dans la scêne s'affiche plus haut à l'écran.
    #[test]
    fn l_axe_y_s_inverse_a_l_ecran() {
        let camera = Camera2D::nouvelle();
        let bas = camera.vers_ecran((5.0, 0.0));
        let haut = camera.vers_ecran((5.0, 10.0));
        assert!(
            haut.1 < bas.1,
            "plus haut dans le monde, plus haut à l'écran"
        );
    }

    /// Le centre visé s'affiche au centre du viewport : son décalage
    /// écran est nul.
    #[test]
    fn le_centre_vise_est_au_centre_du_viewport() {
        let camera = Camera2D {
            centre_mm: (12.0, -7.0),
            echelle: 3.0,
        };
        assert_eq!(camera.vers_ecran((12.0, -7.0)), (0.0, 0.0));
    }
}
