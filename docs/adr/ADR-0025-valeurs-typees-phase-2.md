# ADR-0025 — Valeurs typées phase 2 : nœuds typés, arithmétique d'unités, trigonométrie (phase D)

- **Statut : ACCEPTÉ** — lot ADR-0022..0027 validé par Raphaël Olivier le 2026-10-25.
- **Jalon : 3** — CAO paramétrique + éditeur nodal de valeurs.
- **Source :** préparation du 2026-10-22, digest de session du 2026-10-25 (4 axes tranchés, validation du lot).

**Objet : appliquer ADR-0007 phase 2 (acté : « nœuds typés `Value::{Length, Angle, Speed, Number}`, conversions implicites refusées, déclencheur = connecteurs typés de l'éditeur nodal ») et satisfaire la demande du 2026-10-21 (arithmétique d'unités).**

**Décisions** :
- **D1 — Types de nœuds phase 2 : `Number`, `Length`, `Angle`, `Area`, `Volume`** (Speed attend le jalon CAM). Le type est DÉRIVÉ de l'unité déclarée (mm→Length, deg→Angle, mm²→Area) : aucune migration de schéma — les événements v2 actuels portent déjà l'unité ; les nouvelles unités `mm²`, `mm³`, `rad` rejoignent `Unite` (codes from_code/Display, réserve documentée).
- **D2 — Arithmétique dimensionnelle à l'évaluation** (le graphe ne fait pas de conversions implicites — ADR-0007 intact) : `L±L→L`, `A±A→A`, `L×L→Area`, `Area×L→Volume`, `L/L→Number`, `Area/L→L`, `L×Number→L` (idempotent) ; toute combinaison hors table → **OF-VAL-0010 « opération dimensionnellement invalide »** (registre + tests bidirectionnels), message i18n avec les deux types attendus/reçus. Les incohérences à la DÉFINITION restent OF-VAL-0007 (une expression déclarée `deg` lisant du `mm` est refusée dès la pose, comportement inchangé).
- **D3 — Trigonométrie et conversions explicites** : `sin/cos/tan` exigent un `Angle` et rendent un `Number` ; `atan2(Number, Number)→Angle`, `sqrt(Number)→Number`, `sqrt(Area)→Length` ; fonctions de conversion explicites non cassantes : `mm(x)/deg(x)/rad(x)/carre(x)/cube(x)` convertissent depuis `Number` (et entre eux via Number, jamais implicitement) — c'est la réponse à « arithmétique d'unités longueur↔angle » : le trajet existe, il est explicite et typé.
- **D4 — Indicateur d'unité partout** (demande du 2026-10-21) : le panneau de liaisons affiche le symbole d'unité sur chaque ligne (y compris les cotes `esquisse.*` et `cad.*`), les badges d'erreur portent les types en clair ; les DragValue d'entrée gardent leur unité déclarée à l'affichage.
- **D5 — Connecteurs typés de l'éditeur nodal** = déclencheur acté ADR-0007 : les ports de l'éditeur (ADR-0026) refusent au dessin une arête entre types incompatibles (pré-validée `prepare_expression`, même garde que D2), une arête valide propose la conversion explicite duale.

**Alternatives rejetées** : conversions implicites (ADR-0007 : refusées, récidive) ; `Speed` au jalon 3 (pas de consommateur avant le CAM) ; unités composées générales (kg·m/s²…) = moteur d'unités complet hors périmètre (une table dimensionnelle fermée suffit et se teste) ; stocker le type dans l'événement (le type dérive de l'unité, jamais de double vérité).

**Critères de sortie** : registre OF-VAL-0010 testé bidirectionnel ; expressions `sin(angle_deg)` et `largeur * hauteur` (→ Area) validées en tests ; `deg` lisant `mm` toujours refusé à la pose ; panneau de liaisons avec unités affichées ; tests desktop des badges.
