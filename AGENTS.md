# AGENTS.md — guide de travail pour les agents IA sur OmniFab

Ce fichier rassemble les **préférences de Raphaël Olivier** et les règles de
travail qui s'imposent à tout agent IA (assistant conversationnel ou agent de
code) intervenant sur ce dépôt. Le *pourquoi* de chaque règle vit dans la
Knowledge : [docs/knowledge/](docs/knowledge/README.md) — commencer par
[l'état actuel](docs/knowledge/etat-actuel/KNOWLEDGE.md), puis les fiches
conventions et architecture. Les décisions tranchées vivent dans
[docs/adr/](docs/adr/).

## Langue et pédagogie

- La langue de référence du projet est le **français** (doc, commits, issues).
- Raphaël n'est pas développeur confirmé : toute commande à exécuter en local
  est donnée **exacte, une par une, dans l'ordre**, dans un encart `bash`.
  Les standards, eux, ne se compromettent jamais (CI stricte, ADR, verrous).

## Gouvernance (décisions)

- **Pré-approbation des micro-décisions** : bugfixes, convergence rustfmt/CI,
  points de robustesse — le silence de Raphaël vaut acceptation.
- **TOUJOURS à validation explicite** : les six verrous, licences, schémas de
  format, sécurité machine, frontières de crates, toute décision de fond
  (l'ADR la porte).
- Un ADR par décision tranchée, avec alternatives rejetées.
- La validation se fait au fil de l'eau : soumettre dès qu'une décision est
  mûre, poursuivre le travail non dépendant en parallèle.

## Textes destinés à être copiés

- Tout texte à copier par Raphaël (prompt, commande, message) est présenté
  dans un **encart de code adapté à son type** (`bash` pour les commandes,
  texte brut pour les messages, langage concerné pour le code) — jamais en
  citation. **Un seul encart par livrable**, titre inclus en première ligne.

## Nommage

- Discussions et lots : format **[JX] — <sujet précis>** (X = jalon dominant,
  J0 = transversal/gestion de projet).

## Git et CI

- Docs, ADR, fiches Knowledge : **push direct** sur main.
- Code Rust : **branche courte + PR** fusionnée seulement quand `lint-test`
  est vert (lots ≥ ~3 fichiers ou multi-crates ; micro-correctifs tolérés en
  direct). À activer dès que la CI est repassée au vert.
- La protection de branche main (status check `lint-test`) est à
  (re)prendre une fois la CI verte ; en attendant (fenêtre de convergence,
  CI rouge), la règle branches/PR ci-dessus s'applique par discipline.
  Quand la protection est active : ne pas la contourner.
- Après tout push via un connecteur : **vérifier le contenu re-fetché**
  (roundtrip) — des pushes tronqués/altérés ont déjà eu lieu sans trace dans
  la réponse du connecteur.
- Le vert de CI se conclut sur les **conclusions des jobs**, jamais sur la
  seule absence d'issue dans les minutes suivant un push.
- Ne pas modifier le mécanisme d'issues auto-créées de `ci.yml` (rapports
  d'échec). Les workflows dédiés (galerie, etc.) vivent dans leurs fichiers.

## Documentation vivante

- `docs/knowledge/` est la copie de référence de la Knowledge du projet
  (ADR-0028) : **tout bump de version d'une fiche ⇒ la même fiche repoussée
  au dépôt dans la foulée**. La copie de travail vit sur la plateforme de l'assistant ;
  `docs/knowledge/` au dépôt est la copie de référence (ADR-0028), et le
  dépôt reste la seule source de vérité du code.
- Chaque fiche porte `Version : vN (date)` en tête et une section
  `Historique` en pied ; les identifiants stables (`ADR-NNNN`,
  `OF-XXX-NNNN`, `F:<domaine>:<fonction>`) se cherchent par `grep`.
- `docs/knowledge/etat-actuel/KNOWLEDGE.md` est mis à jour à chaque fin de
  lot : ce qui compile, état de la CI, dette ouverte.

## Interdits absolus

- Empiler du code métier non validable sur main quand la CI est rouge
  (préparer en fiches de travail, ne pas pousser).
- `unsafe` hors frontières FFI ; `f64` nu dans les API métier ; GPL pour
  les données ; code de logiciels propriétaires (comportement documenté
  uniquement) ; EULA acceptés.
