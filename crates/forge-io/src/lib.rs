#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-io — import/export du noyau OmniFab
//!
//! (ADR-0009.) Frontière unique octets ↔ géométrie : transformer des
//! fichiers en géométrie forge-geom, et inversement. La crate est
//! **pure** — aucune dépendance à forge-doc ni forge-session ;
//! l'orchestration (lecture du fichier, écriture des blobs, undo)
//! revient aux commandes de session (ADR-0008). Aucun I/O disque ni
//! thread ici : l'appelant peut parser hors du fil UI (ADR-0009).
//!
//! Sécurité (ADR-0009) : chaque parseur travaille sous budget de
//! ressources réglable, sans DTD ni XXE ; tout dépassement est une
//! erreur typée, levée avant toute allocation proportionnelle.
//!
//! Jalon 1 : STL ([`stl`], parseur maison), SVG ([`svg`], crate
//! `usvg`) et DXF ([`dxf`], crate `dxf`). Convention d'unités : une
//! unité de fichier = 1 mm (SVG et DXF) ; le STL est sans unité, lu
//! et écrit en millimètres.

mod error;
pub mod dxf;
pub mod stl;
pub mod svg;

pub use error::IoError;
