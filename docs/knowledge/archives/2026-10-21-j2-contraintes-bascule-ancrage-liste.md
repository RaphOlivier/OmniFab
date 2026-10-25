# [J2] Contraintes : bascule Hor/Ver, ancrage des points, liste des contraintes (ADR-0021) — 2026-10-21

## Contexte

Re-validation §7 par Raphaël après le lot ADR-0020 (base `b943398`) — trois retours sur les contraintes de l'esquisse (verbatim) :
1. « Contraite hor/ver fonctionne mais cliquer l'un apres l'autre écrase la ligne »
2. « distance ouvre un panneau mais n'applique pas de contraite »
3. « fixer point ne fonctionne pas »

## Diagnostic

- **Hor/Ver** : poser Verticale après Horizontale ajoutait une SECONDE contrainte sur la même ligne → système surcontraint, ligne « écrasée » à la résolution. Décision de design à trancher (deux orientations opposées sur une même ligne).
- **Distance** : la contrainte ÉTAIT posée (valeur par défaut = distance courante) mais INVISIBLE — aucune liste des contraintes posées dans l'UI ; Raphaël voyait « fenêtre fermée, rien ne bouge ».
- **PointFixe** : la contrainte était posée mais `saisir_poignee` saisissait quand même un point ancré → le point reprenait sa graine à la résolution suivante, d'où « ça ne fonctionne pas ».

## Décisions tranchées par Raphaël (digest ask_user_question)

1. **Bascule** : poser le genre opposé sur une ligne déjà contrainte → **remplacer** la contrainte existante (pas d'empilement, pas de refus).
2. **Diagnostic Distance confirmé** : contrainte posée mais invisible (pas un bug de pose).
3. **Retour visuel voulu** : « Liste seule avec une surbrillance momentanée ».
4. **Point ancré** : **refuser le glisser** (le clic seul sélectionne toujours).

## Implémentation (ADR-0021, 7 fichiers, 2 rides)

- **D1 bascule** — `AddConstraint::plan` (forge-session `command.rs`, ~l. 526) : le genre opposé sur une ligne déjà contrainte émet `EVENT_SKETCH_CONSTRAINT_UPDATED` (schéma v1) à IDENTIFIANT CONSERVÉ (`contrainte_en_json(id_existante, …)`) ; reposer le MÊME genre → refus OF-SES-0001 « la ligne {ligne} porte déjà une contrainte {genre} ». Règle au plan SEULEMENT, jamais dans `raison_refus_contrainte` — les vieux journaux portant deux orientations restent rejouables (compatibilité du rejeu).
- **D2 liste** — panneau esquisse (`atelier_esquisse.rs`, ~l. 606) : liste `contraintes()` avec libellés i18n (`libelle_contrainte`, 14 clés FTL `contrainte-libelle-*` + `esquisse-contraintes-poses` + 5 pages d'aide) ; surbrillance 1,5 s (`DUREE_SURBRILLANCE_S`, champs DemoApp `derniere_contrainte: Option<(u32, f64)>` + `instant` mis à jour chaque trame via `ctx.input(|i| i.time)`, helpers `est_surlignee`/`surbrillance_vivante`, `request_repaint_after` borné) ; `ajouter_contrainte` marque l'id depuis le dernier événement du journal (`data["id"]`).
- **D3 ancrage** — `point_ancre()` pub sur `EsquisseProjection` (forge-session `esquisse.rs`) ; `saisir_poignee` refuse la saisie d'un point ancré (~l. 397, le clic seul sélectionne toujours) ; `MovePoint::plan` refuse → OF-SES-0001 (~l. 637) ; liseré turquoise `COULEUR_ANCRAGE = from_rgb(64, 224, 208)` sur la poignée ancrée dans `dessiner` (boucle poignées du mode Sélection).
- ADR-0021 au dépôt : `docs/adr/ADR-0021-bascule-ancrage-liste-contraintes.md` (copie prep identique).
- Tests : session `la_bascule_d_orientation_remplace_l_opposee` + `le_deplacement_d_un_point_ancre_est_refuse` (`lot1b-tests-session.rs`) ; desktop `la_bascule_depuis_l_atelier_remplace_et_surligne` + `un_point_ancre_ne_se_saisit_plus` (`correctif-desktop-main.rs`) — **62 tests desktop verts**.

## Convergence CI (2 rides)

- `caa81af` (lot complet, 7 fichiers) → issue #271 : fmt SEUL (8 diffs : let-else à initialiseur court REJOINT + appels `ajouter_contrainte` courts rejoints + `assert!` éclaté dans main.rs ; chaînage let-chain retraité + `genre` multi-lignes + `format!` éclaté dans command.rs ; bloc `use` regroupé dans atelier).
- `6c0d02f` (ride 2 : diffs exacts du fmt.log appliqués) → **CI VERTE** (~12 min de silence). Issue #271 fermée avec commentaire de traçabilité + state_reason completed, OPEN = [].

## Leçons fmt du lot

- Un let-else dont l'initialiseur tient sur une ligne est **REJOINT** (même si la chaîne dépasse 60 colonnes).
- Un appel de 3 args courts (~86 col) est **REJOINT** — mais un `assert!` avec args > 60 col est **ÉCLATÉ**.
- Let-chain longue : `=` seul en fin de ligne, puis chaînage retraité un maillon par ligne indenté +4.
- `let genre = if b { "x" } else { "y" };` au-delà de ~100 col : **ÉCLATÉ** multi-lignes.
- `use` : rustfmt ordonne `EVENT_*` entre CroquisResolu et EntiteEsquisse et regroupe plusieurs items par ligne.

## Vigilance

- Les journaux hérités (pré-ADR-0021) peuvent encore porter deux orientations sur une même ligne : le rejeu les ACCEPTE (la règle vit au plan, pas à la projection) et la ligne reste écrasée à la résolution — un tel projet est à refaire depuis un état sain.

## Prochaines étapes

- Re-test manuel §7 de Raphaël sur `cargo run -p omnifab-desktop` (base `6c0d02f`) : bascule Hor↔Ver sur la même ligne, liste « Contraintes posées » + surbrillance de la dernière posée, point ancré (liseré turquoise, glisser refusé).
- Puis lot d'ADR jalon 3 (~28-31 oct 2026).

## Liens

- ADR-0021 : `docs/adr/ADR-0021-bascule-ancrage-liste-contraintes.md` (dépôt), `preparation/ADR-0021-bascule-ancrage-liste-contraintes.md` (prep) ; entrée du registre dans la fiche Architecture.
- Transition : `preparation/transition-2026-10-19-jalon2-suite.md` (item 4, sous-puce ADR-0021).
