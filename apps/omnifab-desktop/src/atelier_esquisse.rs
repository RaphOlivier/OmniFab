//! Mode esquisse du viewport 2D (ADR-0018, jalon 2 phase D, lot 3c) :
//! outils du croquis contraint, gestes multi-clics et entités
//! dessinées au painter.
//!
//! Les commandes du croquis (`AddEntity`, `AddConstraint`,
//! `MovePoint`) passent par la session : jamais de commande de
//! résolution, jamais de position résolue sérialisée (ADR-0018 D3).
//! L'affichage montre les positions résolues du cache quand il est
//! frais, les graines du journal sinon — la vérité affichable
//! (ADR-0018 D7). Le point visé s'accroche à la grille et aux
//! points existants, l'aperçu du geste n'écrit jamais au journal
//! (lot 3c-2). La sélection et le déplacement d'un point au
//! glissement complètent le lot 3c-3 : aperçu local pendant le
//! geste sans écrire au journal, commande MovePoint au relâchement
//! (ADR-0018 D4/D5).

use crate::DemoApp;
use crate::aide::avec_aide;
use eframe::egui;
use forge_render::{Camera2D, pas_de_grille};
use forge_session::{AddEntity, CroquisResolu, EntiteEsquisse, EsquisseProjection, MovePoint};
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
/// Couleur de l'aperçu du geste : la teinte des entités,
/// translucide pour se lire au-dessus du croquis posé —
/// `Color32::from_rgba_unmultiplied` n'est pas const (E0015) :
/// une fonction la construit à chaque usage.
fn couleur_apercu() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(255, 130, 30, 140)
}

/// Outil actif du mode esquisse : un clic du viewport 2D
/// s'interprète selon lui (ADR-0018, lot 3c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutilEsquisse {
    /// Sélection et caméra : un clic sélectionne le point visé,
    /// un glissement de poignée le déplace (ADR-0018 D4/D5).
    Selection,
    /// Point libre : un clic pose une entité à l'endroit visé.
    Point,
    /// Ligne : deux clics, l'origine puis l'extrémité.
    Ligne,
    /// Cercle : deux clics, le centre puis le rayon.
    Cercle,
    /// Arc : trois clics, le centre, le début puis la fin.
    Arc,
}

impl OutilEsquisse {
    /// Clé FTL de l'étiquette du bouton de l'outil.
    pub(crate) fn cle(&self) -> &'static str {
        match self {
            Self::Selection => "esquisse-outil-selection",
            Self::Point => "esquisse-outil-point",
            Self::Ligne => "esquisse-outil-ligne",
            Self::Cercle => "esquisse-outil-cercle",
            Self::Arc => "esquisse-outil-arc",
        }
    }

    /// `help_id` du bouton de l'outil (ADR-0010).
    pub(crate) fn aide(&self) -> &'static str {
        match self {
            Self::Selection => "aide-esquisse-outil-selection",
            Self::Point => "aide-esquisse-outil-point",
            Self::Ligne => "aide-esquisse-outil-ligne",
            Self::Cercle => "aide-esquisse-outil-cercle",
            Self::Arc => "aide-esquisse-outil-arc",
        }
    }
}

/// Geste en cours d'un outil de croquis multi-clics (ADR-0018,
/// lot 3c-2) : l'origine posée attend la suite. État de l'app
/// UI, jamais un événement — changer d'outil l'abandonne.
#[derive(Debug, Clone, Copy)]
pub(crate) enum GesteEsquisse {
    /// Ligne : origine posée, extrémité attendue.
    Ligne {
        /// Point d'origine déjà posé.
        origine: PointVise,
    },
    /// Cercle : centre posé, rayon attendu.
    Cercle {
        /// Point centre déjà posé.
        centre: PointVise,
    },
    /// Arc : centre posé, point de début attendu.
    ArcCentre {
        /// Point centre déjà posé.
        centre: PointVise,
    },
    /// Arc : début posé, point de fin attendu.
    ArcDebut {
        /// Point centre déjà posé.
        centre: PointVise,
        /// Point de début déjà posé.
        debut: PointVise,
    },
}

