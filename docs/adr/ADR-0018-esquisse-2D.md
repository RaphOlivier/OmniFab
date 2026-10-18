# ADR-0018 — Esquisse 2D contrainte (jalon 2, phase D)

- **Statut : ACCEPTÉ — validé par Raphaël Olivier le 2026-10-18.**
- **Date : 2026-10-18.**
- **Objet : ouvrir la phase D du jalon 2 (ADR-0012) — le croquis contraint du démonstrateur, de bout en bout : commandes → événements → solveur → undo/redo.**

## Contexte

La phase A (ADR-0013) a livré le solveur : crate forge-gcs (trait « Solveur2D », implémentation « Systeme2D » native au-dessus du shim ABI C de planegcs), arène de paramètres à adresses croissantes (reproductibilité FreeCAD #33399, test de reproductibilité ×2 vert), unités strictes à la frontière (Length/Angle, jamais de f64 nu), codes OF-GCS-0001..0004. Le trait expose déjà tout ce que la phase D consomme :

- entités : points, lignes (extrémités partagées), cercles, arcs ;
- contraintes : coïncidence, horizontale, verticale, distance, angle, même longueur, point sur ligne, parallèles, perpendiculaires, rayon de cercle, point sur cercle ;
- fixation de point (ancrage), résolution (BFGS / Levenberg-Marquardt / DogLeg), diagnostic (degrés de liberté, conflits, redondances, reproductibilité).

Les cotes (distance, angle, rayon) sont des **paramètres fixes** du solveur : c'est le support déjà prévu du pilotage par expressions et alias (phase E, ADR-0012).

forge-session (ADR-0008/0011) porte commandes typées, projections par rejeu et undo/redo par curseur ; forge-render (ADR-0015) rend la vue 2D en wgpu ; le desktop a un fil de travail std::thread+mpsc (ADR-0016 C1), l'i18n Fluent et l'aide help_id (ADR-0010).

Critère de sortie de la phase D (ADR-0012) : « croquis contraint modifiable et défaisable de bout en bout (commandes → événements → solveur → undo/redo), tests de non-régression du solveur. »

## Décision

**D1 — Le croquis vit dans forge-session comme citoyen event-sourcing de première classe.** Trois types d'événements, schémas v1 :

- « sketch.entity.added » : une entité (Point{x, y}, Ligne{p1, p2}, Cercle{centre, rayon}, Arc{centre, debut, fin, rayon, angle_debut, angle_fin}) — identifiants séquentiels (u32) émis par la commande, ordre d'ajout = ordre du journal ;
- « sketch.constraint.added » : une contrainte typée (les 11 du trait) référençant des identifiants d'entités, les cotes en valeurs (mm, radians) ;
- « sketch.point.moved » : nouvelle position de départ d'un point (le « germe » posé par un glissement).

Un seul croquis par session (comme l'import courant de tracés) ; le multi-croquis attend le jalon 3 (feature tree). Les positions de départ sont des graines d'initialisation du solveur, jamais la vérité : la vérité = entités + contraintes + dernières graines, toutes au journal.

**D2 — Commandes : AddEntity, AddConstraint, MovePoint.** Les plans sont validés contre l'état courant (identifiants référencés existants, valeurs finies et positives pour les cotes, rayon > 0) ; tout refus = « CommandRejected » OF-SES-0001, rien au journal, aucun message de succès (ADR-0016 C4). **Amendement à l'ADR-0012 (à valider explicitement)** : pas de commande « Solve » — résoudre n'est pas une intention de l'utilisateur, c'est la conséquence déterministe de ses intentions ; une commande qui n'émettrait que l'appel du solveur ne laisserait aucune trace rejouable et une événement portant les positions résolues mettrait de la géométrie au journal (voir alternatives rejetées).

**D3 — La résolution est un état dérivé, calculé par la projection.** « SketchProjection » reconstruit le système planegcs depuis le journal (ordre d'ajout figé = reproductibilité), résout via le trait « Solveur2D » **injecté** par l'application (forge-session ne construit jamais « Systeme2D » elle-même), met les positions en cache (invalidé par toute écriture au journal) et expose : positions des points, rayons/angles des entités, degrés de liberté, diagnostic. Algorithme par défaut : DogLeg (le plus robuste). Aucune position résolue n'est jamais sérialisée.

**D4 — La résolution tourne sur le fil de travail.** Pattern ADR-0016 C1 : le travail « ResoudreEsquisse » part sur le std::thread existant avec la version du journal ; l'UI affiche les dernières positions connues pendant la résolution. Pendant le glissement d'une poignée, l'UI montre le point sous le curseur (aperçu local, sans re-résolution) ; au relâchement, la commande « MovePoint » pose la graine et la résolution repart. (Le re-solve en continu pendant le drag — comportement FreeCAD — reste une évolution d'UI possible sans ADR si la latence le permet.)

**D5 — UI d'esquisse dans le viewport 2D existant.** Mode esquisse à côté des imports : outils point, ligne, cercle, arc (segments wgpu pour la géométrie, painter pour les points/poignées et les symboles de contrainte) ; sélection au clic, poignées aux points ; snapping sur la grille ET sur les points existants (un clic sur un point existant réutilise son identifiant au lieu d'en créer un — liaison naturelle des extrémités) ; suppression d'entités/contraintes HORS périmètre v1 (évolution : commandes + événements « *.removed », ajout non cassant). Affichage des degrés de liberté et des contraintes ; le diagnostic de redondance n'est affiché qu'avec **hystérésis** (stable sur deux résolutions consécutives — instabilité documentée, proposition FPA #106, ADR-0013). i18n : clés FTL + pages d'aide complètes pour chaque outil et bouton (ADR-0010).

**D6 — Frontières et licences.** forge-session (MPL-2.0) dépend de forge-gcs (LGPL-2.1+) **pour le trait et les types d'identifiants seulement** ; l'application instancie le solveur natif et l'injecte — c'était le plan de l'ADR-0013 (« forge-session consommera ce trait »), la LGPL reste confinée derrière la frontière déjà actée. forge-geom inchangée : la projection expose des positions typées (Length), le desktop les convertit en segments forge-render ; aucun export de croquis en v1 (les exports d'importés existants restent inchangés).

