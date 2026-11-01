# OmniFab

[![CI](https://github.com/RaphOlivier/OmniFab/actions/workflows/ci.yml/badge.svg)](https://github.com/RaphOlivier/OmniFab/actions/workflows/ci.yml)
[![Licence : GPL-3.0](https://img.shields.io/badge/licence-GPL--3.0--or--later-blue)](LICENSE)

> **🚧 Statut : jalon 3 (CAO paramétrique) en cours — phase A clôturée (binding OpenCascade validé).** Jalons 1 et 2 livrés (noyau + undo + imports/exports SVG/DXF/STL, viewport wgpu, esquisse 2D à contraintes, moteur de liaisons). La CI est **verte** sur main (multi-OS, cargo-deny). Suivi d'avancement détaillé : [`docs/knowledge/etat-actuel/KNOWLEDGE.md`](docs/knowledge/etat-actuel/KNOWLEDGE.md).

> 🗺️ **Se repérer dans le projet :** le [sommaire général](docs/sommaire.md) cartographie **toute** la documentation et le dépôt — chaque entrée est cliquable et expliquée en une ligne.

**OmniFab** est un logiciel libre de fabrication numérique A→Z : **CAO** (conception assistée par ordinateur), **CAM** (fabrication assistée par ordinateur), génération de **G-code** et **pilotage machine**.

L'objectif : couvrir toute la chaîne, de l'idée du meuble à la pièce usinée, dans une seule application native — sans dépendre de logiciels propriétaires.

## Feuille de route et avancement

Le projet avance par **tranche horizontale** : livrer de bout en bout un cas d'école (fabriquer un meuble — CAO paramétrique → découpe/gravure 2D → CAM 2,5D → LinuxCNC) avant d'élargir.

| # | Jalon | État |
|---|---|---|
| 0 | Architecture + ADR + noyau compilable | ✅ clos (2026-10-12) |
| 1 | Noyau projet + undo + import/export SVG/DXF/STL + aide intégrée | ✅ clos (2026-10-15) |
| 2 | Viewport wgpu + esquisse 2D contrainte + moteur de liaisons | ✅ livré (phases A–E, CI verte multi-OS) |
| 3 | CAO paramétrique + éditeur nodal de valeurs (forge-cad / OCCT) | ◐ **en cours** — phase A clôturée (binding OpenCascade validé) |
| 4 | CAM 2,5D + multi-procédés + éditeur G-code + simulateur | ⏸ prochain — gelé jusqu'au meuble fabriqué |
| 5 | forge-link : LinuxCNC + gestion multi-machines | ⏸ gelé |
| 6 | Painting raster + calques + filtres | ⏸ gelé |
| 7 | Android (même noyau, UI adaptée) | ⏸ gelé |
| 8 | Plugins WASM + communauté | ⏸ gelé |
| 9 | Cible web (WASM) : egui web + pont machine WebSocket | ⏸ gelé |
| 10 | Traitement par lots + bibliothèques + assistants de conception guidée | ⏸ gelé |

> **Gel de roadmap** : les jalons 5 à 10 sont **gelés** tant que le meuble du cas d'école n'est pas fabriqué de bout en bout — ils restent listés comme cadre de vision, sans design ni code. Le *pourquoi* de chaque jalon et décision : [`docs/knowledge/`](docs/knowledge/README.md).

Légende : ✅ clos · ◐ en cours · ⏸ gelé (jalons 5-10) / en attente (jalon 4).

## Vision courte

- Une application modulaire en crates, du modèle paramétrique jusqu'au pilotage machine.
- Des **assistants de conception** (meubles, caissons…) définis comme des *templates déclaratifs* — des données, pas du code.
- Une **simulation d'abord** : aucun matériel physique n'est requis pour développer ; le pilotage machine est validé sur simulateur.
- Interopérabilité openBIM : import **IFC**, export **DSTV/NC1**, classification multi-référentielle (OmniClass / Uniclass / UniFormat).

## Stack technique

| Domaine | Choix |
|---|---|
| Langage | Rust |
| UI | egui (plan B mobile : Slint) |
| Rendu | wgpu |
| Géométrie | OpenCascade (OCCT) via bindings |
| Contraintes 2D | planegcs (via FFI) |
| Concurrence | tokio + canaux (jalons machine — pas encore active, cf. gel) |
| Licence | GPL-3.0+ (MPL/Apache pour les crates isolées) |

> 📄 **Licences** : GPL-3.0 à la racine (`LICENSE`), MPL-2.0 pour chaque crate (`crates/*/LICENSE`). Les données (matériaux, machines, normes) seront publiées sous licences ouvertes dédiées (ODbL/CC0/CC-BY) — jamais GPL.

## Structure du dépôt

| Chemin | Rôle |
|---|---|
| `crates/forge-units` | Newtypes d'unités strictes (longueur, angle) — zéro dépendance (verrou 5) |
| `crates/forge-doc` | Conteneur ZIP versionné + journal d'événements JSON Lines, migrations, snapshots non autoritaires (ADR-0001) |
| `crates/forge-values` | Moteur unique de liaisons : évaluation paresseuse + expressions déterministes (ADR-0002) |
| `crates/forge-geom` | Géométrie minimale : points, chemins 2D, maillages (codes OF-GEO) |
| `crates/forge-io` | Import/export SVG, DXF, STL — parseurs budgetés, courbes discrétisées (ADR-0009) |
| `crates/forge-session` | Commandes, projections, undo/redo par curseur, tout-ou-rien (ADR-0008, ADR-0011) |
| `crates/forge-i18n` | i18n Fluent + pages d'aide intégrées, clés vérifiées par tests (ADR-0010) |
| `crates/forge-render` | Rendu wgpu 2D/3D : caméras, scènes, pipelines TriangleList/LineList (ADR-0015) |
| `crates/forge-gcs-sys`, `crates/forge-gcs` | planegcs (solveur de contraintes de FreeCAD) via FFI + shim C++ (ADR-0013) |
| `crates/forge-cad` | CAO paramétrique sur OpenCascade (MPL-2.0, liaison dynamique) — jalon 3 (ADR-0023) |
| `apps/omnifab-desktop` | Démonstrateur egui : esquisse contrainte, liaisons, imports/exports, viewport 2D/3D |
| `docs/architecture.md` | Schéma cible des crates et invariants de dépendances (garde-fou) |
| `docs/adr/` | Décisions d'architecture (ADR-0000 à 0028) |
| `docs/error-codes.md` | Registre central des codes d'erreur `OF-<DOMAINE>-<NNNN>` (ADR-0003) |
| `docs/knowledge/` | Copies de référence de la Knowledge du projet : fiches de décisions, archives, feuille de route détaillée (ADR-0028) |

Les crates restantes (forge-paint, forge-cam, forge-gcode, forge-link, omnifab-android, omnifab-web) seront créées à leur jalon, conformément à l'ADR-0000 : pas de crates vides.

## Développement

```bash
cargo test --workspace        # tests (dont vérification du registre des codes d'erreur)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p omnifab-desktop  # démonstrateur : esquisse, liaisons, viewport, ZIP + rejeu
```

Prérequis natifs (forge-gcs/forge-cad) : CMake ≥ 3.26, Eigen, fmt, Boost, libclang, OpenCascade ≥ 7.8 (dynamique), sous-module `third_party/planegcs` (`git clone --recurse-submodules`).

### Prérequis natifs par OS

La chaîne Rust est pilotée par `rust-toolchain.toml` (stable, Rust ≥ 1.88). Les crates `forge-gcs` (planegcs via FFI) et `forge-cad` (OpenCascade) compilent du C++ : il faut un compilateur C++17, CMake ≥ 3.26, Eigen, fmt, Boost, libclang (bindgen) et OpenCascade ≥ 7.8 en bibliothèque **dynamique**. Cloner avec `--recurse-submodules` (`third_party/planegcs`).

**Linux (Debian/Ubuntu)** :

```bash
sudo apt install build-essential cmake pkg-config clang libclang-dev \
  libeigen3-dev libfmt-dev libboost-dev libocct-*
# vérifier OCCT ≥ 7.8 :
dpkg -l | grep libocctfoundation
```

**Linux (Fedora)** : `sudo dnf install gcc-c++ cmake clang clang-devel eigen3-devel fmt-devel boost-devel opencascade-devel`.

**Windows** : Visual Studio (MSVC C++17) + [vcpkg](https://vcpkg.io/) : `vcpkg install opencascade eigen3 fmt boost`, puis `set VCPKG_TRIPLET=x64-windows` et câbler `CMAKE_TOOLCHAIN_FILE` vers le toolchain vcpkg. Premier build ~30-60 min (cache par triplet ensuite).

**macOS** : `brew install cmake llvm eigen fmt boost opencascade` (le `llvm` de Homebrew fournit libclang).

> Après installation, `cargo build --workspace --locked` doit réussir — c'est le test du checkpoint « build local » qui valide le `Cargo.lock`.

Toutes commandes : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p omnifab-desktop` (démonstrateur).

## Documentation

- [`docs/sommaire.md`](docs/sommaire.md) — **sommaire général** : carte cliquable de toute la documentation et du dépôt.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — conventions de développement obligatoires.
- [`docs/architecture.md`](docs/architecture.md) — architecture cible du workspace (garde-fou).
- [`docs/adr/`](docs/adr/) — décisions d'architecture, avec alternatives rejetées.
- [`docs/audit-protocole.md`](docs/audit-protocole.md) — protocole d'audit du dépôt (à lire par tout auditeur externe avant tout signalement).
- [`docs/error-codes.md`](docs/error-codes.md) — registre central des codes d'erreur.
- [`docs/knowledge/`](docs/knowledge/README.md) — **la Knowledge du projet** : fiches de décisions, état actuel, archives des discussions, journal des fonctions (ADR-0028).

## Développement avec assistance IA

OmniFab est développé avec l'assistance d'outils d'intelligence artificielle (rédaction du code, des tests et de la documentation). Toute contribution est relue, validée et fusionnée par le mainteneur, qui en assume l'entière responsabilité. Les messages envoyés vers d'autres projets (issues, pull requests, commentaires) issus de ce travail portent une mention explicite d'assistance IA.

## Langue

> 🌍 La documentation de ce dépôt est **en français** — version de référence. Les pages d'entrée sont traduites en anglais dans [`docs/en/`](docs/en/README.md) (README, CONTRIBUTING, index des ADR, en-têtes de la matrice), synchronisées par lots ; en cas de divergence, le français fait foi.

---

*Projet personnel — conçu avec soin, publié libre.*
