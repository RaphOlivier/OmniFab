# Sommaire général — la carte du projet

> **Rôle de cette page :** vue d'ensemble cliquable de **toute** la documentation et de l'intégralité du dépôt — chaque entrée mène au fichier correspondant, avec une ligne d'explication. À garder ouverte comme plan de repérage ; le détail vit dans chaque document.

## Pages d'entrée

| Fichier | Contenu |
|---|---|
| [`README.md`](../README.md) | Présentation du projet : feuille de route, vision, stack technique, structure du dépôt, build par OS |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | Conventions de développement obligatoires : statut, prérequis natifs, branches/PR, tests de régression |
| [`AGENTS.md`](../AGENTS.md) | Règles de contribution pour les agents IA (outillage, conventions, limites) |
| [`LICENSE`](../LICENSE) | GPL-3.0+ — licence de l'application (crates isolées : MPL-2.0, cf. `crates/*/LICENSE`) |

## Documentation — `docs/`

| Document | Contenu |
|---|---|
| [`architecture.md`](architecture.md) | Architecture cible du workspace : schéma des crates, invariants de dépendances (garde-fou) |
| [`matrice-fonctionnalites.md`](matrice-fonctionnalites.md) | Inventaire exhaustif ≈ 277 fonctionnalités × ~60 logiciels de référence (garde-fou anti-oubli, pas une liste de tâches) |
| [`prompt-fondateur.md`](prompt-fondateur.md) | La vision fondatrice en 15 points + annexes (source de la roadmap) |
| [`error-codes.md`](error-codes.md) | Registre central des codes d'erreur typés `OF-<DOMAINE>-<NNNN>` (vérifié par CI) |
| [`audit-protocole.md`](audit-protocole.md) | Protocole d'audit du dépôt — à lire par tout auditeur externe avant tout signalement |
| [`ci-cargo-deny.md`](ci-cargo-deny.md) | Audit supply-chain des dépendances : licences/CVE via `deny.toml` (CI) |
| [`en/`](en/README.md) | Couche d'accueil en anglais (README, CONTRIBUTING, index ADR, en-têtes de la matrice) — le français fait foi |
| [`adr/`](adr/ADR-0000-decisions-initiales-phase-0.md) | Toutes les décisions d'architecture, avec alternatives rejetées — détail ci-dessous |

## ADR — décisions d'architecture (`docs/adr/`)

