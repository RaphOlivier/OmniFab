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
//! (ADR-0018 D4/D5). Les contraintes du lot 3c-4 partent de la
//! sélection : le panneau affiche les degrés de liberté et le
//! diagnostic de redondance n'apparaît que stable sur deux
//! résolutions consécutives (ADR-0018 D5). La cote de distance
//! passe par le dialogue de cote (ADR-0019 §6) : valeur fixe ou
//! expression pilotée par un alias sous le préfixe réservé
//! « esquisse. », posée par la commande AddConstraint seulement
//! au clic sur « Poser ». Le refus d'orientation sur une ligne déjà
//! orientée (alignement FreeCAD, ADR-0021 D1 révisée), la liste des
//! contraintes posées avec surbrillance momentanée et l'ancrage
//! non saisissable complètent la reprise de validation (ADR-0021).

use crate::DemoApp;
use crate::aide::{avec_aide, avec_aide_sur_calque};
use eframe::egui;
use forge_i18n::Argument;
use forge_render::{Camera2D, pas_de_grille};
use forge_session::{
    AddConstraint, AddEntity, ContrainteEsquisse, CroquisResolu, EVENT_SKETCH_CONSTRAINT_ADDED,
    EVENT_SKETCH_CONSTRAINT_UPDATED, EntiteEsquisse, EsquisseProjection, GenreEntite, MovePoint,
    ValeurCote,
};
use forge_units::{Angle, Length};

/// Couleur des entités du croquis, distincte des tracés importés
/// (jaune) et du rectangle paramétrique (bleu, contour blanc).
const COULEUR_ESQUISSE: egui::Color32 = egui::Color32::from_rgb(255, 130, 30);
/// Largeur du trait des entités du croquis, en pixels logiques.
const LARGEUR_ESQUISSE_PX: f32 = 1.5;
/// Rayon des marqueurs de points, en pixels logiques.
const RAYON_POINT_PX: f32 = 3.0;
/// Couleur du liseré des points ancrés (ADR-0021 D3) : turquoise
/// distinct des entités (orange), de la sélection et de la grille.
const COULEUR_ANCRAGE: egui::Color32 = egui::Color32::from_rgb(64, 224, 208);
/// Durée de la surbrillance d'une contrainte posée dans la liste
/// (ADR-0021 D2), en secondes.
const DUREE_SURBRILLANCE_S: f64 = 1.5;
/// Segments de la discrétisation des arcs au painter.
const SEGMENTS_ARC: usize = 32;
/// Taille maximale de la sélection (ADR-0018 D5, lot 3c-4) : les
/// contraintes relient au plus deux entités — la plus ancienne
/// cède sa place à la nouvelle.
const SELECTION_MAX: usize = 2;
/// Dialogue de cote de distance (ADR-0019 §6) : valeur fixe ou
/// expression pilotée par alias — état de l'app UI, jamais un
/// événement tant que « Poser » n'est pas cliqué.
pub(crate) struct DialogueCote {
    /// Premier point de la cote.
    pub(crate) p1: u32,
    /// Second point de la cote.
    pub(crate) p2: u32,
    /// `true` : valeur fixe ; `false` : expression pilotée.
    pub(crate) fixe: bool,
    /// Valeur fixe, en millimètres.
    pub(crate) mm: f64,
    /// Texte de l'expression pilotée.
    pub(crate) texte: String,
    /// Alias de la cote : suffixe sous le préfixe réservé
    /// `esquisse.`.
    pub(crate) alias: String,
}

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
    let cle = (app.session.esquisse().edition(), app.generation_liaisons);
    app.croquis_resolu
        .as_ref()
        .filter(|(cle_cache, _)| *cle_cache == cle)
        .and_then(|(_, resultat)| resultat.as_ref().ok())
}

/// Résolution affichable (retour de la re-validation §7) : celle de
/// l'édition courante si le cache la tient, sinon la dernière
/// résolution connue — même dans la trame du clic, avant que la
/// demande de résolution ne soit partie. Le croquis contraint ne
/// « saute » jamais aux graines pendant le recalcul : la déformation
/// au moment du clic disparaît. Les visées et accroches suivent
/// l'affichage (on clique là où le croquis est DESSINÉ) ; seuls les
/// degrés de liberté du panneau restent sur la résolution fraîche.
pub(crate) fn resolu_affichage(app: &DemoApp) -> Option<&CroquisResolu> {
    let frais = resolu_frais(app);
    if frais.is_some() {
        return frais;
    }
    app.croquis_resolu
        .as_ref()
        .and_then(|(_, resultat)| resultat.as_ref().ok())
}

