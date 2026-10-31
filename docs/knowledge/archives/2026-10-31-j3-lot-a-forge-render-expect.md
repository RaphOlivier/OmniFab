# [J3] Lot A forge-render expect : renderer absent = frame sautée, poison assumé, invariants préfixés

**Date :** 2026-10-31 (soir) — **Jalon dominant :** J3 — **Statut :** clôturé (PR #557 fusion squash → main `0be73ef`).

## Contexte

Deuxième lot de la campagne expect (fiche `preparation/patchs-expect.md`) : forge-render portait 12 sites `expect()` — 4 sur `resources.get::<Renderer2D>()`, 4 sur les verrous `Mutex` de frame, 4 invariants de garde dans `scene.rs`/`scene3d.rs`.

## Déroulé

1. Branche `fix/expect-a-forge-render` depuis main `4e7ac34`, PR draft #557 (A1/A2/A3 + abandon du test A1 justifié).
2. Rouge de compilation E0596 sur `bb94af2` (issues #558/#559/#560) : les 2 `let verrou` d'écriture de `prepare` devaient être `mut` — `*verrou = Some(Frame2D { … })` exige DerefMut. Correctif `3620ad7`.
3. Vert de branche sur `3620ad7` : lint-test (fmt+clippy+tests+rustdoc) ✅, ubuntu ✅, macos ✅, aarch64 ✅, APK ✅ — le risque fmt des formes let-else multi-lignes et chaînes éclatées (prédites d'après les leçons rustfmt, non exécutées localement) ne s'est PAS matérialisé. Windows en cours à la fusion (non arbitre, patron établi), recoupé au run main #503.

## Arbitrages

- **A1 (renderer absent)** : let-else qui saute la frame — `return Vec::new()` dans `prepare`, `return;` dans `paint`. Le renderer est inséré dans `CallbackResources` à l'initialisation de l'app : son absence est un bug d'app, pas du noyau ; jamais de panique au rendu.
- **A2 (poison de verrou)** : `unwrap_or_else(|poison| poison.into_inner())` — poison = panique déjà survenue ailleurs, on continue (décision inventaire-expect §3.3).
- **A3 (invariants)** : forme éclatée `.expect("invariant : garde len < 2 → continue ci-dessus")` — l'invariant reste un `expect` assumé, le message dit où vit la garde.
- **Test A1 abandonné** : déclencher `prepare` sans renderer exigerait un harnais wgpu complet (device en CI), hors budget du lot ; la garde let-else est inspectable et le comportement trivial. Justification portée dans la PR.

## Verdict

Fusion squash → main `0be73ef` ; issues #558/#559/#560 fermées completed avec motifs différenciés (titres : rouge E0596 corrigé par 0be73ef (PR #557)), 0 ouverte. Run main ci.yml #503 sur `0be73ef` en cours à la capitalisation — verdict au v62.1. Arbitre final du lot C constaté au passage : run main ci.yml #498 sur `4e7ac34` SUCCESS en 6 m 4 s.

## Leçons

- Introduire un `let` pour factoriser un `.lock().expect(...)` en `unwrap_or_else` peut exiger `mut` là où l'expression temporaire précédente n'en avait pas besoin (E0596).
- Les formes rustfmt prédites d'après les leçons (chaîne mesurée après le récepteur, let-else multi-lignes) peuvent passer du premier coup : les leçons capitalisées ont de la valeur prédictive.

## Fiches touchées

etat-actuel v62, journal-fonctions (F:render:renderer-absent-frame-sautee, F:render:poison-assume), archives v27 (cette entrée).

*rédigé avec l'assistance de Vibe (IA) — relu et validé par Raphaël, mainteneur OmniFab (conventions v10-v15).*
