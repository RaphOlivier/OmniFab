//! Build de forge-gcs-sys (ADR-0013) : deux étapes, dans cet ordre.
//!
//! 1. **bindgen** sur le header C du shim
//!    (`third_party/planegcs-shim/gcs_c_api.h`) : le header est
//!    volontairement minuscule, la sortie est reviewable.
//! 2. **CMake** (crate `cmake`) pilote le sous-module planegcs + le shim
//!    (`third_party/planegcs-shim/CMakeLists.txt`) et produit deux
//!    bibliothèques statiques : `gcs_shim` et `planegcs`, liées
//!    explicitement ci-dessous.
//!
//! Dépendances natives requises à la construction : Eigen3 et fmt
//! (découvrables par `find_package`, publiées par la cible
//! `PlaneGCS::planegcs`) ; Boost n'est requis que par planegcs lui-même
//! (headers) ; libclang est requis par bindgen. Sur Windows avec vcpkg,
//! la variable d'environnement `VCPKG_INSTALLATION_ROOT` active
//! automatiquement le toolchain vcpkg (triplet `VCPKG_TARGET_TRIPLET`
//! ou `x64-windows` par défaut).

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR requis"),
    );
    let racine = manifest
        .parent()
        .and_then(|c| c.parent())
        .expect("forge-gcs-sys doit vivre dans crates/ à la racine")
        .to_path_buf();
    let shim = racine.join("third_party").join("planegcs-shim");
    let entete = shim.join("gcs_c_api.h");

    // 1) Bindings : bindgen sur le header du shim.
    // Les constantes #define (GCS_OK, GCS_STATUT_*, GCS_ERR_*) sont
    // forcées signées : forge-gcs les compare à des i32.
    let bindings = bindgen::Builder::default()
        .header(entete.to_str().expect("chemin du header en UTF-8"))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .generate()
        .expect("generation des bindings depuis gcs_c_api.h");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR requis"));
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("ecriture des bindings");

    // 2) Compilation : CMake (planegcs + shim, liaison statique).
    let mut config = cmake::Config::new(&shim);
    config.define("CMAKE_BUILD_TYPE", "Release");
    // Windows + vcpkg : toolchain et triplet depuis l'environnement CI.
    if let Ok(racine_vcpkg) = env::var("VCPKG_INSTALLATION_ROOT") {
        let triplet = env::var("VCPKG_TARGET_TRIPLET")
            .unwrap_or_else(|_| "x64-windows".to_string());
        config.define("VCPKG_TARGET_TRIPLET", &triplet);
        let toolchain = PathBuf::from(&racine_vcpkg)
            .join("scripts")
            .join("buildsystems")
            .join("vcpkg.cmake");
        if toolchain.exists() {
            config.define("CMAKE_TOOLCHAIN_FILE", toolchain);
        }
    }
    let dest = config.build();

    println!("cargo:rustc-link-search=native={}", dest.join("lib").display());
    println!("cargo:rustc-link-lib=static=gcs_shim");
    println!("cargo:rustc-link-lib=static=planegcs");

    // fmt est une dépendance publique de planegcs : la bibliothèque
    // finale doit la relier explicitement (Eigen est header-only).
    let mut fmt_lie = false;
    if let Ok(racine_vcpkg) = env::var("VCPKG_INSTALLATION_ROOT") {
        let triplet = env::var("VCPKG_TARGET_TRIPLET")
            .unwrap_or_else(|_| "x64-windows".to_string());
        let dir = PathBuf::from(&racine_vcpkg)
            .join("installed")
            .join(&triplet)
            .join("lib");
        if dir.join("fmt.lib").exists() {
            println!("cargo:rustc-link-search=native={}", dir.display());
            println!("cargo:rustc-link-lib=static=fmt");
            fmt_lie = true;
        }
    }
    if !fmt_lie {
        for candidat in ["/opt/homebrew/lib", "/usr/local/lib"] {
            let chemin = PathBuf::from(candidat);
            if chemin.join("libfmt.dylib").exists() {
                println!("cargo:rustc-link-search=native={}", candidat);
                break;
            }
        }
        // Linux : fmt système dans les chemins par défaut du linker.
        println!("cargo:rustc-link-lib=dylib=fmt");
    }

    println!("cargo:rerun-if-changed={}", entete.display());
    println!(
        "cargo:rerun-if-changed={}",
        shim.join("gcs_c_api.cpp").display()
    );
}
