# OmniFab

> **⚠️ Statut : phase 0 en préparation.** Aucun code fonctionnel pour l'instant — ce dépôt accueille la conception préalable (ADR, conventions) avant les premières crates.

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

## Documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — conventions de développement obligatoires.
- [`docs/adr/`](docs/adr/) — décisions d'architecture (ADR).

## Langue

> 📝 Toute la documentation de ce dépôt est **en français** pendant la phase de développement privé. Elle sera traduite en anglais avant le passage public du dépôt.

---

*Projet personnel — conçu avec soin, publié libre.*
