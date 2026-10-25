# 2026-10-20 — [J2] Lot 1b du moteur de liaisons (phase E) : liaisons v2 dans forge-session

Discussion d'implémentation (exécution libre, gouvernance du 2026-10-19) du lot 1b de la phase E, sur le design validé `ADR-0019-design-phase-E.md`. Suite directe du lot 1a (forge-values, `d83b498`).

## Ce qui a été livré (forge-session + wiring desktop)

- **Schémas d'événements versionnés + migrations** (ADR-0019 §1) : `value.input_set`/`value.expression_set` v2 (+ migration v1→v2 ajoutant `"unit": null`), `sketch.constraint.added` v2 (+ migration cote Distance nombre-nu → `{"type":"fixe","mm":…}`), `value.alias_removed` et `sketch.constraint.updated` nés en v1. La projection accepte v1|v2 pour les événements de liaisons (helper `verifie_schema_parmi`), jamais de lecture en devinant.
- **Commandes** : `SetInput`/`SetExpression` avec `unite: Option<Unite>` et noms à espaces de noms (`verifie_nom_chemin`, ADR-0006), `RemoveAlias` avec `ModeSuppression` (Simple/Cascade/Remplacer, une commande = un groupe d'undo), `AddConstraint` Distance-Expression (nœud système `esquisse.*` + `value.expression_set` en mm dans la même transaction), `sketch.constraint.updated` à identifiant conservé.
- **Cotes pilotées** : `ValeurCote::{Fixe,Expression}` ; `Session::croquis_pour_resolution()` fige chaque cote Expression via le graphe avant de résoudre (le solveur reçoit la valeur, jamais l'expression — ADR-0019 décision 4).
- **Routage projection** : noms sous `PREFIXES_RESERVES` → `Origine::Systeme` ; bras `alias_removed` strict (nœud absent = événement invalide).
- **Desktop** : 9 sites `unite: None`, résolution via croquis figé, cotes Distance en `ValeurCote::Fixe` (demo + atelier), import `ValeurCote`.
- **Tests** : 13 tests dans `tests/liaisons.rs` (unités dans les événements, OF-VAL-0007/0008/0009, Simple/Cascade/Remplacer avec undo/redo en groupe, figeage pour résolution Systeme2D, migration v1→v2 par rejeu, survie de `sketch.constraint.updated` au journal), tests session/projections ajustés (versions refusées 2→3, `unite: None`).

## Convergence CI : 5 rides, l'oracle = les logs des issues

1. `354cf9c` (lot complet, 9 fichiers) : échec compile (**littéraux de chaîne adjacents** — 3e occurrence du piège SVG_DEMO, Rust ne concatène pas) + 8 diffs rustfmt (`fn_call_width` 60 : chaînages/asserts éclatés même sous 100 colonnes ; let-else et use au contraire REJOINTS). Issues #250-252, #255.
2. `403aff0` : littéral unique (message raccourci sous 100 col). #253 : fmt restant.
3. `aae4414` : diffs exacts du fmt.log → **fmt vert** (confirmé par #254 : section fmt vide). #254 : clippy ×4 (dead_code `VERSION_SCHEMA_CONTRAINTE`, collapsible_if ×3) + 2 tests faux (`verifie_nom` refusait les noms à points ; le test undo croyait le journal tronqué — ADR-0008 : les événements défaits restent pour le redo).
4. `a4214ca` : fixes clippy (const référencée, let-chins de `figer_cote`/`migre_contrainte_v2`) + `verifie_nom_chemin` + tests revus en asserts d'état → **tous les tests verts**. #256 : 2 `mut` superflus + la let-chain courte REJOINTE par rustfmt.
5. `40d9f15` : les deux derniers détails → vert attendu.

## Leçons (capitalisées dans la fiche Conventions et la transition)

- **L'oracle rustfmt = le fmt.log CI, jamais la simulation** : une let-chain courte (58 col) est REJOINTE sur une ligne, une longue éclate avec `&&` en début de ligne et `{` seul — imprévisible a priori ; sur échec, copier le diff exact.
- **Le lint-test échoue d'abord à la compile** : clippy/tests ne parlent qu'aux rides suivants (vagues successives) — sans boucle locale, prévoir 3-5 rides pour un lot neuf.
- **Garde statique nouvelle** : détecter les littéraux adjacents (ligne finissant par `"` suivie d'une ligne débutant par `"`) avant tout push — la compile seule les voit sinon.
- Bug réel attrapé par la CI : `verifie_nom` refusait les noms à points sur `SetInput`/`SetExpression` alors que les espaces de noms (ADR-0006) sont le design — les tests liaisons l'ont révélé.

## Reste de la phase E

Lot 2 desktop : panneau de liaisons (largeur/hauteur/diagonale, +alias, pop-up suppression Cascade/Remplacer), dialogue de cote avec expression (design §5/§6) ; lot 3 : convergence ; puis checklist manuelle §7 (validation Raphaël).