/// Traite un clic du viewport 2D selon l'outil actif et le geste
/// en cours (ADR-0018, lot 3c-2) : le clic vise la grille et les
/// points existants, les entités partent à la session par
/// `AddEntity` — jamais de commande de résolution, jamais de
/// position résolue sérialisée (ADR-0018 D3).
pub(crate) fn traiter_clic(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let resolu = resolu_affichage(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, None);
    match (app.outil, app.geste.take()) {
        (OutilEsquisse::Selection, _) => {
            // Sélection au clic (ADR-0018 D5, lot 3c-3/3c-4) : le
            // point visé prime, la ligne sinon, la courbe enfin
            // (ADR-0020) ; un clic sur une entité déjà sélectionnée
            // la retire, un clic ailleurs vide la sélection. Au plus
            // deux entités restent sélectionnées : les contraintes
            // du lot 3c-4 relient au plus deux entités.
            let cible = if vise.id != app.session.esquisse().prochaine_id_entite() {
                Some(vise.id)
            } else {
                let resolu = resolu_affichage(app);
                ligne_visee(app.session.esquisse(), resolu, pas, x, y)
                    .or_else(|| courbe_visee(app.session.esquisse(), resolu, pas, x, y))
            };
            match cible {
                Some(id) => {
                    if let Some(position) = app.selection.iter().position(|&saisi| saisi == id) {
                        app.selection.remove(position);
                    } else {
                        app.selection.push(id);
                        if app.selection.len() > SELECTION_MAX {
                            app.selection.remove(0);
                        }
                    }
                }
                None => app.selection.clear(),
            }
            // La cote de distance suit la sélection : deux points
            // sélectionnés démarrent à leur distance affichable
            // courante (graine ou position résolue).
            if let Some((p1, p2)) = deux_points_selectionnes(app) {
                let resolu = resolu_affichage(app);
                if let (Some((x1, y1)), Some((x2, y2))) = (
                    position_point(app.session.esquisse(), resolu, p1),
                    position_point(app.session.esquisse(), resolu, p2),
                ) {
                    app.cote_distance = distance_mm(
                        (x1.millimeters(), y1.millimeters()),
                        (x2.millimeters(), y2.millimeters()),
                    );
                }
            }
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
            // Le balayage signé part de zéro au point de départ : le
            // sens suivra le déplacement du curseur autour du centre
            // (ADR-0020).
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            app.balayage_arc = 0.0;
            app.angle_curseur_arc =
                (vise.y.millimeters() - centre_mm.1).atan2(vise.x.millimeters() - centre_mm.0);
            app.geste = Some(GesteEsquisse::ArcDebut {
                centre,
                debut: vise,
            });
        }
        (OutilEsquisse::Arc, Some(GesteEsquisse::ArcDebut { centre, debut })) => {
            point_du_clic(app, vise);
            // Le clic présent participe au balayage (ADR-0020) : le
            // geste est déjà pris par le match, le delta direct
            // depuis le dernier angle du curseur donne le sens du
            // clic — positif trigonométrique, négatif horaire.
            let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
            let angle = (y - centre_mm.1).atan2(x - centre_mm.0);
            app.balayage_arc = balaie(app.balayage_arc, app.angle_curseur_arc, angle);
            let debut_mm = (debut.x.millimeters(), debut.y.millimeters());
            let rayon = Length::from_millimeters(distance_mm(centre_mm, debut_mm));
            let angle_debut =
                Angle::from_radians((debut_mm.1 - centre_mm.1).atan2(debut_mm.0 - centre_mm.0));
            // Balayage signé (ADR-0020) : positif dans le sens
            // trigonométrique, négatif dans le sens horaire ;
            // confondu avec le départ, le tour entier.
            let mut balayage = app.balayage_arc;
            if balayage.abs() < 1e-9 {
                balayage = std::f64::consts::TAU;
            }
            poser(
                app,
                EntiteEsquisse::Arc {
                    centre: centre.id,
                    debut: debut.id,
                    fin: vise.id,
                    rayon,
                    angle_debut,
                    angle_fin: Angle::from_radians(angle_debut.radians() + balayage),
                },
            );
        }
        // Un geste résiduel d'un autre outil ne survit pas au
        // changement d'outil (le panneau l'abandonne) : ce clic
        // ne pose rien.
        (_, Some(_)) => {}
    }
}

/// Suit le déplacement du curseur autour du centre pendant le
/// geste de l'arc (ADR-0020) : le balayage signé s'accumule au fil
/// du mouvement, le sens de l'arc suit le geste au lieu d'être figé
/// dans le sens trigonométrique — état dérivé de l'interface, jamais
/// sérialisé.
pub(crate) fn suivre_geste_arc(app: &mut DemoApp, x: f64, y: f64) {
    let Some(GesteEsquisse::ArcDebut { centre, .. }) = app.geste.as_ref() else {
        return;
    };
    let centre_mm = (centre.x.millimeters(), centre.y.millimeters());
    let angle = (y - centre_mm.1).atan2(x - centre_mm.0);
    app.balayage_arc = balaie(app.balayage_arc, app.angle_curseur_arc, angle);
    app.angle_curseur_arc = angle;
}

/// Accumule le balayage signé de l'arc (ADR-0020) : le delta
/// d'angle autour du centre se replie dans ]-π, π] — positif dans
/// le sens trigonométrique, négatif dans le sens horaire — et le
/// balayage total reste borné à un tour.
fn balaie(balayage: f64, precedent: f64, angle: f64) -> f64 {
    let mut delta = (angle - precedent).rem_euclid(std::f64::consts::TAU);
    if delta > std::f64::consts::PI {
        delta -= std::f64::consts::TAU;
    }
    let borne = std::f64::consts::TAU;
    (balayage + delta).clamp(-borne, borne)
}

