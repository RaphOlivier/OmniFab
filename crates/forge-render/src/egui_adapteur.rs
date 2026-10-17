//! Adaptateur egui du viewport : soumet la scêne 2D et le wireframe
//! 3D au painter via `egui_wgpu::Callback` (ADR-0015).
//!
//! Le cœur de `forge-render` ne dépend pas d'egui ; ce module vit
//! derrière la feature `egui`. Le callback prépare ses sommets dans
//! `prepare` (qui reçoit la file GPU) puis les dessine dans `paint`
//! (qui reçoit la passe de rendu d'egui — egui-wgpu y pose déjà le
//! scissors du clip ET le viewport, borné au rect du callback : les
//! sommets sont relatifs au rect, jamais à la fenêtre).

use crate::camera::Camera2D;
use crate::camera3d::Camera3D;
use crate::renderer::{Renderer2D, lignes, preparer_sommets, triangles};
use crate::scene::Scene2D;
use crate::scene3d::Scene3D;
use std::sync::Mutex;

/// Une frame préparée : le tampon de sommets et leur nombre.
struct Frame2D {
    /// Tampon des sommets, recréé à chaque frame.
    sommets: wgpu::Buffer,
    /// Nombre de sommets à dessiner.
    nombre: u32,
}

/// Callback de rendu d'une scêne 2D : porte la scêne et la caméra
/// d'une frame, de `prepare` à `paint`.
pub struct Callback2D {
    /// Scêne dessinée pendant la frame.
    scene: Scene2D,
    /// Caméra de la frame.
    camera: Camera2D,
    /// Rect du viewport à l'écran, en points logiques.
    rect: egui::Rect,
    /// Frame préparée par `prepare`, consommée par `paint`.
    preparee: Mutex<Option<Frame2D>>,
}

impl egui_wgpu::CallbackTrait for Callback2D {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer = resources
            .get::<Renderer2D>()
            .expect("Renderer2D initialisé");
        // egui-wgpu borne la passe au rect du callback (« courtesy
        // viewport ») : l'uniform reçoit la taille du rect, pas celle
        // de la fenêtre (bug de la validation C1, 2026-10-16).
        let ppp = screen.pixels_per_point;
        let taille_px = [self.rect.width() * ppp, self.rect.height() * ppp];
        renderer.ecrire_uniforme(queue, taille_px);
        let sommets = triangles(
            &self.scene,
            &self.camera,
            (self.rect.width() / 2.0, self.rect.height() / 2.0),
            ppp,
        );
        let nombre = sommets.len() as u32;
        let tampon = preparer_sommets(device, &sommets);
        *self.preparee.lock().expect("verrou de frame") = Some(Frame2D {
            sommets: tampon,
            nombre,
        });
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(frame) = self.preparee.lock().expect("verrou de frame").take() else {
            return;
        };
        if frame.nombre == 0 {
            return;
        }
        let renderer = resources
            .get::<Renderer2D>()
            .expect("Renderer2D initialisé");
        render_pass.set_pipeline(renderer.pipeline());
        render_pass.set_bind_group(0, renderer.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, frame.sommets.slice(..));
        render_pass.draw(0..frame.nombre, 0..1);
    }
}

/// Callback de rendu d'un wireframe 3D : porte la scêne et la caméra
/// orbite d'une frame, de `prepare` à `paint` — pipeline de lignes
/// (ADR-0015, phase C2).
pub struct Callback3D {
    /// Scêne dessinée pendant la frame.
    scene: Scene3D,
    /// Caméra orbite de la frame.
    camera: Camera3D,
    /// Rect du viewport à l'écran, en points logiques.
    rect: egui::Rect,
    /// Frame préparée par `prepare`, consommée par `paint`.
    preparee: Mutex<Option<Frame2D>>,
}

impl egui_wgpu::CallbackTrait for Callback3D {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer = resources
            .get::<Renderer2D>()
            .expect("Renderer2D initialisé");
        // egui-wgpu borne la passe au rect du callback : l'uniform
        // reçoit la taille du rect, en pixels physiques (leçon de la
        // validation C1).
        let ppp = screen.pixels_per_point;
        let taille_px = [self.rect.width() * ppp, self.rect.height() * ppp];
        renderer.ecrire_uniforme(queue, taille_px);
        let sommets = lignes(&self.scene, &self.camera, taille_px[0], taille_px[1], ppp);
        let nombre = sommets.len() as u32;
        let tampon = preparer_sommets(device, &sommets);
        *self.preparee.lock().expect("verrou de frame") = Some(Frame2D {
            sommets: tampon,
            nombre,
        });
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(frame) = self.preparee.lock().expect("verrou de frame").take() else {
            return;
        };
        if frame.nombre == 0 {
            return;
        }
        let renderer = resources
            .get::<Renderer2D>()
            .expect("Renderer2D initialisé");
        render_pass.set_pipeline(renderer.pipeline_lignes());
        render_pass.set_bind_group(0, renderer.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, frame.sommets.slice(..));
        render_pass.draw(0..frame.nombre, 0..1);
    }
}

/// Soumet le wireframe 3D au painter : le viewport wgpu dessine les
/// segments de la scêne en lignes, arêtes dédupliquées et sans
/// plafond (ADR-0015, phase C2).
pub fn dessiner_wireframe(
    painter: &egui::Painter,
    rect: egui::Rect,
    scene: Scene3D,
    camera: Camera3D,
) {
    painter.add(egui_wgpu::Callback::new_paint_callback(
        rect,
        Callback3D {
            scene,
            camera,
            rect,
            preparee: Mutex::new(None),
        },
    ));
}

/// Soumet la scêne au painter : le viewport wgpu dessine les segments
/// dans le rectangle donné (ADR-0015).
pub fn dessiner_scene(painter: &egui::Painter, rect: egui::Rect, scene: Scene2D, camera: Camera2D) {
    painter.add(egui_wgpu::Callback::new_paint_callback(
        rect,
        Callback2D {
            scene,
            camera,
            rect,
            preparee: Mutex::new(None),
        },
    ));
}
