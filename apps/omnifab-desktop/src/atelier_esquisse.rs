//! Mode esquisse du viewport 2D (ADR-0018, jalon 2 phase D, lot 3c) :
//! outils du croquis contraint et entités dessinées au painter.
//!
//! Les commandes du croquis (`AddEntity`, `AddConstraint`,
//! `MovePoint`) passent par la session : jamais de commande de
//! résolution, jamais de position résolue sérialisée (ADR-0018 D3).
//! L'affichage montre les positions résolues du cache quand il est
//! frais, les graines du journal sinon — la vérité affichable
//! (ADR-0018 D7).

use crate::DemoApp;
use crate::aide::avec_aide;
use eframe::egui;
use forge_render::Camera2D;
use forge_session::{CroquisResolu, EntiteEsquisse, EsquisseProjection};
use forge_units::{Angle, Length};

/// Couleur des entités du croquis, distincte des tracés importés
/// (jaune) et du rectangle paramétrique (bleu, contour blanc).
const COULEUR_ESQUISSE: egui::Color32 = egui::Color32::from_rgb(255, 130, 30);
/// Largeur du trait des entités du croquis, en pixels logiques.
const LARGEUR_ESQUISSE_PX: f32 = 1.5;
/// Rayon des marqueurs de points, en pixels logiques.
const RAYON_POINT_PX: f32 = 3.0;
/// Segments de la discrétisation des arcs au painter.
const SEGMENTS_ARC: usize = 32;

/// Outil actif du mode esquisse : un clic du viewport 2D
/// s'interprète selon lui (ADR-0018, lot 3c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutilEsquisse {
    /// Sélection et caméra : un clic ne pose rien.
    Selection,
    /// Point libre : un clic pose une entité à l'endroit visé.
    Point,
}

impl OutilEsquisse {
    /// Clé FTL de l'étiquette du bouton de l'outil.
    pub(crate) fn cle(&self) -> &'static str {
        match self {
            Self::Selection => "esquisse-outil-selection",
            Self::Point => "esquisse-outil-point",
        }
    }

    /// `help_id` du bouton de l'outil (ADR-0010).
    pub(crate) fn aide(&self) -> &'static str {
        match self {
            Self::Selection => "aide-esquisse-outil-selection",
            Self::Point => "aide-esquisse-outil-point",
        }
    }

    /// Entité posée par un clic à la position monde donnée : `None`
    /// quand l'outil n'écrit rien (sélection, caméra).
    pub(crate) fn entite_du_clic(&self, x: Length, y: Length) -> Option<EntiteEsquisse> {
        match self {
            Self::Selection => None,
            Self::Point => Some(EntiteEsquisse::Point { x, y }),
        }
    }
}

/// Panneau des outils du croquis, en vue 2D de l'atelier : chaque
/// bouton porte son aide au clic droit (ADR-0010, ADR-0018 lot 3c).
pub(crate) fn panneau(ui: &mut egui::Ui, app: &mut DemoApp) {
    let reponse = ui.heading(app.texte("esquisse-titre"));
    avec_aide(&reponse, "aide-esquisse-titre", &mut app.aide);
    ui.horizontal(|ui| {
        for outil in [OutilEsquisse::Selection, OutilEsquisse::Point] {
            let libelle = app.texte(outil.cle());
            let reponse = ui.selectable_label(app.outil == outil, libelle);
            if reponse.clicked() {
                app.outil = outil;
            }
            avec_aide(&reponse, outil.aide(), &mut app.aide);
        }
    });
}