/// Saisit la poignée sous le curseur au démarrage d'un glissement
/// (ADR-0018 D4/D5, lot 3c-3) : le point le plus proche sous le
/// seuil d'un quart de pas devient le point saisi et la sélection —
/// états de l'app UI, jamais des événements. Sans point visé, rien
/// ne se passe : le glissement déplace la vue.
pub(crate) fn saisir_poignee(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let resolu = resolu_affichage(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, None);
    // Un point ancré ne se saisit pas (ADR-0021 D3) : le grab est
    // refusé — pas de glissement fantôme qui suivrait le curseur
    // avant d'être repoussé au relâchement ; le clic seul reste
    // libre de le sélectionner.
    if vise.id != app.session.esquisse().prochaine_id_entite()
        && !app.session.esquisse().point_ancre(vise.id)
    {
        app.saisie = Some(vise.id);
        app.selection = vec![vise.id];
    }
}

/// Relâche le point saisi (ADR-0018 D4, lot 3c-3) : la position
/// visée — accrochée à la grille et aux points existants, le point
/// saisi exclu de l'accroche — devient sa graine par `MovePoint`,
/// la résolution repart au fil de travail. Un relâchement sans
/// changement effectif de la graine n'écrit rien au journal
/// (ADR-0011) ; sans point saisi, rien ne se passe. Le point lâché
/// reste affiché à sa position visée pendant le vol de la
/// résolution (anti-flash du relâchement, re-validation §7).
pub(crate) fn relacher_poignee(app: &mut DemoApp, pas: f64, x: f64, y: f64) {
    let Some(id) = app.saisie.take() else {
        return;
    };
    let resolu = resolu_affichage(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, Some(id));
    // La graine ne change pas : rien à écrire au journal
    // (ADR-0011) — la comparaison porte sur la graine courante,
    // pas sur les coordonnées d'origine de l'entité.
    if let Some((x, y)) = app.session.esquisse().position_graine(id)
        && x.approx_eq(vise.x)
        && y.approx_eq(vise.y)
    {
        return;
    }
    let edition_avant = app.session.esquisse().edition();
    let _ = app.executer(&MovePoint {
        point: id,
        x: vise.x,
        y: vise.y,
    });
    // Anti-flash du relâchement (re-validation §7) : le point lâché
    // reste affiché à sa position visée pendant le vol de la
    // résolution — l'affichage ne retombe pas sur la résolution
    // d'avant le glissement.
    if app.session.esquisse().edition() != edition_avant {
        let edition = app.session.esquisse().edition();
        app.surcharge_relachee = Some((edition, id, vise.x, vise.y));
    }
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
    let resolu = resolu_affichage(app);
    let vise = viser(app.session.esquisse(), resolu, pas, x, y, Some(id));
    Some((id, vise.x, vise.y))
}

