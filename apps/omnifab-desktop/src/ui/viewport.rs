//! Viewport wgpu 2D/3D, scènes et cadrage (ADR-0015,
//! découpage étape 5 de `preparation/decoupage-main-rs.md`,
//! déplacement mécanique).

use crate::aide::avec_aide;
use crate::atelier_esquisse;
use crate::theme::COULEUR_GRILLE_2D;
use crate::theme::COULEUR_TRACES;
use crate::theme::COULEUR_WIREFRAME;
use crate::theme::GRILLE_CIBLE_PX;
use crate::theme::LARGEUR_TRACES_PX;
use crate::travail::maillage_echelle;
use crate::travail::met_en_cache;
use crate::{DemoApp, Vue};
use eframe::egui;
use forge_geom::{Mesh, Path2};
use forge_i18n::Argument;
use forge_io::{dxf, stl, svg};
use forge_render::{
    Boite2, Boite3D, Camera2D, Scene2D, Scene3D, Segment2D, Segment3D, dessiner_scene,
    dessiner_wireframe, pas_de_grille, pas_decimation,
};
use forge_units::Length;

impl DemoApp {
    /// Boîte englobante 2D de la scêne importée : union des tracés et
    /// du maillage projeté de dessus — une seule échelle pour tout
    /// (ADR-0014).
    pub(crate) fn boite_scene(traces: &[Path2], maillage: Option<&Mesh>) -> Option<Boite2> {
        let mut boite: Option<Boite2> = None;
        for trace in traces {
            for point in trace.points() {
                let x = point.x().millimeters();
                let y = point.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite2::du_point(x, y),
                });
            }
        }
        if let Some(maillage) = maillage {
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y),
                    None => Boite2::du_point(x, y),
                });
            }
        }
        boite
    }

    /// Boîte de cadrage automatique de la caméra : celle de la scêne
    /// importée quand il y en a une — l'import recentre sur les objets,
    /// même éloignés du rectangle paramétrique — sinon celle du
    /// rectangle paramétrique (ADR-0015).
    pub(crate) fn boite_cadrage(width: f64, height: f64, scene: Option<Boite2>) -> Boite2 {
        match scene {
            Some(scene) => scene,
            None => Boite2::du_point(0.0, 0.0).etendre(width, height),
        }
    }

    /// Boîte englobante 3D de la scêne importée : union des tracés
    /// posés sur le plan z = 0 et des sommets du maillage en volume
    /// (ADR-0015, phase C2).
    pub(crate) fn boite_scene_3d(traces: &[Path2], maillage: Option<&Mesh>) -> Option<Boite3D> {
        let mut boite: Option<Boite3D> = None;
        for trace in traces {
            for point in trace.points() {
                let x = point.x().millimeters();
                let y = point.y().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y, 0.0),
                    None => Boite3D::du_point(x, y, 0.0),
                });
            }
        }
        if let Some(maillage) = maillage {
            for sommet in maillage.vertices() {
                let x = sommet.x().millimeters();
                let y = sommet.y().millimeters();
                let z = sommet.z().millimeters();
                boite = Some(match boite {
                    Some(b) => b.etendre(x, y, z),
                    None => Boite3D::du_point(x, y, z),
                });
            }
        }
        boite
    }

    /// Boîte de cadrage automatique de la caméra 3D : celle de la
    /// scêne importée quand il y en a une, sinon le rectangle
    /// paramétrique posé sur le plan z = 0 (ADR-0015, phase C2).
    pub(crate) fn boite_cadrage_3d(width: f64, height: f64, scene: Option<Boite3D>) -> Boite3D {
        match scene {
            Some(scene) => scene,
            None => Boite3D::du_point(0.0, 0.0, 0.0).etendre(width, height, 0.0),
        }
    }

    /// Ajoute à une scêne 2D le wireframe du maillage en projection
    /// de dessus : trois arêtes par triangle, cyan, un pixel —
    /// dessiné par le GPU (ADR-0015, phase C2), plafonné : au-delà
    /// d'un plafond d'arêtes, les triangles sont décimés (audit n°4
    /// A4-02). Le
    /// STL reste visible en vue 2D sans repasser par le painter
    /// (validation C2, round 5).
    pub(crate) fn ajouter_maillage_2d(scene: &mut Scene2D, maillage: &Mesh) {
        let triangles = maillage.triangles();
        // Décimation au-delà du plafond : un maillage démesuré ne
        // déborde plus des tampons GPU (audit n°4 A4-02).
        let pas = pas_decimation(triangles.len());
        let sommets = maillage.vertices();
        for triangle in triangles.iter().step_by(pas) {
            let mut points: Vec<(f64, f64)> = Vec::new();
            for index in triangle.iter() {
                let sommet = sommets[*index as usize];
                points.push((sommet.x().millimeters(), sommet.y().millimeters()));
            }
            let (a, b, c) = (points[0], points[1], points[2]);
            for (depart, arrivee) in [(a, b), (b, c), (c, a)] {
                scene.ajouter(Segment2D {
                    depart,
                    arrivee,
                    couleur: COULEUR_WIREFRAME,
                    largeur_px: 1.0,
                });
            }
        }
    }

    /// Scêne 3D du viewport : grille au sol, tracés posés sur le
    /// plan z = 0, contour du rectangle paramétrique, wireframe du
    /// maillage, donné déjà construit par l'appelant — cache à clé
    /// d'édition (ADR-0027, audit n°4 A4-02). L'ordre d'ajout est
    /// la priorité d'affichage : le
    /// pipeline de lignes ne teste pas la profondeur, le dernier
    /// segment dessiné recouvre les précédents — la grille passe
    /// sous les tracés et le rectangle, qui ne disparaissent plus
    /// quand ils coïncident avec elle (validation C2, round 2).
    pub(crate) fn scene_vue_3d(
        traces: &[Path2],

    /// Engagement d'un champ de dimension pour cette frame, s'il y en a
    /// un : le DragValue écrit son tampon au relâchement du geste ou à
    /// la fin d'une édition clavier, et un engagement sans changement
    /// effectif n'écrit rien — un faux événement tronquerait les
    /// groupes défaits (ADR-0008). Les bugs des rounds 3 à 6 de la
    /// validation C1 viennent de là : dimensions figées au
    /// glissement, historique détruit au double annuler, valeur
    /// gelée pendant le geste.
    pub(crate) fn engagement_params(
        avant: f64,

    /// Valeur affichée d'un champ de dimension : inactif, le tampon
    /// suit le graphe (undo/redo, ouverture) ; actif, il persiste —
    /// c'est lui qui garde la valeur glissée d'une frame à l'autre
    /// pendant le geste (validation C1 round 6).
    pub(crate) fn tampon_apres_inactivite(actif: bool, tampon: f64, graphe: Option<f64>) -> f64 {
        if actif { tampon } else { graphe.unwrap_or(0.0) }
    }

    /// Projette un point du monde à l'écran : la caméra inverse l'axe
    /// Y (convention SVG/DXF) et donne un décalage par rapport au
    /// centre du viewport (ADR-0015).
    pub(crate) fn point_ecran(camera: &Camera2D, rect: egui::Rect, x: f64, y: f64) -> egui::Pos2 {
        let (px, py) = camera.vers_ecran((x, y));
        egui::pos2(rect.center().x + px as f32, rect.center().y + py as f32)
    }

    /// Re-parse les tracés courants si le blob référencé a changé :
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0011) ;
    /// un échec est mémorisé, pas re-parsé à chaque frame
    /// (ADR-0016 C1).
    pub(crate) fn mettre_a_jour_cache(&mut self) {
        let Some(reference) = self.session.geom().traces() else {
            self.cache = None;
            return;
        };
        let (format, blob) = (reference.format.clone(), reference.blob.clone());
        if self.cache.as_ref().is_some_and(|(nom, _)| *nom == blob) {
            return;
        }
        let Some(octets) = self.session.document().blob(&blob) else {
            return;
        };
        let octets = octets.to_vec();
        let parse = move || {
            let parse = if format == "svg" {
                svg::import(&octets, &svg::SvgBudgets::default())
            } else {
                dxf::import(&octets, &dxf::DxfBudgets::default())
            };
            parse.map_err(|erreur| erreur.to_string())
        };
        met_en_cache(&mut self.cache, blob, parse);
        if let Some((_, Err(erreur))) = &self.cache {
            self.error = Some(erreur.clone());
        }
    }

    /// Re-parse le maillage courant si le blob référencé a changé :
    /// cache mémoire non autoritaire, clé = nom du blob (ADR-0014) ;
    /// un échec est mémorisé, pas re-parsé à chaque frame
    /// (ADR-0016 C1).
    pub(crate) fn mettre_a_jour_cache_maillage(&mut self) {
        let Some(reference) = self.session.geom().maillage() else {
            self.cache_maillage = None;
            return;
        };
        let blob = reference.blob.clone();
        if self
            .cache_maillage
            .as_ref()
            .is_some_and(|(nom, _)| *nom == blob)
        {
            return;
        }
        let Some(octets) = self.session.document().blob(&blob) else {
            return;
        };
        let echelle = reference.echelle;
        let octets = octets.to_vec();
        let parse = move || {
            let brut = stl::import(&octets, &stl::StlBudgets::default());
            let maillage = brut.map_err(|erreur| erreur.to_string())?;
            maillage_echelle(maillage, echelle)
        };
        met_en_cache(&mut self.cache_maillage, blob, parse);
        if let Some((_, Err(erreur))) = &self.cache_maillage {
            self.error = Some(erreur.clone());
        }
    }

    /// Reconstruit le wireframe 3D du maillage seulement si la clé
    /// d'édition a changé (ADR-0027, patron `camera_cle`, audit n°4
    /// A4-02) : le dédoublonnage O(triangles) n'est plus payé à
    /// chaque frame, le wireframe décimé vit en cache jusqu'au
    /// prochain import.
    pub(crate) fn rafraichir_wireframe(&mut self, cle: &Option<String>) {
        if self
            .wireframe_maillage
            .as_ref()
            .is_some_and(|(cle_cache, _)| cle_cache == cle)
        {
            return;
        }
        let maillage: Option<&Mesh> = match &self.cache_maillage {
            Some((_, Ok(maillage))) => Some(maillage),
            _ => None,
        };
        let segments = match maillage {
            Some(maillage) => Scene3D::depuis_maillage(maillage, COULEUR_WIREFRAME).segments,
            None => Vec::new(),
        };
        self.wireframe_maillage = Some((cle.clone(), segments));
    }

    pub(crate) fn viewport(&mut self, ui: &mut egui::Ui) {
        let reponse = ui.heading(self.texte("viewport-titre"));
        avec_aide(&reponse, "aide-viewport-titre", &mut self.aide);
        if let Some(error) = &self.error {
            let texte = self.texte_avec(
                "viewport-erreur",
                &[("texte", Argument::from(error.as_str()))],
            );
            ui.label(egui::RichText::new(texte).color(egui::Color32::RED));
        }
        if let Some(info) = &self.info {
            ui.label(egui::RichText::new(info.clone()).color(egui::Color32::LIGHT_GREEN));
        }
        ui.separator();
        // Bascule des vues du viewport : 2D (plan) ou 3D (orbite) —
        // phase C2 (ADR-0015).
        ui.horizontal(|ui| {
            let reponse =
                ui.selectable_label(self.vue == Vue::DeuxD, self.texte("viewport-vue-2d"));
            avec_aide(&reponse, "aide-viewport-vue-2d", &mut self.aide);
            if reponse.clicked() {
                self.vue = Vue::DeuxD;
            }
            let reponse =
                ui.selectable_label(self.vue == Vue::TroisD, self.texte("viewport-vue-3d"));
            avec_aide(&reponse, "aide-viewport-vue-3d", &mut self.aide);
            if reponse.clicked() {
                self.vue = Vue::TroisD;
            }
        });
        self.mettre_a_jour_cache();
        self.mettre_a_jour_cache_maillage();
        let width = self
            .session
            .graph_mut()
            .value("largeur")
            .unwrap_or(1.0)
            .max(1.0);
        let height = self
            .session
            .graph_mut()
            .value("hauteur")
            .unwrap_or(1.0)
            .max(1.0);
        // Une ligne est réservée sous le viewport pour chaque compteur.
        let mut lignes_reservees = 0.0;
        if self.session.geom().traces().is_some() {
            lignes_reservees += 24.0;
        }
        if self.session.geom().maillage().is_some() {
            lignes_reservees += 24.0;
        }
        let hauteur = (ui.available_height() - lignes_reservees).max(100.0);
        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width(), hauteur),
            egui::Sense::click_and_drag(),
        );
        avec_aide(&response, "aide-viewport-camera", &mut self.aide);
        // Mode esquisse (vue 2D) : un clic poursuit le geste de
        // l'outil actif à l'endroit visé — les commandes passent par
        // la session, la résolution part au fil de travail
        // (ADR-0018 D3). Le clic s'exécute avant l'emprunt des
        // tracés importés, qui vit jusqu'au dessin des scènes.
        if response.clicked()
            && self.vue == Vue::DeuxD
            && let Some(curseur) = response.interact_pointer_pos()
        {
            let decalage = curseur - response.rect.center();
            let (x, y) = self
                .camera
                .depuis_ecran((decalage.x as f64, decalage.y as f64));
            let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
            atelier_esquisse::traiter_clic(self, pas, x, y);
        }
        // Glissement d'une poignée du croquis (ADR-0018 D4/D5, lot
        // 3c-3) : en vue 2D avec l'outil Sélection, un glissement
        // qui démarre sur un point le saisit — l'aperçu local le
        // suit sans écrire au journal, le relâchement pose sa
        // graine par `MovePoint`, la résolution repart au fil de
        // travail.
        if self.vue == Vue::DeuxD
            && self.outil == atelier_esquisse::OutilEsquisse::Selection
            && let Some(curseur) = response.interact_pointer_pos()
        {
            let decalage = curseur - response.rect.center();
            let (x, y) = self
                .camera
                .depuis_ecran((decalage.x as f64, decalage.y as f64));
            let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
            if response.drag_started() {
                atelier_esquisse::saisir_poignee(self, pas, x, y);
            }
            if response.drag_stopped() {
                atelier_esquisse::relacher_poignee(self, pas, x, y);
            }
        }
        // Références des blobs tracés et maillage : la caméra se
        // réajuste quand l'une d'elles change (import, undo/redo,
        // ouverture — ADR-0015).
        let cle_traces = self
            .session
            .geom()
            .traces()
            .map(|reference| reference.blob.clone());
        let cle_maillage = self
            .session
            .geom()
            .maillage()
            .map(|reference| reference.blob.clone());
        let cle = (cle_traces, cle_maillage);
        // Wireframe 3D : reconstruit seulement si la clé du maillage
        // a changé, jamais à chaque frame (audit n°4 A4-02).
        self.rafraichir_wireframe(&cle.1);
        let traces: &[Path2] = match &self.cache {
            Some((_, Ok(traces))) => traces,
            _ => &[],
        };
        let maillage: Option<&Mesh> = match &self.cache_maillage {
            Some((_, Ok(maillage))) => Some(maillage),
            _ => None,
        };
        let rect = response.rect;
        if self.camera_cle.as_ref() != Some(&cle) {
            // Le cadrage vise la scêne importée quand il y en a une,
            // le rectangle paramétrique sinon : un import éloigné de
            // l'origine reste centré sur ses objets (ADR-0015).
            let scene = Self::boite_scene(traces, maillage);
            let boite = Self::boite_cadrage(width, height, scene);
            self.camera
                .ajuster(rect.width() as f64, rect.height() as f64, boite, 20.0);
            let scene3d = Self::boite_scene_3d(traces, maillage);
            let boite3d = Self::boite_cadrage_3d(width, height, scene3d);
            self.camera3d.ajuster(boite3d);
            self.camera_cle = Some(cle);
        }
        // Vue 2D : la molette zoome au curseur, un glissement déplace
        // la vue. Vue 3D : la molette zoome, un glissement fait orbiter
        // la caméra (ADR-0015, phase C2).
        if response.hovered() {
            let molette: f64 = response
                .ctx
                .input(|i| i.raw_scroll_delta.y as f64)
                .clamp(-600.0, 600.0);
            if molette.abs() > 1e-3 {
                match self.vue {
                    Vue::DeuxD => {
                        let position =
                            response.hover_pos().unwrap_or(rect.center()) - rect.center();
                        self.camera.zoom_au_curseur(
                            (position.x as f64, position.y as f64),
                            (molette / 600.0).exp(),
                        );
                    }
                    Vue::TroisD => self.camera3d.zoomer((molette / 600.0).exp()),
                }
            }
        }
        // Le pan cède à la poignée saisie : seul le glissement du
        // fond déplace la vue (ADR-0018 D4, lot 3c-3).
        if response.dragged() && self.saisie.is_none() {
            let delta = response.drag_delta();
            match self.vue {
                Vue::DeuxD => self.camera.deplacer((delta.x as f64, delta.y as f64)),
                Vue::TroisD => self.camera3d.orbiter((delta.x as f64, delta.y as f64)),
            }
        }
        // Vue 2D : rectangle paramétrique par le painter (remplissage
        // et contour, une seule entité), grille, tracés et wireframe
        // du maillage en projection de dessus par le GPU. Vue 3D :
        // rectangle, tracés et grille au sol posés sur le plan z = 0,
        // wireframe du maillage en volume — arêtes dédupliquées, sans
        // plafond (ADR-0015, phase C2).
        match self.vue {
            Vue::DeuxD => {
                let coin = Self::point_ecran(&self.camera, rect, 0.0, 0.0);
                let oppose = Self::point_ecran(&self.camera, rect, width, height);
                let shape = egui::Rect::from_two_pos(coin, oppose);
                // Priorité d'affichage = ordre de dessin (leçon du
                // round 2 de la validation C2) : la grille GPU au
                // fond, puis le carré bleu — une seule entité dont
                // le contour est rendu juste sur son remplissage
                // (round 5 : les tracés importés ne s'intercalent
                // pas entre les deux) — puis les tracés et le
                // wireframe du maillage par-dessus, la cote en
                // dernier pour rester lisible. Sans wgpu, la
                // scêne 2D n'est pas dessinée et le viewport le
                // dit (ADR-0015).
                let feuille = egui::Color32::from_rgb(38, 96, 152);
                let contour = egui::Stroke::new(1.5_f32, egui::Color32::WHITE);
                if self.viewport_wgpu {
                    let mut scene_grille = Scene2D::vide();
                    let visible = self
                        .camera
                        .boite_visible(rect.width() as f64, rect.height() as f64);
                    scene_grille.ajouter_grille(
                        visible,
                        self.camera.echelle,
                        GRILLE_CIBLE_PX,
                        COULEUR_GRILLE_2D,
                    );
                    dessiner_scene(&painter, rect, scene_grille, self.camera);
                    painter.rect_filled(shape, 4.0, feuille);
                    painter.rect_stroke(shape, 4.0, contour);
                    let mut scene_haut =
                        Scene2D::depuis_traces(traces, COULEUR_TRACES, LARGEUR_TRACES_PX);
                    if let Some(maillage) = maillage {
                        Self::ajouter_maillage_2d(&mut scene_haut, maillage);
                    }
                    dessiner_scene(&painter, rect, scene_haut, self.camera);
                } else {
                    painter.rect_filled(shape, 4.0, feuille);
                    painter.rect_stroke(shape, 4.0, contour);
                    ui.label(
                        egui::RichText::new(self.texte("viewport-wgpu-indisponible"))
                            .color(egui::Color32::YELLOW),
                    );
                }
                // Entités du croquis contraint (ADR-0018, lot 3c) :
                // positions résolues du cache si fraîches, graines du
                // journal sinon — par-dessus la scène, sous la cote.
                // Geste de poignée (re-test du 24-10) : pendant le
                // glissement, la position brute du curseur ne
                // s'affiche plus — le dessin ne montre que des états
                // résolus, une ligne contrainte horizontalement ou
                // verticalement ne devient jamais une diagonale, même
                // le temps d'un aller-retour de résolution. La
                // surcharge ne survit qu'au vol du relâchement
                // (anti-flash §7).
                let pas = pas_de_grille(self.camera.echelle, GRILLE_CIBLE_PX);
                let curseur = response.interact_pointer_pos().map(|position| {
                    let decalage = position - response.rect.center();
                    self.camera
                        .depuis_ecran((decalage.x as f64, decalage.y as f64))
                });
                let (surcharge_drag, cible_du_geste) =
                    atelier_esquisse::affichage_du_geste(self, pas, curseur);
                // La forme liée suit le point saisi pendant le
                // glissement (re-test du 2026-10-23) : la
                // résolution d'aperçu part à la position visée
                // — sémantique du relâchement, affichage seul.
                if let Some((id, vise_x, vise_y)) = cible_du_geste {
                    self.demande_apercu_glissement(id, vise_x, vise_y);
                }
                // Résolution d'affichage du geste (re-test du
                // 24-10) : l'aperçu tient le glissement ET le vol
                // du relâchement — purgé quand les deux sont finis.
                self.purge_apercu_si_geste_fini();
                let resolu_apercu = self.resolution_du_geste();
                atelier_esquisse::dessiner(&painter, self, rect, surcharge_drag, resolu_apercu);
                // Aperçu du geste en cours (ADR-0018, lot 3c-2) : le
                // point visé suit le curseur, accroché à la grille
                // et aux points existants — état dérivé d'affichage,
                // jamais écrit au journal.
                if self.outil != atelier_esquisse::OutilEsquisse::Selection
                    && let Some(curseur) = response.hover_pos()
                {
                    let decalage = curseur - response.rect.center();
                    let (x, y) = self
                        .camera
                        .depuis_ecran((decalage.x as f64, decalage.y as f64));
                    // Le geste de l'arc suit le curseur (ADR-0020) :
                    // le balayage signé s'accumule avant l'aperçu.
                    atelier_esquisse::suivre_geste_arc(self, x, y);
                    atelier_esquisse::dessiner_apercu(&painter, self, rect, x, y);
                }
                // Cote du rectangle par le painter, après les
                // callbacks GPU : elle reste lisible par-dessus les
                // tracés (validation C2, round 5).
                painter.text(
                    shape.left_bottom() + egui::vec2(0.0, 16.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "{} × {}",
                        Length::from_millimeters(width),
                        Length::from_millimeters(height)
                    ),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            }
            Vue::TroisD => {
                if self.viewport_wgpu {
                    let wireframe: &[Segment3D] = match &self.wireframe_maillage {
                        Some((_, segments)) => segments,
                        None => &[],
                    };
                    let scene =
                        Self::scene_vue_3d(traces, wireframe, &self.camera3d, width, height);
                    dessiner_wireframe(&painter, rect, scene, self.camera3d);
                } else {
                    ui.label(
                        egui::RichText::new(self.texte("viewport-wgpu-indisponible"))
                            .color(egui::Color32::YELLOW),
                    );
                }
            }
        }
        // Compteurs de géométrie importée, sous le viewport (ADR-0011).
        if let Some(traces) = self.session.geom().traces() {
            let libelle = self.texte_avec(
                "geom-traces-compteur",
                &[
                    ("traces", Argument::from(traces.traces)),
                    ("sommets", Argument::from(traces.sommets)),
                ],
            );
            let reponse = ui.label(libelle);
            avec_aide(&reponse, "aide-geom-compteur", &mut self.aide);
        }
        if let Some(maillage) = self.session.geom().maillage() {
            let libelle = self.texte_avec(
                "geom-maillage-compteur",
                &[("triangles", Argument::from(maillage.triangles))],
            );
            let reponse = ui.label(libelle);
            avec_aide(&reponse, "aide-geom-compteur", &mut self.aide);
        }
    }
}
