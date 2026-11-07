# ADR-0037 — Contours réels : graphe d'extrémités, trous par parité, `Cercle` explicite (audit n°5 : A5-01 + A4-17 + A5-02 + A4-18 + A5-03)

- **Statut : ACCEPTÉ** — validé par Raphaël Olivier le 2026-11-03.
- **Jalon : 3** — phase B, ordre 2 du backlog audit n°5, **avant** le branchement OCCT (lot 3) ; suit la passe ADR-0036 (ordre 1).
- **Source :** audit externe n°5 — A5-01 (majeur), A5-02 (majeur), A5-03 (mineur/doc) ; compléments A4-17/A4-18 ; backlog ordre 2.

**Objet : un croquis réel contient plusieurs boucles, des trous, des segments en désordre et des queues.** Le chaînage actuel est glouton et ordonné : deux boucles partageant un sommet sont fusionnées en « huit », un embranchement fait perdre la boucle, un seul contour fermé est retenu (trous/îlots ignorés en silence), O(n²). Et un arc d'esquisse effondré (fin == début) devient un cercle complet via la convention « balayage nul = cercle » — le choix lot 1 devient atteignable depuis l'esquisse (A5-02).

**Décisions** :
- **D1 — `Segment2::Cercle` explicite, `Arc` de balayage nul refusé (OF-GEO-0007)** : nouveau variant `Cercle { centre, rayon }` (boucle fermée à lui seul) ; un `Arc` avec `|balayage| ≤ 1e-12` est refusé au chaînage et à la discrétisation (OF-GEO-0007, défense en profondeur). Côté esquisse, un arc effondré (angle_fin == angle_debut) est **omis** par `segments_du_croquis` (patron « position absente saute l'entité ») → le croquis n'a pas de contour fermé → refus au plan OF-SES-0001, jamais un cercle. Les importeurs SVG/DXF produisent `Cercle` directement ; **rejeu tolérant** : à la désérialisation, un `Arc` à balayage nul des anciens journaux est normalisé en `Cercle` (patron migrations lot 2, aucun journal cassé).
- **D2 — `forge-geom::contours` devient un graphe d'extrémités** : soudure des extrémités à `TOLERANCE_MM` (index spatial en grille, O(n)) ; **élagage itératif des queues** (arêtes à extrémité libre supprimées jusqu'à stabilisation — le carré à queue rend son carré) ; ensuite **tout sommet de degré > 2 = refus typé OF-GEO-0008** (embranchement réel, jamais de choix arbitraire) ; les composantes restantes sont des cycles → `Contour2`, inversion automatique des segments selon le sens de parcours (test « inversions »). Plafond d'entités `MAX_ENTITES_CHAINAGE = 10_000` (const publique), dépassement refusé OF-GEO-0009. **OF-GEO-0006 (`ChainageRompu`) devient obsolète** : l'entrée est désormais non ordonnée, un écart n'est plus une erreur mais des composantes distinctes — entrée du registre marquée retirée.
- **D3 — Régions par orientation + parité ; une feature = une région** : les boucles extraites sont orientées (extérieur trigonométrique, trous horaires ; aire signée recentrée, ADR-0036 D3) et regroupées par parité (point-dans-polygone) en `Region { exterieur: Contour2, trous: Vec<Contour2> }`. L'extrusion consomme une région : le moteur analytique du lot 2 calcule l'aire nette (extérieur − trous), le noyau OCCT du lot 3 recevra la face à trous. **Un croquis à plusieurs régions disjoints est refusé au plan** (OF-SES-0001, message d'état non destructif) — le multi-corps reste interdit au jalon 3 (ADR-0024). `forge-session::cad::contours_des_segments` (glouton) est **supprimé** ; `premier_contour_ferme` devient « région du croquis ».
- **D4 — A5-03 : commentaire rectifié** : la doc de `segments_du_croquis` dit « trigonométrique si fin précède ou égale début » alors que le code fait trigo si `fin >= début` (conforme à ADR-0020, le sens suit le geste) — commentaire corrigé + test de non-régression sur les deux sens.

**Alternatives rejetées** : glouton corrigé à la marge (fusion/embranchements au cœur du signalement) ; degré > 2 tranché arbitrairement (glouton déguisé, récidive) ; tolérer tous les embranchements et extraire « au mieux » (contours faux silencieux — le pire cas de l'audit) ; extraire toutes les régions en une feature (multi-corps refusé, ADR-0024) ; garder l'encodage « arc à balayage nul = cercle » (ambigu et atteignable depuis l'esquisse — le bug A5-02) ; refuser tout croquis à queue (l'élagage donne le contour attendu, une queue est un usage d'esquisse légitime) ; tri au lieu d'index spatial (O(n²) de retour).

**Critères de sortie** (test d'abord, rouge avant / vert après, un commit par point) :
- carrés (0,0)-(1,1) et (1,1)-(2,2) → refus typé OF-GEO-0008, **jamais** 1 contour fusionné ;
- carré + queue de 1 mm au coin → 1 contour fermé ;
- carré avec carré intérieur → 1 région (extérieur + trou), orientations normalisées ; deux cercles concentriques → anneau (trou) ;
- entités en désordre et inversées → mêmes régions ;
- arc début=fin=30° → segment omis, jamais un cercle de rayon r ; `Arc` nul résiduel → OF-GEO-0007 ;
- ancien journal avec `Arc{debut==fin}` → rejeu normalisé en `Cercle`, mêmes volumes ;
- MAX_ENTITES + 1 segments → OF-GEO-0009 ; croquis à deux régions disjointes → refus au plan ;
- non-régression des deux sens d'arc (A5-03), commentaire conforme au code ;
- rejeu ×2 → mêmes volumes ; CI verte.

**Amendements** :
- **2026-11-07 (ordre 3)** — Le variant `Segment2::Cercle` porte un champ `sens : SensHoraire` (`Cercle { centre, rayon, sens }`), s'écartant de la signature `Cercle { centre, rayon }` de D1 : D3 (régions par orientation + parité) exige que le cercle, boucle fermée à lui seul, participe à l'orientation normalisée des régions, impossible sans sens. Le rejeu tolérant conserve le sens (`Arc` nul relu en `Cercle`).