/// Point visé par un clic ou le curseur (ADR-0018, lot 3c-2) :
/// l'accroche colle à la grille et aux points existants, et
/// l'identifiant désigne le point repris ou le prochain posé.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PointVise {
    /// Identifiant du point : existant ou prévu par l'ajout.
    pub(crate) id: u32,
    /// Abscisse d'accroche.
    pub(crate) x: Length,
    /// Ordonnée d'accroche.
    pub(crate) y: Length,
}

/// Point visé à la position monde donnée (ADR-0018, lot 3c-2) :
/// le point existant le plus proche l'emporte sous le seuil d'un
/// quart de pas, la grille sinon — l'identifiant prévu est
/// `prochaine_id_entite()` ; le point exclu par le glissement
/// d'une poignée n'est pas candidat à l'accroche (lot 3c-3).
pub(crate) fn viser(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    pas: f64,
    x: f64,
    y: f64,
    exclu: Option<u32>,
) -> PointVise {
    let seuil = 0.25 * pas;
    let mut le_plus_proche: Option<PointVise> = None;
    let mut distance_min = seuil;
    for (id, _) in esquisse.entites() {
        // Le point exclu n'est pas candidat : le glissement d'une
        // poignée ne vise pas le point saisi lui-même (lot 3c-3).
        if exclu == Some(*id) {
            continue;
        }
        if let Some((px, py)) = position_point(esquisse, resolu, *id) {
            let distance = distance_mm((x, y), (px.millimeters(), py.millimeters()));
            if distance < distance_min {
                le_plus_proche = Some(PointVise {
                    id: *id,
                    x: px,
                    y: py,
                });
                distance_min = distance;
            }
        }
    }
    if let Some(vise) = le_plus_proche {
        return vise;
    }
    PointVise {
        id: esquisse.prochaine_id_entite(),
        x: Length::from_millimeters((x / pas).round() * pas),
        y: Length::from_millimeters((y / pas).round() * pas),
    }
}

/// Résolution fraîche du cache pour l'affichage ou l'accroche
/// (ADR-0018 D7) : celle de l'édition courante du croquis, aucune
/// sinon — les positions retombent sur les graines du journal.
fn resolu_frais(app: &DemoApp) -> Option<&CroquisResolu> {
    app.croquis_resolu
        .as_ref()
        .filter(|(edition, _)| *edition == app.session.esquisse().edition())
        .and_then(|(_, resultat)| resultat.as_ref().ok())
}