/// Surcharge d'affichage du point relâché pendant le vol de sa
/// résolution (anti-flash du relâchement, re-validation §7) : le
/// relâchement pose la graine par `MovePoint` et la résolution
/// repart au fil de travail — sans elle, l'affichage retomberait
/// sur la dernière résolution connue, celle d'AVANT le glissement,
/// et la forme bondirait en arrière le temps du vol. Le point
/// lâché reste dessiné à sa position visée jusqu'à l'arrivée du
/// frais ; toute autre écriture au journal la rend caduque. `None`
/// sans relâchement en vol.
pub(crate) fn surcharge_du_relachement(app: &DemoApp) -> Option<(u32, Length, Length)> {
    let (edition_relachee, id, x, y) = app.surcharge_relachee?;
    if app.session.esquisse().edition() != edition_relachee {
        return None;
    }
    if resolu_frais(app).is_some() {
        return None;
    }
    Some((id, x, y))
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
    // Contraintes du croquis (ADR-0018 D5, lot 3c-4) : la sélection
    // courante active les boutons — une ligne pour Horizontale et
    // Verticale, un point pour l'ancrage, deux points pour la cote
    // de distance. Un refus de la commande n'écrit rien au journal
    // et le message d'état reste fidèle (ADR-0016 C4).
    {
        let reponse = ui.heading(app.texte("esquisse-contraintes"));
        avec_aide(&reponse, "aide-esquisse-contraintes", &mut app.aide);
    }
    {
        let reponse = ui.add_enabled(
            ligne_selectionnee(app).is_some(),
            egui::Button::new(app.texte("esquisse-contrainte-horizontale")),
        );
        if reponse.clicked()
            && let Some(ligne) = ligne_selectionnee(app)
        {
            ajouter_contrainte(app, ContrainteEsquisse::Horizontale { ligne });
        }
        avec_aide_sur_calque(
            ui,
            &reponse,
            "aide-esquisse-contrainte-horizontale",
            &mut app.aide,
        );
    }
    {
        let reponse = ui.add_enabled(
            ligne_selectionnee(app).is_some(),
            egui::Button::new(app.texte("esquisse-contrainte-verticale")),
        );
        if reponse.clicked()
            && let Some(ligne) = ligne_selectionnee(app)
        {
            ajouter_contrainte(app, ContrainteEsquisse::Verticale { ligne });
        }
        avec_aide_sur_calque(
            ui,
            &reponse,
            "aide-esquisse-contrainte-verticale",
            &mut app.aide,
        );
    }
    {
        let reponse = ui.add_enabled(
            point_selectionne(app).is_some(),
            egui::Button::new(app.texte("esquisse-contrainte-point-fixe")),
        );
        if reponse.clicked()
            && let Some(point) = point_selectionne(app)
        {
            ajouter_contrainte(app, ContrainteEsquisse::PointFixe { point });
        }
        avec_aide_sur_calque(
            ui,
            &reponse,
            "aide-esquisse-contrainte-point-fixe",
            &mut app.aide,
        );
    }
    {
        let reponse = ui.add_enabled(
            deux_points_selectionnes(app).is_some(),
            egui::Button::new(app.texte("esquisse-contrainte-distance")),
        );
        if reponse.clicked()
            && let Some((p1, p2)) = deux_points_selectionnes(app)
        {
            // Dialogue de cote (ADR-0019 §6) : valeur fixe ou
            // expression pilotée, l'alias vit sous le préfixe
            // réservé « esquisse. » ; la valeur de départ suit la
            // sélection (traiter_clic).
            let cotes = app
                .session
                .graph()
                .names()
                .iter()
                .filter(|nom| nom.starts_with("esquisse."))
                .count();
            app.dialogue_cote = Some(DialogueCote {
                p1,
                p2,
                fixe: true,
                mm: app.cote_distance,
                texte: String::new(),
                alias: format!("cote-{cotes}"),
            });
        }
        avec_aide_sur_calque(
            ui,
            &reponse,
            "aide-esquisse-contrainte-distance",
            &mut app.aide,
        );
    }
    // Contraintes posées (ADR-0021 D2) : la liste montre chaque
    // contrainte du croquis dans l'ordre de pose — la dernière
    // touchée se surligne un court instant, le temps de voir où
    // elle est tombée. Sans contrainte, la section disparaît. Un
    // état dérivé d'affichage, jamais sérialisé.
    if !app.session.esquisse().contraintes().is_empty() {
        let poses: Vec<(u32, String)> = app
            .session
            .esquisse()
            .contraintes()
            .iter()
            .map(|(id, contrainte)| (*id, libelle_contrainte(app, contrainte)))
            .collect();
        let reponse = ui.heading(app.texte("esquisse-contraintes-poses"));
        avec_aide_sur_calque(
            ui,
            &reponse,
            "aide-esquisse-contraintes-poses",
            &mut app.aide,
        );
        for (id, libelle) in poses {
            let texte = if est_surlignee(app, id) {
                egui::RichText::new(libelle)
                    .color(couleur_apercu())
                    .strong()
            } else {
                egui::RichText::new(libelle).weak()
            };
            ui.label(texte);
        }
        // La surbrillance s'estompe au fil des trames : redemande
        // le rafraîchissement tant qu'elle vit.
        if surbrillance_vivante(app) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    // Degrés de liberté (ADR-0018 D5) : lus sur le cache résolu
    // frais — rien pendant une résolution en vol, la valeur
    // affichée reste celle d'un croquis cohérent avec le journal.
    if let Some(croquis) = resolu_frais(app) {
        ui.label(app.texte_avec("esquisse-dof", &[("n", Argument::from(croquis.dof()))]));
    }
    // Redondances (ADR-0018 D5) : l'hystérésis ne montre le
    // diagnostic que stable sur deux résolutions consécutives —
    // jamais de faux conflit clignotant (FPA #106).
    if let Some(redondances) = app.redondances_affichees.as_ref().filter(|r| !r.is_empty()) {
        ui.label(app.texte_avec(
            "esquisse-redondances",
            &[("n", Argument::from(redondances.len()))],
        ));
    }
}

/// Identifiants de la sélection du genre demandé, dans l'ordre de
/// sélection (ADR-0018 D5, lot 3c-4).
fn selection_du_genre<'a>(app: &'a DemoApp, genre: GenreEntite) -> impl Iterator<Item = u32> + 'a {
    app.selection
        .iter()
        .filter(move |id| app.session.esquisse().genre_entite(**id) == Some(genre))
        .copied()
}

/// La seule ligne de la sélection (ADR-0018 D5, lot 3c-4) : les
/// contraintes Horizontale et Verticale portent sur une ligne.
/// `None` sans ligne sélectionnée, ou avec plusieurs.
fn ligne_selectionnee(app: &DemoApp) -> Option<u32> {
    let mut lignes = selection_du_genre(app, GenreEntite::Ligne);
    let ligne = lignes.next()?;
    lignes.next().is_none().then_some(ligne)
}

/// Le seul point de la sélection (ADR-0018 D5, lot 3c-4) : `None`
/// sans point sélectionné, ou avec plusieurs.
fn point_selectionne(app: &DemoApp) -> Option<u32> {
    let mut points = selection_du_genre(app, GenreEntite::Point);
    let point = points.next()?;
    points.next().is_none().then_some(point)
}