**D7 — Aucun nouveau code d'erreur.** Refus de plan : OF-SES-0001 (existant). Échec de résolution (croquis conflictuel, non reproductible) : les codes OF-GCS-* existants remontent en message d'état non destructif — les positions affichées restent les dernières valides, rien n'est écrit, l'undo reste là.

**D8 — Déterminisme du rejeu, garanti par construction.** Le journal fait foi pour l'ordre d'ajout ; l'arène à adresses croissantes (phase A) rend la séquence d'appels au solveur identique au premier jet et au rejeu. Test obligatoire : rejeu complet d'un croquis → positions identiques au premier passage (écart ≤ 1e-9 mm) et reproductibilité signalée ; tolérance géométrique du projet 1e-6 mm pour les assertions métier.

## Alternatives rejetées

1. **Commande « Solve » + événement portant les positions résolues ( esquissé dans ADR-0012)** : met de la géométrie au journal (contre ADR-0001/0011), crée une double vérité (positions vs graines), rend le journal non rejouable si le solveur ou la plateforme divergent, et alourdit l'undo d'un événement sans intention.
2. **Croquis stocké comme blob d'octets (à la manière des imports)** : un croquis n'est pas un fichier d'origine, c'est de la géométrie native paramétrique — le blob interdirait l'undo fin par intention et le feature tree du jalon 3.
3. **Résolution dans l'application (forge-session sans forge-gcs)** : deux frontends = deux logiques de croquis, le noyau n'est plus complet, l'Android devrait dupliquer ; contredit la frontière « forge-session consomme le trait » (ADR-0013).
4. **Solveur Rust provisoire / maison** : rejeté dès l'ADR-0012 (deux solveurs à maintenir, verrou n°3 : planegcs, pas de solveur maison).
5. **Résolution à chaque frame sans cache** : régression du pattern C1 (re-parse par frame) — un croquis moyen résolu par frame gèlerait l'UI pour rien.
6. **Re-solve en continu pendant le glissement en v1** : latence imprévisible à travers la FFI, complexité d'annulation du geste ; l'aperçu local + résolution au relâchement livre le critère sans le risque (évolution possible sans ADR).
7. **Sérialiser les graines de points comme état interne de la projection (hors journal)** : l'undo/redo et la reprise multi-appareils casseraient — tout ce qui influence le résultat doit être au journal (verrou n°2).

