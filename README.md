# OmniFab

> **⚠️ Statut : jalon 0 en cours.** ADR-0000 à 0004 validés (2026-10-11) ; le noyau minimal est implémenté : `forge-units`, `forge-doc`, `forge-values` et le démonstrateur `omnifab-desktop`. Reste à livrer pour clore le jalon : la preuve de concept egui sur Android (tactile, clavier virtuel).

**OmniFab** est un logiciel libre de fabrication numérique A→Z : **CAO** (conception assistée par ordinateur), **CAM** (fabrication assistée par ordinateur), génération de **G-code** et **pilotage machine**.

L'objectif : couvrir toute la chaîne, de l'idée du meuble à la pièce usinée, dans une seule application native — sans dépendre de logiciels propriétaires.

## Vision courte

- Une application modulaire en crates, du modèle paramétrique jusqu'au pilotage machine.
- Des **assistants de conception** (meubles, caissons…) définis comme des *templates déclaratifs* — des données, pas du code.
- Une **simulation d'abord** : aucun matériel physique n'est requis pour développer ; le pilotage machine est validé sur simulateur.
- Interopérabilité openBIM : import **IFC**, export **DSTV/NC1**, classification multi-référentielle (OmniClass / Uniclass / UniFormat).

## Stack technique

| Domaine | Choix |
|---|---|
| Langage | Rust |
| UI | egui (plan B mobile : Slint) |
| Rendu | wgpu |
| Géométrie | OpenCascade (OCCT) via bindings |
| Contraintes 2D | planegcs (via FFI) |
| Concurrence | tokio + canaux |
| Licence | GPL-3.0+ (MPL/Apache pour les crates isolées) |

> 📝 Le fichier `LICENSE` sera ajouté en local depuis la source canonique (gnu.org) avant toute publication publique.

## Structure du dépôt

| Chemin | Rôle |
|---|---|
| `crates/forge-units` | Newtypes d'unités strictes (longueur, angle) — crate de base, zéro dépendance (ADR-0000, verrou 5) |
| `crates/forge-doc` | Conteneur ZIP versionné + journal d'événements JSON Lines, migrations, snapshots non autoritaires (ADR-0001) |
| `crates/forge-values` | Moteur unique de liaisons : évaluation paresseuse + expressions déterministes (ADR-0002) |
| `apps/omnifab-desktop` | Démonstrateur egui : fenêtre, viewport, journal d'événements affiché, aller-retour ZIP + rejeu |
| `docs/architecture.md` | Schéma cible des crates et invariants de dépendances (garde-fou) |
| `docs/adr/` | Décisions d'architecture (ADR-0000 à 0004 acceptés) |
| `docs/error-codes.md` | Registre central des codes d'erreur `OF-<DOMAINE>-<NNNN>` (ADR-0003) |

Les crates suivantes (forge-geom, forge-cad, forge-render, forge-paint, forge-cam, forge-gcode, forge-link, forge-io, omnifab-android, omnifab-web) seront créées à leur jalon, conformément à l'ADR-0000 : pas de crates vides.

## Développement

```bash
cargo test --workspace        # tests (dont vérification du registre des codes d'erreur)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p omnifab-desktop  # démonstrateur : paramètres, journal, viewport, ZIP + rejeu
```

## Documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — conventions de développement obligatoires.
- [`docs/architecture.md`](docs/architecture.md) — architecture cible du workspace (garde-fou).
- [`docs/adr/`](docs/adr/) — décisions d'architecture (ADR).
- [`docs/error-codes.md`](docs/error-codes.md) — registre central des codes d'erreur.

## Langue

> 📝 Toute la documentation de ce dépôt est **en français** pendant la phase de développement privé. Elle sera traduite en anglais avant le passage public du dépôt.

---

*Projet personnel — conçu avec soin, publié libre.*