/// Les deux points de la sélection, dans l'ordre (ADR-0018 D5,
/// lot 3c-4) : `None` sans eux, ou en nombre différent de deux.
fn deux_points_selectionnes(app: &DemoApp) -> Option<(u32, u32)> {
    let mut points = selection_du_genre(app, GenreEntite::Point);
    let p1 = points.next()?;
    let p2 = points.next()?;
    points.next().is_none().then_some((p1, p2))
}

/// Ajoute une contrainte au croquis (ADR-0018 D2, lot 3c-4) : la
/// commande `AddConstraint` passe par la session — un refus n'écrit
/// rien au journal et le message d'état reste fidèle (ADR-0016 C4).
/// Une pose effective marque la contrainte touchée pour la
/// surbrillance momentanée de la liste (ADR-0021 D2) : l'identifiant
/// vient du dernier événement du journal — une pose neuve.
pub(crate) fn ajouter_contrainte(app: &mut DemoApp, contrainte: ContrainteEsquisse) {
    if app.executer(&AddConstraint { contrainte }).is_err() {
        return;
    }
    let Some(evenement) = app.session.document().events().last() else {
        return;
    };
    let contrainte_posee = evenement.event_type == EVENT_SKETCH_CONSTRAINT_ADDED
        || evenement.event_type == EVENT_SKETCH_CONSTRAINT_UPDATED;
    let Some(id) = evenement.data.get("id").and_then(serde_json::Value::as_u64) else {
        return;
    };
    if contrainte_posee {
        app.derniere_contrainte = Some((id as u32, app.instant));
    }
}

/// La contrainte est-elle celle posée en dernier, encore dans sa
/// fenêtre de surbrillance (ADR-0021 D2) ?
fn est_surlignee(app: &DemoApp, id: u32) -> bool {
    if let Some((id_posee, _)) = app.derniere_contrainte {
        id_posee == id && surbrillance_vivante(app)
    } else {
        false
    }
}

/// Une contrainte posée récemment est-elle encore dans sa fenêtre
/// de surbrillance (ADR-0021 D2) ?
fn surbrillance_vivante(app: &DemoApp) -> bool {
    if let Some((_, instant)) = app.derniere_contrainte {
        app.instant - instant < DUREE_SURBRILLANCE_S
    } else {
        false
    }
}

/// Libellé d'une contrainte de la liste des posées (ADR-0021 D2) :
/// genre traduit, cotes en millimètres et angle en degrés, entités
/// désignées par leurs identifiants — identifiant à identifiant, le
/// croquis reste relisible.
fn libelle_contrainte(app: &DemoApp, contrainte: &ContrainteEsquisse) -> String {
    match contrainte {
        ContrainteEsquisse::Coincidence { p1, p2 } => app.texte_avec(
            "contrainte-libelle-coincidence",
            &[
                ("p1", Argument::from(i64::from(*p1))),
                ("p2", Argument::from(i64::from(*p2))),
            ],
        ),
        ContrainteEsquisse::Horizontale { ligne } => app.texte_avec(
            "contrainte-libelle-horizontale",
            &[("ligne", Argument::from(i64::from(*ligne)))],
        ),
        ContrainteEsquisse::Verticale { ligne } => app.texte_avec(
            "contrainte-libelle-verticale",
            &[("ligne", Argument::from(i64::from(*ligne)))],
        ),
        ContrainteEsquisse::Distance { p1, p2, valeur } => match valeur {
            ValeurCote::Fixe { mm } => app.texte_avec(
                "contrainte-libelle-distance-fixe",
                &[
                    ("p1", Argument::from(i64::from(*p1))),
                    ("p2", Argument::from(i64::from(*p2))),
                    ("mm", Argument::from(format!("{:.1}", mm.millimeters()))),
                ],
            ),
            ValeurCote::Expression { alias, .. } => app.texte_avec(
                "contrainte-libelle-distance-expression",
                &[
                    ("p1", Argument::from(i64::from(*p1))),
                    ("p2", Argument::from(i64::from(*p2))),
                    ("alias", Argument::from(alias.as_str())),
                ],
            ),
        },
        ContrainteEsquisse::Angle {
            ligne1,
            ligne2,
            valeur,
        } => app.texte_avec(
            "contrainte-libelle-angle",
            &[
                ("l1", Argument::from(i64::from(*ligne1))),
                ("l2", Argument::from(i64::from(*ligne2))),
                (
                    "deg",
                    Argument::from(format!("{:.1}", valeur.radians().to_degrees())),
                ),
            ],
        ),
        ContrainteEsquisse::MemeLongueur { ligne1, ligne2 } => app.texte_avec(
            "contrainte-libelle-meme-longueur",
            &[
                ("l1", Argument::from(i64::from(*ligne1))),
                ("l2", Argument::from(i64::from(*ligne2))),
            ],
        ),
        ContrainteEsquisse::PointSurLigne { point, ligne } => app.texte_avec(
            "contrainte-libelle-point-sur-ligne",
            &[
                ("p", Argument::from(i64::from(*point))),
                ("l", Argument::from(i64::from(*ligne))),
            ],
        ),
        ContrainteEsquisse::Paralleles { ligne1, ligne2 } => app.texte_avec(
            "contrainte-libelle-paralleles",
            &[
                ("l1", Argument::from(i64::from(*ligne1))),
                ("l2", Argument::from(i64::from(*ligne2))),
            ],
        ),
        ContrainteEsquisse::Perpendiculaires { ligne1, ligne2 } => app.texte_avec(
            "contrainte-libelle-perpendiculaires",
            &[
                ("l1", Argument::from(i64::from(*ligne1))),
                ("l2", Argument::from(i64::from(*ligne2))),
            ],
        ),
        ContrainteEsquisse::RayonCercle { cercle, valeur } => app.texte_avec(
            "contrainte-libelle-rayon-cercle",
            &[
                ("c", Argument::from(i64::from(*cercle))),
                ("mm", Argument::from(format!("{:.1}", valeur.millimeters()))),
            ],
        ),
        ContrainteEsquisse::PointSurCercle { point, cercle } => app.texte_avec(
            "contrainte-libelle-point-sur-cercle",
            &[
                ("p", Argument::from(i64::from(*point))),
                ("c", Argument::from(i64::from(*cercle))),
            ],
        ),
        ContrainteEsquisse::PointFixe { point } => app.texte_avec(
            "contrainte-libelle-point-fixe",
            &[("p", Argument::from(i64::from(*point)))],
        ),
    }
}

