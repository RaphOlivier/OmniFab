---
name: 2026-11-01-j3-lot-e-clippy-expect-workspace
description: "Lot E de la campagne expect : le lint clippy expect_used passe à warn au niveau workspace, les 22 invariants restants portent #[expect] daté, échappatoires datées pour build.rs, la graine de démo, l'exemple du spike et les helpers de tests d'intégration."
---

# [J3] Lot E clippy expect_used workspace — lint à warn, 22 #[expect], échappatoires datées

**Date** : 2026-11-01 (nuit) — **PR** : #563 (branche `fix/expect-e-clippy-expect-used`) — **fusion squash** : `2fd1e0e` — **fiche prep** : `preparation/patchs-expect.md` (lot E).

## Contexte

Cinquième et dernier lot de la campagne expect (ordre C → A → B → D → E). Les lots précédents ont corrigé ou préfixé tous les expect de production ; le lot E pose le garde lui-même : le lint `expect_used` est activé au niveau du workspace, la CI refuse tout NOUVEAU expect sans `#[expect]` ni échappatoire datée. Test-first par nature : le commit qui active le lint est le test, le vert prouve le balayage.

## Contenu poussé

- `[workspace.lints.clippy]` `expect_used = "warn"` à la racine du Cargo.toml + `[lints] workspace = true` dans les 13 membres.
- `clippy.toml` : `allow-expect-in-tests = true` (couvre les fns `#[test]` et les modules `#[cfg(test)]`).
- 22 `#[expect(clippy::expect_used)]` au niveau le plus étroit : forge-io stl ×7, svg ×2, dxf ×2 ; forge-render scene ×2, scene3d ×2 ; omnifab-desktop i18n ×1, aide ×3, main ×3.
- Échappatoires datées « Convention expect v2 (2026-11-01, lot E) » : `build.rs` (forge-gcs-sys), graine `preparer_demo` (décision 4 — reprise au découpage main.rs), exemple `spike_phase_a`, et les 10 fichiers de tests d'intégration à helpers.

## Leçon du lot : allow-expect-in-tests ne couvre pas les helpers

`allow-expect-in-tests` ne couvre QUE les fns `#[test]` et les modules `#[cfg(test)]` — PAS les helpers des fichiers d'intégration `tests/` (`fn point`, `fn triangle`, `fn mm`, `fn croquis_pilote`, `fn dossier_temporaire`…). Premier échec constaté : `forge-io/tests/svg.rs:12`, par l'issue auto #565. Balayage exhaustif des 31 fichiers `tests/` : 10 fichiers fautifs, corrigés par un `#![allow]` daté en tête de fichier (bornes, finite, suppression, dxf, stl, svg, graine_rejeu, liaisons, session, sauvegarde) ; le reste est propre (croquis dorés gcs = `?`, forge-doc = `unwrap`).

## Déroulé

Branche `fix/expect-e-clippy-expect-used` ; PR #563 draft puis ready. Rouge initial #564 : 12 expect de l'exemple `spike_phase_a` (`allow-expect-in-tests` ne couvre pas `examples/` non plus), corrigé par `6084651`. Rouge #565 (helper svg.rs:12), corrigé par `16e855c`+`6e0c632`. Vert de branche sur la tête `6e0c632` : lint-test (fmt+clippy+tests+rustdoc) ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en cours à la fusion, non arbitre, patron établi) ; fusion squash → main `2fd1e0e`. 0 issue auto ouverte à la fusion. Journal des fonctions inchangé : un lint et des échappatoires, pas une fonction.

## Leçons

- `clippy --workspace` s'arrête à la première crate en échec : tant que l'exemple du spike échouait, les crates suivantes n'avaient jamais été vérifiées — le balayage statique (recherche + lecture par ref) reste la vérité, l'incrément CI ne révèle qu'une crate à la fois.
- Le mécanisme d'issue auto exige 0 issue « CI : » ouverte : fermer l'ancienne AVANT d'attendre le rapport d'un nouvel échec, sinon le canal reste muet.
- Un push crée 2 runs (push + pull_request synchronize) : le run décisif est le synchronize, vérifié par le sha de tête et l'horodatage.

## Suite

Bascule deny (`expect_used` "warn" → "deny" dans le Cargo.toml workspace) + bump v2 de la fiche `preparation/patchs-expect.md` (miroir privé, liste datée des sites survivants), puis découpage main.rs (fiche `preparation/decoupage-main-rs.md` — la graine `preparer_demo` reprise à l'étape demo).
