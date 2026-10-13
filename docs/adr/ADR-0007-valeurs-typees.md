# ADR-0007 : valeurs typées dans le moteur de liaisons (point 11 de la revue)

## Statut

Proposé le 2026-10-13 — validé par Raphaël Olivier le 2026-10-13.
Phase 1 au jalon 2, phase 2 au jalon 3 (critère de bascule explicite).

## Contexte

La revue (point 11) constate que `ValueGraph` stocke des `f64` nus — en
tension avec la convention « jamais de f64 nu (newtypes longueur/angle) »
(fiche conventions). Mais le graphe doit aussi porter des grandeurs sans
dimension (facteurs, quantités, résultat d'une comparaison ADR-0006).
La question : typer les nœuds d'emblée, ou déclarer l'unité à côté ?

## Décisions

**Phase 1 (jalon 2) — unité déclarée par valeur :**

- chaque nœud porte `unit: Option<Unit>` (forge-units expose `Unit` :
  Length, Angle, Speed, Number — dimension pleine, pas newtype) ;
- cohérence vérifiée **à la définition** de l'expression :
  `longueur_mm + angle_deg` → erreur typée OF-VAL-0006 (nouveau code) ;
- les comparaisons et `si` retournent `Number` ; les fonctions typées :
  `sqrt` d'un Length² est un Length, `min`/`max` exigent des unités
  identiques ;
- forge-units continue de fournir les newtypes `Length`, `Angle`, `Speed`
  pour les couches au-dessus (CAO/CAM) : le graphe reste la frontière
  « déclarative » (nombres + unité), les newtypes sont la frontière
  « programmatique ».

**Phase 2 (jalon 3, éditeur nodal) — nœuds typés :**

- `Value::{Length, Angle, Speed, Number}` dans le graphe ;
- conversions implicites refusées ; explicites via fonctions
  (`mm(pouces)`, `deg(rad)`) ;
- critère de bascule : l'éditeur nodal du jalon 3 a besoin de couleurs/
  connecteurs par type — c'est le déclencheur, pas la date.

## Alternatives rejetées

- **Nœuds typés d'emblée** : obligerait à fermer la liste des grandeurs
  maintenant, sans le retour du jalon CAO/CAM ; coût immédiat, bénéfice
  différé, migration du journal si on se trompe.
- **f64 nu permanent** : viole la convention newtypes et laisse passer
  `longueur + angle` silencieusement.
- **Système d'unités complet (uom, dimension analysis)** : dépendance
  externe lourde ; forge-units garde la main sur la liste fermée.

## Conséquences

- Nouveau code d'erreur **OF-VAL-0006** (incohérence d'unités) à réserver
  dans docs/error-codes.md dès l'implémentation de la phase 1.
- Le champ `unit` des valeurs sera sérialisé dans les événements du
  journal (type d'événement `value.set` v2, cf. ADR-0006 pour le passage
  de version).
- Les deux phases gardent ADR-0002 intact : déterminisme et rejeu
  identiques (l'unité est une métadonnée de validation, pas un état
  d'exécution).
