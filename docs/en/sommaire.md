> **English translation.** The French documentation is authoritative: [`docs/sommaire.md`](../sommaire.md). Translated as of 2026-10-26 — the French original is updated continuously and may be ahead. All links below point to the authoritative French files.

# General table of contents — the project map

> **Role of this page:** a clickable overview of **all** the documentation and the entire repository — every entry links to the corresponding file with a one-line explanation. Keep it open as a wayfinding map; the details live in each document.

## Entry pages

| File | Contents |
|---|---|
| [`README.md`](../../README.md) | Project presentation: roadmap, vision, technical stack, repository layout, per-OS build |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | Mandatory development conventions: status, native prerequisites, branches/PRs, regression tests |
| [`AGENTS.md`](../../AGENTS.md) | Contribution rules for AI agents (tooling, conventions, limits) |
| [`LICENSE`](../../LICENSE) | GPL-3.0+ — application license (isolated crates: MPL-2.0, see `crates/*/LICENSE`) |

## Documentation — `docs/`

| Document | Contents |
|---|---|
| [`architecture.md`](../architecture.md) | Target workspace architecture: crate map, dependency invariants (guardrail) |
| [`matrice-fonctionnalites.md`](../matrice-fonctionnalites.md) | Exhaustive inventory of ≈ 277 features × ~60 reference software (anti-omission guardrail — French, [English headers](matrice-fonctionnalites.md)) |
| [`prompt-fondateur.md`](../prompt-fondateur.md) | The founding vision in 15 points + annexes (French) |
| [`error-codes.md`](../error-codes.md) | Central registry of typed error codes `OF-<DOMAIN>-<NNNN>` (CI-checked) |
| [`audit-protocole.md`](../audit-protocole.md) | Repository audit protocol — read by any external auditor before filing findings (French) |
| [`ci-cargo-deny.md`](../ci-cargo-deny.md) | Supply-chain audit of dependencies: licenses/CVE via `deny.toml` (CI) |
| [`adr/`](../adr/ADR-0000-decisions-initiales-phase-0.md) | All architecture decision records, with rejected alternatives — [English ADR index](adr.md) |

## Knowledge — the *why* of the project (`docs/knowledge/`, French)

Public reference copy of the decision notes (ADR-0028): reasoning, rejected alternatives, history.

| Note | Contents |
|---|---|
| [`README.md`](../knowledge/README.md) | Reading guide (recommended paths, conventions) |
| [`KNOWLEDGE.md`](../knowledge/KNOWLEDGE.md) | General index: what each note covers |
| [`etat-actuel/`](../knowledge/etat-actuel/KNOWLEDGE.md) | One-page status report, updated every work batch: what compiles, CI state, open debt |
| [`architecture/`](../knowledge/architecture/KNOWLEDGE.md) | Crate structure, six technical locks, milestones 0-10, ADR summaries |
| [`conventions/`](../knowledge/conventions/KNOWLEDGE.md) | Conventions: strict units, i18n, licenses, branches/PRs, documentation language |
| [`formats/`](../knowledge/formats/KNOWLEDGE.md) | Interoperability matrix by domain (SVG, STEP, glTF, G-code…) |
| [`machines/`](../knowledge/machines/KNOWLEDGE.md) | Controllers (LinuxCNC first), multi-machine, G-code editor |
| [`securite/`](../knowledge/securite/KNOWLEDGE.md) | Threat model and security rules per milestone |
| [`historique/journal-fonctions.md`](../knowledge/historique/journal-fonctions.md) | Append-only function journal (grep on `F:<domain>:<function>`) |
| [`archives/`](../knowledge/archives/KNOWLEDGE.md) | Dated summaries of significant discussions |

## Code

| Path | Role |
|---|---|
| [`crates/forge-units`](../../crates/forge-units) … [`crates/forge-cad`](../../crates/forge-cad) | The 11 workspace crates — see the [repository structure table](README.md) in the English README for the role of each |
| [`apps/omnifab-desktop`](../../apps/omnifab-desktop) | egui demonstrator: sketch, bindings, viewport, imports/exports |
| [`apps/omnifab-android`](../../apps/omnifab-android) | Android target (aarch64 APK, same core — milestone 7) |
| [`third_party/`](../../third_party) | Third-party submodules (planegcs) |

## CI — `.github/workflows/`

| Workflow | Role |
|---|---|
| [`ci.yml`](../../.github/workflows/ci.yml) | Multi-OS lint + tests (Linux x86_64/aarch64, Windows, macOS) — `--locked`, clippy, fmt, code registry |
| [`android.yml`](../../.github/workflows/android.yml) | Android APK build (aarch64) + signing + artifact |
| [`cargo-lock.yml`](../../.github/workflows/cargo-lock.yml) | Automatic `Cargo.lock` completion on `Cargo.toml` changes |
| [`galerie.yml`](../../.github/workflows/galerie.yml) | Headless screenshots of the demonstrator, archived on the `galerie` branch (visual history) |
| [`liens-md.yml`](../../.github/workflows/liens-md.yml) | Markdown link checker (workflow_dispatch) |

---

> 🗺️ This page is maintained together with the repository structure: any new document, crate or workflow must appear here. Authoritative French version: [`docs/sommaire.md`](../sommaire.md).
