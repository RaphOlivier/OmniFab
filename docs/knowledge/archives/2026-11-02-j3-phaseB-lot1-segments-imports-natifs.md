# [J3] Phase B lot 1 : segments typés, Path2 en chaîne, imports natifs — 2026-11-02

**Jalon dominant : J3.** Ouverture de la phase B du jalon 3 (« croquis → solide », ADR-0022/0024, designs validés le 2026-10-26) : checklist de début de phase livrée puis lot 1 exécuté en test-first dans la foulée. Suite directe de la discussion « Découpage main.rs 6/6 » ; départ main code `3b35f5b` / docs `6459f89`.

## Thème

La phase B remplace les chemins « points aplatis » par de la vraie géométrie : `forge-geom` gagne un `Segment2` typé (lignes, arcs à balayage signé, Béziers), `Path2` devient une chaîne de segments (la fermeture se dérive, plus de drapeau), et les imports SVG/DXF produisent des primitives natives au lieu d'aplatir chaque courbe en polyligne fixe. Lot 1 du design D1 de l'ADR-0024.

## Le fil

### 1. Rouge test-first (le rouge attendu)

- Branche `feat/phase-b-lot1-segments-types` coupée depuis main ; les tests du lot 1 poussés avant l'implémentation : rouge `6c193e5` constaté par les issues auto [#597](https://github.com/RaphOlivier/OmniFab/issues/597) (check aarch64), [#598](https://github.com/RaphOlivier/OmniFab/issues/598) (build ubuntu), [#599](https://github.com/RaphOlivier/OmniFab/issues/599) (build macos), [#600](https://github.com/RaphOlivier/OmniFab/issues/600) (lint-test).

### 2. Correctif `123be8e` : Segment2 dans forge-geom

- `Segment2` : `Ligne`, `Arc` (balayage signé, balayage nul = cercle complet), `Bezier` quadratique/cubique ; discrétisation pilotée par la flèche (tolérance 0,01 mm, budgets publics `MAX_POINTS_DISCRETISATION`/`MAX_PROFONDEUR_SUBDIVISION`), refus typés OF-GEO-0004/0005, chaînage des contours OF-GEO-0006, sérialisation serde pour les sessions ; `Contour2` et `contours()` enchaînent les segments.

### 3. Migration `cd58593` (5 fichiers, roundtrip 5/5 identiques)

- `forge-geom` : `Path2` sans drapeau — champ `segments: Vec<Segment2>`, fermeture dérivée (`is_closed` = le dernier segment rejoint le premier à la tolérance près), `from_points` en compatibilité polyligne (fermeture explicite si écart réel), `points()` = discrétisation bornée à `TOLERANCE_DISCRETISATION_MM`, `discretiser()` = version stricte.
- `forge-io` SVG : les Béziers quadratiques/cubiques passent telles quelles, la fermeture `Z` devient un segment explicite ; la const d'aplatissement à 16 segments disparaît — **écart documenté** : usvg a déjà converti arcs elliptiques et primitives en Béziers cubiques, il n'existe aucun arc natif côté SVG.
- `forge-io` DXF : la boucle d'entités produit des `Vec<Segment2>` natifs — cercle = arc à balayage nul, arcs par span corrigé, bulges du code 42 par les mathématiques du bulge (`arc_de_bulge`) ; les splines restent échantillonnées (de Boor) ; `POINTS_CERCLE`/`SEGMENTS_ARC` supprimées.
- Tests : `tests/svg.rs` réécrit sur 5 cas (courbe après `Z`, quadratique, cubique, cercle primitif, arc — flèche ≤ 0,011 mm) ; `tests/dxf.rs` : cercle et arc deviennent des `Arc` natifs (centre, rayon, points exacts).
- Journal des fonctions : 3 lignes J3 poussées sur la branche (`e60f059`) — `F:geom:segments-types`, `F:geom:path2-segments`, `F:import:arcs-natifs`.

### 4. PR, CI, boucle fix-forward, fusion

