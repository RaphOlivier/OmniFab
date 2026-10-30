//! Scêne 3D du viewport : segments en millimètres, alimentés par
//! l'application (ADR-0015, phase C2).

use crate::scene::Couleur;
use forge_geom::{Mesh, Path2, Point2};
use std::collections::HashSet;

/// Boîte englobante 3D, en millimètres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boite3D {
    /// Borne inférieure en x, en millimètres.
    pub min_x: f64,
    /// Borne inférieure en y, en millimètres.
    pub min_y: f64,
    /// Borne inférieure en z, en millimètres.
    pub min_z: f64,
    /// Borne supérieure en x, en millimètres.
    pub max_x: f64,
    /// Borne supérieure en y, en millimètres.
    pub max_y: f64,
    /// Borne supérieure en z, en millimètres.
    pub max_z: f64,
}

impl Boite3D {
    /// Première boîte d'une scêne : réduite au point donné.
    pub fn du_point(x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            min_z: z,
            max_x: x,
            max_y: y,
            max_z: z,
        }
    }

    /// Étend la boîte pour englober le point donné.
    pub fn etendre(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            min_z: self.min_z.min(z),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
            max_z: self.max_z.max(z),
        }
    }

    /// Centre de la boîte, en millimètres.
    pub fn centre(&self) -> (f64, f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
            (self.min_z + self.max_z) / 2.0,
        )
    }

    /// Rayon de la sphère englobante : demi-diagonale de la boîte.
    pub fn rayon(&self) -> f64 {
        let dx = (self.max_x - self.min_x) / 2.0;
        let dy = (self.max_y - self.min_y) / 2.0;
        let dz = (self.max_z - self.min_z) / 2.0;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Segment dessiné par le viewport 3D : extrémités en millimètres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment3D {
    /// Départ, en millimètres.
    pub depart: (f64, f64, f64),
    /// Arrivée, en millimètres.
    pub arrivee: (f64, f64, f64),
    /// Couleur RGBA normalisée.
    pub couleur: Couleur,
}

/// Scêne 3D du viewport : une liste de segments en millimètres
/// (ADR-0015, phase C2). Alimentée par l'application, jamais par la
/// crate.
#[derive(Debug, Default)]
pub struct Scene3D {
    /// Segments de la scêne.
    pub segments: Vec<Segment3D>,
}

impl Scene3D {
    /// Scêne vide.
    pub fn vide() -> Self {
        Self::default()
    }

    /// Ajoute un segment.
    pub fn ajouter(&mut self, segment: Segment3D) {
        self.segments.push(segment);
    }

    /// Wireframe d'un maillage : une arête par paire de sommets
    /// voisine, dédupliquée — chaque arête est dessinée une fois et le
    /// plafond d'affichage de la phase B est supprimé (ADR-0015).
    pub fn depuis_maillage(maillage: &Mesh, couleur: Couleur) -> Self {
        let sommets = maillage.vertices();
        let mut vues: HashSet<(u32, u32)> = HashSet::new();
        let mut scene = Self::vide();
        for triangle in maillage.triangles() {
            for arete in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                let (a, b) = if arete.0 <= arete.1 {
                    arete
                } else {
                    (arete.1, arete.0)
                };
                if !vues.insert((a, b)) {
                    continue;
                }
                let depart = sommets[a as usize];
                let arrivee = sommets[b as usize];
                scene.ajouter(Segment3D {
                    depart: (
                        depart.x().millimeters(),
                        depart.y().millimeters(),
                        depart.z().millimeters(),
                    ),
                    arrivee: (
                        arrivee.x().millimeters(),
                        arrivee.y().millimeters(),
                        arrivee.z().millimeters(),
                    ),
                    couleur,
                });
            }
        }
        scene
    }

    /// Tracés 2D posés sur le plan z = 0 : les tracés importés vivent
    /// dans le plan du monde (ADR-0015, phase C2).
    pub fn depuis_traces(traces: &[Path2], couleur: Couleur) -> Self {
        let mut scene = Self::vide();
        for trace in traces {
            let points = trace.points();
            if points.len() < 2 {
                continue;
            }
            let plat = |point: &Point2| (point.x().millimeters(), point.y().millimeters(), 0.0);
            for paire in points.windows(2) {
                scene.ajouter(Segment3D {
                    depart: plat(&paire[0]),
                    arrivee: plat(&paire[1]),
                    couleur,
                });
            }
            if trace.is_closed() {
                let premier = points.first().expect("longueur vérifiée");
                let dernier = points.last().expect("longueur vérifiée");
                scene.ajouter(Segment3D {
                    depart: plat(dernier),
                    arrivee: plat(premier),
                    couleur,
                });
            }
        }
        scene
    }

    /// Grille au sol : lignes aux multiples du pas sur le plan z = 0,
    /// couvrant les bornes x et y de la boîte donnée.
    pub fn ajouter_grille(&mut self, boite: &Boite3D, pas: f64, couleur: Couleur) {
        let mut x = (boite.min_x / pas).ceil() * pas;
        while x <= boite.max_x {
            self.ajouter(Segment3D {
                depart: (x, boite.min_y, 0.0),
                arrivee: (x, boite.max_y, 0.0),
                couleur,
            });
            x += pas;
        }
        let mut y = (boite.min_y / pas).ceil() * pas;
        while y <= boite.max_y {
            self.ajouter(Segment3D {
                depart: (boite.min_x, y, 0.0),
                arrivee: (boite.max_x, y, 0.0),
                couleur,
            });
            y += pas;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::Point3;
    use forge_units::Length;

    /// Point 2D de test aux coordonnées données (mm).
    fn point2(x: f64, y: f64) -> Point2 {
        Point2::new(Length::from_millimeters(x), Length::from_millimeters(y))
            .expect("coordonnées finies")
    }

    /// Point 3D de test aux coordonnées données (mm).
    fn point3(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(
            Length::from_millimeters(x),
            Length::from_millimeters(y),
            Length::from_millimeters(z),
        )
        .expect("coordonnées finies")
    }

    /// Le tétraèdre dessine ses six arêtes, pas douze : chaque arête
    /// partagée n'est émise qu'une fois (phase C2, ADR-0015).
    #[test]
    fn le_tetraedre_dessine_six_aretes() {
        let sommets = vec![
            point3(0.0, 0.0, 0.0),
            point3(1.0, 0.0, 0.0),
            point3(0.0, 1.0, 0.0),
            point3(0.0, 0.0, 1.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];
        let maillage = Mesh::from_parts(sommets, triangles).expect("maillage valide");
        let scene = Scene3D::depuis_maillage(&maillage, [0.0, 1.0, 1.0, 1.0]);
        assert_eq!(scene.segments.len(), 6, "une arête par paire unique");
    }

    /// L'arête partagée par deux triangles n'est pas dupliquée.
    #[test]
    fn l_arete_partagee_n_est_pas_dupliquee() {
        let sommets = vec![
            point3(0.0, 0.0, 0.0),
            point3(1.0, 0.0, 0.0),
            point3(0.0, 1.0, 0.0),
            point3(0.0, 0.0, 1.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 1, 3]];
        let maillage = Mesh::from_parts(sommets, triangles).expect("maillage valide");
        let scene = Scene3D::depuis_maillage(&maillage, [0.0, 1.0, 1.0, 1.0]);
        assert_eq!(scene.segments.len(), 5, "cinq arêtes uniques");
    }

    /// Le wireframe n'a plus de plafond : un maillage au-delà des
    /// 20 000 triangles de la phase B dessine toutes ses arêtes.
    #[test]
    fn le_wireframe_n_a_plus_de_plafond() {
        let sommets = vec![
            point3(0.0, 0.0, 0.0),
            point3(1.0, 0.0, 0.0),
            point3(0.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2]; 21_000];
        let maillage = Mesh::from_parts(sommets, triangles).expect("maillage valide");
        let scene = Scene3D::depuis_maillage(&maillage, [0.0, 1.0, 1.0, 1.0]);
        assert_eq!(scene.segments.len(), 3, "toutes les arêtes, dédupliquées");
    }

    /// Le tracé fermé est refermé sur son premier sommet, posé sur
    /// le plan z = 0.
    #[test]
    fn trace_ferme_pose_sur_le_plan_z() {
        let trace = Path2::from_points(
            vec![point2(0.0, 0.0), point2(10.0, 0.0), point2(0.0, 10.0)],
            true,
        );
        let scene = Scene3D::depuis_traces(&[trace], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(scene.segments.len(), 3, "deux arêtes et fermeture");
        for segment in &scene.segments {
            assert_eq!(segment.depart.2, 0.0, "z nul au départ");
        }
    }

    /// La grille au sol couvre les bornes x et y de la boîte.
    #[test]
    fn la_grille_couvre_les_bornes_de_la_boite() {
        let mut scene = Scene3D::vide();
        let boite = Boite3D::du_point(0.0, 0.0, 0.0).etendre(200.0, 100.0, 50.0);
        scene.ajouter_grille(&boite, 50.0, [1.0, 1.0, 1.0, 1.0]);
        // Verticales : x = 0, 50, 100, 150, 200 ; horizontales :
        // y = 0, 50, 100.
        assert_eq!(
            scene.segments.len(),
            8,
            "cinq verticales et trois horizontales"
        );
    }

    /// Plafond de lignes de grille attendu par axe (audit n°4
    /// A4-01).
    const MAX_LIGNES_GRILLE: usize = 4096;

    /// Un pas négatif, nul ou non fini ne boucle pas : la grille
    /// reste vide (audit n°4 A4-01 — l'API publique accepte
    /// n'importe quel flottant).
    #[test]
    fn grille_refuse_pas_invalide() {
        let mut scene = Scene3D::vide();
        let boite = Boite3D::du_point(0.0, 0.0, 0.0).etendre(200.0, 100.0, 50.0);
        scene.ajouter_grille(&boite, -1.0, [1.0, 1.0, 1.0, 1.0]);
        assert!(scene.segments.is_empty(), "pas négatif : aucune ligne");
        scene.ajouter_grille(&boite, 0.0, [1.0, 1.0, 1.0, 1.0]);
        assert!(scene.segments.is_empty(), "pas nul : aucune ligne");
        scene.ajouter_grille(&boite, f64::NAN, [1.0, 1.0, 1.0, 1.0]);
        assert!(scene.segments.is_empty(), "pas non fini : aucune ligne");
    }

    /// Une boîte démesurée ne met pas la grille en boucle infinie :
    /// la grille doit être plafonnée par axe (audit n°4 A4-01).
    #[test]
    fn grille_bornee_zone_demensuree() {
        let mut scene = Scene3D::vide();
        let boite = Boite3D {
            min_x: 2.0e12,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 2.0e12 + 1.0e-3,
            max_y: 1.0e-3,
            max_z: 1.0,
        };
        scene.ajouter_grille(&boite, 1.0e-4, [1.0, 1.0, 1.0, 1.0]);
        assert!(
            scene.segments.len() <= 2 * MAX_LIGNES_GRILLE,
            "grille plafonnée à deux axes de {} lignes, ici {}",
            MAX_LIGNES_GRILLE,
            scene.segments.len(),
        );
    }
}