/// Traite un clic du viewport 2D selon l'outil actif et le geste
/// en cours (ADR-0018, lot 3c-2) : le clic vise la grille et les
/// points existants, les entités partent à la session par
/// `AddEntity` — jamais de commande de résolution, jamais de
/// position résolue sérialisée (ADR-0018 D3).
pub(crate) fn traiter_clic(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let resolu = resolu_frais(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, None);
    match (app.outil, app.geste.take()) {
        (OutilEsquisse::Selection, _) => {
            // Sélection au clic (ADR-0018 D5, lot 3c-3) : le point
            // visé devient la sélection — poignée au glissement ;
            // un clic ailleurs la vide.
            let existant = vise.id != app.session.esquisse().prochaine_id_entite();
            app.selection = existant.then_some(vise.id);
        }
        (OutilEsquisse::Point, _) => {
            point_du_clic(app, vise);
        }
        (OutilEsquisse::Ligne, None) => {
            point_du_clic(app, vise);
            app.geste = Some(GesteEsquisse::Ligne { origine: vise });
        }
        (OutilEsquisse::Ligne, Some(GesteEsquisse::Ligne { origine })) => {
            point_du_clic(app, vise);
            poser(
                app,
                EntiteEsquisse::Ligne {
                    p1: origine.id,
                    p2: vise.id,
                },
            );
        }
        (OutilEsquisse::Cercle, None) => {
            point_du_clic(app, vise);
            app.geste = Some(GesteEsquisse::Cercle { centre: vise });
        }
        (OutilEsquisse::Cercle, Some(GesteEsquisse::Cercle { centre })) => {
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            let vise_mm = (vise.x.millimeters(), vise.y.millimeters());
            let rayon = Length::from_millimeters(distance_mm(centre_mm, vise_mm));
            poser(
                app,
                EntiteEsquisse::Cercle {
                    centre: centre.id,
                    rayon,
                },
            );
        }
        (OutilEsquisse::Arc, None) => {
            point_du_clic(app, vise);
            app.geste = Some(GesteEsquisse::ArcCentre { centre: vise });
        }
        (OutilEsquisse::Arc, Some(GesteEsquisse::ArcCentre { centre })) => {
            point_du_clic(app, vise);
            app.geste = Some(GesteEsquisse::ArcDebut {
                centre,
                debut: vise,
            });
        }
        (OutilEsquisse::Arc, Some(GesteEsquisse::ArcDebut { centre, debut })) => {
            point_du_clic(app, vise);
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            let debut_mm = (debut.x.millimeters(), debut.y.millimeters());
            let fin_mm = (vise.x.millimeters(), vise.y.millimeters());
            let rayon = Length::from_millimeters(distance_mm(centre_mm, debut_mm));
            let angle_debut =
                Angle::from_radians((debut_mm.1 - centre_mm.1).atan2(debut_mm.0 - centre_mm.0));
            let angle_fin =
                Angle::from_radians((fin_mm.1 - centre_mm.1).atan2(fin_mm.0 - centre_mm.0));
            poser(
                app,
                EntiteEsquisse::Arc {
                    centre: centre.id,
                    debut: debut.id,
                    fin: vise.id,
                    rayon,
                    angle_debut,
                    angle_fin,
                },
            );
        }
        // Un geste résiduel d'un autre outil ne survit pas au
        // changement d'outil (le panneau l'abandonne) : ce clic
        // ne pose rien.
        (_, Some(_)) => {}
    }
}

/// Saisit la poignée sous le curseur au démarrage d'un glissement
/// (ADR-0018 D4/D5, lot 3c-3) : le point le plus proche sous le
/// seuil d'un quart de pas devient le point saisi et la sélection —
/// états de l'app UI, jamais des événements. Sans point visé, rien
/// ne se passe : le glissement déplace la vue.
pub(crate) fn saisir_poignee(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let resolu = resolu_frais(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, None);
    if vise.id != app.session.esquisse().prochaine_id_entite() {
        app.saisie = Some(vise.id);
        app.selection = Some(vise.id);
    }
}

/// Relâche le point saisi (ADR-0018 D4, lot 3c-3) : la position
/// visée — accrochée à la grille et aux points existants, le point
/// saisi exclu de l'accroche — devient sa graine par `MovePoint`,
/// la résolution repart au fil de travail. Un relâchement sans
/// changement effectif de la graine n'écrit rien au journal
/// (ADR-0011) ; sans point saisi, rien ne se passe.
pub(crate) fn relacher_poignee(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let Some(id) = app.saisie.take() else {
        return;
    };
    let resolu = resolu_frais(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, Some(id));
    if let Some((_, EntiteEsquisse::Point { x, y })) = app
        .session
        .esquisse()
        .entites()
        .iter()
        .find(|(i, _)| *i == id)
    {
        // La graine ne change pas : rien à écrire au journal.
        if x.approx_eq(vise.x) && y.approx_eq(vise.y) {
            return;
        }
    }
    let _ = app.executer(&MovePoint {
        point: id,
        x: vise.x,
        y: vise.y,
    });
}

/// Surcharge d'affichage du point saisi pendant le glissement
/// (ADR-0018 D4, lot 3c-3) : le point se dessine à la position
/// visée du curseur, les entités liées suivent — état dérivé
/// d'affichage, jamais écrit au journal. Sans point saisi, `None`.
pub(crate) fn surcharge_du_drag(
    app: &DemoApp,
    pas: f64,
    x: f64,
    y: f64,
) -> Option<(u32, Length, Length)> {
    let id = app.saisie?;
    let resolu = resolu_frais(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, Some(id));
    Some((id, vise.x, vise.y))
}

