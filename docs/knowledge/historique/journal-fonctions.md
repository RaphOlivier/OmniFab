# Journal des fonctions (append-only)

Format : `AAAA-MM-JJ | F:<id> | J<jalon> | <changement> | source: <ADR/commit/discussion>`
Consultation : grep sur l'ID de la fonction (ne jamais lire le fichier entier).
Amorcé le 2026-10-25 à partir des ADR et archives de discussions J2 ; les fonctions antérieures ne sont reconstituées que si un besoin l'exige.

```
2026-10-21 | F:esquisse:dessin-arc | J2 | migration v2 : arcs à balayage signé suivant le geste ; refus du cycle traversé à la définition | source: ADR-0020, commit a84db22
2026-10-21 | F:esquisse:contrainte-bascule-hor-ver | J2 | bascule Hor↔Ver à identifiant conservé via sketch.constraint.updated ; refus du même genre (OF-SES-0001) ; règle au plan seulement | source: ADR-0021, commit caa81af
2026-10-21 | F:esquisse:liste-contraintes | J2 | liste des contraintes posées avec surbrillance 1,5 s | source: ADR-0021, commit caa81af
2026-10-21 | F:esquisse:ancrage-point | J2 | ancrage des points : glisser refusé, liseré turquoise | source: ADR-0021, commit caa81af
2026-10-21 | F:esquisse:ancrage-point | J2 | Révision D2 : commande AncrerPoint capture la position AFFICHÉE au journal (rejeu déterministe) | source: ADR-0021 D2, discussion 2026-10-22
2026-10-21 | F:esquisse:epingle-point-relache | J2 | épinglage du point relâché : 2e passe de résolution + Solveur2D::vider(), repli sans conflit | source: discussion 2026-10-21, commit 8e153e6
2026-10-22 | F:esquisse:reparation-graines-confondues | J2 | réparation SYSTÉMATIQUE des graines confondues (reparer_graines_confondues, epsilon 1e-3 mm) après erreur OF-GCS-0002 | source: discussion 2026-10-22, commit fdf759c
2026-10-22 | F:esquisse:invariance-ordre-contraintes | J2 | contrat à deux niveaux : entièrement contraint = positions identiques à 1e-6 quel que soit l'ordre ; sous-contraint = mêmes DoF | source: discussion 2026-10-22, tests encodés
2026-10-20 | F:liaisons:panneau-alias | J2 | panneau de liaisons desktop (alias, unités, erreurs OF-VAL i18n), dialogues nouvel alias/suppression 3 modes | source: discussion 2026-10-20, phase E lot 2
2026-10-20 | F:liaisons:cotes-pilotees | J2 | cotes pilotées figées dans forge-session (schémas v2 + migrations, commandes) | source: discussion 2026-10-20, phase E lot 1b
2026-10-22 | F:import:seuil-stl | J2 | seuil STL mètres 1 → 10 mm (objet Blender de 3 m) | source: discussion 2026-10-22, checklist J2
2026-10-25 | F:liaisons:noms-chemin | J0 | lexeur accepte les identifiants à points : cotes esquisse.* et alias en chemin référençables dans les formules ; segment fautif « a.5 » refusé (OF-VAL-0003) | source: audit 2 N1, commit d152080
2026-10-25 | F:liaisons:substitution-alias | J0 | remplacer_identifiants respecte les noms en chemin, les décimales et épargne les appels de fonction ; alias nommés comme des fonctions (min, abs…) refusés au plan | source: audit 2 N2, commit e7c4f9f
2026-10-25 | F:liaisons:parseur-expression | J0 | profondeur d'imbrication bornée à 256 (parseur, chaînes de négations comprises) + collecte des dépendances transitives itérative — plus de débordement de pile sur expression hostile | source: audit 2, commit e4e491a
2026-10-25 | F:desktop:export-fichiers | J0 | export desktop écrit en tmp + sync_all + rename : un échec ne tronque plus le fichier de l'utilisateur, temporaire nettoyé | source: audit 2 N7, commit 628d709
2026-10-25 | F:export:svg | J0 | boîte exportée plancher à 0,1 mm par dimension : plus de viewBox de hauteur nulle (invisible chez les navigateurs) sur un tracé plat | source: audit 2 N9, commit 4315374
```
