# MANIFEST — third_party/planegcs

Vendoring de l'extraction standalone de planegcs (décision ADR-0013,
jalon 2 phase A). Ce fichier est versionné par le dépôt OmniFab, hors du
sous-module (l'intérieur de `third_party/planegcs/` appartient au dépôt
amont et n'est jamais modifié localement).

| Champ | Valeur |
|---|---|
| Dépôt amont | https://github.com/2wiceUponATime/PlaneGCS |
| Commit figé (sous-module) | `2a1d6e427edee35210ac076fd51b8624936271b6` — à vérifier après l'ajout : `git -C third_party/planegcs rev-parse HEAD` |
| Version annoncée | 1.1.1 (CMakeLists.txt du sous-module) |
| Branche FreeCAD synchronisée | `releases/FreeCAD-1-1` (major.minor = branche FreeCAD, patch = re-synchronisations de l'extraction) |
| Licence | LGPL-2.1-or-later (fichier `LICENSE` du sous-module, texte intégral recopié dans les `LICENSE` de forge-gcs-sys et forge-gcs) |
| Cible CMake | `PlaneGCS::planegcs` (bibliothèque statique par défaut) |
| Dépendances au build | Eigen3 (headers), fmt, Boost (headers — Boost.Graph uniquement, privé à planegcs) ; CMake ≥ 3.26, C++20 |
| Ajouté par | ADR-0013 (jalon 2 phase A), 2026-10-15 |

## Règles de mise à jour

- **Jamais de modification locale** du contenu du sous-module : le dépôt
  amont régénère `include/` et `src/` depuis FreeCAD (`update.sh`).
- Une re-synchronisation FreeCAD se fait par : bump du commit figé
  (`git -C third_party/planegcs fetch && git -C … checkout <sha>` puis
  commit du gitlink), mise à jour de ce MANIFEST (commit + patch), et
  vérification que le shim `gcs_c_api` absorbe les changements d'API —
  l'ABI C ne doit pas bouger.
- L'absence de tag amont est assumée : le commit SHA est la référence
  unique (aucun tag publié par l'extraction au 2026-10-15).
