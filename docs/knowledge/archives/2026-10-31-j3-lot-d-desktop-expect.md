---
name: 2026-10-31-j3-lot-d-desktop-expect
description: "Lot D de la campagne expect : les 7 expect de production d'omnifab-desktop deviennent des invariants commentés — messages uniquement, zéro changement de comportement."
---

# [J3] Lot D desktop expect — 7 messages d'invariant préfixés, graine hors lot

**Date** : 2026-10-31 (soir) — **PR** : #562 (branche `fix/expect-d-desktop`) — **fusion squash** : `ea00b75` — **fiche prep** : `preparation/patchs-expect.md` (lot D).

## Contexte

Quatrième lot exécuté de la campagne expect (ordre de la fiche prep v24 : C → A → B/D → E). Après C (forge-session : le seul vrai chemin de panique, devenu refus typé), A (forge-render : frames sautées, poison assumé) et B (forge-io : 11 messages d'invariant), restent les expect de production du démonstrateur desktop — tous des invariants par construction : le FTL est embarqué (`include_str!`) et pincé par les tests de complétude de la CI (`toutes_les_cles_existent`, `toutes_les_pages_d_aide_sont_completes`).

## Sites corrigés (7)

- `i18n.rs` ×1 : FTL embarqué, pincé par les tests i18n.
- `aide.rs` ×3 : clés d'aide vérifiées par toutes_les_cles_existent (CI).
- `main.rs` ×3 : fonctions `texte`, `texte_avec` et `main` — même clé FTL vérifiée par la CI.

Tous reçoivent le préfixe « invariant : » avec la justification locale (décision 2 de l'inventaire-expect). Messages uniquement, zéro changement de comportement — aucun rouge test-first possible, cas analogue aux invariants des lots A et B.

## Graine de démo hors lot (décision 4)

`preparer_demo` (8 sites : svg de démo, tracés importés, coin posé, côté posé, contrainte posée, cote posée ×2, coin fixé) reste inchangée : la décision 4 de la fiche prep la reporte au découpage main.rs. Le lot E posera une échappatoire datée le temps du découpage.

## Déroulé

Branche `fix/expect-d-desktop` depuis main ; PR #562 draft puis ready ; vert de branche intégral sur la tête `0c89f25` — lint-test (fmt+clippy+tests+rustdoc) ✅, build ubuntu ✅, build windows ✅, build macos ✅, check aarch64 ✅ (windows vert AVANT la fusion) ; fusion squash → main `ea00b75`. 0 issue auto ouverte à la fusion. Journal des fonctions inchangé : des messages, pas une fonction.

## Leçons

- Les expect du démonstrateur sont tous des invariants de catalogue : la CI (tables de complétude) est le véritable garde, pas le message d'expect.
- La graine de démo est un scénario scripté, pas un chemin utilisateur : la traiter avec le découpage main.rs évite de mélanger messages et restructuration.

## Suite

Lot E : clippy `expect_used` workspace warn (spec fiche prep — workspace lints, [lints] workspace = true, allow build.rs daté, clippy.toml allow-expect-in-tests, échappatoires des invariants et de la graine), puis bascule deny au lot suivant, puis découpage main.rs.
