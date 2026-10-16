//! Adaptateur egui du viewport 2D : soumet la scène au painter via
//! `egui_wgpu::Callback` (ADR-0015).
//!
//! Le cœur de `forge-render` ne dépend pas d'egui ; ce module vit
//! derrière la feature `egui`. Le callback prépare ses sommets dans
//! `prepare` (qui reçoit la file GPU) puis les dessine dans `paint`
//! (qui reçoit la passe de rendu d'egui — le scissors du clip est
//! déjà posé par egui-wgpu).

use crate::camera::Camera2D;
use crate::renderer::{Renderer2D, preparer_sommets, triangles};
use crate::scene::Scene2D;
use std::sync::Mutex;

/// Une frame préparée : le tampon de sommets et leur nombre.
struct Frame2D {
    /// Tampon des sommets, recréé à chaque frame.
    sommets: wgpu::Buffer,
    /// Nombre de sommets à dessiner.
    nombre: u32,
}

/// Callback de rendu d'une scène 2D : porte la scène et la caméra
/// d'une frame, de `prepare` à `paint`.
pub struct Callback2D {
    /// Scène dessinée pendant la frame.
    scene: Scene2D,
    /// Caméra de la frame.
    camera: Camera2D,
    /// Centre du viewport à l'écran, en points logiques.
    centre: egui::Pos2,
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
        let taille_px = [
            screen.size_in_pixels[0] as f32,
            screen.size_in_pixels[1] as f32,
        ];
        renderer.ecrire_uniforme(queue, taille_px);
        let sommets = triangles(
            &self.scene,
            &self.camera,
            (self.centre.x, self.centre.y),
            screen.pixels_per_point,
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

/// Soumet la scène au painter : le viewport wgpu dessine les segments
/// dans le rectangle donné (ADR-0015).
pub fn dessiner_scene(painter: &egui::Painter, rect: egui::Rect, scene: Scene2D, camera: Camera2D) {
    painter.add(egui_wgpu::Callback::new_paint_callback(
        rect,
        Callback2D {
            scene,
            camera,
            centre: rect.center(),
            preparee: Mutex::new(None),
        },
    ));
}
