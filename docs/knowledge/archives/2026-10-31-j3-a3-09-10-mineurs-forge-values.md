# [J3] Mineurs forge-values A3-09/10 — pas de nœud fantôme après un refus, unités des dépendants revalidées

**Date :** 2026-10-31 (soir) — **PR :** [#530](https://github.com/RaphOlivier/OmniFab/pull/530) — **Fusion squash :** main `e0a8a17`

## Contexte (audit n°3, deuxième lot des mineurs)

- **A3-09** : set_expression* crée le nœud (Input 0.0) AVANT la validation — une expression refusée sur un nœud NOUVEAU laissait un nœud fantôme : `value("a")` retournait `Ok(0.0)` sur un alias jamais défini.
- **A3-10** : `set_input_unite` ne revalidait pas les dépendants — OF-VAL-0007 se contournait en réécrivant l'unité de la source (entrée `a` lue par `b = a * 2` en mm, repasser `a` en degré était accepté).

## Protocole test-first en trois actes

1. **Rouge `87560cb`** (2 tests poussés seuls) → issues auto [#529](https://github.com/RaphOlivier/OmniFab/issues/529)/[#531](https://github.com/RaphOlivier/OmniFab/issues/531) : le rouge était une **erreur E0382 du test** (`matches!` par valeur → déplacement partiel de `name`), pas les paniques attendues — un rouge de compilation n'est pas un rouge par assertion.
2. **Reprise `203f799`** : test corrigé (`matches!(&lecture, …)` — le motif emprunte), lib revenue à l'état de main → **rouge PAR ASSERTION** constaté par [#532](https://github.com/RaphOlivier/OmniFab/issues/532) avec les deux paniques exactes : « nœud fantôme après un refus : Ok(0.0) » et « le changement d'unité devait être refusé : Ok(()) ». Les runs push de la branche sont annulés par le groupe de concurrency au profit du run PR (#431/#433 annulés, #432 puis #434 arbitres) — l'arbitre de branche est le run PR.
3. **Correctif `148a0d7`** → reprise rustfmt `8bfc43ba` (let-else multi-lignes, [#533](https://github.com/RaphOlivier/OmniFab/issues/533)) + phrase de doc rétablie `cd8203d` → **vert** : run PR #438 CI SUCCESS 13 m 35 s, CI Android #241 SUCCESS.

## Correctifs

- **A3-09** : `set_expression_unite` note `let neuf = !self.names.contains_key(name)` avant `ensure_node` ; au refus de `definir_expression` sur un nœud neuf, `retirer_id(id)` — l'état antérieur tient, plus aucun nœud fantôme. Un refus sur un nœud EXISTANT garde la valeur précédente (comportement inchangé).
- **A3-10** : `set_input_unite` boucle sur `dependants_ids(id)` AVANT de muter le nœud ; tout dépendant direct déclarant une unité différente de la nouvelle refuse par `ValuesError::UniteIncoherente` — la cohérence ne se contourne plus par la source.

## Leçons

- `matches!` sur une valeur par valeur dans un motif avec binding, réutilisée ensuite = E0382 (déplacement partiel) — toujours `matches!(&x, …)`.
- Un rouge « par compilation » n'est pas un rouge par assertion : la reprise retire le correctif le temps de constater les paniques (leçon v48 appliquée), le rouge faible reste documenté par les issues.
- rustfmt refuse le let-else compact (`else { continue; };` sur une ligne) — forme multi-lignes exigée.
- Une PR ouverte fait annuler les runs push de la branche par le groupe de concurrency : l'arbitre de branche est le run PR (synchronize).

## Vérifications et verdicts

- Vert de branche : run PR **#438 SUCCESS 13 m 35 s** + CI Android **#241 SUCCESS**.
- Issues [#529](https://github.com/RaphOlivier/OmniFab/issues/529), [#531](https://github.com/RaphOlivier/OmniFab/issues/531), [#532](https://github.com/RaphOlivier/OmniFab/issues/532), [#533](https://github.com/RaphOlivier/OmniFab/issues/533) fermées avec motifs différenciés (rouge test-first attendu / reprise E0382 du test corrigé / rouge par assertion / reprise rustfmt), 0 issue ouverte.
- Run main ci.yml **#439** sur `e0a8a17` : à relever (arbitre final).
- Branche `fix/a3-09-10-mineurs-forge-values` à supprimer par Raphaël.

*Discussion menée avec l'assistance de Vibe (IA) — relu et validée par Raphaël, mainteneur OmniFab.*
