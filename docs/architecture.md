# Architecture des crates

Schéma de dépendances du noyau : chaque crate ne voit que ses
dépendances directes, jamais au-delà (garde-fou de la phase 0, ADR-0000 ;
frontières mises à jour par l'ADR-0013 pour le solveur de contraintes).

```mermaid
flowchart TD
    subgraph L1[L1 — applications]
        android["apps/omnifab-android — cible mobile (jalon 5)"]
        desktop["apps/omnifab-desktop — démonstrateur egui"]
    end

    subgraph L2[L2 — domaine métier]
        session["forge-session — commandes, projections, undo/redo"]
        values["forge-values — moteur de liaisons (expressions, alias)"]
        io["forge-io — import/export à budgets"]
        gcs2d["forge-gcs — solveur de contraintes 2D (trait Solveur2D)"]
        gcssys["forge-gcs-sys — bindings brutes (shim ABI C planegcs)"]
        planegcs[("planegcs (C++) — visible uniquement dans forge-gcs")]
    end

    subgraph L3[L3 — fondations]
        geom["forge-geom — géométrie minimale"]
        units["forge-units — newtypes d'unités"]
        i18n["forge-i18n — catalogue Fluent + aide intégrée"]
        doc["forge-doc — conteneur ZIP + journal"]
    end

    subgraph L4[L4 — futures]
        cad["forge-cad (CAO) — jalon 6"]
        cam["forge-cam (FAO) — jalon 7"]
        occt[("OCCT (C++) — visible uniquement dans forge-cad")]
    end

    desktop --> session
    desktop --> io
    desktop --> values
    desktop --> i18n
    session --> values
    session --> io
    session --> geom
    io --> geom
    values --> units
    geom --> units
    i18n --> doc
    gcs2d --> units
    gcs2d --> gcssys
    gcssys --> planegcs
    session -. "esquisse (phase D)" .-> gcs2d
    cad --> session
    cad --> gcs2d
    cad --> occt
```

Règles de frontière (phase 0, complétées par l'ADR-0013) :

1. Le journal des événements et le conteneur ZIP appartiennent à
   forge-doc ; personne d'autre ne touche aux octets.
2. OCCT et planegcs sont des dépendances C++ **enfermées** : OCCT est
   invisible hors de forge-cad (ADR-0004), planegcs est invisible hors
   de forge-gcs (ADR-0013) — la frontière visible du projet est le trait
   `Solveur2D`. `forge-gcs-sys` (bindings brutes) et le shim
   `third_party/planegcs-shim` ne sont consommés que par `forge-gcs`.
3. forge-values est la **seule** source d'évaluation des expressions ;
   forge-cad (jalon 6) l'appellera pour le paramétrage, et pilotera les
   cotes de forge-gcs par ses expressions (phase E).
4. L'affichage passe par les messages Fluent de forge-i18n : pas de
   chaîne visible en dur dans le noyau.
5. Les unités ne circulent jamais en `f64` nu : newtypes de forge-units
   à la frontière de chaque crate (forge-gcs réintroduit les newtypes
   au-dessus de l'ABI C, qui parle en `double`).
6. Événements et blobs suivent le schéma de forge-doc (ADR-0001) : les
   gros objets (géométrie importée) vivent dans des blobs référencés,
   jamais dans le journal.

Licences des crates : GPL-3.0-or-later à la racine, MPL-2.0 pour les
crates isolées ; forge-gcs, forge-gcs-sys et le shim sont en
LGPL-2.1-or-later (héritage planegcs, ADR-0013) — compatibles avec la
GPL du dépôt ; les crates MPL-2.0 n'en dépendent pas.

Vendoring : `third_party/planegcs` est un sous-module git figé, décrit
par `third_party/planegcs-MANIFEST.md` (ADR-0013) ; son contenu n'est
jamais modifié localement.