| ADR | Décision |
|---|---|
| [`ADR-0000`](adr/ADR-0000-decisions-initiales-phase-0.md) | Décisions initiales phase 0 : jalons, gouvernance des crates (pas de crate vide) |
| [`ADR-0001`](adr/ADR-0001-format-evenements.md) | Format des événements : conteneur ZIP versionné + journal JSON Lines (verrou 1) |
| [`ADR-0002`](adr/ADR-0002-evaluation-forge-values.md) | Évaluation des liaisons forge-values : paresseuse, déterministe (verrou 6) |
| [`ADR-0003`](adr/ADR-0003-erreurs-typees.md) | Erreurs typées : codes `OF-<DOMAINE>-<NNNN>`, jamais de panique silencieuse |
| [`ADR-0004`](adr/ADR-0004-binding-occt.md) | Interfaçage OpenCascade (choix du crate de bindings) |
| [`ADR-0005`](adr/ADR-0005-outillage-build-android.md) | Outillage de build Android (APK, cible aarch64) |
| [`ADR-0006`](adr/ADR-0006-langage-expression.md) | Langage d'expression des liaisons (grammaire, analyse) |
| [`ADR-0007`](adr/ADR-0007-valeurs-typees.md) | Valeurs typées phase 1 (unités strictes, verrou 5) |
| [`ADR-0008`](adr/ADR-0008-crate-forge-session.md) | Crate forge-session : commandes, projections, undo/redo tout-ou-rien |
| [`ADR-0009`](adr/ADR-0009-forge-io.md) | forge-io : imports/exports SVG/DXF/STL, parseurs budgetés, courbes discrétisées |
| [`ADR-0010`](adr/ADR-0010-i18n-aide-fluent.md) | i18n Fluent + aide intégrée (`help_id` sur chaque widget) |
| [`ADR-0011`](adr/ADR-0011-integration-desktop.md) | Intégration desktop egui (démonstrateur) |
| [`ADR-0012`](adr/ADR-0012-decoupage-jalon-2.md) | Découpage du jalon 2 en phases |
| [`ADR-0013`](adr/ADR-0013-planegcs-ffi.md) | planegcs (solveur de contraintes FreeCAD) via FFI + shim C++ (verrou 3) |
| [`ADR-0014`](adr/ADR-0014-quick-wins-phase-B.md) | Quick-wins de la phase B du jalon 2 |
| [`ADR-0015`](adr/ADR-0015-viewport-wgpu.md) | Viewport wgpu : caméras, scènes, pipelines 2D/3D |
| [`ADR-0016`](adr/ADR-0016-correctifs-robustesse.md) | Correctifs de robustesse (audits du 2026-10-17) : session transactionnelle, sauvegarde sûre, conteneur durci, fil de travail desktop, export SVG interopérable |
| [`ADR-0017`](adr/ADR-0017-unites-stl.md) | Unités à l'import/export STL |
| [`ADR-0018`](adr/ADR-0018-esquisse-2D.md) | Esquisse 2D contrainte (design du sketcher) |
| [`ADR-0019`](adr/ADR-0019-liaisons-phase-E.md) | Moteur de liaisons forge-values (phase E) : schémas d'événements v2, namespaces d'alias, cotes d'esquisse pilotées, panneau de liaisons |
| [`ADR-0020`](adr/ADR-0020-correctifs-validation-jalon2.md) | Correctifs de validation du jalon 2 |
| [`ADR-0021`](adr/ADR-0021-bascule-ancrage-liste-contraintes.md) | Bascule d'ancrage de la liste des contraintes |
| [`ADR-0022`](adr/ADR-0022-decoupage-jalon-3.md) | Découpage du jalon 3 en phases A-E |
| [`ADR-0023`](adr/ADR-0023-forge-cad-occt.md) | forge-cad : CAO paramétrique sur OpenCascade (liaison dynamique, OCCT 7.8/7.9) |
| [`ADR-0024`](adr/ADR-0024-croquis-solide-feature-tree.md) | Croquis → solide → arbre de features (design phase B/C) |
| [`ADR-0025`](adr/ADR-0025-valeurs-typees-phase-2.md) | Valeurs typées phase 2 : arithmétique dimensionnelle (mm²/mm³/rad) |
| [`ADR-0026`](adr/ADR-0026-editeur-nodal-tableau.md) | Éditeur nodal de valeurs + tableau sémantique |
| [`ADR-0027`](adr/ADR-0027-desktop-cao.md) | Desktop CAO : espace CAO, arbre de projet, pipeline 3D (design phase E) |
| [`ADR-0028`](adr/ADR-0028-copies-knowledge-depot.md) | Copies de référence de la Knowledge au dépôt (`docs/knowledge/`) |
| [`TEMPLATE.md`](adr/TEMPLATE.md) | Modèle pour rédiger un nouvel ADR |

## Knowledge — le *pourquoi* du projet (`docs/knowledge/`)

Copie de référence publique des fiches de décisions (ADR-0028) : la raisonnement, les alternatives rejetées, l'historique.