/// Dessine les entités du croquis dans le viewport 2D : positions
/// résolues du cache quand il est frais, graines du journal sinon
/// (ADR-0018 D7) — état dérivé d'affichage, jamais sérialisé.
pub(crate) fn dessiner(
    painter: &egui::Painter,
    camera: &Camera2D,
    rect: egui::Rect,
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
) {
    for (id, entite) in esquisse.entites() {
        match entite {
            EntiteEsquisse::Point { .. } => {
                if let Some((x, y)) = position_point(esquisse, resolu, *id) {
                    let centre = point_ecran(camera, rect, x.millimeters(), y.millimeters());
                    painter.circle_filled(centre, RAYON_POINT_PX, COULEUR_ESQUISSE);
                }
            }
            EntiteEsquisse::Ligne { p1, p2 } => {
                let debut = position_point(esquisse, resolu, *p1);
                let fin = position_point(esquisse, resolu, *p2);
                if let (Some((x1, y1)), Some((x2, y2))) = (debut, fin) {
                    painter.line_segment(
                        [
                            point_ecran(camera, rect, x1.millimeters(), y1.millimeters()),
                            point_ecran(camera, rect, x2.millimeters(), y2.millimeters()),
                        ],
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, COULEUR_ESQUISSE),
                    );
                }
            }
            EntiteEsquisse::Cercle { centre, rayon } => {
                if let Some((x, y)) = position_point(esquisse, resolu, *centre) {
                    let centre_ecran = point_ecran(camera, rect, x.millimeters(), y.millimeters());
                    let rayon_px = (rayon.millimeters() * camera.echelle) as f32;
                    painter.circle_stroke(
                        centre_ecran,
                        rayon_px,
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, COULEUR_ESQUISSE),
                    );
                }
            }
            EntiteEsquisse::Arc {
                centre,
                rayon,
                angle_debut,
                angle_fin,
                ..
            } => {
                if let Some((x, y)) = position_point(esquisse, resolu, *centre) {
                    let centre_mm = (x.millimeters(), y.millimeters());
                    let rayon_mm = rayon.millimeters();
                    let mut points = Vec::with_capacity(SEGMENTS_ARC + 1);
                    for angle in angles_arc(*angle_debut, *angle_fin, SEGMENTS_ARC) {
                        points.push(point_ecran(
                            camera,
                            rect,
                            centre_mm.0 + rayon_mm * angle.cos(),
                            centre_mm.1 + rayon_mm * angle.sin(),
                        ));
                    }
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, COULEUR_ESQUISSE),
                    ));
                }
            }
        }
    }
}

/// Position d'affichage d'un point du croquis : résolue si le cache
/// en tient une fraîche, graine du journal sinon (ADR-0018 D7).
pub(crate) fn position_point(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    id: u32,
) -> Option<(Length, Length)> {
    if let Some(position) = resolu.and_then(|croquis| croquis.position(id)) {
        return Some(position);
    }
    if let Some((_, EntiteEsquisse::Point { x, y })) =
        esquisse.entites().iter().find(|(i, _)| *i == id)
    {
        return Some((*x, *y));
    }
    None
}

/// Angles de la discrétisation d'un arc (radians), sens
/// trigonométrique : de l'angle de début à l'angle de fin, cercle
/// entier quand les bornes sont nulles ou inversées.
pub(crate) fn angles_arc(debut: Angle, fin: Angle, segments: usize) -> Vec<f64> {
    let (debut_rad, fin_rad) = if fin.radians() > debut.radians() {
        (debut.radians(), fin.radians())
    } else {
        (0.0, std::f64::consts::TAU)
    };
    let denominateur = segments.max(1) as f64;
    let mut angles = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let part = i as f64 / denominateur;
        angles.push(debut_rad + (fin_rad - debut_rad) * part);
    }
    angles
}

/// Projette un point du monde à l'écran : la caméra inverse l'axe Y
/// et donne un décalage par rapport au centre du viewport (ADR-0015).
fn point_ecran(camera: &Camera2D, rect: egui::Rect, x: f64, y: f64) -> egui::Pos2 {
    let (px, py) = camera.vers_ecran((x, y));
    egui::pos2(rect.center().x + px as f32, rect.center().y + py as f32)
}
