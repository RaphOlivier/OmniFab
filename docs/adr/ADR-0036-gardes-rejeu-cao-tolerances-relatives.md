# ADR-0036 — Gardes au rejeu CAO et tolérances relatives du contour (audit n°5 : A5-04 + A4-16)

- **Statut : ACCEPTÉ** — validé par Raphaël Olivier le 2026-11-03.
- **Jalon : 3** — phase B, passe « gardes avant OCCT » (backlog audit n°5, ordre 1), **avant** le branchement du noyau OCCT (lot 3).
- **Source :** audit externe n°5 (Claude, 2026-11-03) — A5-04 et A4-16 (majeurs, verdicts validés en bloc) ; consigne archivée (recentrage centroïde, prédicats robustes, test « carré de 1 µm ») ; leçon n°5 du lot 2.

**Objet : la garde vit à la frontière, pas dans OCCT.** Aujourd'hui la profondeur/couleur d'une feature n'est validée qu'à l'écriture (command.rs::feature_valide — sans borne supérieure : un `Fixe{mm: 1e300}` passe au plan), le rejeu du journal passe les valeurs au moteur sans contrôle, et `valider_contour` juge alignement et intersections avec des epsilons absolus (`f64::EPSILON`, `EPS_AIRE` 1e-12). Le lot 3 branche OCCT : ces deux brèches deviennent exactement le scénario d'abort du spike.

**Décisions** :
- **D1 — Garde commune `profondeur_valide()`, sur la valeur résolue** : finie, > 0 strict, ≤ `MAGNITUDE_MAX_MM` (1e9 mm). Appelée aux trois frontières : (a) `reconstruire()` avant chaque appel moteur — invalide → feature `EnEchec` (OF-CAD-0005, le corps survit, patron D7 ADR-0018) ; (b) `apply()` — l'événement est appliqué (le journal fait foi, l'id séquentiel reste continu) mais la feature est marquée `EnEchec` dans la projection ; (c) `command.rs::feature_valide` à l'écriture, qui y gagne la borne supérieure manquante.
- **D2 — Bornes couleur et « à travers »** : couleur rejouée/écrite (`couleur_de_journal`) : chaque composant fini et dans 0..=1 ; « à travers » : la valeur résolue (aujourd'hui 1,1 × diagonale de la boîte) suit D1 — boîte dégénérée (diagonale 0) → `EnEchec`, jamais transmise au moteur.
- **D3 — Tolérances relatives dans `valider_contour`** : (a) **recentrage sur le centroïde** avant aire de lacet et orientations (tue la cancellation aux grandes coordonnées) ; (b) alignement jugé en hauteur moyenne : `aire/périmètre < TOLERANCE_MM` → refus « points alignés » (remplace `aire.abs() < f64::EPSILON`) ; (c) **prédicats exacts** : crate `robust` (georust, portage Rust des prédicats adaptatifs de Shewchuk, no_std, MIT/Apache) remplace `EPS_AIRE = 1e-12` absolu pour les orientations des tests d'intersection.
- **D4 — Portée bornée, pas de big-bang** : seuls `valider_contour` + la garde rejeu changent ; les autres epsilons dispersés (dxf.rs, segments.rs, esquisse.rs, renderer.rs) attendent `forge-geom::tolerance` (A5-07, ordre 4, type `Tolerances` dérivé de l'étendue — conseil 1 de la consigne). `robust` = seule nouvelle dépendance (édition du lock = checkpoint PC, même mécanique que le mini-lot zip).

**Alternatives rejetées** : valider seulement à l'écriture (le journal rejouable est la vérité, pas la commande — un journal ancien/édité y échappe) ; refuser l'événement au rejeu (casse l'id séquentiel des événements suivants ; la granularité est la feature, pas le journal) ; revalider dans le noyau OCCT au lot 3 (récidive de la leçon n°5 du lot 2) ; abaisser les epsilons absolus à 1e-15 (le contre-exemple de l'audit 5 est relatif par nature) ; arithmétique adaptative maison (coût/risque contre une crate éprouvée) ; écart minimal sommet-droite par sommet (l'autre option de la consigne : plus coûteux, même verdict sur les cas visés — en réserve) ; tout centraliser maintenant (branche trop large, l'ordre 4 le fera à froid).

**Critères de sortie** (test d'abord, rouge avant / vert après, un commit par point) :
- rejeu : `Fixe{mm}` = 0, −5, 1e300 → feature `EnEchec` OF-CAD-0005, corps survit ; `rgba = [2,0,0,1]` → idem ; « à travers » sur boîte de diagonale 0 → idem ;
- écriture : `Fixe{mm: 1e300}` refusé au plan (OF-SES-0001, rien au journal) ;
- `valider_contour` : triangle (0,0),(1000,0),(500,5e-7) **refusé** (contre-exemple audit 5) ; contour de 1e6 mm à 3 points alignés à 1e-7 mm **refusé** ; carré de 1 µm (côté 1e-3 mm) **accepté** ; l'exemple faux « lame 1 mm × 1e-9 mm » n'est pas re-chassé ;
- rejeu ×2 → mêmes volumes (déterminisme préservé) ; CI au vert, cargo-deny incluant `robust`.
