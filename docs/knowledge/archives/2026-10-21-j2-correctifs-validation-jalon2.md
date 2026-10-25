# [J2] Correctifs de la validation manuelle §7 (ADR-0020) — 2026-10-21

## Contexte

Raphaël a exécuté la checklist §7 de la phase E (ADR-0019, `preparation/ADR-0019-design-phase-E.md`) sur `cargo run -p omnifab-desktop` (base `008f27e`). Sur 11 points : 1, 2, 3, 6, 8, 9, 10, 11 verts ; 4, 5, 7 en échec + une liste de retours d'ergonomie. Trois retours relevaient du **design** (pas de correctif possible sans décision) : arcs à sens unique, grille 1-2-5 incohérente au zoom, cycle `a = b` puis `b = a` accepté puis gelé.

## Décisions tranchées par Raphaël (ADR-0020, réponses au digest)

1. **Sens des arcs : « par rapport à la position du curseur »** → `angle_fin` v2 = `début + balayage` **signé** (positif trigonométrique, négatif horaire) ; le geste accumule le balayage autour du centre ; migration v1→v2 `fin = début + balayage positif` (visuel inchangé). Rejeté : trois points de passage (change le geste et l'entité), champ `sens` booléen (double source de vérité).
2. **Grille : « pas avec multiple »** → série **dyadique** `2^ceil(log2(brut))` — chaque pas est le double du précédent, toute ligne survit au zoom dans la ligne du pas suivant, l'aimantation ne saute plus.
3. **Cycle traversé refusé à la définition** (`prepare_expression` + helper `chemin_vers`, OF-VAL-0001 avec chemin « b → a → b ») : la valeur précédente tient, rien n'entre au journal ; les erreurs arithmétiques restent différées à la lecture (ADR-0002 intact — seules les erreurs structurelles sont refusées à la pose).

Correctifs d'exécution sans décision : calque d'aide uniquement sur widgets **désactivés** (il volait le clic gauche d'Annuler/Rétablir/exports), fenêtre d'aide et dialogue de cote **près du curseur**, page d'aide = **dernier** code du message (un refus de session enveloppe le code de la cause), sélection étendue aux cercles et arcs, surbrillance par entité (couleur d'aperçu, largeur +1,5), panneau de liaisons redimensionnable (min 220 px), rectangle de démo 240×160 (les 800×600 débordaient de l'écran), FTL de l'aide outil arc.

## Comportements voulus par le design (pas des bugs — dits à Raphaël)

- deg sur une longueur refusé (OF-VAL-0007) : unités strictes ADR-0007 phase 1 ; l'arithmétique longueur↔angle (opérateurs ou conversions explicites) est notée pour le jalon 3, avec un indicateur d'unité par alias.
- Suppression de la largeur/hauteur impossible : les deux sont nécessaires au rectangle (et `esquisse.*` est système, jamais supprimable).
- Hauteurs de panneaux automatiques (egui) ; seule la largeur du panneau d'événements est réglable.
- Vue 3D = tracés/maillages ; l'esquisse 2D plane n'y est pas projetée (jalon 4+).

## Implémentation et convergence CI (4 rides)

- `aa36451` (lot complet, 11 fichiers : forge-values lib + tests bindings, forge-session esquisse/command/lib, forge-render scene, desktop aide/atelier_esquisse/main + FTL, ADR-0020 dans `docs/adr/`) → issue #268 : clippy `question_mark` forge-values + 4 diffs rustfmt + test grille à recalibrer (pas 64 mm → 6 segments, 4 verticales).
- `5008fde` (ride 2) → #269 : clippy `filter_next` (`aide_du_code` : `filter().next_back()` → `rfind`), 5 diffs fmt (chaînes pos aide/atelier, `or_else`, `atan2`, `get_temp`), et **un test à attente fausse** : `rebranchement_sans_cycle_accepte` attendait `c = 2` alors que `c = b+1` avec `b = a+1 = 2` vaut 3 — le moteur était correct, l'attente venait du test (2.0 → 3.0).
- `22f3405` (ride 3) → #270 : clippy et fmt enfin verts, 5 tests desktop en échec — **bug réel : le 3ᵉ clic d'arc ne balayait plus** (`app.geste.take()` dans le `match` vide le geste avant `suivre_geste_arc` → balayage 0 → règle « confondu = tour entier » → `angle_fin = 2π`) ; fix : helper `balaie(balayage, precedent, angle)` (repli ]-π, π], borné à un tour) partagé entre le clic et le suivi du curseur ; + 3 tests aux attentes calibrées sur l'ancienne graine 800×600 recalées sur 240×160 (largeur/hauteur rétablies 240/160, diagonale 288,44, cote pilotée 120).
- `b943398` (ride 4) → **CI VERTE** (clippy, fmt, doc, 60 tests desktop, 13 liaisons, bindings verts ; ~15 min de silence, page checks : aarch64 vert). Issues #268-#270 fermées avec commentaire de traçabilité (short SHA du correctif par issue) + state_reason completed, OPEN = [].

## Leçons

- **Vérifier l'arithmétique d'un test qui échoue AVANT de toucher au moteur** : l'oracle CI révèle aussi les tests faux (bindings 2.0 vs 3.0).
- **`take()` dans un `match` vide le geste** : tout helper du bras qui relit `app.geste` sort immédiatement — passer les données du pattern, pas l'état global.
- **Compter les lignes des sections d'issue** : le tail à 120 peut être le log COMPLET (ride 2 : 53 lignes, header « Diff in » visible = tout l'oracle) ou coupé (ride 1 : exactement 120 lignes). Déduire avant de supposer des diffs cachés.
- **Ordre cargo test = paquets alphabétiques** : les tests desktop ne tournent qu'après forge-values — un échec en amont les masque aux rides 1-2 (prévoir les vagues).
- Les attentes de tests doivent suivre la graine de démo : changer la graine (800×600 → 240×160) casse tous les tests qui la codent en dur.
- Le volume knowledge peut être momentanément injoignable (EIO WebDAV) : les copies locales (`cat >` vers /home/user) dépannent pour les pushes.

## Points d'attention pour Raphaël

- **Re-validation §7** demandée sur `cargo run -p omnifab-desktop` (base `b943398`).
- Un projet sauvé pendant la §7 avec un cycle accepté par l'ancien comportement peut échouer à la réouverture — à refaire depuis un état sain.
- Test 4 désormais conforme : `b = c - 1` (cycle b→c→b) refusé à la définition, B garde 1 ; test 5 : OF-VAL-0007 voulu (unités strictes).
- Demandes notées pour jalon 3 : arithmétique d'unités longueur↔angle, indicateur d'unité par alias.

## Liens

- ADR-0020 : `docs/adr/ADR-0020-correctifs-validation-jalon2.md` (dépôt), `preparation/ADR-0020-correctifs-validation-jalon2.md` (prep) ; entrée du registre dans la fiche Architecture.
- Design d'origine : `preparation/ADR-0019-design-phase-E.md` (§7 = la checklist).
- Transition : `preparation/transition-2026-10-19-jalon2-suite.md` (item 4, sous-puce ADR-0020).
