//! Pipeline wgpu des segments 2D : un quad par segment (ADR-0015).
//!
//! wgpu ne garantit qu'une largeur de trait d'un pixel : l'épaisseur
//! des tracés est extrudée côté CPU, perpendiculairement au segment.
//! Les sommets montent par frame dans un tampon recréé à chaque draw
//! (dette technique de la phase C1, ADR-0015).

use crate::camera::Camera2D;
use crate::scene::Scene2D;
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

/// Uniform du viewport : taille de la cible en pixels physiques.
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    /// Taille de la cible, en pixels physiques.
    taille_px: [f32; 2],
    /// Remplissage : un uniform dépasse 16 octets (WebGL).
    remplissage: [f32; 2],
}

/// Sommet GPU : position à l'écran et couleur.
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Sommet {
    /// Position à l'écran, en pixels physiques.
    pub position_px: [f32; 2],
    /// Couleur RGBA normalisée.
    pub couleur: [f32; 4],
}

/// Shader WGSL du viewport 2D (ADR-0015 : WGSL seul).
const WGSL: &str = include_str!("trace2d.wgsl");

/// Pipeline 2D du viewport : créé une fois par surface, partagé via
/// les ressources de callback d'egui-wgpu (ADR-0015).
pub struct Renderer2D {
    /// Pipeline de rendu des segments.
    pipeline: wgpu::RenderPipeline,
    /// Uniform partagé (une seule vue 2D par frame).
    uniform_buffer: wgpu::Buffer,
    /// Bind group de l'uniform.
    bind_group: wgpu::BindGroup,
}

impl Renderer2D {
    /// Crée le pipeline pour le format de la surface d'egui.
    pub fn nouveau(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("forge-render trace2d"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forge-render uniforme"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("forge-render trace2d"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("forge-render trace2d"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forge-render uniforme"),
            contents: bytemuck::bytes_of(&Uniforms {
                taille_px: [1.0, 1.0],
                remplissage: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-render uniforme"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// Écrit l'uniform de la frame : taille de la cible.
    pub fn ecrire_uniforme(&self, queue: &wgpu::Queue, taille_px: [f32; 2]) {
        let uniforme = Uniforms {
            taille_px,
            remplissage: [0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforme));
    }

    /// Pipeline du viewport (pour le callback egui).
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// Bind group de l'uniform (pour le callback egui).
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// Prépare le tampon de sommets d'une frame : recréé à chaque draw
/// (dette technique de la phase C1, ADR-0015).
pub(crate) fn preparer_sommets(device: &wgpu::Device, sommets: &[Sommet]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("forge-render sommets"),
        contents: bytemuck::cast_slice(sommets),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

/// Triangule la scène : six sommets par segment (deux triangles
/// extrudés perpendiculairement), en pixels physiques.
pub(crate) fn triangles(
    scene: &Scene2D,
    camera: &Camera2D,
    origine: (f32, f32),
    pixels_par_point: f32,
) -> Vec<Sommet> {
    let mut sommets = Vec::with_capacity(scene.segments.len() * 6);
    for segment in &scene.segments {
        let depart = camera.vers_ecran(segment.depart);
        let arrivee = camera.vers_ecran(segment.arrivee);
        let ax = (origine.0 + depart.0 as f32) * pixels_par_point;
        let ay = (origine.1 + depart.1 as f32) * pixels_par_point;
        let bx = (origine.0 + arrivee.0 as f32) * pixels_par_point;
        let by = (origine.1 + arrivee.1 as f32) * pixels_par_point;
        let dx = bx - ax;
        let dy = by - ay;
        let longueur = dx.hypot(dy);
        if longueur < 1e-6 {
            continue;
        }
        let demi = segment.largeur_px * pixels_par_point / 2.0;
        let nx = -dy / longueur * demi;
        let ny = dx / longueur * demi;
        let un = [ax + nx, ay + ny];
        let deux = [bx + nx, by + ny];
        let trois = [ax - nx, ay - ny];
        let quatre = [bx - nx, by - ny];
        for position in [un, trois, deux, deux, trois, quatre] {
            sommets.push(Sommet {
                position_px: position,
                couleur: segment.couleur,
            });
        }
    }
    sommets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Segment2D;

    /// Un segment produit deux triangles : extrudés
    /// perpendiculairement, à la largeur demandée.
    #[test]
    fn un_segment_produit_deux_triangles() {
        let mut scene = Scene2D::vide();
        scene.ajouter(Segment2D {
            depart: (0.0, 0.0),
            arrivee: (10.0, 0.0),
            couleur: [1.0, 0.0, 0.0, 1.0],
            largeur_px: 2.0,
        });
        // Caméra identité (1 px/mm) et échelle d'écran double.
        let sommets = triangles(&scene, &Camera2D::nouvelle(), (0.0, 0.0), 2.0);
        assert_eq!(sommets.len(), 6, "deux triangles par segment");
        assert_eq!(sommets[0].position_px, [0.0, 2.0], "bord extrudé du départ");
        assert_eq!(sommets[1].position_px, [0.0, -2.0], "bord opposé du départ");
        assert_eq!(
            sommets[2].position_px,
            [20.0, 2.0],
            "bord extrudé de l'arrivée"
        );
        assert_eq!(
            sommets[5].position_px,
            [20.0, -2.0],
            "dernier triangle refermé"
        );
    }

    /// Le segment éteint (longueur nulle) n'émet aucun triangle.
    #[test]
    fn segment_nul_n_emet_rien() {
        let mut scene = Scene2D::vide();
        scene.ajouter(Segment2D {
            depart: (5.0, 5.0),
            arrivee: (5.0, 5.0),
            couleur: [1.0, 0.0, 0.0, 1.0],
            largeur_px: 2.0,
        });
        let sommets = triangles(&scene, &Camera2D::nouvelle(), (0.0, 0.0), 1.0);
        assert!(sommets.is_empty(), "aucun triangle pour un point");
    }
}
