# OmniFab

> **English translation.** The French documentation is authoritative: [`README.md`](../../README.md) · [`CONTRIBUTING.md`](../../CONTRIBUTING.md) · [ADR index](adr.md). Translated as of 2026-10-26 (commit `74cd704`) — the French originals are updated continuously and may be ahead.

[![CI](https://github.com/RaphOlivier/OmniFab/actions/workflows/ci.yml/badge.svg)](https://github.com/RaphOlivier/OmniFab/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](../../LICENSE)

> **🚧 Status: milestone 3 (parametric CAD) in progress — phase A.** Milestones 1 and 2 are delivered (core + undo + SVG/DXF/STL imports/exports, wgpu viewport, constraint-based 2D sketch, binding engine). CI is **temporarily red**: `Cargo.lock` must be regenerated after the `forge-cad` crate (OpenCascade, system installation required) was added — the fix is already planned. Detailed progress tracking (French): [`docs/knowledge/etat-actuel/KNOWLEDGE.md`](../knowledge/etat-actuel/KNOWLEDGE.md).

**OmniFab** is free (libre) software for the full digital fabrication chain: **CAD** (computer-aided design), **CAM** (computer-aided manufacturing), **G-code** generation and **machine control**.

The goal: cover the entire chain — from the idea of a piece of furniture to the machined part — in a single native application, without depending on proprietary software.

## Roadmap and progress

The project advances by **horizontal slices**: deliver one reference case end-to-end (build a piece of furniture — parametric CAD → 2D cutting/engraving → 2.5-axis CAM → LinuxCNC) before broadening.

| # | Milestone | Status |
|---|---|---|
| 0 | Architecture + ADRs + compilable core | ✅ closed (2026-10-12) |
| 1 | Project core + undo + SVG/DXF/STL import/export + built-in help | ✅ closed (2026-10-15) |
| 2 | wgpu viewport + constraint-based 2D sketch + binding engine | ✅ delivered (phases A–E, green multi-OS CI) |
| 3 | Parametric CAD + nodal value editor (forge-cad / OCCT) | ◐ **in progress** — phase A opened (2026-10-25) |
| 4 | 2.5-axis CAM + multi-process + G-code editor + simulator | ⏸ next — frozen until the furniture case is fully built |
| 5 | forge-link: LinuxCNC + multi-machine management | ⏸ frozen |
| 6 | Raster painting + layers + filters | ⏸ frozen |
| 7 | Android (same core, adapted UI) | ⏸ frozen |
| 8 | WASM plugins + community | ⏸ frozen |
| 9 | Web target (WASM): egui web + WebSocket machine bridge | ⏸ frozen |
| 10 | Batch processing + libraries + guided design assistants | ⏸ frozen |

> **Roadmap freeze**: milestones 5 to 10 are **frozen** until the reference-case piece of furniture is built end-to-end — they remain listed as a vision framework, without design or code. The *why* behind each milestone and decision (French): [`docs/knowledge/`](../knowledge/README.md).

Legend: ✅ closed · ◐ in progress · ⏸ frozen (milestones 5-10) / waiting (milestone 4).

## Short vision

- A modular application built from crates, from the parametric model down to machine control.
- **Design assistants** (furniture, cabinets…) defined as *declarative templates* — data, not code.
- **Simulation first**: no physical hardware is required to develop; machine control is validated on a simulator.
- openBIM interoperability: **IFC** import, **DSTV/NC1** export, multi-classification (OmniClass / Uniclass / UniFormat).

## Technical stack

| Domain | Choice |
|---|---|
| Language | Rust |
| UI | egui (mobile plan B: Slint) |
| Rendering | wgpu |
| Geometry | OpenCascade (OCCT) via bindings |
| 2D constraints | planegcs (via FFI) |
| Concurrency | tokio + channels (machine milestones — not active yet, see freeze) |
| License | GPL-3.0+ (MPL/Apache for isolated crates) |

> 📄 **Licenses**: GPL-3.0 at the root (`LICENSE`), MPL-2.0 for each crate (`crates/*/LICENSE`). Data (materials, machines, standards) will be published under dedicated open licenses (ODbL/CC0/CC-BY) — never GPL.

## Repository structure

| Path | Role |
|---|---|
| `crates/forge-units` | Strict unit newtypes (length, angle) — zero dependencies (lock 5) |
| `crates/forge-doc` | Versioned ZIP container + JSON Lines event journal, migrations, non-authoritative snapshots (ADR-0001) |
| `crates/forge-values` | Single binding engine: lazy evaluation + deterministic expressions (ADR-0002) |
| `crates/forge-geom` | Minimal geometry: points, 2D paths, meshes (OF-GEO codes) |
| `crates/forge-io` | SVG/DXF/STL import/export — budgeted parsers, discretized curves (ADR-0009) |
| `crates/forge-session` | Commands, projections, cursor-based undo/redo, all-or-nothing (ADR-0008, ADR-0011) |
| `crates/forge-i18n` | Fluent i18n + built-in help pages, keys checked by tests (ADR-0010) |
| `crates/forge-render` | wgpu 2D/3D rendering: cameras, scenes, TriangleList/LineList pipelines (ADR-0015) |
| `crates/forge-gcs-sys`, `crates/forge-gcs` | planegcs (FreeCAD's constraint solver) via FFI + C++ shim (ADR-0013) |
| `crates/forge-cad` | Parametric CAD on OpenCascade (MPL-2.0, dynamic linking) — milestone 3 (ADR-0023) |
| `apps/omnifab-desktop` | egui demonstrator: constraint sketch, bindings, imports/exports, 2D/3D viewport |
| `docs/architecture.md` | Target crate map and dependency invariants (guardrail) |
| `docs/adr/` | Architecture decision records (ADR-0000 to 0028) — [English index](adr.md) |
| `docs/error-codes.md` | Central registry of `OF-<DOMAIN>-<NNNN>` error codes (ADR-0003) |
| `docs/knowledge/` | Reference copies of the project Knowledge: decision notes, archives, detailed roadmap (ADR-0028) |

The remaining crates (forge-paint, forge-cam, forge-gcode, forge-link, omnifab-android, omnifab-web) will be created at their milestone, per ADR-0000: no empty crates.

## Development

```bash
cargo test --workspace        # tests (including error-code registry validation)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p omnifab-desktop  # demonstrator: sketch, bindings, viewport, ZIP + replay
```

Native prerequisites (forge-gcs/forge-cad): CMake ≥ 3.26, Eigen, fmt, Boost, libclang, OpenCascade ≥ 7.8 (dynamic), submodule `third_party/planegcs` (`git clone --recurse-submodules`).

### Native prerequisites per OS

The Rust toolchain is driven by `rust-toolchain.toml` (stable, Rust ≥ 1.88). The `forge-gcs` (planegcs via FFI) and `forge-cad` (OpenCascade) crates compile C++: you need a C++17 compiler, CMake ≥ 3.26, Eigen, fmt, Boost, libclang (bindgen) and OpenCascade ≥ 7.8 as a **dynamic** library. Clone with `--recurse-submodules` (`third_party/planegcs`).

**Linux (Debian/Ubuntu)**:

```bash
sudo apt install build-essential cmake pkg-config clang libclang-dev \
  libeigen3-dev libfmt-dev libboost-dev libocct-*
# check OCCT ≥ 7.8:
dpkg -l | grep libocctfoundation
```

**Linux (Fedora)**: `sudo dnf install gcc-c++ cmake clang clang-devel eigen3-devel fmt-devel boost-devel opencascade-devel`.

**Windows**: Visual Studio (MSVC C++17) + [vcpkg](https://vcpkg.io/): `vcpkg install opencascade eigen3 fmt boost`, then `set VCPKG_TRIPLET=x64-windows` and wire `CMAKE_TOOLCHAIN_FILE` to the vcpkg toolchain. First build ~30-60 min (then cached per triplet).

**macOS**: `brew install cmake llvm eigen fmt boost opencascade` (Homebrew's `llvm` provides libclang).

> After installation, `cargo build --workspace --locked` must succeed — this is the "local build" checkpoint test that validates `Cargo.lock`.

All commands: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p omnifab-desktop` (demonstrator).

## Documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — mandatory development conventions ([English](CONTRIBUTING.md)).
- [`docs/architecture.md`](../architecture.md) — target workspace architecture (guardrail).
- [`docs/adr/`](../adr/) — architecture decision records, with rejected alternatives ([English index](adr.md)).
- [`docs/audit-protocole.md`](../audit-protocole.md) — repository audit protocol (French; to be read by any external auditor before filing findings).
- [`docs/error-codes.md`](../error-codes.md) — central error-code registry.
- [`docs/knowledge/`](../knowledge/README.md) — **the project Knowledge** (French): decision notes, current state, discussion archives, function journal (ADR-0028).

## Language

> 🌍 The documentation of this repository is **in French** — it is the authoritative version. English translations of the entry pages live in [`docs/en/`](README.md) and are refreshed in batches; in case of divergence, the French original prevails.

---

*Personal project — designed with care, published as free software.*
