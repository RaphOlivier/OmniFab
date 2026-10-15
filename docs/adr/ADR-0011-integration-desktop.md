# ADR-0011 : intégration du noyau dans le démonstrateur — session, undo/redo, import/export forge-io

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-15)
- **Date :** 2026-10-15
- **Décideurs :** Raphaël Olivier

## Contexte

Le jalon 1 exige « noyau + undo + import/export SVG/DXF/STL + help_id + aide ».
Les briques existent et sont vertes : forge-session (ADR-0008 : commandes,
projections, undo/redo par curseur), forge-io (ADR-0009 : parseurs à budgets,
codes OF-IO-0001..0009), forge-doc (ADR-0000/0001 : conteneur ZIP, journal,
blobs — `Document::set_blob`/`blob`, les gros objets ne vivent jamais dans le
journal). Mais le démonstrateur omnifab-desktop (jalon 0) manipule encore
`Document` et `ValueGraph` en direct — il duplique la projection (`record`,
`apply`, `rebuild_from_journal`), n'a ni undo ni redo, et n'importe/exporte
rien : forge-io et forge-session ne sont pas branchés. La fiche architecture
pose l'étape : brancher forge-io et forge-session dans le démonstrateur pour
clore le jalon 1.

## Décision

1. **Le démonstrateur adopte `Session`** (ADR-0008) : `DemoApp` porte une
   `Session` au lieu de `Document` + `ValueGraph` ; les méthodes dupliquées
   `record`/`apply`/`rebuild_from_journal` disparaissent (la session fait
   déjà ce travail, mieux) ; les actions existantes (saisies, expression)
   passent par les commandes `SetInput`/`SetExpression` ;
   « Enregistrer/Ouvrir » passent par `Session::save_to_path` /
   `Session::from_document`. L'acteur devient `desktop.demo`.

2. **Undo/redo visibles** : boutons « Annuler » / « Rétablir » dans la barre
   du haut (désactivés quand `can_undo`/`can_redo` est faux) et raccourcis
   clavier Ctrl+Z / Ctrl+Y. Une annulation impossible par raccourci est
   ignorée silencieusement (l'erreur `UndoEmpty` reste une erreur d'API pour
   les programmeurs, pas un message d'interface).

3. **Commandes d'import** : deux nouvelles commandes dans forge-session,
   `ImportTraces` (SVG ou DXF → `Vec<Path2>`) et `ImportMesh` (STL →
   `Mesh`), une transaction = un groupe d'undo. Nouveaux types d'événements
   schéma v1 : `geom.traces.imported` et `geom.mesh.imported`, dont les
   `data` portent `format` (`svg`/`dxf`/`stl`), `blob` (nom du blob) et les
   compteurs (`traces`, `sommets` ou `triangles`).

4. **Le contrat `Command` évolue pour porter des blobs** : `plan` retourne
   événements **et** blobs à déposer (`CommandPlan`). L'exécution est
   tout-ou-rien : si une projection refuse, ni blobs ni événements ne sont
   écrits (le rejeu du journal appliqué rétablit l'état d'avant, mécanisme
   déjà en place). `SetInput`/`SetExpression` retournent zéro blob — les
   implémenteurs sont tous internes au dépôt (crate 0.1.0, pas de
   compatibilité externe à garantir).

5. **Géométrie dans des blobs, jamais dans le journal** (ADR-0001) : le blob
   contient les **octets d'origine** du fichier importé (ex.
   `objects/geometry/traces.svg`) ; l'événement n'en porte que la référence.
   La conversion octets → objets (`forge_io::svg::import`, etc.) reste à la
   lecture, sous budgets, avec cache mémoire non autoritaire côté
   application (clé = nom du blob, invalidée par toute nouvelle projection).

6. **`GeomProjection` dans forge-session** (sans dépendance à forge-geom) :
   reconstruit par rejeu l'état « géométrie importée » — références et
   métadonnées seulement (format, blob, compteurs), pas les objets lourds.
   Construite par défaut dans `Session::new` comme `ValuesProjection`.
   Sémantique des types inconnus affinée : un type « ignoré » pour
   l'interface est un type inconnu de **toutes** les projections
   (intersection) — sinon chaque événement de géométrie apparaîtrait comme
   « ignoré » dans le panneau journal (type connu de GeomProjection,
   simplement hors du domaine des liaisons).

7. **GC des blobs orphelins** (prévu par ADR-0008, devient nécessaire) : le
   trait `Projection` gagne `referenced_blobs()` (défaut : vide) ; quand la
   session abandonne les événements défaisables (nouvelle commande après
   undo, sauvegarde), elle purge du document les blobs référencés par
   aucun événement appliqué (nouvelle méthode de retrait dans forge-doc,
   symétrique de `set_blob`).

