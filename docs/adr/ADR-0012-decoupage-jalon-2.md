# ADR-0012 : découpage du jalon 2 — ordre d'attaque et critères de sortie

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-15)
- **Date :** 2026-10-15
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 2 — « Viewport wgpu + esquisse 2D contrainte + moteur de liaisons (alias/expressions) »

## Contexte

Le jalon 1 est clos (ADR-0011, CI verte sur `26aa2a4`) : session event sourcing, undo/redo,
import/export SVG/DXF/STL, i18n + aide. Le jalon 2 est le plus lourd depuis le début du
projet : il cumule un changement de stack de rendu (egui painter → wgpu), la première
intégration C++ du projet (planegcs, l'un des six verrous) et une extension du moteur de
liaisons. S'y ajoutent deux décisions suspendues par l'ADR-0011 : sélecteur de fichiers /
bibliothèque de projets, et rendu du maillage STL. Le protocole du projet exige un découpage
explicite avant d'écrire du code : une discussion = un objectif, un ADR par décision tranchée.

## Décision

Le jalon 2 est découpé en **cinq phases séquentielles**, chacune ouverte par un ADR
technique dédié et fermée par des critères de sortie mesurables (CI verte incluse).

**Phase A — Dérisquage planegcs (ADR-0013).** Crate `forge-gcs-sys` (build CMake + shim
ABI C + bindgen) puis crate `forge-gcs` (wrapper sûr + trait d'abstraction `Solveur2D`,
feature `native`/`wasm`). Pas d'UI. Critère de sortie : un test d'intégration résout un
croquis paramétrique de référence (triangle + contraintes) sur la matrice CI multi-OS,
sans régression du temps de build du dépôt au-delà d'un plafond défini (+ ~5 min max par job).

**Phase B — Quick wins suspendus de l'ADR-0011.** Sélecteur de fichiers natif pour
import/export (décision `rfd` vs natif par OS à trancher en ADR), bibliothèque de projets
(dossier de projets + écran d'accueil), rendu wireframe du maillage STL dans le viewport
existants. Critère de sortie : le démonstrateur ouvre/enregistre des fichiers choisis par
l'utilisateur et affiche le maillage importé.

**Phase C — Viewport wgpu.** ADR technique (architecture renderer : pipeline 2D d'abord —
tracés et grille —, caméra zoom/pan, puis 3D wireframe). La surface « dessiner les tracés »
actuelle migre du painter egui vers wgpu. Critère de sortie : parité fonctionnelle 2D avec
le painter actuel + caméra fluide, sur Linux et Windows.

**Phase D — Esquisse 2D contrainte.** ADR : modèle d'esquisse (entités + contraintes)
intégré à l'event sourcing de forge-session (commandes `AddEntity`/`AddConstraint`/`Solve`),
appel du solveur via `forge-gcs`, UI d'esquisse (snapping, sélection, poignées). Critère de
sortie : croquis contraint modifiable et défaisable de bout en bout (commandes → événements
→ solveur → undo/redo), tests de non-régression du solveur.

**Phase E — Moteur de liaisons étendu.** ADR : pont forge-values ↔ paramètres d'esquisse
(expressions et alias pilotent des paramètres de contraintes), sémantique de propagation.
**Exigence du fondateur (Raphaël, 2026-10-15)** : les alias doivent pouvoir être utilisés
**dans des formules à l'intérieur d'un tableau** — sémantique type tableur : des cellules
d'un tableau de valeurs référencent des alias par des formules, les alias/nœuds liés
restant la source de vérité. Le modèle de forge-values doit le permettre dès sa conception
(les alias sont des références de première classe, évaluables depuis des cellules), même si
l'UI tableau n'arrive qu'à la phase E — à préciser dans l'ADR d'ouverture de la phase.
L'éditeur nodal complet (type Xpresso) reste au jalon 3. Critère de sortie : une expression
du panneau valeurs pilote une cote d'esquisse, avec undo/redo cohérent ; une cellule de
tableau référence un alias par formule.

Règle transverse : **aucune phase ne commence avant la validation de l'ADR qui l'ouvre**,
et une phase n'est close que CI verte vérifiée + capitalisation Knowledge faite.

## Alternatives rejetées

- **Esquisse d'abord, avec un solveur Rust provisoire** : deux solveurs à maintenir, dette
  assurée, et le vrai risque (planegcs) repoussé — contradiction avec l'objectif de dérisquage.
- **Viewport wgpu d'abord** : retarderait le dérisquage planegcs, qui est le seul point du
  jalon capable de faire changer les plans (le rendu, lui, ne peut qu'être différé).
- **Tout en parallèle (big bang)** : incompatible avec le rythme du projet (convergence CI
  itérative, validation explicite à chaque décision) et avec « une discussion = un objectif ».
- **Quick wins en dernier** : le sélecteur de fichiers et le rendu maillage sont petits,
  utiles, et rassurants entre deux phases lourdes ; les repousser n'économiserait rien
  (l'esquisse ne peut pas les utiliser de toute façon au démarrage de la phase D).

## Conséquences

- **Positives** : le risque maximal (planegcs, seule introduction C++ du projet) est
  traité en premier, seul, sans dépendre du rendu ; chaque phase livre une valeur
  démontrable ; les critères de sortie empêchent d'empiler des demi-succès.
- **Négatives / coûts acceptés** : la phase A ne produit aucune fonctionnalité visible
  (investissement d'infrastructure pur) ; le rendu wgpu (phase C) arrive après les quick
  wins, qui devront être migrés du painter vers wgpu (travail borné : rectangle, tracés,
  wireframe maillage).
- **Risques** : la CI multi-OS doit installer Eigen3/fmt/Boost (phase A) — matrices à
  préparer soigneusement ; la phase D est la plus grande charge de conception du jalon.
- **Décisions futures suspendues** : bibliothèque de projets complète (produit, jalon 2+),
  solveur wasm en production (jalon 9), éditeur nodal (jalon 3).
