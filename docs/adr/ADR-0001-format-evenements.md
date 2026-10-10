# ADR-0001 : Format du journal d'événements (event sourcing)

- **Statut : accepté**
- **Date : 2026-10-10**
- **Décideurs : Raphaël Olivier**

## Contexte

Le verrou nº2 (ADR-0000) impose l'event sourcing : chaque opération utilisateur est un événement sérialisé et rejouable. Il faut trancher le format du journal : sérialisation, versionnage du schéma, gestion du volume, et stratégie de migration entre versions du logiciel.

## Décision

Le journal d'événements est stocké en **JSON lisible**, un événement par ligne (JSON Lines) dans le conteneur ZIP du projet (`events.jl`). Chaque événement porte : un identifiant unique, son **type** (nom stable, jamais renommé une fois publié), la **version de schéma** de ce type, l'horodatage, l'identifiant de l'acteur/zone UI, et les données propres à l'événement. Les types d'événements sont de petits enregistrements fermés (une struct Rust par type, sérialisée via serde) ; les gros objets (maillages, images, géométrie OCCT) ne sont jamais dans les événements — ils vivent comme fichiers/blobs dans l'archive et les événements ne portent que des références. Le modèle de document est reconstruit par **projection** (rejeu du journal) ; des **snapshots** périodiques (état projeté sérialisé, versionné) accélèrent l'ouverture des projets longs, sans jamais faire autorité : un snapshot incompatible est ignoré et le journal rejoué. Les migrations sont des fonctions pures `v_n -> v_n+1` par type d'événement, testées en CI sur des journaux réels archivés (corpus de fixtures).

## Alternatives rejetées

- **Sérialisation binaire compacte (bincode/postcard)** : ~2-4× plus compact et plus rapide à parser, mais illisible (impossible d'ouvrir un projet dans un éditeur de texte pour diagnostiquer), et les migrations de schéma binaire sont douloureuses. Le volume n'est pas un souci : les gros objets sont hors journal. Réévaluation possible plus tard comme **format d'export interne** du snapshot, sans toucher au journal.
- **Un seul gros document JSON** (au lieu d'un événement par ligne) : lecture plus simple d'un coup, mais réécriture totale à chaque événement et diffs illisibles.
- **SQLite pour le journal** : requêtable, mais alourdit le conteneur, complique l'archivage manuel et ajoute une dépendance C.
- **Schéma unique global versionné** (au lieu d'une version par type d'événement) : simple au départ, mais toute modification d'un seul type forcerait un numéro de version global — migrations en cascade inutiles.

## Conséquences

- **Positives** : projets débogables à la main ; migrations par type, fines et testables ; rejeu = undo illimité et reprise multi-appareils ; snapshots = ouverture rapide.
- **Négatives / coûts acceptés** : journal plus volumineux qu'en binaire (acceptable : gros objets exclus) ; discipline stricte : jamais renommer un type publié, toujours ajouter un champ optionnel ou créer un nouveau type.
- **Décisions futures suspendues** : compaction du journal (fusion de segments rejoués) ; signature/intégrité de l'archive ; format du snapshot (JSON au départ, binaire envisageable).
