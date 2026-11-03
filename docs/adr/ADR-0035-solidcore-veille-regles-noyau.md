# ADR-0035 : Position vis-à-vis de Tangency Labs — « B-light » pour le noyau CAO, veille solidcore

**Statut : ACCEPTÉ** (validé par Raphaël Olivier le 2026-11-03, étude d'intégration Tangency Labs). Recopié au dépôt le 2026-11-03, anticipé sur le plan initial (audit externe imminent).

## Contexte

Tangency Labs publie une famille « Craft » en Rust pur : **solidcore** (noyau B-rep, port du SMLib de NVIDIA, Apache-2.0, sûr/déterministe/wasm, crates.io v1.0.1 du 2026-10-30 mais API encore expérimentale dans le dépôt, aucun traducteur STEP), **solidcraft** (CAO paramétrique complète, 13,3k ★), **fieldcraft** (FEM/CFD de bout en bout), **pcbcraft** (EDA compatible KiCad, GPL-3.0, embryonnaire — 5 commits). L'étude de faisabilité du 2026-11-03 conclut : seul solidcore est un candidat sérieux à intégration de code, mais il ne peut pas remplacer OCCT à court terme (pas de STEP, couverture difftest partielle, API instable) et le jalon 3 phase A est déjà engagé sur OCCT (ADR-0023). L'event sourcing d'OmniFab est un atout : la géométrie ne vit jamais dans le journal (ADR-0011), donc un changement de noyau ne toucherait ni le format de projet, ni l'undo, ni le rejeu.

## Décision (B-light)

1. **OCCT reste le noyau unique** (ADR-0004/0023 inchangés). Aucune dépendance à une crate Tangency.
2. **Règles de conception en phase A du jalon 3** (coût quasi nul, appliquées au module interne `occt` de forge-cad) : aucun type OCCT dans l'API publique de forge-cad ; identifiants topologiques propres (jamais de handles OCCT exposés) ; tolérance 1e-6 mm fixée à la frontière du module ; codes d'erreur `OF-CAD-*` propres ; golden tests de géométrie dès le premier lot. Objectif : le jour où un critère de bascule se déclenche, un trait « backend » s'extrait de la surface réellement utilisée sans refonte du feature tree.
3. **Veille structurée** : fiche `veille-ecosysteme` (critères de bascule, répertoire de veille, révision à chaque fin de jalon).
4. **Réutilisations sans dépendance** : corpus `tangency-labs/models` (CC0) versé aux golden tests ADR-0004 et au corpus d'import forge-io (ADR-0009) ; méthode oracle SolidCraft (validations volume/aire/centre de masse contre référence FreeCAD) adoptée pour les tests de géométrie.
5. **Interop par formats, pas par code** : workflows documentés STEP/3MF/glTF → FieldCraft (simulation, vision matrice v9 §10.4) et `.kicad_pcb` → pcb2gcode → CAM PCB. Les domaines Simulation (FEM/CFD) et Électronique/PCB sont inscrits comme **cadre de vision** (matrice v9 §10.4) — aucun jalon, aucun design, aucun code tant que le gel de roadmap post-jalon 4 tient.

## Alternatives rejetées

- **A — statu quo strict (OCCT, rien de plus)** : rejeté ; ne protège pas le feature tree d'une fuite de concepts OCCT qui rendrait toute bascule future ruineuse. Le B-light ne coûte rien aujourd'hui et préserve l'option.
- **B — trait « backend » multi-noyaux complet dès la phase A** : rejeté (abstraction spéculative — interface inventée avant l'usage ; charge de test combinatoire golden tests × noyaux × plateformes ; risque de plus petit dénominateur commun face à un OCCT plus riche ; CI doublée).
- **C — bascule vers solidcore** : rejetée (API expérimentale, pas de STEP, difftest incomplet, jalon 3 engagé sur OCCT, bus factor inconnu de l'organisation).

## Critères de bascule (la veille déclenche un ADR si l'un est rempli)

1. API solidcore stabilisée (≥ 0.1 sémantique) et booléens + congés + offset complets au difftest C++.
2. Un chemin STEP indépendant d'OCCT, éprouvé pour nous.
3. Jalon 9 (web/wasm) en approche et poids d'OCCT en wasm (~50-70 Mo) devient le facteur bloquant — **cas d'usage principal** de solidcore.
4. Dégradation d'opencascade-rs : la bascule vers le wrapper cxx (plan B déjà acté par ADR-0004) **prime** sur toute aventure solidcore.

## Conséquences

- Dette nulle aujourd'hui ; effort de veille marginal (fiche + révision fin de jalon).
- Les règles de conception du point 2 sont à rappeler/contrôler dans chaque lot du jalon 3 touchant forge-cad (revue de code).
- Toute réouverture (trait, second backend, bascule) passe par un nouvel ADR + validation explicite de Raphaël.

**Liens :** matrice v9 (dépôt, commit `646fdb9`, §10.1 + §10.4) ; fiche Knowledge `veille-ecosysteme` ; ADR-0004, ADR-0009, ADR-0011, ADR-0023.
