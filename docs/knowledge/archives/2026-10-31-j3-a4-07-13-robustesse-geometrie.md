# [J3] Hygiène A4-07/13 : robustesse géométrique — zoom gardé, borne de magnitude (OF-GEO-0003), saturation usize, garde ADR-0020

**Date :** 2026-10-31 (soir) — **PR [#547](https://github.com/RaphOlivier/OmniFab/pull/547), fusion squash → main `cc7e1d0`** ; vert de branche prouvé par les check runs du run PR #468 (tête `a01a5fc`) : lint-test fmt+clippy+tests+rustdoc ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en cours, non arbitre).

## Contexte

Quatre signalements d'hygiène de l'audit n°4 :

- **A4-07** : le balayage signé des arcs (ADR-0020, migration v2) pouvait diverger de l'ancienne lecture trigonométrique v1 — à prouver par une garde de non-régression avant toute retouche du dessin d'arcs.
- **A4-13a** : un zoom à facteur non fini (NaN/±∞) ou ≤ 0 empoisonnait la caméra (l'échelle suivante hérite du facteur pourri).
- **A4-13b** : forge-geom acceptait des coordonnées finies de magnitude astronomique (1e300 mm) — rendu et solveur exploitaient ensuite des nombres dégénérés.
- **A4-13c** : le `From<usize>` du catalogue forge-i18n wrappait silencieusement un identifiant énorme en `i64` négatif.

## Protocole test-first (rouge constaté AVANT correctif)

- Rouge `6c06fc3` (5 fichiers de tests) → run #463 FAILED 8 m 5 s → issue auto [#546](https://github.com/RaphOlivier/OmniFab/issues/546) : panique exacte « point_hors_borne_refuse » (unwrap_err sur Ok — un Point2 à 1e300 mm était accepté) + diff fmt (double ligne vide esquisse.rs).
- Correctifs `67ac89c` (A4-13a/c) puis `d6a71ac` (A4-13b + garde A4-07) ; PR #547 ouverte → run PR #466 (merge ref `f895d4a`) FAILED 6 m 14 s → issue [#548](https://github.com/RaphOlivier/OmniFab/issues/548) : deux reprises — fmt esquisse.rs (accolade fermante de raison_finie retombée en colonne 0) et le test `registre_coherent_dans_les_deux_sens` (« code OF-GEO-0003 du registre inconnu de GeomError » — le test énumère les variantes en dur).
- Reprise `a01a5fc` → run PR #468 : vert (check runs lint-test/ubuntu/macos/aarch64/APK ✅).

## Correctifs

1. **A4-13a — zoom gardé** : camera.rs et camera3d.rs ignorent un facteur non fini ou ≤ 0 (garde documentée) — la caméra ne peut plus être empoisonnée.
2. **A4-13b — borne de magnitude** : forge-geom `MAGNITUDE_MAX_MM = 1e9` (pub const) contrôlée dans les constructeurs Point2/Point3 (variante `GeomError::CoordonneeHorsBornes`, code **OF-GEO-0003** — registre + docs/error-codes.md + test bidirectionnel enrichi) ; forge-session en miroir `BORNE_COORDONNEE_MM` (duplication à dessein : forge-session ne dépend pas de forge-geom), `raison_finie` réécrite en cascade if/else-if (non fini, puis hors borne) — refus au plan (raison_refus_entite/raison_refus_deplacement) comme pour un non-fini.
3. **A4-13c — saturation usize** : `From<usize>` du catalogue forge-i18n sature à `i64::MAX`.
4. **A4-07 — garde ADR-0020** : verdict NON DIVERGENT encodé en deux tests de non-régression dans atelier_esquisse.rs (le balayage 270°→90° passe par 180° — l'arc suit le geste ; le balayage nul dessine le cercle entier).

## Leçons

1. **Chirurgie d'accolades** : l'indentation de l'accolade fermante d'une fonction vit DANS la région remplacée — remplacer le corps sans la fin de ligne fait retomber l'accolade en colonne 0 (diff fmt de #548).
2. **Test de cohérence bidirectionnel** : `toutes_les_variantes()` énumère les variantes en dur — toute nouvelle variante d'erreur doit toucher le registre ET ce test (précédent #538) ; un test qui codifie la spec évolue avec elle.
3. Le run PR teste le merge ref (sha ≠ tête de branche : `f895d4a` pour `d6a71ac`) ; le run push est annulé par le groupe de concurrency au profit du run PR — l'arbitre de branche est le run PR.
4. La garde vit à la frontière : un point finie mais hors borne est refusé au plan comme un non-fini, dans la continuité des gardes A3-08 (forge-cad) et A3-17 (forge-units).

## Clôture

Issues #546/#548 fermées completed avec motifs différenciés ; 0 issue ouverte ; run main ci.yml sur `cc7e1d0` à relever (arbitre final) ; branche `fix/a4-07-13-robustesse-geometrie` à supprimer par Raphaël. Journal des fonctions : +4 lignes (F:render:zoom-garde-facteur, F:geom:borne-magnitude, F:i18n:usize-saturation, F:esquisse:garde-balayage-adr0020). Suite : hygiène A4-08, A4-09, A4-10, A4-12 (restes).