| Fiche | Contenu |
|---|---|
| [`README.md`](knowledge/README.md) | Guide de lecture de la Knowledge (parcours recommandés, conventions) |
| [`KNOWLEDGE.md`](knowledge/KNOWLEDGE.md) | Index général : ce que couvre chaque fiche |
| [`etat-actuel/`](knowledge/etat-actuel/KNOWLEDGE.md) | **Relevé d'une page**, mis à jour à chaque lot : ce qui compile, état de la CI, dette ouverte |
| [`architecture/`](knowledge/architecture/KNOWLEDGE.md) | Structure des crates, six verrous techniques, roadmap jalons 0-10, résumé des ADR |
| [`conventions/`](knowledge/conventions/KNOWLEDGE.md) | Conventions : unités strictes, i18n, licences, branches/PR, langue de la doc |
| [`formats/`](knowledge/formats/KNOWLEDGE.md) | Matrice d'interopérabilité par domaine (SVG, STEP, glTF, G-code…) |
| [`machines/`](knowledge/machines/KNOWLEDGE.md) | Contrôleurs (LinuxCNC prioritaire), multi-machines, éditeur G-code |
| [`securite/`](knowledge/securite/KNOWLEDGE.md) | Modèle de menace et règles de sécurité par jalon |
| [`historique/`](knowledge/historique/KNOWLEDGE.md) | Protocole de versionnage des fiches |
| [`historique/journal-fonctions.md`](knowledge/historique/journal-fonctions.md) | Journal append-only : une ligne par évolution de fonction (grep sur `F:<domaine>:<fonction>`) |
| [`archives/`](knowledge/archives/KNOWLEDGE.md) | Résumés datés des discussions significatives — index ci-dessous |

### Archives des discussions

| Entrée | Sujet |
|---|---|
| [`2026-10-19-gouvernance-adr0019-bibliotheques`](knowledge/archives/2026-10-19-gouvernance-adr0019-bibliotheques.md) | Gouvernance de validation + bibliothèques (ADR-0019) |
| [`2026-10-20-phaseE-lot1b-liaisons-session`](knowledge/archives/2026-10-20-phaseE-lot1b-liaisons-session.md) | J2 — phase E lot 1b : liaisons forge-session |
| [`2026-10-20-phaseE-lot2-desktop-liaisons`](knowledge/archives/2026-10-20-phaseE-lot2-desktop-liaisons.md) | J2 — phase E lot 2 : liaisons desktop |
| [`2026-10-21-j2-contraintes-bascule-ancrage`](knowledge/archives/2026-10-21-j2-contraintes-bascule-ancrage-liste.md) | J2 — contraintes, bascule d'ancrage (ADR-0021) |
| [`2026-10-21-j2-correctifs-revalidation-suite`](knowledge/archives/2026-10-21-j2-correctifs-revalidation-suite.md) | J2 — correctifs, revalidation en suite |
| [`2026-10-21-j2-correctifs-validation-jalon2`](knowledge/archives/2026-10-21-j2-correctifs-validation-jalon2.md) | J2 — audit n°1 : correctifs de validation |
| [`2026-10-22-j2-ride-stl-ancrage-orientation`](knowledge/archives/2026-10-22-j2-ride-stl-ancrage-orientation.md) | J2 — STL : ride, ancrage, orientation |
| [`2026-10-25-j0-adr0028-knowledge-depot`](knowledge/archives/2026-10-25-j0-adr0028-knowledge-depot.md) | J0 — ADR-0028 : Knowledge au dépôt |
| [`2026-10-25-j0-audit2-correctifs`](knowledge/archives/2026-10-25-j0-audit2-correctifs.md) | J0 — audit n°2 : correctifs |
| [`2026-10-25-j0-versionnage-knowledge`](knowledge/archives/2026-10-25-j0-versionnage-knowledge.md) | J0 — versionnage des fiches Knowledge |
| [`2026-10-25-j3-ouverture-lot-adr`](knowledge/archives/2026-10-25-j3-ouverture-lot-adr.md) | J3 — ouverture du lot d'ADR du jalon 3 |
| [`2026-10-25-j3-phaseA-lot1-forge-cad`](knowledge/archives/2026-10-25-j3-phaseA-lot1-forge-cad.md) | J3 — phase A lot 1 : forge-cad |
| [`2026-10-26-arch-extraits-jalons0-2`](knowledge/archives/2026-10-26-arch-extraits-jalons0-2.md) | J0 — extraits d'architecture jalons 0-2 (découpage de la fiche) |
| [`2026-10-26-j0-audit3-claude`](knowledge/archives/2026-10-26-j0-audit3-claude.md) | J0 — audit externe n°3 (rapport + verdicts) |
| [`2026-10-26-j0-presentation-docs-diagrammes`](knowledge/archives/2026-10-26-j0-presentation-docs-diagrammes.md) | J0 — présentation des docs, diagrammes |
| [`2026-10-26-j0-protocole-audit`](knowledge/archives/2026-10-26-j0-protocole-audit.md) | J0 — création du protocole d'audit |
| [`2026-10-26-j0-revue-miroir-dette`](knowledge/archives/2026-10-26-j0-revue-miroir-dette.md) | J0 — revue du miroir, dette documentaire |

