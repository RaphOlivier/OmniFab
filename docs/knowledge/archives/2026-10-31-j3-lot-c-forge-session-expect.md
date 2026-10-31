# [J3] Lot C forge-session expect : graine de passe absente au rejeu = refus typé

**Date :** 2026-10-31 (soir) — **Jalon dominant :** J3 — **Statut :** clôturé (PR #555 fusion squash → main `4e7ac34`).

## Contexte

Premier lot de la campagne expect (fiche `preparation/patchs-expect.md`, décisions v2 validées le 2026-10-26) : l'inventaire des 44 `expect()` de production avait isolé UN seul site de forge-session atteignable par un journal hostile — la boucle des graines de passe du rejeu (`esquisse.rs`, ex-L1537).

## Déroulé

1. Test-first : 2 tests poussés (`crates/forge-session/tests/graine_rejeu.rs`) — journal forgé « sketch.entity.added » Point sans « sketch.point.moved » → le rejeu devait résoudre sans panique à l'origine (dof 2) ; témoin légitime avec « sketch.point.moved » → dernière graine.
2. Trois relances CI documentées sur l'issue #556 : (a) fmt — la chaîne de 63 car. après le récepteur `.esquisse().resoudre(&mut solveur).expect(…)` devait être ÉCLATÉE ; (b) à l'inverse le `.expect("résolu")` de 50 car. devait rester UNE LIGNE — leçon : rustfmt mesure la chaîne APRÈS le récepteur, chain_width 60 ; (c) segfault aléatoire OCCT (signal 11) dans forge-cad golden, hors périmètre du lot, flocon — relance `a7c5a3e` → lint-test vert.
3. Correctif : let-else — graine de passe absente → refus typé `GcsError::IdInconnu { contexte: « graine de passe du point {id} absente — journal hostile au rejeu » }` ; 0 expect restant dans le lot.

## Arbitrage (documenté dans la PR)

Le site expect était INATTEIGNABLE par construction : la boucle des graines insère une graine pour tout Point (avec repli origine). Traité en cas garde — pas de nouvelle variante d'erreur, pas de registre touché, contrairement à la pré-spécification initiale (OF-SES à allouer) : l'API existante `GcsError::IdInconnu` portait déjà la sémantique, le contexte la documente.

## Verdict

Vert de branche run PR 41235226694 (tête `a7c5a3e`) : lint-test (fmt+clippy+tests+rustdoc) ✅, ubuntu ✅, macos ✅, aarch64 ✅, APK ✅ ; windows en cours à la fusion (non arbitre, patron établi sur 9 lots), recoupé au run main. Fusion squash → main `4e7ac34` ; issue #556 fermée avec titre « rouge fmt/segfault corrigé par 4e7ac34 (PR #555) ».

## Leçons

- rustfmt chain_width 60 : la largeur de chaîne se mesure APRÈS le récepteur ; 63 car. → éclaté, 50 car. → une ligne.
- Un flocon OCCT (segfault forge-cad golden) se relance par commit plutôt que de suspecter le lot — les tests passaient aux runs précédents.
- L'arbitrage d'un site expect « inatteignable par construction » va au cas garde, pas à la création d'une variante d'erreur : le registre reste stable.

## Fiches touchées

etat-actuel v61, journal-fonctions (F:session:graine-absente-rejeu), archives v26 (cette entrée).

*rédigé avec l'assistance de Vibe (IA) — relu et validé par Raphaël, mainteneur OmniFab (conventions v10-v15).*
