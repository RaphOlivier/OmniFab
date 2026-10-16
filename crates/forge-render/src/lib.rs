//! # forge-render — viewport wgpu (ADR-0015)
//!
//! Rendu du viewport du démonstrateur et, à terme, de l'atelier : une
//! crate **cœur sans egui** (scène, caméra, pipeline wgpu, shader
//! WGSL) dont l'adaptateur egui vit derrière la feature `egui` (pas
//! de crate séparée tant que la duplication n'existe pas).
//!
//! Frontières (ADR-0015) : dépendances limitées à `wgpu`, `bytemuck`
//! et `forge-geom` ; `forge-doc` et `forge-session` sont interdites —
//! la scène est alimentée par l'application, qui seule connaît la
//! session. Une seule version de wgpu dans le graphe : celle dictée
//! par egui-wgpu, jamais une version choisie librement.
//!
//! Phase C1 : pipeline 2D — grille de fond et tracés importés, caméra
//! à la molette (zoom au curseur) et au glisser (pan), parité visuelle
//! avec le painter egui (mêmes couleurs, mêmes largeurs de trait).
//! Phase C2 : wireframe 3D, caméra orbite, suppression du plafond
//! d'affichage.

#![deny(missing_docs)]

mod camera;
mod camera3d;
#[cfg(feature = "egui")]
pub mod egui_adapteur;
mod renderer;
mod scene;
mod scene3d;

pub use camera::Camera2D;
pub use camera3d::Camera3D;
#[cfg(feature = "egui")]
pub use egui_adapteur::{dessiner_scene, dessiner_wireframe};
pub use renderer::Renderer2D;
pub use scene::{Boite2, Couleur, Scene2D, Segment2D, pas_de_grille};
pub use scene3d::{Boite3D, Scene3D, Segment3D};
