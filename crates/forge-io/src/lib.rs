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
//! Jalon 1 : STL ([`stl`]). SVG (usvg) et DXF (crate `dxf`) seront
//! ajoutés dès que `Cargo.lock` embarquera leurs arbres de
//! dépendances (régénération locale nécessaire).

mod error;
pub mod stl;

pub use error::IoError;
