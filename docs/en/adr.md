# Architecture Decision Records (ADR) — English index

> **English translation.** The French documentation is authoritative. Translated as of 2026-10-26 (commit `74cd704`) — the French originals are updated continuously and may be ahead. Each entry links to the **French original**; the one-line summaries below are informative only.

The full ADRs are written in French in [`docs/adr/`](../adr/). Every settled structural decision gets an ADR with its rejected alternatives.

| ADR | Subject (summary) |
|---|---|
| [ADR-0000](../adr/ADR-0000-decisions-initiales-phase-0.md) | Initial phase-0 decisions: stack (Rust/egui/wgpu/OCCT/planegcs), six architecture locks, no empty crates, GPL-3.0+ |
| [ADR-0001](../adr/ADR-0001-format-evenements.md) | Event format: versioned ZIP container, JSON Lines journal, schema-versioned events |
| [ADR-0002](../adr/ADR-0002-evaluation-forge-values.md) | forge-values evaluation: lazy, deterministic (no clock/random/disk), replay-safe |
| [ADR-0003](../adr/ADR-0003-erreurs-typees.md) | Typed error codes `OF-<DOMAIN>-<NNNN>`, central registry checked by CI |
| [ADR-0004](../adr/ADR-0004-binding-occt.md) | OCCT binding strategy: opencascade-rs first, isolated behind forge-cad; targeted cxx wrapper only as a bounded exception |
| [ADR-0005](../adr/ADR-0005-outillage-build-android.md) | Build tooling, Android APK CI, caching |
| [ADR-0006](../adr/ADR-0006-langage-expression.md) | Expression language: deterministic subset, Unicode identifiers, `^`, `si(cond; a; b)`, depth limit |
| [ADR-0007](../adr/ADR-0007-valeurs-typees.md) | Typed values in two phases: declared units, then typed nodes (milestone 3) |
| [ADR-0008](../adr/ADR-0008-crate-forge-session.md) | forge-session: commands, projections, cursor-based undo/redo, all-or-nothing application |
| [ADR-0009](../adr/ADR-0009-forge-io.md) | forge-io: budgeted import/export (SVG/DXF/STL), typed errors before any proportional allocation |
| [ADR-0010](../adr/ADR-0010-i18n-aide-fluent.md) | Fluent i18n + built-in help, keys and help pages checked by tests |
| [ADR-0011](../adr/ADR-0011-integration-desktop.md) | Desktop integration: apps never touch `Document` directly, only commands and projections |
| [ADR-0012](../adr/ADR-0012-decoupage-jalon-2.md) | Milestone 2 decomposition (viewport, sketch, bindings) |
| [ADR-0013](../adr/ADR-0013-planegcs-ffi.md) | planegcs via FFI + C++ shim, isolated in forge-gcs/forge-gcs-sys |
| [ADR-0014](../adr/ADR-0014-quick-wins-phase-B.md) | Phase B quick wins |
| [ADR-0015](../adr/ADR-0015-viewport-wgpu.md) | wgpu viewport: cameras, scenes, TriangleList/LineList pipelines |
| [ADR-0016](../adr/ADR-0016-correctifs-robustesse.md) | Robustness fixes (external audits of 2026-10-17): transactional session, safe saving, hardened container, worker thread, interoperable SVG export |
| [ADR-0017](../adr/ADR-0017-unites-stl.md) | STL units (millimeter convention, tolerance threshold) |
| [ADR-0018](../adr/ADR-0018-esquisse-2D.md) | Constraint-based 2D sketch |
| [ADR-0019](../adr/ADR-0019-liaisons-phase-E.md) | Binding engine forge-values (phase E): v2 event schemas, alias namespaces, expression-driven sketch dimensions, bindings panel |
| [ADR-0020](../adr/ADR-0020-correctifs-validation-jalon2.md) | Milestone-2 validation fixes |
| [ADR-0021](../adr/ADR-0021-bascule-ancrage-liste-contraintes.md) | Constraint toggle, point anchoring, constraint list |
| [ADR-0022](../adr/ADR-0022-decoupage-jalon-3.md) | Milestone 3 decomposition: phases A→E (CAD core → nodal editor → desktop CAD) |
| [ADR-0023](../adr/ADR-0023-forge-cad-occt.md) | forge-cad: OCCT consolidation, dynamic linking mandatory, phase-A risks |
| [ADR-0024](../adr/ADR-0024-croquis-solide-feature-tree.md) | Sketch→solid, extrusion, feature tree with bounded topological naming |
| [ADR-0025](../adr/ADR-0025-valeurs-typees-phase-2.md) | Typed values phase 2: unit arithmetic with Area/Volume derivatives |
| [ADR-0026](../adr/ADR-0026-editeur-nodal-tableau.md) | Nodal editor + semantic spreadsheet |
| [ADR-0027](../adr/ADR-0027-desktop-cao.md) | Desktop CAD space: project tree, File menu, rich 3D rendering |
| [ADR-0028](../adr/ADR-0028-copies-knowledge-depot.md) | Reference copies of the project Knowledge in the repository (`docs/knowledge/`), bump ⇒ repush rule |

> ADR-0016 and ADR-0019 were added to `docs/adr/` on 2026-10-26 — the numbering 0000-0028 is now complete.

For the full reasoning (including rejected alternatives), read the French originals.
