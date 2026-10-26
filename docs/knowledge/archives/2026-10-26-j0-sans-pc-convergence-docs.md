# 2026-10-26 — [J0] Session sans PC n°2 : convergence documentation + patchs `expect` pré-spécifiés

Suite de la session « designs B-E » (résumé de transition 2026-10-26) : les trois objectifs sans PC ont été enchaînés dans la même discussion, plus deux demandes explicites de Raphaël.

## Objectif 1 — Spécifs de tests test-first de la phase D

Fiche `preparation/specifs-tests-phase-d.md` (21 tests TD-1.1..TD-4.3) couvrant les lots 1-4 du design VALIDÉ : forge-units (mm²/mm³/rad, complétude des codes), table dimensionnelle + **OF-VAL-0010** seul nouveau code, fonctions/conversions, grammaire non cassante (golden journal v1) ; forge-session (réservation de préfixe `values.tableau.created`, OF-VAL-0008 étendu, undo seul/groupe, `PastePlage` atomique, cascade de bout en bout = preuve de non-régression ADR-0002) ; desktop automatisable seulement (pré-validation des arêtes au dessin = garde en forge-values, positions jamais sérialisées, cohérence trois vues) — drag & drop et rendu restent à la checklist manuelle §7 du design. Aucun point SUSPENDU AU SPIKE (phase D sans OCCT confirmé). Exécution dès la CI verte, indépendamment de B/C.

## Objectif 2 — Lot EN n°2

`docs/en/matrice-fonctionnalites.md` (préambule, titres des 11 sections, bilan traduits ; les ~277 lignes restent FR pures — périmètre conventions v6) + README FR section Langue étendue + conventions **v6**. **Leçon :** le README EN était déjà complet (le lot n°1 avait tout couvert, y compris les ajouts postérieurs) — vérifier l'état réel avant de « finir un reste » ; les candidats « sections README restantes » n'existaient plus. Reste EN : index ADR à resynchroniser quand ADR-0029..0034 atteindront le dépôt.

## Demande Raphaël — Sommaire général du projet

« Se repérer dans le projet dans son intégralité, références cliquables, accessible dès la page d'accueil » : `docs/sommaire.md` (FR canonique — pages d'entrée, 6 documents, 27 ADR décrits un par un, Knowledge + 20 archives, code, 5 workflows CI au rôle vérifié à la source) + `docs/en/sommaire.md` + bandeau 🗺️ en tête du README. Conventions **v7** : règle de maintenance « tout nouveau document, crate ou workflow doit figurer au sommaire ». Coquille « parseurs budgetsés » corrigée au passage.

## Diagnostic + réparation — ADR-0016 et ADR-0019 manquants au dépôt

Question de Raphaël (« est-ce que le reste est cohérent ? ») : diagnostic complet — les deux ADR existent, **acceptés** (0016 le 2026-10-17, 0019 le 2026-10-19), jamais poussés (pas un renumérotage) ; 61 fichiers les référencent en texte (aucun lien mort) ; l'écart était annoté dans l'index EN. Numérotation 0000→0028 trouée uniquement là. **Décision Raphaël : pousser, 0019 décisions+design fusionnés en un fichier** — fait (`40d5c45`) : ADR-0016 tel quel, ADR-0019 fusionné (note de fusion en pied), sommaire + index EN mis à jour.

## Objectif 3 — Patchs `expect()` pré-spécifiés

Fiche `preparation/patchs-expect.md` sur lecture du code réel (tête `40d5c45`), décisions v2 de l'inventaire appliquées : **un seul vrai chemin de panique** (`esquisse.rs` L1537, Point sans graine d'un journal hostile → refus typé OF-SES à allouer + test test-first) ; le reste = skip de frame `Renderer2D`, `into_inner` poison (décision 3), 15 messages « invariant : » (décision 2), activation clippy warn→deny + allow build.rs daté (décision 1). **Trouvailles à la relecture :** reliquat « // PRÉPARATION » en tête de `aide.rs` au dépôt (fichier de travail poussé tel quel — suppression planifiée lot D) ; sites restants après les lots = invariants commentés + build.rs + graine de démo (décision 4, découpage main.rs). Ordre C → A → B/D → E, branche+PR par lot.

## Leçons / points de méthode

- Le sommaire des archives a été construit sur le listing du **dépôt** : il manquait 2 entrées côté plateforme (index miroir en retard v10 vs v12) — détecté au protocole de fin par recoupement index ↔ listage ; le protocole de fin (cohérence croisée) est bien le dernier filet.
- Vérifier l'existant avant d'exécuter un « reste » hérité d'un résumé de transition (README EN complet).
- Lecture du code réel plutôt que les numéros de ligne d'une fiche antérieure (tête a bougé : af5a398 → 40d5c45).

## Fiches touchées

Conventions v5→v7, état actuel v19→v25, inventaire-expect v3, specifs-tests-phase-d v1 (nouvelle), patchs-expect v1 (nouvelle). Dépôt : commits `1323e4a` (lot EN), `ed2b4c4` (sommaire), `21eb8d4` (conventions v7 + coquille), `40d5c45` (ADR-0016/0019), + resynchronisation miroir des archives (3 fichiers + index).

## Suite

Les trois objectifs sans PC du résumé de transition sont épuisés ; l'action bloquante n°1 reste le **checkpoint PC** (build local → push Cargo.lock → spike wire→prisme→STEP), suivi des branches de correction dans l'ordre A3-01→… puis des lots expect C→A→B/D→E.
