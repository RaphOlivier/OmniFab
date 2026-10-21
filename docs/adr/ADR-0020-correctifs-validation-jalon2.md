# ADR-0020 : correctifs de la validation manuelle du jalon 2 (§7 phase E)

- **Statut :** accepté
- **Date :** 2026-10-20
- **Décideurs :** Raphaël Olivier (validation manuelle §7, choix tranchés en réponse au digest)

## Contexte

La checklist de validation manuelle §7 de la phase E (ADR-0019) a révélé, au-delà des bugs d'exécution (calque d'aide volant le clic gauche des boutons actifs, fenêtres centrées loin du curseur, code d'erreur terminal enfoui dans l'aide), trois comportements qui relèvent du design et ne pouvaient pas être corrigés sans décision :

1. **Arcs à sens unique** : l'arc balaie toujours dans le sens trigonométrique — une décision de la validation jalon 2 (l'arc ne devait pas changer de forme selon le côté du troisième clic). Le besoin réel est de choisir le sens.
2. **Grille 1-2-5 incohérente au zoom** : le pas adaptatif saute de 2 à 5 (et inversement) — 5 n'est pas un multiple de 2, les lignes de la grille changent de place d'un zoom au suivant et l'aimantation suit, ce qui dérange la lecture.
3. **Cycle traversé accepté puis gelé** : `a = b` puis `b = a` était accepté à la définition (seule l'auto-référence était refusée), le cycle n'éclatait qu'à la lecture — les deux alias partaient en erreur et la valeur précédente ne tenait plus, contrairement à l'attente « refus/erreur claire, aucune valeur gelée » du test §7.4.

## Décision

1. **L'arc porte un balayage signé, le sens suit le geste du curseur.** `angle_fin` est réinterprété en v2 comme `début + balayage` signé : positif dans le sens trigonométrique, négatif dans le sens horaire ; le geste accumule le balayage au fil du déplacement du curseur autour du centre et le troisième clic pose l'arc dans le sens suivi. Migration `sketch.entity.added` v1 → v2 : les arcs v1 (toujours trigonométriques) sont réencodés `fin = début + balayage positif` — visuel inchangé.
2. **Le pas de grille est dyadique** : la plus petite puissance de deux atteignant la cible en pixels remplace la série 1-2-5 — chaque pas est le double du précédent, toute ligne survit au zoom dans la ligne suivante, l'aimantation ne saute plus.
3. **Un cycle traversé est refusé à la définition** (`prepare_expression` remonte `OF-VAL-0001` avec le chemin du cycle avant toute modification) : la valeur précédente tient, rien n'entre au journal. L'erreur reste différée à la lecture pour les fautes d'arithmétique (ADR-0002 inchangé sur ce point) — seules les erreurs **structurelles** (cycles) sont refusées à la pose.

Les bugs d'exécution constatés sont corrigés dans le même lot sans décision : calque d'aide posé uniquement sur les widgets désactivés, fenêtre d'aide et dialogue de cote ouverts près du curseur, page d'aide liée au **dernier** code du message, sélection étendue aux cercles et arcs, surbrillance de la sélection sur toutes les entités, panneau de liaisons redimensionnable, rectangle de démonstration réduit à 240 × 160 mm.

## Alternatives rejetées

- **Arc : garder le sens trigonométrique** (statu quo jalon 2) — contredit le besoin exprimé à la validation ; **arc en trois points de passage** — plus intuitif à terme mais il change le geste et l'entité pour un gain nul ici (le balayage signé couvre le besoin avec la même entité) ; **champ `sens` booléen séparé** — deux sources de vérité là qu'un balayage signé suffit.
- **Grille : pas fixe de 1 mm indépendant de l'affichage** — l'aimantation ne suit plus la grille affichée en zoom arrière, pire pour la lecture ; **série 1-2-5 avec hystérésis** — atténue le flottement à la frontière mais ne répare pas l'alignement des lignes.
- **Cycle : refus à la lecture uniquement** (statu quo) — une valeur déjà acceptée puis journalisée contredit « aucune valeur gelée » ; **détection à l'évaluation seul** — trop tard : le journal est déjà pollué.

## Conséquences

- Les journaux v1 s'ouvrent et se rejouent via le registre de migrations ; les arcs migrés gardent leur visuel d'origine.
- Un projet sauvé pendant la validation avec un cycle accepté par l'ancien comportement peut échouer au rejeu de cet événement : à re-valider depuis un état sain.
- planegcs reçoit des angles de fin hors (-π, π] : sans incidence — aucune contrainte ne référence un arc, les règles internes du solveur lient les extrémités aux angles comme paramètres bruts.
- La sélection de cercles est pour l'instant visuelle : les contraintes sur cercles (tangentes, rayon piloté) restent à faire en jalon 3.