8. **Export hors commande** : l'export ne change pas l'état — pas de
   journalisation. L'application lit la référence courante de la
   `GeomProjection`, récupère les octets du blob
   (`session.document().blob(...)`, lecture seule, la frontière ADR-0008
   « ne pas toucher Document » est une frontière d'écriture), re-parse via
   forge-io puis exporte via forge-io (`svg::export`, `dxf::export`,
   `stl::export`) vers des chemins de démonstration fixes
   (`demo-export.svg`, `.dxf`, `.stl`). L'import lit de même
   `demo-import.svg`, `.dxf`, `.stl` — **pas de sélecteur de fichier au
   jalon 1**.

9. **Viewport** : les tracés importés sont dessinés par-dessus le rectangle
   de démonstration (couleur distincte, mise à l'échelle dans le viewport) ;
   le maillage STL n'est pas dessiné au jalon 1 (rendu wgpu au jalon 2) —
   un compteur (tracés, sommets, triangles) s'affiche à la place.

10. **Interface, aide, erreurs** : nouvelles clés FTL et pages d'aide
    complètes (titre/description/exemple, CI) pour chaque nouveau widget :
    annuler, rétablir, les trois imports, les trois exports, le compteur de
    géométrie ; messages d'information (import réussi avec compteurs,
    export réussi avec chemin). Aucun nouveau code d'erreur : les refus de
    blob existent (OF-DOC-0011/0013), les refus de parse (OF-IO-0001..0009)
    s'affichent par le canal d'erreur existant du viewport.

## Alternatives rejetées

- **Garder `Document` + `ValueGraph` en direct dans le démonstrateur**
  (statu quo) : pas d'undo/redo, projection dupliquée et divergente, en
  contradiction avec la frontière ADR-0008.
- **Géométrie inline dans les événements (JSON)** : viole ADR-0001 (les gros
  objets ne vivent jamais dans le journal) ; un SVG de 100 000 sommets
  rendrait le JSON Lines illisible et le conteneur énorme.
- **`GeomProjection` dépendante de forge-geom** : inutile — la projection
  ne porte que références et compteurs ; les objets proviennent de forge-io
  à la lecture. Une dépendance de plus sans bénéfice.
- **Export comme commande journalisée** : l'export ne change pas l'état du
  document ; le journaliser polluerait l'historique d'annulation.
- **Sélecteur de fichiers (rfd ou natif) au jalon 1** : dépendance et UX
  hors périmètre du démonstrateur ; le vrai gestionnaire de projets arrive
  avec le produit (jalon 2+).
- **Affichage du maillage STL au jalon 1** : le rendu 3D est wgpu, jalon 2 ;
  un faux affichage 2D d'un maillage mentirait sur la capacité réelle.
- **GC des blobs par convention de nommage dans forge-doc** : forge-doc ne
  peut pas savoir quels blobs les événements référencent (les références
  vivent dans les `data` JSON, interprétées par les projections) ;
  c'est aux projections de déclarer leurs références.
- **Import sans undo (mutation directe)** : l'import est une action
  utilisateur ; ne pas l'annuler briserait la promesse « toute action est
  un événement rejouable » dès le premier jalon livré.

## Conséquences

- **Positives** : le démonstrateur prouve le jalon 1 de bout en bout
  (ouvrir, importer, annuler, rétablir, exporter, enregistrer, rejouer) ;
  une seule implémentation de projection (forge-session) au lieu de deux
  divergentes ; le journal reste léger et lisible ; les blobs orphelins
  disparaissent mécaniquement après undo+sauvegarde.
- **Négatives / coûts acceptés** : évolution du trait `Command` (0.1.0,
  implémenteurs internes uniquement) et de `Projection` (méthode à défaut,
  non cassante) ; re-parse de la géométrie à la lecture (borné par les
  budgets, cache non autoritaire) ; chemins de fichiers fixes, assumés
  comme démonstrateur ; l'intersection des types inconnus est une sémantique
  plus fine à tester.
- **Risques d'implémentation** : règle de validation des noms de blobs
  (OF-DOC-0013) à vérifier pour le nom choisi ; ordre truncate → GC →
  dépôt → écriture à bien séquencer dans `execute` ; rustfmt : ne jamais
  laisser une ligne à ±2 colonnes d'un seuil (leçons du jalon 1).
- **Décisions futures suspendues** : sélecteur de fichiers et bibliothèque
  de projets (jalon 2) ; rendu des maillages (jalon 2, wgpu) ; format de
  géométrie compact dédié dans les blobs si le re-parse devient un
  goulot (mesurer d'abord) ; commandes par lot (suspens ADR-0008).
