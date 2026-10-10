# Contribuer à OmniFab

Ce document décrit les **conventions non négociables** du projet. Chaque contribution (PR, commit, issue) doit les respecter. Ces conventions sont raffinées au fil des ADR ; en cas de doute, ouvrir une issue pour en discuter avant de coder.

## Statut du projet

Nous sommes en **phase 0 (conception)** : pas de code avant validation explicite. Chaque décision tranchée fait l'objet d'un **ADR** dans [`docs/adr/`](docs/adr/).

## Conventions de code

- **Rust idiomatique**, `cargo fmt` et `cargo clippy` sans avertissement.
- **rustdoc obligatoire** sur toute API publique — la CI échoue en son absence.
- **Unités strictes** : jamais de `f64` nu pour une grandeur physique. Newtypes dédiés pour longueur, angle, etc. Tolérance de comparaison : 1e-6 mm.
- **Codes d'erreur typés** (`thiserror`), pas de `unwrap()` hors des tests.
- **Wasm-friendly** : feature flags `native`/`wasm` ; aucun accès natif hors trait d'abstraction.

## Conventions produit & architecture

- **Six verrous d'architecture** : conteneur ZIP versionné ; event sourcing ; planegcs pour le solveur de contraintes ; tokio + canaux ; unités strictes ; moteur unique de liaisons (alias + nodal).
- **`help_id` sur chaque widget** de l'interface : toute aide contextuelle est adressable.
- **i18n dès le jalon 1** avec Fluent (`.ftl`). Aucune chaîne utilisateur codée en dur.
- **Assistants = templates déclaratifs** : les assistants de conception sont des données au-dessus de `forge-values`/`forge-cad`/`forge-cam`/`forge-io`, jamais du code spécialisé.
- **Simulation d'abord** : le pilotage machine se développe contre un simulateur ; aucune machine physique requise.

## Données

- Matériaux, machines, normes : licences ouvertes dédiées (**ODbL / CC0 / CC-BY**), sourcées et citées. **Jamais GPL pour des données.**

## Logiciels propriétaires

- S'inspirer **uniquement du comportement et des workflows documentés** (Revit, PolyBoard, Fusion 360, TopSolid…). **Jamais** de code, jamais de EULA accepté.

## Processus

1. Une décision → un **ADR** (`docs/adr/TEMPLATE.md`), avec alternatives rejetées.
2. Une PR → build + clippy + rustdoc + tests verts sur la **CI multi-plateformes** (Linux, Windows, macOS).
3. Une discussion GitHub = un objectif ; le dépôt est la seule source de vérité du code.
