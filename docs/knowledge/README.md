# Knowledge OmniFab — guide de lecture

Ce dossier est la **copie de référence publique** de la Knowledge du
projet (ADR-0028) : fiches de décisions, archives de discussions et
journal des fonctions. La copie de travail vit sur la plateforme de
l'assistant (Vibe / Mistral), qui **fait foi** en cas de divergence ;
ce miroir est resynchronisé à chaque mise à jour d'une fiche.

## Pourquoi ce dossier existe

Le dépôt Git est la source de vérité du **code** ; cette Knowledge est
la source de référence du **pourquoi** — le raisonnement, les
alternatives rejetées et l'historique qui ont mené à chaque décision.
Elle est destinée autant aux humains qu'aux IA qui découvrent le
projet (un agent peut charger ce dossier comme contexte de travail).

## Parcours recommandés

**Comprendre le projet en général**
1. [`KNOWLEDGE.md`](KNOWLEDGE.md) — index général (ce que couvre chaque fiche) ;
2. [`architecture/KNOWLEDGE.md`](architecture/KNOWLEDGE.md) — crates, six verrous
   techniques, jalons 0-10, résumé de tous les ADR (0000 à 0027) ;
3. [`prompt-fondateur.md`](../prompt-fondateur.md) (au `docs/` parent) — la vision
   en 15 points.

**Comprendre une décision précise**
1. La fiche du domaine concerné : `architecture/`, `conventions/`,
   `formats/`, `machines/` ou `securite/` ;
2. L'ADR complet correspondant dans [`../adr/`](../adr/) — chaque ADR
   porte les alternatives rejetées ;
3. Le résumé de l'entrée d'archive dans [`archives/`](archives/) —
   le fil et les arguments qui y ont mené.

**Retracer l'évolution d'une fonction**
- [`historique/journal-fonctions.md`](historique/journal-fonctions.md) —
  journal append-only, une ligne par évolution, consulté par `grep`
  sur l'identifiant de la fonction (ex. `grep "F:liaisons:" ...`).

**Inventaire des fonctionnalités et logiciels de référence**
- [`../matrice-fonctionnalites.md`](../matrice-fonctionnalites.md) —
  ≈ 277 fonctions × logiciels libres et commerciaux, incluant les
  inspirations (comportements uniquement, jamais de code — cf.
  [`conventions/KNOWLEDGE.md`](conventions/KNOWLEDGE.md)).

**Codes d'erreur**
- [`../error-codes.md`](../error-codes.md) — registre central `OF-<DOM>-NNNN`,
  vérifié par CI.

## Conventions de lecture

- **Versionnage** : chaque fiche porte `**Version :** vN (date)` en tête
  et une section `Historique` en pied — la fraîcheur d'un sujet se lit
  d'un coup d'œil, et le delta de la dernière mise à jour y est
  résumé.
- **Jalon** : les jalons 0-10 structurent la roadmap ; `[JX]` en tête
  d'une archive indique le jalon dominant de la discussion.
- **ADR** : *architecture decision record* — un document de décision,
  numéroté, avec alternatives rejetées. Les numéros sont référencés
  partout (`ADR-0018`, `ADR-0028`…) et se lisent dans `../adr/`.
- **Six verrous** : les six décisions techniques irréversibles
  (conteneur ZIP, event sourcing, planegcs, tokio, unités strictes,
  moteur de liaisons unique) — détaillées dans
  [`architecture/KNOWLEDGE.md`](architecture/KNOWLEDGE.md).
- **Langue** : le français est la langue de référence du projet.

## Notes pour les agents IA

- Pour chercher une décision : `grep` sur les identifiants stables —
  `ADR-NNNN`, `OF-XXX-NNNN`, `F:<domaine>:<fonction>` (journal),
  versions `vN` des fiches.
- Ce dossier ne contient **pas** : les documents de préparation
  (obsolètes par nature), la documentation externe de référence
  (licences), ni les notes de process de l'assistant — seules les
  fiches publiables sont miroitées (ADR-0028).
- La cohérence entre ce miroir et le dépôt n'est pas vérifiée par
  un test automatisé : en cas de doute sur un point précis, la
  section `Historique` de la fiche indique la date de dernière
  synchronisation.

## Périmètre exclu

`references/` (documentation FreeCAD/planegcs externe), `preparation/`
(designs de travail), fiches de process assistant — cf. ADR-0028 pour
la justification complète.