- [PR #601](https://github.com/RaphOlivier/OmniFab/pull/601) (« feat : segments typés et imports natifs — phase B lot 1 »), draft puis ready.
- **Boucle fix-forward, cinq couches d'oignon** : chaque run s'arrête à la première erreur, chaque correctif révélait donc la couche suivante.
  1. **Échec fmt sur le run PR de `e60f059`** : lint-test échoué en 2 min 22, logs tronqués aux 120 dernières lignes — diagnostic par revue exhaustive du code. Correctif poussé en `95f670d`… **partiellement faux** : le tri lexicographique du bloc `pub use segments` place en réalité `MAX_POINTS_DISCRETISATION` avant `MAX_PROFONDEUR_SUBDIVISION` (P-O-I < P-R-O, comparaison caractère par caractère), et les chaînes `point_d_angle(...).unwrap_or(*centre)` exigeaient des patterns de struct éclatés.
  2. **Issue [#602](https://github.com/RaphOlivier/OmniFab/issues/602)** (rouge @`95f670d`) : 10 diffs fmt + E0716 (`Vec` temporaire emprunté par `.last()` dans un test) → corrigé `48aa695`.
  3. **Issue [#603](https://github.com/RaphOlivier/OmniFab/issues/603)** (rouge @`48aa695`) : clippy `len_zero` + 2 diffs fmt (tri du bloc use à rétablir, champs du pattern `Arc` de `debut()` un par ligne) + E0689 (`let mut pire = 0.0` ambigu dans un helper de test) → corrigé `b4f7105`, roundtrip vérifié.
  4. **Issue [#604](https://github.com/RaphOlivier/OmniFab/issues/604)** (rouge @`b4f7105`) : fmt et clippy enfin verts ; échec du test `cercle_complet_debut_egal_fin` — `assert_eq` exact sur 1 + 5·cos(π/2) = 1.0000000000000002 en f64 → corrigé `e4fc2a0` (assertion à 1e-9 près, l'ordonnée sin(π/2) = 1.0 exacte reste en `assert_eq`).
  - Les issues #602/#603/#604 ont été fermées **manuellement** avec motif après chaque correctif — le dédoublonnage des issues auto (une issue ouverte du job bloque la suivante) l'exige.
- Vert de branche sur le run PR (tête `e4fc2a0`) : lint-test + ubuntu + macos + aarch64 check + APK + cargo-deny SUCCESS (run PR 41374292933, windows recoupé au run main) — première exécution réelle des tests du lot, forge-geom comme forge-io (SVG/DXF) inclus.
- Fusion squash → main `bd57c4c`. Arbitre main ci.yml : #581 SUCCESS 8 m 48 s, 0 issue ouverte.
- Issues [#597](https://github.com/RaphOlivier/OmniFab/issues/597)/[#598](https://github.com/RaphOlivier/OmniFab/issues/598)/[#599](https://github.com/RaphOlivier/OmniFab/issues/599)/[#600](https://github.com/RaphOlivier/OmniFab/issues/600) fermées **manuellement** avec motif (rouge test-first attendu, corrigé par `123be8e`/`cd58593`).

## Écarts et dettes signalés

- **Perf rendu** : le rendu 2D consomme toujours des polylignes (`Scene2D::depuis_traces` discrétise à chaque affichage) — dette perf reportée au cache clé d'édition ADR-0027 D3, prévu au lot 4 ; consignée dans la note d'exécution de la fiche prep.
- **A3-08.e** (formes_max réel — le binding 0.3.0 n'itère pas les sous-formes) : inchangée, assumée, reportée au lot 2, non re-signalable en audit.
- La dette « discrétisation à tolérance (cercle/arc/flèche) » et le « chaînage de contours » du backlog audit n°2 sont couverts par le présent lot côté géométrie/imports.

## Leçons

1. usvg ne donne accès à aucun arc natif : tout arc elliptique arrive déjà converti en Béziers cubiques — la doc du module le dit au lieu de promettre des « arcs natifs SVG » que le parseur ne peut pas fournir.
2. Le rouge test-first multi-plateformes produit une issue par job (4 issues pour un seul rouge) : fermeture manuelle groupée avec le même motif, aucune fermeture automatique à attendre.
3. La fermeture d'un chemin se dérive des segments (écart à la tolérance) : un drapeau `ferme` porté par la structure serait une seconde source de vérité.
4. La concurrency du ci.yml sérialise le run PR derrière le run de branche (« waiting for another serialized run ») : les checks du SHA mélangent alors les jobs annulés du run push et les jobs non encore démarrés du run PR — ne jamais conclure à un échec sur ce mélange ; l'arbitre reste les issues auto ouvertes plus la page des runs.
5. Une erreur de compilation en masque d'autres : le run s'arrête à la première crate fautive, le pelage d'oignon est inévitable — fermer l'issue auto après chaque correctif pour débloquer le signalement suivant.
6. Jamais d'assertion exacte sur un flottant trigonométrique : 1 + 5·cos(π/2) = 1.0000000000000002 en f64. Un test test-first ne prouve rien avant sa première exécution réelle — chaque run précédent s'était arrêté avant les tests.
7. Le log fmt des issues auto est tronqué aux 120 dernières lignes : le diagnostic sans logs se fait par revue exhaustive du code, et le tri lexicographique de rustfmt compare caractère par caractère — `MAX_POINTS` avant `MAX_PROFONDEUR` (P-O < P-R).

## Fiches touchées

- [état-actuel](https://github.com/RaphOlivier/OmniFab/blob/main/docs/knowledge/etat-actuel/KNOWLEDGE.md) **v72** ;
- fiche archives **v39** (présente entrée) ;
- [journal-fonctions](https://github.com/RaphOlivier/OmniFab/blob/main/docs/knowledge/historique/journal-fonctions.md) : 3 lignes J3 (`e60f059`) ;
- [sommaire](https://github.com/RaphOlivier/OmniFab/blob/main/docs/sommaire.md) : ligne de la présente entrée (règle v7) ;
- [`preparation/ADR-0024-design-phase-B.md`](https://github.com/RaphOlivier/omnifab-prive/blob/main/knowledge/omnifab/preparation/ADR-0024-design-phase-B.md) : note d'exécution lot 1 (écart usvg, dette perf rendu → cache ADR-0027 D3 au lot 4).

## Suite proposée

« [J3] — Phase B : suite du lot 1 puis lot 2 (dette A3-08.e formes_max au lot 2) »
