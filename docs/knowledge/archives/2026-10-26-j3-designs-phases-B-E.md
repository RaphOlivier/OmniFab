[J3] — Designs détaillés des phases B-E du jalon 3 (session sans PC, 2026-10-26)

**Thème :** rédiger les designs d'exécution des phases B à E du jalon 3 prévus par le protocole d'ADR-0022 (schémas d'événements exacts, migrations, codes OF-*, checklists de validation manuelle livrées au DÉBUT de phase, découpages en lots) — sans retrancher l'architecture (ADR-0022..0027 déjà validés), sans PC, indépendamment du spike OCCT de la phase A (points tributaires marqués « SUSPENDU AU SPIKE »).

**Méthode :** patrons des jalons 1-2 repris (ADR-0018 phase D, ADR-0019 design phase E) ; seuls les points de schéma non couverts par les ADR ont été soumis à digest (gouvernance : schéma = validation explicite).

**Digest schémas — TRANCHÉ par Raphaël Olivier le 2026-10-26 (4/4) :**
1. `Path2` : Béziers **quadratiques + cubiques** (fidèle au SVG ; deux variantes de plus) — contre la reco « cubiques seuls ».
2. Profondeur d'extrusion : **duales `ValeurCote | ATravers`** (option « à travers » retenue, enveloppe bornée 1,1× diagonale, dérivée déterministe) — contre la reco « exacte seulement ».
3. Alias des cotes de feature : **auto-nommés `cad.<type><id>.<param>`** (ex. `cad.extrusion3.profondeur`), comme `esquisse.*` — reco suivie.
4. Tableaux : **événement `values.tableau.created {nom}`** réservant le préfixe à la création, undo le libère — reco suivie.

**Décisions de design notables (dérivation mécanique des ADR, consignée pour la relecture) :** migrations v2 du croquis par champ optionnel `croquis` (absent = 0, patron `values.input.set`) ; `cad.feature.updated` à id conservé mais `croquis` non éditable en v1 ; OF-CAD-0005 « feature en échec à la reconstruction » porté par le trait `MoteurCad` (forge-session ne dépend pas de forge-cad) ; tessellation à normales produite dès la phase B (évite de retoucher le schéma) ; suppression de croquis entier = suspens (extension non cassante) ; matériaux par feature = préférence UI non persistée, assumé et documenté dans l'aide ; phase E conditionnée au découpage de main.rs AVANT le lot 2.

**Livré :** quatre fiches prep — `preparation/ADR-0024-design-phase-B.md`, `preparation/ADR-0024-design-phase-C.md`, `preparation/ADR-0025-0026-design-phase-D.md`, `preparation/ADR-0027-design-phase-E.md`. État actuel v16 (rédaction) puis v17 (validation).

**Validation — TRANCHÉ par Raphaël Olivier le 2026-10-26 (2e round, 3 amendements) :**
1. `cad.feature.updated` autorise le **changement de croquis référencé** dès v1 (contre la restriction initiale « recréer la feature ») — gardes au plan : croquis existant + contour fermé.
2. **Couleurs de feature journalisées** (contre « paramètre UI non persisté » de l'ADR-0027 D3, qui est AMENDÉ) : champ optionnel `couleur {rgba, speculaire}` dans le payload `cad.feature.*` v1, posé en phase B avant tout code ; restaurées à l'ouverture.
3. Suppression de croquis entier : **confirmée en suspens** (extension non cassante `croquis.removed`).
Les quatre fiches passent **VALIDÉ** — l'exécution reste bloquée à la clôture de la phase A (checkpoint PC : lock Cargo + spike OCCT).

**Note matériaux (Raphaël, 2026-10-26, après validation — sémantique de fond actée, implémentation différée)** : en définitive les matériaux, y compris leur apparence, doivent être **référencés par une base de données** (données ouvertes, sourcées — conventions) : la couleur par défaut d'une feature = **couleur du matériau réel**, surchargeable par une **couleur d'affichage**, avec option **couleur aléatoire** (distinction des pièces/assemblages). Traduction dans les schémas, posée AVANT tout code pour éviter toute migration : `couleur` v1 = surcharge d'affichage ; champ futur optionnel `materiau` = ajout non cassant au payload `cad.feature.*` (apparence par défaut) ; « couleur aléatoire » = rgba concret tiré à la pose et journalisé (jamais un drapeau « aléatoire » — déterminisme du rejeu). Base matériaux = lot dédié ultérieur (BOM/assistants), hors jalon 3 ; fiches B et E annotées, état actuel v18.

**Veille bindings OCCT (même session, objectif suivant choisi par Raphaël — fiche `preparation/veille-occt-2026-10-26.md`)** : recherche en ligne du 2026-10-26 — **OCCT 8.x est sorti en plein jalon 3** (8.0.0 mai-juil. 2026, 8.0.1 le 30-07) et opencascade-rs 0.3.0 le refuse (épinglage 7.x, issue #237 ouverte le 25-10) ; **vcpkg a basculé son port sur 8.0.0 en mai 2026** → le job Windows CI restera probablement rouge même après le push du lock (gate 7.x + bug MSVC #234) — attente du checkpoint ajustée par annotation (échec attendu ≠ échec du checkpoint, ADR-0023 D8 fournit les issues de secours) ; Linux sain (Ubuntu 7.9, Debian 7.8). occt-rs rejeté définitivement (AGPL-3.0) ; CVE OCCT 8.0.0rc5 (STL/OBJ) non exposés (parseurs maison + STEP/BREP seulement). Aucun changement de plan : jalon 3 sur OCCT 7.8/7.9, montée = golden tests d'abord. État actuel v19.

**Points ouverts / suite :** le spike OCCT tranche les points SUSPENDU AU SPIKE (API opencascade-rs, MakeWire sur arcs, paniques, normales, budgets tessellation) ; la phase D (pure forge-values/session/desktop, sans OCCT) est notée exécutable dès la CI verte si l'attente de la phase A le justifie ; à la clôture de chaque phase : capitalisation puis checklist manuelle de la phase suivante.