## Code

| Chemin | Rôle |
|---|---|
| [`Cargo.toml`](../Cargo.toml) | Workspace : membres, features, dépendances communes |
| [`Cargo.lock`](../Cargo.lock) | Graphe de dépendances figé (builds reproductibles, `--locked`) |
| [`rust-toolchain.toml`](../rust-toolchain.toml) | Chaîne Rust épinglée (stable ≥ 1.88) |
| [`deny.toml`](../deny.toml) | Politique licences/CVE des dépendances (cargo-deny, CI) |
| [`.gitmodules`](../.gitmodules) | Sous-modules (planegcs dans `third_party/`) |
| [`crates/forge-units`](../crates/forge-units) | Newtypes d'unités strictes (longueur, angle) — zéro dépendance |
| [`crates/forge-doc`](../crates/forge-doc) | Conteneur ZIP versionné + journal d'événements JSON Lines |
| [`crates/forge-values`](../crates/forge-values) | Moteur unique de liaisons : évaluation paresseuse + expressions |
| [`crates/forge-geom`](../crates/forge-geom) | Géométrie minimale : points, chemins 2D, maillages |
| [`crates/forge-io`](../crates/forge-io) | Import/export SVG, DXF, STL |
| [`crates/forge-session`](../crates/forge-session) | Commandes, projections, undo/redo tout-ou-rien |
| [`crates/forge-i18n`](../crates/forge-i18n) | i18n Fluent + pages d'aide intégrées |
| [`crates/forge-render`](../crates/forge-render) | Rendu wgpu 2D/3D : caméras, scènes, pipelines |
| [`crates/forge-gcs-sys`](../crates/forge-gcs-sys) | Liaisons FFI planegcs (shim C++) |
| [`crates/forge-gcs`](../crates/forge-gcs) | Solveur de contraintes 2D (planegcs) — API Rust sûre |
| [`crates/forge-cad`](../crates/forge-cad) | CAO paramétrique sur OpenCascade (jalon 3) |
| [`apps/omnifab-desktop`](../apps/omnifab-desktop) | Démonstrateur egui : esquisse, liaisons, viewport, imports/exports |
| [`apps/omnifab-android`](../apps/omnifab-android) | Cible Android (APK aarch64, même noyau — jalon 7) |
| [`third_party/`](../third_party) | Dépendances tierces en sous-modules (planegcs) |

## CI — `.github/workflows/`

| Workflow | Rôle |
|---|---|
| [`ci.yml`](../.github/workflows/ci.yml) | Lint + tests multi-OS (Linux x86_64/aarch64, Windows, macOS) — `--locked`, clippy, fmt, registre des codes |
| [`android.yml`](../.github/workflows/android.yml) | Build APK Android (aarch64) + signature + artefact |
| [`cargo-lock.yml`](../.github/workflows/cargo-lock.yml) | Complétion automatique du `Cargo.lock` à chaque changement de `Cargo.toml` (issue de rapport en cas d'échec de résolution) |
| [`galerie.yml`](../.github/workflows/galerie.yml) | Captures d'écran headless du démonstrateur à chaque push, archivées sur la branche `galerie` (historique visuel) |
| [`liens-md.yml`](../.github/workflows/liens-md.yml) | Contrôle des liens Markdown (workflow_dispatch) |

---

> 🗺️ Cette page est maintenue en même temps que la structure du dépôt : tout nouveau document, crate ou workflow doit y figurer. Version anglaise : [`en/sommaire.md`](en/sommaire.md) (le français fait foi).