/// Pose le point visé s'il n'existe pas encore : l'identifiant
/// prévu est `prochaine_id_entite()` avant l'ajout, celui du
/// point repris sinon (ADR-0018, lot 3c-2).
fn point_du_clic(app: &mut DemoApp, vise: PointVise) {
    if vise.id == app.session.esquisse().prochaine_id_entite() {
        poser(
            app,
            EntiteEsquisse::Point {
                x: vise.x,
                y: vise.y,
            },
        );
    }
}

/// Pose une entité du croquis par la commande de session : tout
/// clic d'outil écrit au journal, jamais au croquis directement
/// (ADR-0018 D3).
fn poser(app: &mut DemoApp, entite: EntiteEsquisse) {
    let _ = app.executer(&AddEntity { entite });
}

/// Distance monde entre deux positions (millimètres).
fn distance_mm(debut: (f64, f64), fin: (f64, f64)) -> f64 {
    ((fin.0 - debut.0).powi(2) + (fin.1 - debut.1).powi(2)).sqrt()
}

/// Panneau des outils du croquis, en vue 2D de l'atelier : chaque
/// bouton porte son aide au clic droit (ADR-0010, ADR-0018 lot 3c).
pub(crate) fn panneau(ui: &mut egui::Ui, app: &mut DemoApp) {
    let reponse = ui.heading(app.texte("esquisse-titre"));
    avec_aide(&reponse, "aide-esquisse-titre", &mut app.aide);
    ui.horizontal(|ui| {
        for outil in [
            OutilEsquisse::Selection,
            OutilEsquisse::Point,
            OutilEsquisse::Ligne,
            OutilEsquisse::Cercle,
            OutilEsquisse::Arc,
        ] {
            let libelle = app.texte(outil.cle());
            let reponse = ui.selectable_label(app.outil == outil, libelle);
            if reponse.clicked() {
                // Changer d'outil abandonne le geste et la saisie
                // de poignée en cours.
                app.outil = outil;
                app.geste = None;
                app.saisie = None;
            }
            avec_aide(&reponse, outil.aide(), &mut app.aide);
        }
    });
}

