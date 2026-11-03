---
name: veille-ecosysteme
description: "Veille écosystème : projets libres externes stratégiques pour OmniFab (Tangency Labs / solidcore, solidcraft, fieldcraft, pcbcraft), critères de bascule du noyau CAO, réutilisations autorisées sans dépendance."
---

**Version :** v3 (2026-11-03) — ajout de la position stratégique face à la famille Craft (continuer OmniFab, contribution amont opportuniste) + section « Outillage du processus » (security-audit-skill ADOPTÉ, superpowers picoré, impeccable écarté) ; v2 (2026-11-03) — ajout de solidcraft-library (référence structurelle pour la future bibliothèque paramétrique d'OmniFab) ; v1 (2026-11-03) — création à la suite de l'étude Tangency Labs (décisions validées par Raphaël Olivier le même jour).

# Veille écosystème

Projets libres externes qui méritent une attention suivie parce qu'ils touchent la vision OmniFab (noyau CAO, simulation, électronique). **Règle transversale : veille et interop par formats, jamais de dépendance de code tant que le gel de roadmap post-jalon 4 tient** (ADR-0035).

## Décisions actées (2026-11-03, étude Tangency Labs — ADR-0035)

1. **Noyau CAO : « B-light »** — OCCT reste l'unique noyau (ADR-0023 inchangé) ; le module interne `occt` de forge-cad suit des règles de conception dès la phase A du jalon 3 (aucun type OCCT dans l'API publique, identifiants topologiques propres, tolérance 1e-6 mm à la frontière, codes `OF-CAD-*`, golden tests dès le premier lot) pour qu'un trait « backend » puisse être extrait un jour sans refonte. Pas de trait générique maintenant (anti-spéculation).
2. **Golden tests** — corpus `tangency-labs/models` (CC0) versé aux golden tests ADR-0004 et au corpus d'import forge-io (ADR-0009) ; méthode oracle SolidCraft (volume/aire/centre de masse validés contre FreeCAD) adoptée pour les tests de géométrie.
3. **Interop, pas de code** — workflows documentés : export STEP/3MF/glTF → FieldCraft (FEM/CFD) ; `.kicad_pcb` → pcb2gcode → CAM PCB (KiCad reste la porte d'entrée électronique).
4. **Domaines de vision** — Simulation (FEM/CFD) et Électronique/PCB inscrits en matrice v9 §10.4 : cadre de vision et garde-fou anti-oubli uniquement, aucun jalon.

## solidcore — critères de bascule (l'ADR-0035 fait foi)

Un seul critère rempli ⇒ ouvrir un ADR de révision :

1. API stabilisée (≥ 0.1 sémantique) et booléens + congés + offset complets au difftest C++ ;
2. un chemin STEP indépendant d'OCCT disponible et éprouvé ;
3. jalon 9 (web/wasm) en approche et poids d'OCCT en wasm (~50-70 Mo) devenant le facteur bloquant — **cas d'usage principal** de solidcore ;
4. dégradation d'opencascade-rs ⇒ le plan B cxx (ADR-0004) **prime** sur toute aventure solidcore.

## Répertoire de veille

| Projet | Rôle | Statut relevé (2026-11-03) | Signaux à surveiller |
|---|---|---|---|
| **solidcore** (Tangency Labs) | Noyau B-rep pur Rust, port du SMLib de NVIDIA, Apache-2.0 | crates.io v1.0.1 (2026-10-30) ; API encore expérimentale (0.0.x au dépôt), difftest partiel, **aucun traducteur STEP** | stabilité de l'API, complétude booléens/congés/offset, bus factor (organisation sans membres publics) |
| **solidcraft** | CAO paramétrique 3D pure Rust, 13,3k ★, desktop+web+MCP | référence comportementale (jamais une brique) pour les jalons 3 et 9 | UX feature timeline, solveur de contraintes maison, build wasm |
| **fieldcraft** | FEM/CFD pur Rust (maillage tet/hex certifié, solveurs non linéaires, LBM, decks Abaqus/Nastran/LS-DYNA) | vision (matrice v9 §10.4), interop par formats | publication de crates (aucune à ce jour), maturité contact/flambement |
| **pcbcraft** | EDA Rust compatible KiCad, GPL-3.0-or-later, MCP-first | embryonnaire (5 commits) ; licence alignée si intégration un jour | croissance du projet, export `.kicad_pcb` |
| `tangency-labs/models` | Corpus CAO/maillages CC0 | **réutilisé dès maintenant** (golden tests + corpus import) | nouveaux jeux de données |
| **solidcraft-library** | Bibliothèque de ressources « authoring » : 2 991 enregistrements (apparences, matériaux physiques source-backed, références scientifiques, rendu) ; catégories réservées dont **« parts »** (composants réutilisables) | **référence structurelle pour la future bibliothèque paramétrique OmniFab** (jalon 10) — voir ci-dessous | ouverture de la catégorie « parts », apparition d'un format de pack installable, stabilité du schéma |

## solidcraft-library — référence structurelle pour la bibliothèque paramétrique (constat Raphaël, 2026-11-03)

Le dépôt valide l'**architecture** de la bibliothèque visée par la vision fondatrice (base de données de **paramètres**, pas de géométrie cuite), mais n'en livre pas encore le contenu. À emprunter tel quel quand le jalon 10 s'ouvrira :

- **descripteur JSON Schema versionné** (`solidcraft-library-asset/1`) : id stable, catégorie, famille, `definition` + recettes éditables, `relations` typées (`derived_from`, `physical_reference`, `element_reference`…), `runtime_dependencies` séparées des références bibliographiques, `evidence_baseline` en sha256 — même logique que notre event sourcing (version de schéma par type, ADR-0001) ;
- **gouvernance de données** : licences par asset (socle Apache/MIT + attributions par enregistrement), provenance et preuves individuelles, catalog machine-readable **généré** et validé en CI (`catalog.py build/check`, `validate.py`) — convergent avec notre convention « données jamais GPL » (ODbL/CC0/CC-BY) et la chaîne d'approvisionnement sécurisée ;
- **honnêteté des catégories** : les catégories vides sont déclarées « planned » sans publier d'actifs ni promettre des fonctions.

Limites constatées : contenu actuel centré rendu/matériaux (aucun « part » publié — visserie/quincaillerie/murs restent à créer), pas de releases ni de format de pack installable, appareil de provenance/restauration lourd (archive/legacy, binaires restaurés séparément) — leçon : démarrer plus simple et n'ajouter la lourdeur qu'au besoin. Aucune décision de code maintenant (gel) ; réutiliser leurs packs « parts » Apache/MIT le jour où ils existent serait possible sans friction de licence.

**Rythme de révision :** à chaque fin de jalon, ou plus tôt sur signal majeur (abandon, changement de licence, publication d'une API stable chez solidcore).

## Position stratégique face à la famille Craft (Raphaël, 2026-11-03)

**Continuer OmniFab.** L'overlap avec SolidCraft se limite au noyau CAO paramétrique ; CAM, G-code, pilotage machine, multi-procédés, BIM et bibliothèque paramétrique ne sont couverts par personne chez Tangency — la vision fondatrice ne peut pas vivre dans leur roadmap (produit Apache/MIT de design pour le web/agents). **Contribution amont opportuniste uniquement** (rapports de bugs, retours sur leur corpus, correctifs de docs) quand elle est bon marché et sans dispersion — jamais un pivot. Les critères de bascule de solidcore restent le seul cadre d'une éventuelle relation de dépendance. Les vrais risques ne sont pas la concurrence mais la dispersion (le gel existe pour ça) et le bus factor solo.

## Outillage du processus (agents de développement — 2026-11-03)

Aucun des trois n'est une brique du produit (aucune dépendance Cargo) : ce sont des *skills* pour agents de codage, donc des outils du **processus de développement**. Licences permissives (MIT/MIT/Apache-2.0).

| Dépôt | Rôle | Décision |
|---|---|---|
| **cloudflare/security-audit-skill** (MIT) | Audit sécurité en 6 phases : registre de couverture, chasse par classes d'attaques, **réfutation par un vérificateur frais**, verdicts confirmé/à valider/réfuté, runs additifs | **ADOPTÉ** (Raphaël, 2026-11-03) : `docs/audit-protocole.md` amendé — nouveau §5 « Registre de couverture et réfutation » (commit `39043d5`) ; l'audit n°5 utilisera la méthode |
| **obra/superpowers** (MIT) | Méthodologie complète pour agents de code (spec → design validé par morceaux → plan → sous-agents, TDD strict, YAGNI/DRY) | **PICORÉ** : 2 concepts only (fiche economie-tokens §7) — plan « lisible par un junior enthousiaste », sous-agents par lot. Pas d'installation en bloc : ~70 % déjà couvert par le protocole OmniFab (ADR phase 0, rouge constaté avant correctif, rejets anti-spéculation, budget anti-saturation) |
| **pbakaus/impeccable** (Apache-2.0) | Vocabulaire de design web + détecteur déterministe de « slop » (62 règles, moteur Rust), audit accessibilité/responsive P0-P3 | **ÉCARTÉ** : cible DOM/CSS, OmniFab = egui desktop. À revisiter au jalon 9 si interfaces HTML ; idée transposable retenue : règles déterministes de qualité en CI (déjà pratiquée pour i18n/`help_id`) |

## Historique

- v1 (2026-11-03) : création — décisions D1-D4 de l'étude Tangency Labs (ADR-0035), critères de bascule solidcore, répertoire de veille initial ; matrice passée en v9 au dépôt (commit `646fdb9`).
- v2 (2026-11-03) : ajout de solidcraft-library — référence structurelle pour la future bibliothèque paramétrique (descripteur JSON Schema versionné, gouvernance par asset, catégories réservées) ; signaux à surveiller : catégorie « parts », format de pack.
- v3 (2026-11-03) : position stratégique face à la famille Craft (continuer OmniFab, contribution opportuniste) + section « Outillage du processus » : security-audit-skill ADOPTÉ (protocole d'audit §5, commit `39043d5`), superpowers picoré (economie-tokens §7), impeccable écarté (note jalon 9).
