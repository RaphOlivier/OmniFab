---
name: etat-actuel
description: "État actuel d'OmniFab sur une page : ce qui compile, état de la CI, dette ouverte. Mise à jour à chaque lot de travail — la fraîcheur de toutes les autres fiches se lit ici d'un coup d'œil."
---
**Version :** v3 (2026-10-26) — galerie visuelle étendue (historique durable branche `galerie`) + diagnostic de la veille du matin éclairci (commits nocturnes identifiés, `list_issues` du connecteur défaillant)

# État actuel du projet

**Date de ce relevé : 2026-10-26.** Protocole : cette fiche est mise à jour à chaque fin de lot de travail ; elle ne remplace pas les fiches de décisions (le *pourquoi* y vit), elle dit où en est le projet *maintenant*. Jalonnement : jalon 3 (CAO paramétrique), phase A ouverte.

## Ce qui est livré et vert

- Noyau jalon 1-2 complet : forge-units, forge-doc, forge-values, forge-geom, forge-io (SVG/DXF/STL), forge-session (undo/redo), forge-i18n (aide intégrée), forge-render (wgpu 2D/3D), forge-gcs (planegcs via FFI) + démonstrateur desktop egui (esquisse contrainte, moteur de liaisons, imports/exports).
- Correctifs de l'audit n°2 (2026-10-25) : lexeur à points, substitution d'alias sûre, récursion bornée, export atomique, SVG plat — poussés sur main (`d152080`→`4315374`), tests inclus.
- Miroir Knowledge au dépôt (`docs/knowledge/`, ADR-0028) synchronisé ; README du miroir écrit.
- **Historique visuel automatisé** (galerie Xvfb, `.github/workflows/galerie.yml`) : à chaque push de code sur main (`apps/**` + `crates/**`), captures headless des scénarios `accueil`, `atelier-2d`, `atelier-3d` (15 s d'exécution, contrôle « capture noire »), archivées **durablement sur la branche `galerie`** (`captures/<date>-<sha>/` + INDEX.md par lot, branche créée orpheline au premier lot) — l'artefact de run ne sert plus que de commodité (logs). Demande de Raphaël 2026-10-26 : garder un historique visuel de l'évolution de l'application.

## État de la CI — ⚠️ ROUGE (attendue, cause unique)

- **Cargo.lock périmé** depuis le push de forge-cad (`4409a65`, 2026-10-25) : toutes les commandes `--locked` échouent (issues #311-#317). La convergence nécessite un **build local de Raphaël** (OCCT système ≥ 7.8) + push du lock régénéré — déjà planifié pour la phase A.
- Conséquence : **les 5 correctifs d'audit ne sont pas validés par CI** tant que le lock n'est pas poussé (le code est relu, testé à l'écriture, mais non compilé par CI).
- Leçons héritées : ne jamais déclarer le vert sur la seule absence d'issue dans les minutes qui suivent le push (conclure sur les conclusions des jobs).

## Dette ouverte (par ordre de priorité assumée)

1. **CI au vert** : push du Cargo.lock (Raphaël) — bloque toute validation compile des correctifs récents.
2. **Backlog audit n°2 non corrigé** : discrétisation à tolérance (cercle/arc/flèche), DXF ($INSUNITS, INSERT/blocs, extrusion, chaînage de contours, export LWPOLYLINE), RapportImport (ADR-0016), N3 suppression « Remplacer » atomique, N4 résolveur d'esquisse à état historique, N5 suppression d'entités/contraintes, N6 validations dégénérées, N8 catch_unwind fil de travail.
3. **Hygiène build** : `rust-toolchain.toml`/`rust-version` absents (code exige ≥ 1.88) ; LICENSE manquants sur 7 crates/12 ; ~218 `expect()` hors tests (clippy `expect_used` à activer hors tests).
4. **Dette documentaire** : `architecture/KNOWLEDGE.md` ~149 Ko à découper (noyau stable + historique vers archives) ; README/CONTRIBUTING encore jalon 0 ; prérequis C++ (CMake ≥ 3.26, Eigen, fmt, Boost, libclang, sous-module) à documenter ; traduction EN de la doc (condition initiale du passage public, non tenue).
5. **Dette outillage** : contrôle de liens Markdown du miroir en CI (attend CI verte) ; `AGENTS.md` ou fiche non publiée pour les préférences personnelles (nommage `[JX]`, encarts) ; branches/PR vs pushes directs (non tranché) ; main.rs ~5 000 lignes à découper avant la fusion de fonctionnalités risquée.

## Gel en vigueur

**Jalons 5-10 gelés** tant que le meuble du cas d'école n'est pas fabriqué de bout en bout (décision Raphaël, 2026-10-26 — fiche architecture, section Roadmap).

- **Veille CI du 2026-10-26 (matin) — éclaircie dans l'après-midi** : la veille signalait 31 issues « CI : » ouvertes (#311-#341, Windows/macOS/aarch64) et des « commits nocturnes inconnus » ; **ces commits sont identifiés** : ce sont les lots du 25 au soir (correctifs audit 2 `d152080`→`4315374`, miroir ADR-0028 `314c7b3`→`035edbe`, delta `4e0a8ce`, README `dcbb154`, revue miroir `e0329b9` — cf. journal des fonctions et archives). **Anomalie connecteur du jour** : `github_app` fonctionne pour push/lecture de fichiers/recherche, mais `list_issues` renvoie silencieusement 0 (OPEN et CLOSED) — ne jamais se fier à un « OPEN = [] » de ce connecteur sans recoupement (leçon existante : 401 passagers ; ici : vide silencieux) ; recoupement par l'API publique (`api.github.com/.../issues?state=open`, quota IP), qui a confirmé au moins l'issue **#346** ouverte. **État CI inchangé et confirmé** : `Cargo.lock` à la tête (`abd1981`) **ne contient pas forge-cad** — le lock n'a pas été poussé, `--locked` échoue, la CI reste rouge ; Linux (lint-test) est vraisemblablement rouge aussi (les titres d'issues de la veille ne listant pas Linux reste à confirmer à la lecture des corps quand l'accès sera stable). **Aucun commit de Raphaël sur main** à ce jour : le push du lock (build local OCCT) reste l'action bloquante n°1.

## Historique

- v1 (2026-10-26) : création — relevé initial (CI rouge Cargo.lock attendue, backlog audit, gel post-jalon 4, dette documentaire).
- v2 (2026-10-26, matin) : veille CI — 31 issues ouvertes, commits nocturnes à identifier, anomalie d'accès connecteur (à investiguer).
- v3 (2026-10-26, après-midi) : diagnostic éclairci — commits nocturnes identifiés (lots du 25 au soir, connus de la Knowledge), `list_issues` renvoie silencieusement 0 (recouper par API publique), lock confirmé absent de forge-cad à la tête, CI rouge cause unique inchangée ; + galerie visuelle étendue — archivage durable des captures sur la branche `galerie` (historique visuel commit par commit), déclenchement élargi à tout code.