/// Dessine les entités du croquis dans le viewport 2D : positions
/// résolues du cache quand il est frais, graines du journal sinon
/// (ADR-0018 D7) — état dérivé d'affichage, jamais sérialisé.
/// L'outil Sélection montre les poignées des points et le marqueur
/// de la sélection (ADR-0018 D5) ; la surcharge du glissement
/// déplace localement le point saisi, sans écrire (D4).
pub(crate) fn dessiner(
    painter: &egui::Painter,
    app: &DemoApp,
    rect: egui::Rect,
    surcharge: Option<(u32, Length, Length)>,
) {
    let resolu = resolu_frais(app);
    let esquisse = app.session.esquisse();
    let camera = &app.camera;
    for (id, entite) in esquisse.entites() {
        match entite {
            EntiteEsquisse::Point { .. } => {
                if let Some((x, y)) = position_affichee(esquisse, resolu, surcharge, *id) {
                    let centre = point_ecran(camera, rect, x.millimeters(), y.millimeters());
                    painter.circle_filled(centre, RAYON_POINT_PX, COULEUR_ESQUISSE);
                }
            }
            EntiteEsquisse::Ligne { p1, p2 } => {
                let debut = position_affichee(esquisse, resolu, surcharge, *p1);
                let fin = position_affichee(esquisse, resolu, surcharge, *p2);
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
                if let Some((x, y)) = position_affichee(esquisse, resolu, surcharge, *centre) {
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
                if let Some((x, y)) = position_affichee(esquisse, resolu, surcharge, *centre) {
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
    // Poignées de l'outil Sélection (ADR-0018 D5, lot 3c-3) : un
    // anneau par point du croquis, un marqueur distinct sur le
    // point sélectionné — états dérivés d'affichage, jamais
    // sérialisés.
    if app.outil == OutilEsquisse::Selection {
        for (id, entite) in esquisse.entites() {
            let EntiteEsquisse::Point { .. } = entite else {
                continue;
            };
            if let Some((x, y)) = position_affichee(esquisse, resolu, surcharge, *id) {
                let centre = point_ecran(camera, rect, x.millimeters(), y.millimeters());
                painter.circle_stroke(
                    centre,
                    RAYON_POINT_PX + 1.5,
                    egui::Stroke::new(LARGEUR_ESQUISSE_PX, COULEUR_ESQUISSE),
                );
                if app.selection == Some(*id) {
                    painter.circle_stroke(
                        centre,
                        RAYON_POINT_PX + 3.0,
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
                    );
                }
            }
        }
    }
}

/// Aperçu du geste sous le curseur (ADR-0018, lot 3c-2) : le point
/// visé s'accroche à la grille et aux points existants, la forme
/// suit l'outil et le geste en cours — un état dérivé d'affichage,
/// jamais sérialisé, qui n'écrit jamais au journal.
pub(crate) fn dessiner_apercu(
    painter: &egui::Painter,
    app: &DemoApp,
    rect: egui::Rect,
    x: f64,
    y: f64,
) {
    let resolu = resolu_frais(app);
    let pas = pas_de_grille(app.camera.echelle, crate::GRILLE_CIBLE_PX);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, None);
    let camera = &app.camera;
    match app.geste.as_ref() {
        None => {}
        Some(GesteEsquisse::Ligne { origine }) => {
            painter.line_segment(
                [
                    point_ecran(
                        camera,
                        rect,
                        origine.x.millimeters(),
                        origine.y.millimeters(),
                    ),
                    point_ecran(camera, rect, vise.x.millimeters(), vise.y.millimeters()),
                ],
                egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
            );
        }
        Some(GesteEsquisse::Cercle { centre }) | Some(GesteEsquisse::ArcCentre { centre }) => {
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            let vise_mm = (vise.x.millimeters(), vise.y.millimeters());
            let rayon_mm = distance_mm(centre_mm, vise_mm);
            painter.circle_stroke(
                point_ecran(camera, rect, centre_mm.0, centre_mm.1),
                (rayon_mm * camera.echelle) as f32,
                egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
            );
        }
        Some(GesteEsquisse::ArcDebut { centre, debut }) => {
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            let debut_mm = (debut.x.millimeters(), debut.y.millimeters());
            let vise_mm = (vise.x.millimeters(), vise.y.millimeters());
            let rayon_mm = distance_mm(centre_mm, debut_mm);
            let angle_debut = (debut_mm.1 - centre_mm.1).atan2(debut_mm.0 - centre_mm.0);
            let angle_vise = (vise_mm.1 - centre_mm.1).atan2(vise_mm.0 - centre_mm.0);
            let mut points = Vec::with_capacity(SEGMENTS_ARC + 1);
            for angle in angles_arc(
                Angle::from_radians(angle_debut),
                Angle::from_radians(angle_vise),
                SEGMENTS_ARC,
            ) {
                points.push(point_ecran(
                    camera,
                    rect,
                    centre_mm.0 + rayon_mm * angle.cos(),
                    centre_mm.1 + rayon_mm * angle.sin(),
                ));
            }
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
            ));
        }
    }
    marqueur_apercu(painter, camera, rect, &vise);
}

/// Marqueur du point visé : un cercle vide là où le prochain clic
/// se posera, par-dessus les entités du croquis (ADR-0018,
/// lot 3c-2).
fn marqueur_apercu(painter: &egui::Painter, camera: &Camera2D, rect: egui::Rect, vise: &PointVise) {
    let centre = point_ecran(camera, rect, vise.x.millimeters(), vise.y.millimeters());
    painter.circle_stroke(
        centre,
        RAYON_POINT_PX + 1.5,
        egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
    );
}

/// Position d'affichage avec surcharge du glissement (ADR-0018
/// D4, lot 3c-3) : le point saisi s'affiche à la position visée,
/// les autres suivent [`position_point`].
fn position_affichee(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    surcharge: Option<(u32, Length, Length)>,
    id: u32,
) -> Option<(Length, Length)> {
    if let Some((id_saisi, x, y)) = surcharge
        && id_saisi == id
    {
        return Some((x, y));
    }
    position_point(esquisse, resolu, id)
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
