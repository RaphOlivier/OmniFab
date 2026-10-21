# ADR-0021 : bascule d'orientation, ancrage non saisissable, liste des contraintes posées

- **Statut :** accepté
- **Date :** 2026-10-21
- **Décideurs :** Raphaël Olivier (re-validation manuelle §7 du jalon 2, choix tranchés en réponse au diagnostic)

## Contexte

La re-validation manuelle §7 (après le lot ADR-0020) a rapporté trois comportements des contraintes de l'atelier 2D :

1. **Horizontale puis Verticale sur la même ligne écrase la ligne** : `AddConstraint` acceptait les deux orientations sans vérification croisée — seules satisfaisables si les extrémités coïncident, DogLeg « résout » en écrasant la ligne.
2. **Distance ouvre le panneau mais semble ne rien appliquer** : le flux est correct de bout en bout (la valeur de départ du dialogue vaut la distance courante des deux points, la pose au clic sur « Poser » passe bien par `AddConstraint` et la résolution), mais aucune trace visible de la pose n'existe dans l'interface — une cote posée sans changement de valeur ne déplace rien et rien ne confirme la contrainte.
3. **Fixer le point ne fonctionne pas** : l'ancrage `PointFixe` retire le point des inconnues du solveur, mais le glisser passe par `MovePoint` qui change sa graine — le point ancré suivait sa graine et se déplaçait quand même, l'ancrage n'avait aucun effet sur l'interaction.

## Décision

1. **Bascule d'orientation (D1)** : une ligne porte au plus une contrainte d'orientation. Poser `Horizontale` sur une ligne déjà `Verticale` (et inversement) **remplace** la contrainte existante à identifiant conservé (`sketch.constraint.updated`, schéma plat v1) ; reposer le même genre est refusé (`OF-SES-0001`, rien au journal). La règle vit dans le plan de la commande `AddConstraint` — jamais dans la validation de rejeu, les journaux existants (y compris contradictoires) restent lisibles.
2. **Liste des contraintes posées (D2)** : le panneau esquisse liste les contraintes du croquis dans l'ordre de pose, chaque libellé traduit (i18n) désigne les entités par leurs identifiants, cotes en millimètres, angle en degrés. La dernière contrainte touchée — pose neuve comme bascule — se surligne un court instant (1,5 s, couleur d'accent) puis s'estompe ; pas de message de succès. Un état dérivé d'affichage, jamais sérialisé.
3. **Ancrage non saisissable (D3)** : un point portant un `PointFixe` ne se saisit plus — `saisir_poignee` refuse le grab (pas de glissement fantôme), `MovePoint` est refusé au plan (`OF-SES-0001`, la graine tient) pour les chemins scriptés. Un liseré orange distinct signale l'ancrage sur la poignée. Là encore la règle vit dans la commande, jamais dans le rejeu : un journal existant qui déplace un point ancré reste lisible (la graine historique s'applique).

## Alternatives rejetées

- **Bascule : refuser la seconde pose** (comme le cycle ADR-0020) — cohérent mais frustrant : l'utilisateur qui veut réorienter une ligne doit annuler puis reposer ; **poser les deux et signaler le conflit** — rétablit l'écrasement condamné par la validation ; **remplacer par suppression + ajout** — deux événements pour un geste, casse l'identifiant stable.
- **Liste : message de succès seul** — éphémère, la contrainte posée redevient invisible dès l'effacement du message ; **liste + message** — redondant, le refus reste seul à parler quand la pose réussit silencieusement. Un panneau d'inspection détaillé (édition/suppression contrainte par contrainte) reste à faire en jalon 3.
- **Ancrage : laisser le point saisissable, l'ancre suit le geste** (statu quo) — contredit le test §7 : l'ancrage doit tenir ; **dessiner seulement un marqueur sans refuser le grab** — le point suivrait le curseur pendant le glissement avant d'être repoussé au relâchement, glissement fantôme inacceptable.

## Conséquences

- Une ligne garde au plus une orientation : l'invariant tient à la pose, les journaux hérités (avant bascule) peuvent encore porter deux orientations — leur rejeu reste accepté, leur résolution peut encore écraser la ligne (à re-poser depuis l'atelier pour l'annuler).
- `saisir_poignee` n'écrase plus la sélection d'un point ancré : le clic seul le sélectionne toujours, seul le grab est refusé.
- Le liseré d'ancrage (turquoise 64, 224, 208) se distingue des entités et de la sélection (orange) : l'une cerne la poignée, l'autre habite le trait — les deux coexistent à l'écran sans confusion.
- La surbrillance momentanée demande un rafraîchissement egui tant qu'elle vit : `request_repaint_after(100 ms)` borné à la fenêtre de surbrillance.
- Les demandes du jalon 3 restent ouvertes : arithmétique d'unités longueur↔angle, indicateur d'unité par alias, inspection/édition des contraintes de la liste.
