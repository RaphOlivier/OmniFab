# ADR-0000 : Décisions initiales de la phase 0 (cadrage général)

- **Statut : accepté**
- **Date : 2026-10-10**
- **Décideurs : Raphaël Olivier**

## Contexte

OmniFab démarre : logiciel libre Rust de fabrication A→Z (CAO, CAM, G-code, pilotage machine), UI egui, rendu wgpu, géométrie OCCT, contraintes planegcs, concurrence tokio, licence GPL-3.0+. Ce cadrage a été mûri hors dépôt (vision 15 points, matrice fonctionnalité × logiciel v7 ≈ 265 fonctions, ~55 logiciels de référence). Le présent ADR acte formellement les décisions structurantes de phase 0 et renvoie, pour les points techniques ouverts, aux ADR-0001 à ADR-0004.

## Décision

Nous actons les **six verrous** (décisions irréversibles du projet) :

1. **Conteneur ZIP versionné** pour les documents projet : manifeste JSON versionné + un fichier par objet + blobs binaires pour maillages/images, à l'image des .FCStd / OpenDocument / 3MF. Champs `version` et outil de migration dès le jour 1. Porté par la crate `forge-doc` (créée à cet effet).
2. **Event sourcing** comme modèle de document : chaque opération utilisateur est un événement sérialisé et rejouable. Porté par `forge-doc`. Détails du format : ADR-0001.
3. **planegcs** comme solveur de contraintes 2D (le solveur de FreeCAD, LGPL) via FFI — pas de solveur maison. Réécriture Rust éventuelle plus tard, à froid.
4. **tokio + canaux** : tokio pour la communication machine, canaux entre machine et UI, géométrie en blocs parallélisés (jamais en async). Règle d'or : un freeze de l'UI ne coupe jamais le pilotage machine.
5. **Unités strictes** : newtypes (longueur, angle…) dans toutes les API métier, jamais de `f64` nu ; tolérance géométrique 1e-6 mm. Portées par la crate de base `forge-units` (créée à cet effet).
6. **Moteur unique de liaisons** (`forge-values`) : un seul graphe de dépendances réactif + expressions alimente les deux interfaces (alias/expressions et éditeur nodal type Xpresso) — zéro incohérence possible. Mode d'évaluation : ADR-0002.

Et trois **décisions annexes** :

7. **Assistants de conception = templates déclaratifs** : un assistant est une donnée versionnée et partageable (formulaire + règles + valeurs normées), jamais du code ; il s'exécute au-dessus de forge-values / forge-cad / forge-cam / forge-io.
8. **Simulation d'abord** pour le pilotage machine : aucun matériel physique requis pour développer ; les contrôleurs sont validés sur simulateur (jalon 5).
9. **Premier assistant : meuble/caisson** (référence comportementale PolyBoard), aligné sur le cas d'école de la tranche horizontale (fabriquer un meuble de bout en bout).

**Workspace cible** : noyau de crates sans dépendance à l'interface ; frontends fins (desktop egui, Android, web wasm) au-dessus. Ajout de deux crates à la liste initiale : `forge-units` (unités strictes) et `forge-doc` (conteneur ZIP + event sourcing) — les deux verrous les plus centraux n'avaient pas de crate porteuse. Stratégie de création : **noyau minimal au jalon 0** (forge-units, forge-doc, forge-values, app de démo egui desktop + PoC Android) ; les autres crates sont créées à leur jalon, avec rustdoc et feature flags native/wasm dès leur création. Une frontière ne change que par ADR.

## Alternatives rejetées

**Conteneur ZIP** : (a) fichier binaire opaque unique — indéboggable, corruption totale à la moindre casse ; (b) base SQLite — puissante mais hostile au versionnement Git, à l'archivage manuel et aux migrations inspectables. (c) dossier de fichiers simples — laissé pour les sources de données, mais un projet doit être un objet unique, copiable et archivable.

**Event sourcing** : (a) sauvegardes périodiques de l'état — historique perdu entre deux sauvegardes, undo limité ; (b) CRDT — conçu pour l'édition collaborative temps réel (hors périmètre actuel), complexité majeure pour un bénéfice non requis ; (c) arbre de commandes (command pattern) avec états — redondant avec ce que l'event sourcing donne gratuitement.

**planegcs** : (a) réécriture Rust d'un solveur — des années-personnes pour une durabilité numérique inférieure ; (b) autres solveurs existants — moins matures sur le besoin précis (contraintes géométriques 2D CAO).

**tokio + canaux** : (a) threads bruts et mutex — pièges de synchronisation partout ; (b) tout dans le thread UI — un freeze de l'interface arrêterait une broche en pleine pièce : inacceptable pour la sécurité ; (c) async pour la géométrie — l'async n'apporte rien aux calculs CPU et complique les blocs parallélisés.

**Unités strictes** : (a) convention « tout en mm » documentée — une conversion oubliée casse une pièce sans avertissement ; (b) système d'unités à l'exécution — les erreurs de conversion apparaissent chez l'utilisateur au lieu d'apparaître à la compilation.

**Moteur unique de liaisons** : (a) deux moteurs séparés (alias et nodal) — incohérences inévitables entre les deux vues ; (b) éditeur nodal seul — trop abrupt comme interface d'entrée ; (c) alias/expressions seul — plafonne l'expressivité (pas de routage conditionnel, pas de fonctions graphiques).

**Assistants déclaratifs** : assistants codés en dur — non partageables, non versionnables indépendamment du logiciel, réservés aux développeurs.

**Simulation d'abord** : matériel physique immédiat — ajoute risque (collision, casse), dépendance à une machine donnée et freine l'exécution des tests en CI.

**Meuble/caisson en premier** : les autres familles (escaliers, charpente, acier) viennent au jalon 10 ; le meuble maximise la valeur pédagogique de la première tranche horizontale (CAO paramétrique → découpe/gravure → CAM 2,5D → LinuxCNC).

**Crates : squelettes complets dès le jalon 0** : des crates vides pourrissent (aucun code réel pour défendre leur frontière) et des frontières prématurées coûtent cher à déplacer. Le schéma cible reste le garde-fou : toute frontière ne change que par ADR.

## Conséquences

- **Positives** : chaque verrou élimine toute une classe d'erreurs (mélange d'unités, incohérence de paramétrage, historique incomplet) ; le projet reste déboggable et maintenable par des développeurs qui n'ont pas écrit le code.
- **Négatives / coûts acceptés** : event sourcing = discipline stricte sur la forme des événements (tout doit être rejouable) ; planegcs = dette FFI à entretenir ; unités strictes = verbosité d'écriture ; ZIP = surcoût d'implémentation de l'archivage par rapport à un fichier plat.
- **Décisions futures suspendues** : stratégie de migration des événements (ADR-0001), évaluation du graphe (ADR-0002), schéma d'erreurs (ADR-0003), stratégie de binding OCCT (ADR-0004) ; réécriture Rust éventuelle de planegcs (à froid, jalon ≥ 8) ; collaboration temps réel (CRDT) hors périmètre tant que non décidée.