/// Fenêtre du dialogue de cote (ADR-0019 §6) : valeur fixe ou
/// expression pilotée par un alias sous le préfixe réservé
/// « esquisse. ». La pose passe par `AddConstraint` ; un refus
/// laisse le dialogue ouvert, la saisie reste corrigeable
/// (ADR-0016 C4). État de l'app UI tant que « Poser » n'est pas
/// cliqué : jamais un événement avant.
pub(crate) fn fenetre_cote(ctx: &egui::Context, app: &mut DemoApp) {
    if let Some(mut dialogue) = app.dialogue_cote.take() {
        let mut visible = true;
        let mut poser = false;
        let mut fermer = false;
        // Textes traduits avant la fenêtre : la pose n'emprunte
        // plus l'app dans les closures (ADR-0010).
        let titre = app.texte("cote-titre");
        let libelle_fixe = app.texte("cote-fixe");
        let libelle_expression = app.texte("cote-expression");
        let libelle_texte = app.texte("cote-texte");
        let libelle_alias =
            app.texte_avec("cote-alias", &[("prefixe", Argument::from("esquisse."))]);
        let libelle_poser = app.texte("cote-poser");
        let libelle_annuler = app.texte("cote-annuler");
        let demande = &mut app.aide;
        // Première trame de l'ouverture (ADR-0020) : la fenêtre se
        // pose près du curseur, qui vient de cliquer le bouton
        // Distance — la validation jalon 2 la voyait au centre.
        let premiere = ctx.data_mut(|d| {
            let deja = d
                .get_temp::<bool>(egui::Id::new("cote-ouverte"))
                .unwrap_or(false);
            d.insert_temp(egui::Id::new("cote-ouverte"), true);
            !deja
        });
        let pos = ctx
            .pointer_latest_pos()
            .map(|pos| pos + egui::vec2(16.0, 16.0));
        let mut fenetre = egui::Window::new(titre)
            .open(&mut visible)
            .default_width(280.0);
        if premiere && let Some(pos) = pos {
            fenetre = fenetre.current_pos(pos);
        }
        fenetre.show(ctx, |ui| {
            let reponse = ui.radio_value(&mut dialogue.fixe, true, libelle_fixe.as_str());
            avec_aide(&reponse, "aide-cote-fixe", demande);
            let reponse = ui.radio_value(&mut dialogue.fixe, false, libelle_expression.as_str());
            avec_aide(&reponse, "aide-cote-expression", demande);
            if dialogue.fixe {
                let reponse = ui.add(
                    egui::DragValue::new(&mut dialogue.mm)
                        .speed(1.0)
                        .suffix(" mm"),
                );
                avec_aide(&reponse, "aide-cote-mm", demande);
            } else {
                ui.label(&libelle_texte);
                let reponse = ui.text_edit_singleline(&mut dialogue.texte);
                avec_aide(&reponse, "aide-cote-texte", demande);
            }
            ui.label(&libelle_alias);
            let reponse = ui.text_edit_singleline(&mut dialogue.alias);
            avec_aide(&reponse, "aide-cote-alias", demande);
            let texte_pret = !dialogue.texte.trim().is_empty();
            let alias_pret = !dialogue.alias.trim().is_empty();
            let pose_possible = dialogue.fixe || (texte_pret && alias_pret);
            ui.horizontal(|ui| {
                let reponse =
                    ui.add_enabled(pose_possible, egui::Button::new(libelle_poser.as_str()));
                avec_aide(&reponse, "aide-cote-poser", demande);
                if reponse.clicked() {
                    poser = true;
                }
                let reponse = ui.button(&libelle_annuler);
                avec_aide(&reponse, "aide-cote-annuler", demande);
                if reponse.clicked() {
                    fermer = true;
                }
            });
        });
        if visible && !fermer && poser {
            let valeur = if dialogue.fixe {
                ValeurCote::Fixe {
                    mm: Length::from_millimeters(dialogue.mm),
                }
            } else {
                ValeurCote::Expression {
                    texte: dialogue.texte.clone(),
                    alias: format!("esquisse.{}", dialogue.alias.trim()),
                }
            };
            ajouter_contrainte(
                app,
                ContrainteEsquisse::Distance {
                    p1: dialogue.p1,
                    p2: dialogue.p2,
                    valeur,
                },
            );
            // Un refus laisse le dialogue ouvert : la saisie reste
            // corrigeable (ADR-0016 C4).
            if app.error.is_some() {
                app.dialogue_cote = Some(dialogue);
            }
        } else if visible && !fermer {
            app.dialogue_cote = Some(dialogue);
        }
    } else {
        // Fermé : la prochaine ouverture compte pour neuve.
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("cote-ouverte"), false));
    }
}

