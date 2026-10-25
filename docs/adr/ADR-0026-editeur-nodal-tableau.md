# ADR-0026 — Éditeur nodal (type Xpresso) + tableau sémantique tableur (phase D)

- **Statut : ACCEPTÉ** — lot ADR-0022..0027 validé par Raphaël Olivier le 2026-10-25.
- **Jalon : 3** — CAO paramétrique + éditeur nodal de valeurs.
- **Source :** préparation du 2026-10-22, digest de session du 2026-10-25 (4 axes tranchés, validation du lot).

**Objet : la seconde vue du moteur unique (verrou n°6 : alias/expressions ET nodal = deux vues du même graphe, zéro incohérence) + l'exigence fondateur ADR-0012 (sémantique tableur).**

**Décisions** :
- **D1 — L'éditeur nodal est une VUE du graphe forge-values** : un nœud par entrée/expression/alias (les cotes `esquisse.*` et `cad.*` apparaissent en lecture seule, branchables), arêtes = dépendances (calcul du sous-graphe sale visible), ports typés colorés par type (Number/Length/Angle/Area/Volume — D5 ADR-0025), édition inline des expressions, badge d'erreur au nœud. **Glisser une valeur de l'UI vers le graphe** (fondateur point 6 : depuis un DragValue du panneau de liaisons → crée/branche le nœud) ; drag d'un port vers un DragValue cible = lie le paramètre.
- **D2 — Tableau** : « + Tableau » réserve son préfixe à la création (OF-VAL-0008, mécanisme ADR-0019 déjà en place) ; chaque cellule `<tableau>.<colonne><ligne>` = un nœud du graphe (valeur ou formule), formules inter-cellules et vers/depuis les alias, recalcul en cascade (ADR-0002), copier-glisser de plages = commandes par lot (une commande = un groupe d'undo). L'UI est une grille type tableur (colonnes lettres, lignes numéros) ; l'édition d'une cellule écrit `values.input_set`/`values.expression_set` avec le nom complet — AUCUN mécanisme parallèle (verrou n°6).
- **D3 — Layout et persistance** : position des nœuds = état d'affichage UI, jamais sérialisé au journal (comme la sélection) ; auto-layout par tri topologique au premier affichage, déplacement libre ensuite.
- **D4 — i18n + aide** : chaque widget de l'éditeur et du tableau porte son help_id, pages complètes (ADR-0010), aucune chaîne dans le code.

**Alternatives rejetées** : un graphe nodal séparé du moteur d'expressions (casserait le verrou n°6 — c'est le rejet fondateur) ; le tableau comme type de données dédié (les cellules SONT des nœuds, sinon deux moteurs) ; éditeur colorisé d'expressions (suspens ADR-0019, toujours reporté — l'éditeur nodal apporte déjà la lisibilité des formules) ; sauvegarde des positions au journal (état dérivé).

**Critères de sortie** : le même alias édité dans le panneau, l'éditeur nodal et une cellule de tableau reste cohérent (tests desktop) ; une arête entre ports incompatibles est refusée au dessin ; cycle volontaire en branchant = OF-VAL-0001 ; cascade tableau → cote pilotée → géométrie visible au checkpoint.
