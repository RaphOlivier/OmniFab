# ADR-0002 : Modèle d'évaluation de forge-values (graphe de dépendances)

- **Statut : accepté**
- **Date : 2026-10-10**
- **Décideurs : Raphaël Olivier**

## Contexte

Le verrou nº6 (ADR-0000) impose un moteur unique de liaisons : un graphe de dépendances réactif + expressions, exposé par deux interfaces (alias/expressions et éditeur nodal). Le point ouvert : quand et comment le graphe recalcule-t-il ?

## Décision

Évaluation **paresseuse avec marquage dirty** : quand une valeur change, les nœuds dépendants (directement ou transitivement) sont marqués « périmés » et rien n'est recalculé ; le recalcul d'un nœud a lieu à la demande, en remontant ses dépendances sales. L'ordre de recalcul est un tri topologique du sous-graphe sale (détection de cycles = erreur typée `OF-VAL-0001`, jamais une boucle infinie). Les nœuds sont de petits blocs purs (expressions, fonctions, références) ; les expressions sont parsées dans un sous-ensemble déterministe (pas d'horloge, pas d'aléa, pas d'accès disque) pour garantir le rejeu identique exigé par l'event sourcing. L'invalidation est **pull-based** sur la marquage dirty, mais l'UI s'abonne à des « zones » du graphe (le viewport, le panneau de propriétés) pour ne redessiner que ce qui a effectivement changé.

## Alternatives rejetées

- **Réactivité en push continu (type signaux/solid-js)** : recalculs en cascade à chaque changement, y compris pour des nœuds dont personne ne lit le résultat ; pics de charge et interface figée sur les gros modèles paramétriques. Le push reste utilisé **localement** pour la notification de l'UI.
- **Recalcul complet à chaque frame** : simple mais gaspillage majeur — incompatible avec le but de fluidité.
- **Évaluation à la demande sans marquage (pur pull naïf)** : oblige à re-vérifier la fraîcheur de chaque nœud lu ; le marquage dirty donne le même résultat avec un coût proportionnel au changement.

## Conséquences

- **Positives** : coût proportionnel à ce qui est réellement consulté ; pas de pics de charge ; déterminisme compatible event sourcing ; l'éditeur nodal et les alias sont deux vues identiques du même graphe.
- **Négatives / coûts acceptés** : un nœud sale non lu retarde l'apparition d'une erreur (l'erreur apparaît au moment de la lecture — il faut une commande « recalculer tout » pour la validation avant usinage).
- **Décisions futures suspendues** : langage d'expression exact (grammaire, bibliothèque de fonctions) ; évaluation multi-threadée des branches indépendantes ; mémoire de cache par nœud.
