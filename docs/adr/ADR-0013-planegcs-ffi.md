# ADR-0013 : intégration de planegcs via FFI — vendoring, ABI C, crates forge-gcs

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-15)
- **Date :** 2026-10-15
- **Décideurs :** Raphaël Olivier
- **Phase :** jalon 2, phase A (dérisquage) — ouverte par l'ADR-0012

## Contexte

L'esquisse 2D contrainte (jalon 2) exige un solveur géométrique. Le projet s'est verrouillé
sur **planegcs**, le solveur de contraintes du Sketcher de FreeCAD (l'un des six verrous),
via FFI. État des lieux vérifié le 2026-10-15 :

- planegcs est du **C++** sous **LGPL-2.1-or-later**, vivant dans
  `src/Mod/Sketcher/App/planegcs` de FreeCAD ; dépendances : **Eigen3** (header-only),
  **fmt**, **Boost** (headers, Boost.Graph uniquement) ; API instable entre versions
  mineures de FreeCAD.
- Une extraction standalone maintienne existe : **2wiceUponATime/PlaneGCS** (synchronisée
  sur `releases/FreeCAD-1-1`, cible CMake `PlaneGCS::planegcs`, shim console éliminant
  les dépendances Qt/FreeCAD, `add_subdirectory`/`find_package` supportés).
- Le même code compile en **WebAssembly** (Salusoft89/planegcs, port Emscripten +
  annotations TypeScript) — la cible wasm du jalon 9 est donc prouvée faisable.
- Il n'existe **aucun binding Rust** publié (seuls des bindings Python existent :
  spookylukey/planegcs, LGPL-2.1-or-later). Un projet tiers (Forge, PR #256) a déjà posé
  le pattern « planegcs vendé derrière une ABI C, adaptateur écrit contre l'ABI et non
  contre les internals, frontière LGPL = frontière de liaison ».
- Rust ne peut pas appeler C++ directement : il faut une frontière **ABI C** (`extern "C"`).
- **Vérification FreeCAD 26.3 faite le 2026-10-15 (à la demande de Raphaël)** : FreeCAD est
  passé au CalVer (26.3 = première release du calendrier tri-annuel, rien de sauté depuis
  1.1). Les avancées Sketcher de 26.3 sont au niveau outil/UI — outil texte expérimental,
  contrainte de groupe, références externes créées automatiquement au clic, préférences
  d'affichage des cotes — le cœur planegcs n'est pas refondu. L'extraction standalone
  synchronisée sur 1.1 reste donc la base du spike ; une re-synchronisation 26.3+ sera
  évaluée après stabilisation du shim (FreeCAD ne stabilise pas l'API du solveur entre
  versions mineures — le MANIFEST fige la version, le shim absorbe les changements).
  Les avancées UI de 26.3 sont notées pour la phase D (références externes, contrainte
  de groupe : comportements à répliquer côté esquisse, sans le code).

## Décision

1. **Vendoring de l'extraction standalone 2wiceUponATime/PlaneGCS** dans
   `third_party/planegcs/` (sous-module git pointant sur un tag précis + fichier
   `third_party/planegcs/MANIFEST.md` : version FreeCAD source, commit, empreinte,
   licence LGPL-2.1-or-later SPDX). Raison : l'extraction élimine Qt/FreeCAD et expose
   une cible CMake propre ; vendé, elle garantit des builds reproductibles et cloisonne
   la frontière licence au seul répertoire `third_party/`.

2. **Shim C++ minimal `gcs_c_api`** (répertoire `third_party/planegcs-shim/`, licence
   LGPL-2.1-or-later, copyright projet) : une poignée de fonctions `extern "C"` sur des
   handles opaques — créer/détruire un système, ajouter point/ligne/cercle/arc, ajouter
   une contrainte (identifiant + paramètres double), `solve` (algorithme + diagnostics),
   lire/écrire les paramètres. **Chaque point d'entrée attrape toute exception C++**
   (aucune ne fuit à travers l'ABI) — pattern éprouvé par l'intégration Swift de
   3d-modeller-app (PR #18 : handles opaques, validation, re-diagnostic après résolution,
   cartographie des résultats vers les indices de contraintes, budget temps sur un croquis
   de 100 lignes). Toute la richesse C++ (templates, GCS::System) reste derrière cette
   ABI ; l'adaptateur Rust ne voit jamais les internals.

3. **Crate `forge-gcs-sys`** (LGPL-2.1-or-later) : `build.rs` pilote CMake via la crate
   `cmake` (build statique de planegcs + shim), **bindgen** sur le header C du shim
   (header minuscule, reviewable), liens statiques. Codes d'erreur typés `OF-GCS-*`
   au registre (échec de build CMake ≠ erreur d'exécution : wrap `GcsError`).

4. **Crate `forge-gcs`** (LGPL-2.1-or-later) : wrapper sûr — `Systeme2D` (constructeur,
   ajout d'entités/contraintes typées via forge-geom, `resoudre()`, lecture des positions),
   **trait d'abstraction `Solveur2D`** (feature `native` par défaut ; l'implémentation
   `wasm` du même trait arrivera au jalon 9 sur la base du port Emscripten prouvé).
   Aucune dépendance à forge-session : c'est forge-session (phase D) qui consommera
   forge-gcs. Unités strictes : l'ABI parle en `double` millimètres/degrés, les newtypes
   sont réintroduits à la frontière du wrapper.

5. **Licences** : `forge-gcs-sys`, le shim et `forge-gcs` en **LGPL-2.1-or-later**
   (héritage direct de planegcs ; le reste du dépôt reste inchangé — GPL-3.0 racine,
   MPL-2.0 pour les crates isolées, qui ne dépendront pas de forge-gcs). Le statut
   LGPL est documenté dans LICENSE de chaque crate concerné.

6. **Dérisquage mesurable (phase A)** : le spike est clos quand (a) la CI multi-OS
   (Linux, Windows, macOS si disponible, aarch64) construit planegcs via CMake —
   installation des dépendances Eigen3/fmt/Boost par apt/choco/vcpkg dans le workflow,
   mécanisme d'issues « CI : » inchangé ; (b) un test d'intégration `forge-gcs`
   construit un triangle (3 lignes + contraintes coïncidence/longueur/angle), le résout,
   et vérifie les positions à la tolérance 1e-6 mm ; (c) le temps de build des jobs
   reste sous le plafond fixé par l'ADR-0012. Aucune UI dans cette phase.

## Alternatives rejetées

- **bindgen direct sur les headers C++ de planegcs** : bindgen ne supporte qu'une
  fraction du C++ ; templates et namespaces de GCS::System rendent la sortie inutilisable
  et fragile à chaque re-synchronisation FreeCAD.
- **cxx / cxxbridge** : ajoute une couche build lourde pour un périmètre d'API volontai-
  rement minuscule ; l'ABI C manuelle est plus simple à auditer, à stabiliser et à tester.
- **Dépendre du port WASM existant (Salusoft89) via wasmtime en natif** : satisfait la
  sécurité (sandbox) mais coûte un runtime complet, complique le debug natif et ajoute
  une dépendance JS/TypeScript au chemin de build ; gardé comme piste pour l'impl
  `wasm` du jalon 9, pas pour le natif.
- **Réimplémentation Rust du solveur** : des mois de travail, en contradiction avec le
  verrou planegcs, et la maturité de planegcs (DogLeg, Levenberg-Marquardt, BFGS, SQP,
  des années de battle-testing FreeCAD) est précisément ce qu'on cherche.
- **Consommer FreeCAD entier comme bibliothèque** : train de dépendances massif (Qt,
  Python), contradiction avec l'isolation du dépôt.
- **Package système (libplanegcs installé)** : pas packagé partout, versions non
  maîtrisées ; le vendoring garantit la reproductibilité des six plateformes cibles.

## Conséquences

- **Positives** : le plus gros risque technique du jalon 2 est levé isolément ; l'ABI C
  figée rend les re-synchronisations FreeCAD locales au shim ; le trait `Solveur2D` tient
  la contrainte wasm-friendly du projet dès maintenant.
- **Négatives / coûts acceptés** : première dépendance C++ du projet — la CI gagne des
  étapes d'installation (Eigen3, fmt, Boost) et du temps de build (plafond surveillé) ;
  deux crates supplémentaires à documenter (rustdoc exigé) ; LGPL héritée sur les crates
  de la frontière (choix documenté, compatibilité GPL du dépôt vérifiée : la GPL peut
  consommer de la LGPL).
- **Risques** : build CMake sous Windows (vcpkg/choco) — le plus incertain, d'où le
  caractère « spike » de la phase ; API planegcs instable entre versions mineures
  FreeCAD → le MANIFEST fige la version et le shim absorbe les changements ; MSRV de
  bindgen à surveiller à l'ajout ; **non-déterminisme documenté du solveur** (tri de
  listes de pointeurs par adresses — FreeCAD#33399 : le même jeu de contraintes peut
  donner des résultats différents à chaque exécution) → le test du spike exige la
  reproductibilité (deux résolutions successives du même croquis = mêmes positions à
  1e-6 mm) et le wrapper fige l'ordre d'ajout des entités/contraintes ;
  **diagnostic de redondance/conflictualité instable** (le même croquis oscille entre
  « redondant », « sous-contraint » et « conflictuel » — proposition de subvention FPA
  #106, travail upstream prévu fin 2026) → `Solveur2D` expose les diagnostics, mais la
  phase D ne devra pas s'y fier sans hystérésis côté UI.
- **Décisions futures suspendues** : implémentation `wasm` du trait `Solveur2D` (jalon 9) ;
  choix DogLeg/LM/BFGS/SQP par défaut et stratégies de diagnostic (phase D, avec l'UI
  d'esquisse) ; batching des résolutions (jalon 4+).
