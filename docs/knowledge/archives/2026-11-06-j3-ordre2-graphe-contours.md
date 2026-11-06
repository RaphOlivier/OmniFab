# 2026-11-06 — [J3] Ordre 2 ADR-0037 : chaînage en graphe d'extrémités (A5-01 + A4-17) + classeur de régions par parité

**Version de la fiche :** v1 (2026-11-06 — capitalisation de fin d'ordre, discussion Ordre 2 : fusion + arbitre + capitalisation. La discussion précédente avait soldé D2 sur la branche jusqu'à `88a1acb` ; la présente a ajouté D3 et les tests restants, prouvé le vert de branche, fusionné et arbitré.) 

## Pourquoi cet ordre

Le chaînage de contours hérité du lot 1 phase B (`Contour2`/`contours()`, OF-GEO-0006) recollait les segments par un glouton : écart réel accepté silencieusement, pas de refus typé sur les embranchements, pas de notion de trous ni de régions — une esquisse multi-régions ou un croquis avec trous produisait des résultats indéfinissables avant le branchement OCCT du lot 3. L'audit n°5 (A5-01 + A4-17) a exigé un graphe d'extrémités à tolérance : soudure par index spatial, élagage des queues, degré > 2 = refus typé, plafond d'entités ; ADR-0037 « contours réels / graphe-parité » accepté le 2026-11-03 (`docs/adr/ADR-0037-contours-reels-graphe-parite.md`, commit `d43be2b` — déjà poussé).

## Contenu fusionné (PR #630, squash → main `e0eeba8`, mention IA dans le message)

- **D2 chaînage en graphe d'extrémités** : soudure par index spatial en grille au pas TOLERANCE_MM ; élagage itératif des queues par pile des sommets de degré 1 ; degré > 2 → `EmbranchementContour` **OF-GEO-0008** ; `MAX_ENTITES_CHAINAGE` = 10 000 → `PlafondChainageDepasse` **OF-GEO-0009** ; extraction de cycles orientés ; **retrait du glouton et d'OF-GEO-0006**/`ChainageRompu` du code, du match `code()`, des tests error_codes et du registre (note « code retiré, réservé ») ; `lib.rs` exporte `MAX_ENTITES_CHAINAGE`.
- **D3 classeur de régions par parité** : appartenance d'un contour à une région décidée par ray-cast du point témoin, conteneur direct, orientation normalisée par re-soudage du cycle, ordre déterministe des régions.
- **14 tests test-first** : la discussion précédente avait posé les 6 premiers (rouge `e47ab81` prouvé par CI, issue auto #631) ; la présente a porté le lot à 14 (carrés secs en un sommet, carré avec queue, plafond d'entités, entités désordonnées/inversées, trous/parité D3).

## Déroulement CI (protocole : rouge prouvé avant / vert après)

- Rouge test-first prouvé par CI, **issue #632** documente le constat — fermée completed avec motif + mention IA.
- 7 commits sur la branche, du rouge C1 au **fix-forward clippy `is_multiple_of`** ; vert de branche intégral sur la tête `f8f2626`.
- **Fusion squash → main `e0eeba8`** ; arbitre main ci.yml **VERT INTÉGRAL (CI #647, 24 m 50 s)**.
- Reste au bouton GitHub : suppression de la branche distante `fix/a5-01-a4-17-graphe-contours` (Raphaël, pas d'outil du connecteur).

## Débats et décisions

- **Ré-élagage vs orientation au moment du cycle** : l'orientation des contours est normalisée par re-soudage (D3) plutôt que par une heuristique de tri — le ray-cast du point témoin décide de l'appartenance, le conteneur est direct, l'ordre des régions déterministe.
- **OF-GEO-0006 retiré, pas réutilisé** : le code d'écart réel disparaît du registre avec une note « retiré, réservé » — le graphe refuse structurellement ce que l'ancien code tolérait.

## Leçons outillage (reportées en fiche état-actuel v78, suite des leçons v77 (4)-(6))

1. clippy 1.99 exige `is_multiple_of(2)` (manual_is_multiple_of).
2. La protection de branche exige AUSSI les instances (push) des checks requis : les runs push annulés par concurrency bloquent la fusion — il faut re-runner les runs annulés.
3. cargo-deny filtre par chemins : il ne tourne pas sur une PR sans Cargo.lock (workflow_dispatch manuel).
4. Une PR draft est toujours blocked (l'ouverture ready est nécessaire, les checks requis ne suffisent pas sur une draft).
5. Le bac à sable de l'assistant ne permet aucune attente temps réel : l'arbitrage CI se fait par relève après re-ping.

## État en fin de discussion

Ordre 2 A5-01+A4-17 soldé et fusionné (`e0eeba8`), CI main verte intégrale, 0 issue « CI : » ouverte. Capitalisation : fiche état-actuel v78 (ligne Version, section CI, dette n°1 — A5-01+A4-17 barré-soldé, A5-02+A4-18 marqué prochain ordre), journal des fonctions +2 lignes J3 (`F:geom:chainage-graphe`, `F:geom:regions-parite`), miroir dépôt repoussé (etat-actuel v76 → v78 + présent fichier + index). ADR-0037 déjà poussé, rien à faire.

## Suite

**Ordre 3 = A5-02 + A4-18 (`Segment2::Cercle` explicite + refus du balayage nul côté esquisse), puis A5-03 (commentaire du sens d'arc)** — en le fusionnant, la phase déclencheur de l'ouverture multi-contributeurs est atteinte (rappel v76.1 : relire `preparation/ouverture-multi-contributeurs.md`, annoncer le moment opportun à Raphaël, rédiger l'ADR-0040 de gouvernance avant d'ouvrir le niveau 2).

## Liens

ADR-0037 (`docs/adr/ADR-0037-contours-reels-graphe-parite.md`) ; PR #630 ; issue #632 (fermée completed, motif + mention IA) ; fiche état-actuel v78 ; journal des fonctions (2 lignes J3) ; backlog `preparation/backlog-audit5.md` (miroir privé) ; précédente discussion : entrée d'archives ordre 1 (2026-11-05) pour le contexte D2 partiel (`88a1acb`).
