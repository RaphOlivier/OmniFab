//! # forge-gcs-sys — liaisons brutes vers le shim ABI C de planegcs
//!
//! Crate `-sys` de la frontière FFI (ADR-0013) : elle expose les
//! déclarations générées par bindgen depuis le header
//! `third_party/planegcs-shim/gcs_c_api.h` et lie statiquement planegcs
//! et le shim (build piloté par `build.rs` via CMake).
//!
//! Cette crate n'est consommée que par `forge-gcs`, qui fournit le
//! wrapper sûr, les codes d'erreur `OF-GCS-*` et le trait Solveur2D.
//! Les conventions de l'ABI (millimètres, radians, identifiants émis
//! dans l'ordre d'ajout, exceptions C++ toujours rattrapées dans le
//! shim) sont documentées dans le header du shim et dans le MANIFEST
//! de `third_party/planegcs`.
//!
//! Le code ci-dessous est **généré** : ne jamais l'éditer à la main ; le
//! header source fait foi.
//!
//! Licence : LGPL-2.1-or-later (héritée de planegcs, cf. LICENSE).

// Code généré par bindgen : on coupe volontairement tous les lints de
// style pour ne pas polluer la sortie de clippy/rustdoc.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::all
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