/// Ligne du croquis visée par un clic (ADR-0018 D5, lot 3c-4) : la
/// plus proche sous le seuil d'un quart de pas, distance portée au
/// segment affiché — positions résolues du cache frais, graines
/// sinon. `None` sans ligne à portée.
fn ligne_visee(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    pas: f64,
    x: f64,
    y: f64,
) -> Option<u32> {
    esquisse
        .entites()
        .iter()
        .filter_map(|(id, entite)| match entite {
            EntiteEsquisse::Ligne { p1, p2 } => Some((*id, *p1, *p2)),
            _ => None,
        })
        .filter_map(|(id, p1, p2)| {
            let debut = position_affichee(esquisse, resolu, None, p1)?;
            let fin = position_affichee(esquisse, resolu, None, p2)?;
            let distance = distance_segment_mm(
                (debut.0.millimeters(), debut.1.millimeters()),
                (fin.0.millimeters(), fin.1.millimeters()),
                (x, y),
            );
            (distance <= 0.25 * pas).then_some((id, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(id, _)| id)
}

/// Cercle ou arc visé par un clic (ADR-0020, validation jalon 2) :
/// la sélection atteint aussi les courbes, pas seulement les points
/// et les lignes — la plus proche sous le seuil d'un quart de pas,
/// distance portée à la courbe.
fn courbe_visee(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    pas: f64,
    x: f64,
    y: f64,
) -> Option<u32> {
    esquisse
        .entites()
        .iter()
        .filter_map(|(id, entite)| match entite {
            EntiteEsquisse::Cercle { centre, rayon } => {
                let (cx, cy) = position_affichee(esquisse, resolu, None, *centre)?;
                let ecart = (distance_mm((cx.millimeters(), cy.millimeters()), (x, y))
                    - rayon.millimeters())
                .abs();
                Some((*id, ecart))
            }
            EntiteEsquisse::Arc {
                centre,
                rayon,
                angle_debut,
                angle_fin,
                ..
            } => {
                let (cx, cy) = position_affichee(esquisse, resolu, None, *centre)?;
                let centre_mm = (cx.millimeters(), cy.millimeters());
                let rayon_mm = rayon.millimeters();
                let points: Vec<(f64, f64)> = angles_arc(*angle_debut, *angle_fin, SEGMENTS_ARC)
                    .into_iter()
                    .map(|angle| {
                        (
                            centre_mm.0 + rayon_mm * angle.cos(),
                            centre_mm.1 + rayon_mm * angle.sin(),
                        )
                    })
                    .collect();
                let ecart = points
                    .windows(2)
                    .map(|paire| distance_segment_mm(paire[0], paire[1], (x, y)))
                    .fold(f64::INFINITY, f64::min);
                Some((*id, ecart))
            }
            _ => None,
        })
        .filter(|(_, ecart)| *ecart <= 0.25 * pas)
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(id, _)| id)
}

/// Distance d'un point à un segment en millimètres (lot 3c-4) :
/// projection orthogonale bornée aux extrémités du segment — le
/// segment dégénéré retombe sur la distance aux extrémités.
fn distance_segment_mm(debut: (f64, f64), fin: (f64, f64), point: (f64, f64)) -> f64 {
    let dx = fin.0 - debut.0;
    let dy = fin.1 - debut.1;
    let longueur_carree = dx * dx + dy * dy;
    if longueur_carree == 0.0 {
        return distance_mm(debut, point);
    }
    let projection =
        (((point.0 - debut.0) * dx + (point.1 - debut.1) * dy) / longueur_carree).clamp(0.0, 1.0);
    distance_mm(
        (debut.0 + projection * dx, debut.1 + projection * dy),
        point,
    )
}

/// Identifiants des contraintes du croquis signalées redondantes ou
/// partiellement redondantes par le diagnostic du solveur
/// (ADR-0018 D5, lot 3c-4) : le diagnostic parle en tags, la
/// sélection parle en identifiants du croquis.
pub(crate) fn contraintes_redondantes(
    croquis: &CroquisResolu,
    esquisse: &EsquisseProjection,
) -> Vec<u32> {
    esquisse
        .contraintes()
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| est_redondante(croquis, *id))
        .collect()
}

/// La contrainte du croquis est-elle signalée redondante ou
/// partiellement redondante par le diagnostic du solveur ? L'ancrage
/// `PointFixe` n'a pas de tag solveur : jamais redondant.
fn est_redondante(croquis: &CroquisResolu, id: u32) -> bool {
    let Some(tag) = croquis.tag(id) else {
        return false;
    };
    croquis.redondantes().contains(&tag) || croquis.partiellement_redondantes().contains(&tag)
}

/// Diagnostic de redondance stable sur deux résolutions
/// consécutives (ADR-0018 D5, hystérésis FPA #106) : le diagnostic
/// courant ne sort que s'il ne diffère pas du précédent.
pub(crate) fn redondances_stables(
    precedentes: Option<&[u32]>,
    courantes: &[u32],
) -> Option<Vec<u32>> {
    (precedentes == Some(courantes)).then(|| courantes.to_vec())
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
    // Affichage : la dernière résolution connue reste montrée
    // pendant le vol d'une résolution fraîche (anti-flash).
    let resolu = resolu_affichage(app);
    let esquisse = app.session.esquisse();
    let camera = &app.camera;
    for (id, entite) in esquisse.entites() {
        // Surbrillance de la sélection (ADR-0020, validation jalon 2)
        // : l'entité sélectionnée se dessine en accent au trait
        // épaissi — la sélection ne se lit plus uniquement à l'anneau
        // des points.
        let (couleur, largeur) = if app.selection.contains(id) {
            (couleur_apercu(), LARGEUR_ESQUISSE_PX + 1.5)
        } else {
            (COULEUR_ESQUISSE, LARGEUR_ESQUISSE_PX)
        };
        match entite {
            EntiteEsquisse::Point { .. } => {
                if let Some((x, y)) = position_affichee(esquisse, resolu, surcharge, *id) {
                    let centre = point_ecran(camera, rect, x.millimeters(), y.millimeters());
                    painter.circle_filled(centre, RAYON_POINT_PX, couleur);
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
                        egui::Stroke::new(largeur, couleur),
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
                        egui::Stroke::new(largeur, couleur),
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
                        egui::Stroke::new(largeur, couleur),
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
                if app.selection.contains(id) {
                    painter.circle_stroke(
                        centre,
                        RAYON_POINT_PX + 3.0,
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, couleur_apercu()),
                    );
                }
                // Point ancré (ADR-0021 D3) : un liseré distinct
                // signale l'ancrage — la poignée ne se saisit plus,
                // le marqueur dit pourquoi.
                if esquisse.point_ancre(*id) {
                    painter.circle_stroke(
                        centre,
                        RAYON_POINT_PX + 3.0,
                        egui::Stroke::new(LARGEUR_ESQUISSE_PX, COULEUR_ANCRAGE),
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
    // Affichage de l'aperçu : même règle anti-flash que le dessin.
    let resolu = resolu_affichage(app);
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
            let rayon_mm = distance_mm(centre_mm, debut_mm);
            let angle_debut = (debut_mm.1 - centre_mm.1).atan2(debut_mm.0 - centre_mm.0);
            // L'aperçu suit le balayage signé du geste (ADR-0020) :
            // le sens se lit avant le troisième clic ; confondu avec
            // le départ, le tour entier.
            let mut balayage = app.balayage_arc;
            if balayage.abs() < 1e-9 {
                balayage = std::f64::consts::TAU;
            }
            let mut points = Vec::with_capacity(SEGMENTS_ARC + 1);
            for angle in angles_arc(
                Angle::from_radians(angle_debut),
                Angle::from_radians(angle_debut + balayage),
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
/// en tient une fraîche, la graine du journal sinon — dernière
/// graine posée par `MovePoint`, coordonnées d'origine de
/// l'entité sinon (ADR-0018 D7).
pub(crate) fn position_point(
    esquisse: &EsquisseProjection,
    resolu: Option<&CroquisResolu>,
    id: u32,
) -> Option<(Length, Length)> {
    if let Some(position) = resolu.and_then(|croquis| croquis.position(id)) {
        return Some(position);
    }
    esquisse.position_graine(id)
}

/// Angles de la discrétisation d'un arc (radians), balayage signé
/// (ADR-0020) : une fin « après » le début balaie dans le sens
/// trigonométrique, une fin « avant » dans le sens horaire — le
/// sens suit le geste du curseur, une fin confondue avec le début
/// dessine le cercle entier.
pub(crate) fn angles_arc(debut: Angle, fin: Angle, segments: usize) -> Vec<f64> {
    let debut_rad = debut.radians();
    let mut balayage = fin.radians() - debut_rad;
    if balayage == 0.0 {
        balayage = std::f64::consts::TAU;
    }
    let denominateur = segments.max(1) as f64;
    let mut angles = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let part = i as f64 / denominateur;
        angles.push(debut_rad + balayage * part);
    }
    angles
}

/// Projette un point du monde à l'écran : la caméra inverse l'axe Y
/// et donne un décalage par rapport au centre du viewport (ADR-0015).
fn point_ecran(camera: &Camera2D, rect: egui::Rect, x: f64, y: f64) -> egui::Pos2 {
    let (px, py) = camera.vers_ecran((x, y));
    egui::pos2(rect.center().x + px as f32, rect.center().y + py as f32)
}
