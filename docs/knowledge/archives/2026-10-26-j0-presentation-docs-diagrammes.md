# [J0] — Présentation du projet et schémas d'interaction dans la documentation

**Date :** 2026-10-26 · **Jalon dominant :** J0 (transversal) · **Discussion :** présentation OmniFab + intégration des diagrammes dans la doc

## Contexte

Demande de Raphaël : une présentation du projet compréhensible pour un développeur, avec points forts/faibles (structure ET usage de l'IA dans le développement), illustrée d'images générées, puis de diagrammes et schémas d'interaction.

## Ce qui a été produit (hors dépôt)

- Canvas « présentation développeur » (deck paginé : stack, structure, IA — bilan honnête pour/contre), 3 images générées (concept, architecture en couches, duo humain+IA).
- Canvas « diagrammes d'interaction » : 6 schémas Mermaid (architecture, geste utilisateur, event sourcing/undo, chaîne machine, flux A→Z, cycle humain+IA).
- Les deux canvases ont été fusionnés en un seul document Markdown (le rendu « slides » ne dessine ni images ni Mermaid).

## Débat : où ranger ce type de présentation ?

Argumentaire de Vibe, accepté par Raphaël (« Ok ») :

- **Diagrammes d'interaction → doc du dépôt** : ils décrivent des *invariants* (ADR-0008/0016/0018/0021, verrou n°4), vieillissent bien, sont du code versionné diffable en PR, et GitHub rend Mermaid nativement. Le schéma « du clic à l'écran » est le complément manquant des ADR (ils disent *pourquoi*, pas *comment ça circule*).
- **Récit/bilan (points forts/faibles, limites de l'IA, chiffres de vitesse) → pas dans la doc de référence** : subjectif et daté, il deviendrait un exemplaire de plus à resynchroniser (le projet en a déjà trois pour le prompt fondateur) ; sa place est une page de présentation/billet de talk, daté explicitement, renvoyant aux ADR pour la vérité.

## Décisions tranchées

1. Trois schémas Mermaid ajoutés à `docs/architecture.md` (commit `d08e6d7` sur main) : séquence d'un geste utilisateur, états undo/redo + sauvegarde atomique, chaîne machine cible (verrou n°4, jalons 4+). En-tête « Schémas d'interaction » posant la règle : **tout schéma décrit un invariant ; une évolution qui le casse exige un ADR ou met le schéma à jour dans la même PR**.
2. Le schéma de dépendances des crates existant est resté inchangé (aucune redondance créée).
3. Pas d'ADR : documentation d'invariants existants, aucune nouvelle décision technique.
4. La présentation narrative (images + bilan) reste hors dépôt, artefact de communication non normatif.

## Risques / dette

- Dérive possible entre schémas et code si une PR change un invariant sans mettre à jour le diagramme — atténué par la règle posée dans l'en-tête ; un test ou une revue CI dédiée restent possibles si besoin.
- La chaîne machine est un schéma **cible** (forge-link n'existe pas encore) : explicitement étiqueté « jalons 4+ » dans le texte pour éviter de le lire comme de l'existant.
- CI du commit `d08e6d7` à confirmer (docs-only, aucun risque attendu ; règle : OPEN=[] au push n'est pas un verdict).
