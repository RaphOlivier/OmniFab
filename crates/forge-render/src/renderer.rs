//! Pipeline wgpu : segments 2D triangulés et lignes du wireframe 3D
//! (ADR-0015).
//!
//! wgpu ne garantit qu'une largeur de trait d'un pixel : l'épaisseur
//! des tracés est extrudée côté CPU, perpendiculairement au segment.
//! Les sommets montent par frame dans un tampon recréé à chaque draw
//! (dette technique de la phase C1, ADR-0015).

use crate::camera::Camera2D;
use crate::camera3d::Camera3D;
use crate::scene::Scene2D;
use crate::scene3d::Scene3D;
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

/// Uniform du viewport : taille de la cible en pixels physiques.
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    /// Taille du viewport (rect du callback), en pixels physiques.
    taille_px: [f32; 2],
    /// Remplissage : un uniform dépasse 16 octets (WebGL).
    remplissage: [f32; 2],
}

/// Sommet GPU : position à l'écran et couleur.
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Sommet {
    /// Position dans le viewport, en pixels physiques, depuis son coin.
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
    /// Pipeline des lignes du wireframe 3D (topologie LineList,
    /// phase C2).
    pipeline_lignes: wgpu::RenderPipeline,
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
        let pipeline = creer_pipeline(
            device,
            &shader,
            &pipeline_layout,
            format,
            wgpu::PrimitiveTopology::TriangleList,
        );
        let pipeline_lignes = creer_pipeline(
            device,
            &shader,
            &pipeline_layout,
            format,
            wgpu::PrimitiveTopology::LineList,
        );
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
            pipeline_lignes,
            uniform_buffer,
            bind_group,
        }
    }

    /// Écrit l'uniform de la frame : taille du viewport.
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

    /// Pipeline des lignes du wireframe 3D (pour le callback egui,
    /// phase C2).
    pub fn pipeline_lignes(&self) -> &wgpu::RenderPipeline {
        &self.pipeline_lignes
    }

    /// Bind group de l'uniform (pour le callback egui).
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// Crée un pipeline de rendu sur le shader du viewport : triangles 2D
/// ou lignes du wireframe 3D, seule la topologie change (ADR-0015,
/// phase C2).
fn creer_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("forge-render trace2d"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
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
            module: shader,
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
            topology,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
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

/// Triangule la scêne : six sommets par segment (deux triangles
/// extrudés perpendiculairement), en pixels physiques relatifs au
/// coin du viewport — `demi_taille` donne sa demi-taille en points.
pub(crate) fn triangles(
    scene: &Scene2D,
    camera: &Camera2D,
    demi_taille: (f32, f32),
    pixels_par_point: f32,
) -> Vec<Sommet> {
    let mut sommets = Vec::with_capacity(scene.segments.len() * 6);
    for segment in &scene.segments {
        let depart = camera.vers_ecran(segment.depart);
        let arrivee = camera.vers_ecran(segment.arrivee);
        let ax = (demi_taille.0 + depart.0 as f32) * pixels_par_point;
        let ay = (demi_taille.1 + depart.1 as f32) * pixels_par_point;
        let bx = (demi_taille.0 + arrivee.0 as f32) * pixels_par_point;
        let by = (demi_taille.1 + arrivee.1 as f32) * pixels_par_point;
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

/// Projette la scêne 3D en sommets de lignes : deux par segment, en
/// pixels physiques relatifs au coin du viewport — un segment dont une
/// extrémité passe derrière la caméra est ignoré (ADR-0015, phase C2).
pub(crate) fn lignes(
    scene: &Scene3D,
    camera: &Camera3D,
    largeur_px: f32,
    hauteur_px: f32,
    pixels_par_point: f32,
) -> Vec<Sommet> {
    let mut sommets = Vec::with_capacity(scene.segments.len() * 2);
    let largeur = largeur_px as f64 / pixels_par_point as f64;
    let hauteur = hauteur_px as f64 / pixels_par_point as f64;
    for segment in &scene.segments {
        let Some(depart) = camera.vers_ecran(segment.depart, largeur, hauteur) else {
            continue;
        };
        let Some(arrivee) = camera.vers_ecran(segment.arrivee, largeur, hauteur) else {
            continue;
        };
        let ax = depart.0 as f32 * pixels_par_point;
        let ay = depart.1 as f32 * pixels_par_point;
        let bx = arrivee.0 as f32 * pixels_par_point;
        let by = arrivee.1 as f32 * pixels_par_point;
        sommets.push(Sommet {
            position_px: [ax, ay],
            couleur: segment.couleur,
        });
        sommets.push(Sommet {
            position_px: [bx, by],
            couleur: segment.couleur,
        });
    }
    sommets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Segment2D;
    use crate::scene3d::Segment3D;

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

    /// La demi-taille du viewport positionne les sommets depuis son
    /// coin : le point visé par la caméra atterrit au centre du rect
    /// — indépendamment de la fenêtre entière.
    #[test]
    fn la_demi_taille_du_viewport_positionne_depuis_son_coin() {
        let mut scene = Scene2D::vide();
        scene.ajouter(Segment2D {
            depart: (0.0, 0.0),
            arrivee: (10.0, 0.0),
            couleur: [1.0, 0.0, 0.0, 1.0],
            largeur_px: 2.0,
        });
        let sommets = triangles(&scene, &Camera2D::nouvelle(), (500.0, 300.0), 1.0);
        assert_eq!(sommets[0].position_px[0], 500.0, "x décalé du centre");
        assert_eq!(sommets[1].position_px[1], 299.0, "y décalé du centre");
    }

    /// Le wireframe 3D projette deux sommets par segment visible ; un
    /// segment derrière la caméra n'en émet aucun (ADR-0015, phase C2).
    #[test]
    fn le_wireframe_projete_deux_sommets_par_segment() {
        let mut scene = Scene3D::vide();
        scene.ajouter(Segment3D {
            depart: (0.0, 0.0, 0.0),
            arrivee: (10.0, 0.0, 0.0),
            couleur: [1.0, 1.0, 1.0, 1.0],
        });
        scene.ajouter(Segment3D {
            depart: (5000.0, 0.0, 0.0),
            arrivee: (6000.0, 0.0, 0.0),
            couleur: [1.0, 1.0, 1.0, 1.0],
        });
        let mut camera = Camera3D::nouvelle();
        camera.cible_mm = (5.0, 0.0, 0.0);
        camera.distance = 100.0;
        camera.azimut = 0.0;
        camera.elevation = 1.0;
        let sommets = lignes(&scene, &camera, 800.0, 600.0, 1.0);
        assert_eq!(sommets.len(), 2, "le segment visible seul est projeté");
        assert_eq!(
            sommets[0].couleur,
            [1.0, 1.0, 1.0, 1.0],
            "la couleur du segment est portée"
        );
    }
}
