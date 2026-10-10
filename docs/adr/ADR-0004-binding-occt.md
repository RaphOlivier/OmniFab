# ADR-0004 : Stratégie de binding OpenCascade (OCCT)

- **Statut : accepté**
- **Date : 2026-10-10**
- **Décideurs : Raphaël Olivier**

## Contexte

Le noyau CAO repose sur OCCT (LGPL, liaison dynamique obligatoire). Les bindings Rust (opencascade-rs, occt-rs) sont en développement précoce, sans API stabilisée : c'est le risque technique nº1 du projet. Le prompt fondateur prévoit un jalon de consolidation : reprendre un binding existant et le fiabiliser, ou wrapper ciblé via cxx.

## Décision

Partir du binding **opencascade-rs** (le plus actif) et l'isoler **entièrement derrière la couche `forge-cad`** : aucune crate hors forge-cad ne connaît OCCT. forge-cad expose son propre modèle de géométrie (IDs de formes, opérations booléennes haut niveau, types forge-units), traduit vers/depuis OCCT dans une sous-couche interne `occt` (module `forge-cad/src/occt`). Un corpus de **tests de géométrie en or** (golden files : opérations booléennes, fillets, extrusions, résultats attendus sérialisés en BREP/JSON) valide chaque montée de version du binding et chaque patch de consolidation. Si opencascade-rs casse ou stagne : bascule vers un **wrapper ciblé via cxx** limité aux modules OCCT nécessaires (modeling data, modeling algorithms, STEP/BREP) — le périmètre à re-wrapper est borné par l'API déjà isolée. OCCT reste liée dynamiquement (exigence LGPL).

## Alternatives rejetées

- **Wrapper cxx maison dès le départ** : contrôle maximal, mais des mois de travail avant le premier résultat ; la communauté avance sur opencascade-rs en parallèle.
- **Attendre la maturité d'un binding** : le projet démarre avec la géométrie comme brique du cas d'école (jalon 2-3) ; attendre retarde toute la tranche horizontale.
- **Réécriture d'un noyau géométrique en Rust** (hors OCCT) : hors de portée (années-personnes) ; les noyaux alternatifs libres (parasolid-like) n'existent pas en Rust.
- **Sous-ensemble compilé wasm dès maintenant** : reporté au jalon 9 (cible web) ; les règles wasm-friendly (feature flags, pas d'accès natif hors trait) restent appliquées dès le jalon 0 pour ne pas fermer la porte.

## Conséquences

- **Positives** : le risque nº1 est confiné à une sous-couche ; la bascule cxx est bornée et planifiable ; les golden tests donnent une sécurité de non-régression durable.
- **Négatives / coûts acceptés** : couche de traduction à écrire et maintenir ; double modélisation partielle (modèle forge-cad vs OCCT) ; contributions à remonter en amont au binding (positif pour l'écosystème, coût en temps).
- **Décisions futures suspendues** : périmètre exact du sous-ensemble wasm d'OCCT (jalon 9) ; stratégie de contribution amont à opencascade-rs ; fork de consolidation si l'amont refuse les correctifs.