## Conséquences

- **Positives** : le démonstrateur gagne la première fonctionnalité CAO réelle, entièrement event-sourcée et défaisable ; la phase E (pilotage des cotes par expressions) branche directement sur les cotes en paramètres fixes ; l'Android héritera du noyau sans réécriture ; aucun schéma existant n'est modifié (trois nouveaux types d'événements).
- **Coûts acceptés** : forge-session dépend désormais de forge-gcs (LGPL derrière le trait — already acté ADR-0013 pour la forme, documenté ici pour la trace) ; la vue peut s'afficher « en retard » d'une résolution pendant un glissement ; les positions dérivées peuvent diverger marginalement entre plateformes (Eigen/FP) — sans impact, elles ne sont jamais persistées.
- **Risques** : non-déterminisme du solveur (FreeCAD #33399) — mitigré par l'arène ordonnée + test de rejeu obligatoire ; diagnostic de redondance instable (FPA #106) — mitigré par l'hystérésis UI ; latence FFI sur grands croquis — bornée par le fil de travail et le cache.
- **Suspens (évolutions sans ADR sauf mention)** : suppression d'entités/contraintes (événements « *.removed », non cassants) ; contraintes supplémentaires (tangentes, symétrie — extension du shim + variantes, non cassante) ; multi-croquis (jalon 3, ADR requis) ; pilotage des cotes par expressions/alias (phase E, ADR à rédiger) ; export du croquis (SVG/DXF) ; solveur wasm (jalon 9).

## Critères de sortie (mesurables)

1. **forge-session** : croquis rectangle (4 lignes, 4 contraintes H/V, 2 distances, 1 coin fixé) résolu exact à 1e-6 mm, DoF = 0 ; rejeu ×2 → positions identiques (≤ 1e-9) et reproductibilité vraie ; contrainte référençant une entité absente → refus OF-SES-0001, journal inchangé ; undo/redo du croquis (y compris après MovePoint) ; aucune position résolue dans le conteneur sauvegardé.
2. **forge-gcs non-régression** : triangle exact 1e-6 (existant) + 2 croquis dorés (rectangle contraint ; cercle + point sur cercle + rayon).
3. **Desktop** : croquis de bout en bout — outils, contraintes, glissement respectant les contraintes, undo/redo, DoF affiché, aide au clic droit — CI verte multi-OS (15 min de silence, aucune issue ouverte).
4. **Validation manuelle de Raphaël** : checklist fournie (voir ci-dessous), bugs = rounds correctifs un commit / relecture / CI 15 min.

## Découpage d'implémentation proposé

- **Lot 1 — noyau forge-session** : modèle EntiteEsquisse/ContrainteEsquisse sérialisable, commandes AddEntity/AddConstraint/MovePoint, SketchProjection (reconstruction + résolution injectée + cache), événements schémas v1, tests (rejeu, refus, undo/redo, déterminisme). Un commit par point, test d'échec d'abord.
- **Lot 2 — forge-gcs non-régression** : croquis dorés rectangle et cercle (aucun changement de frontière).
- **Lot 3 — desktop** : mode esquisse (outils, sélection, poignées, snapping, affichage contraintes + DoF), i18n + aide complètes, fil de travail ResoudreEsquisse.
- Puis : ADR au dépôt (docs/adr/ADR-0018-esquisse-2D.md) poussé avec le lot 1, comme pour les ADR précédents ; capitalisation Knowledge en fin de phase.

## Checklist de validation manuelle (à fournir à Raphaël à la livraison)

1. Tracer un rectangle : 4 clics → 4 lignes, snap des extrémités, snapping grille.
2. Contraintres H/V + 2 distances + coin fixé → DoF tombe à 0, géométrie s'ajuste en live au relâchement.
3. Glisser un sommet → les contraintes tiennent (le rectangle reste rectangle).
4. Undo complet → croquis vide ; redo complet → croquis restauré.
5. Contrainte redondante → indication stable (hystérésis), pas de faux conflit clignotant.
6. Clic droit sur chaque outil/bouton → page d'aide complète.
