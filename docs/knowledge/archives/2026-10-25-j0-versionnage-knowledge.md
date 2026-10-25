# 2026-10-25 — [J0] Versionnage de la Knowledge : fiches versionnées + journal par fonction

**Thème :** plainte de Raphaël sur le stockage de l'information : « je veux qu'il y ait un historique pour chaque fonction, pour chaque décision. Le principe du versionning. »

**Constat :** les décisions étaient déjà versionnées (ADR append-only) et les discussions archivées, mais les fiches étaient mises à jour en place sans trace de l'état antérieur, et aucune fonction n'avait d'historique individuel.

**Décisions (validées par Raphaël) :**
1. **Mécanisme léger** pour les fiches : n° de version en tête + section `## Historique` (une ligne datée par révision, citant la source ADR/discussion). Pas de snapshots complets — l'ancien texte reste retrouvable via ADR, archives et Git.
2. **Historique par fonction sans excès de tokens** : journal append-only `historique/journal-fonctions.md`, une ligne par événement `date | F:<id> | J<jalon> | changement | source`. IDs stables `F:<domaine>:<nom>` ; consultation par **grep sur l'ID**, jamais en lecture entière.
3. Les ADR restent la source de versionnage des décisions ; journal et changelogs citent toujours l'ADR/discussion.

**Fiches touchées :** toutes les fiches passées en v1 avec section Historique ; nouvelle fiche [Historique & versionnage](../historique/KNOWLEDGE.md) qui porte le protocole ; journal amorcé avec les événements J2 reconstitués depuis les ADR/archives.

**Points ouverts :** fonctions antérieures non reconstituées dans le journal (seulement si besoin) ; le protocole devra être suivi par chaque capitalisation de fin de tâche (le skill omnifab reste à jour en la matière via la fiche historique).
