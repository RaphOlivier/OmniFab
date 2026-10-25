# ADR-0022 — Découpage du jalon 3 en cinq phases

- **Statut : ACCEPTÉ** — lot ADR-0022..0027 validé par Raphaël Olivier le 2026-10-25.
- **Jalon : 3** — CAO paramétrique + éditeur nodal de valeurs.
- **Source :** préparation du 2026-10-22, digest de session du 2026-10-25 (4 axes tranchés, validation du lot).

## Digest de session — décisions tranchées par Raphaël Olivier le 2026-10-25

1. **Features solides du jalon 3** : extrusion **additive + soustractive** (poche, perçage) ; la révolution reste suspens pour le jalon 4.
2. **Arithmétique d'unités** : typage strict + dérivés **Area/Volume** (L×L, L×L×L, simplification dimensionnelle).
3. **Rendu 3D** : **depth_stencil + ombrage directionnel simple + MSAA 4×** (option médiane) ; matériaux simples couleur/brillance par feature, textures hors périmètre.
4. **Import .FCStd** : **INCLUS dès la phase A** — choix explicite de Raphaël, contre la recommandation (suspens) ; lecture seule au jalon 3 (ADR-0023 D4 amendé).

**Objet : ouvrir le jalon 3 de la roadmap (CAO paramétrique + éditeur nodal de valeurs) — livrer la chaîne croquis contraint → solide paramétrique → feature tree défaisable, dans l'esprit ADR-0012 (une phase = un ADR de phase appliqué de bout en bout + CI verte + capitalisation).**

- **Phase A — forge-cad : consolidation OCCT** (ADR-0023). Le jalon de consolidation annoncé par le fondateur : opencascade-rs derrière le module interne `occt`, liaison dynamique vérifiée, golden tests, import/export STEP + BREP, budgets de ressources.
- **Phase B — croquis → solide** (ADR-0024, première moitié) : segments typés `Path2` (D1 de l'ADR-0016, prérequis CAM), pont esquisse→OCCT (vraies courbes, pas de polylignes), première feature **Extrusion** (additive + soustractive — tranché par le digest du 2026-10-25), événements `cad.*`, solide = état dérivé jamais sérialisé, tessellation sur le fil de travail.
- **Phase C — feature tree complet + topological naming borné** (ADR-0024, seconde moitié) : multi-croquis nommés, chaîne de features, réédition amont = rejeu (associativité), suppression `*.removed` non cassante (suspens ADR-0018), règle « références topologiques interdites entre features » tant que le naming n'est pas conçu.
- **Phase D — valeurs phase 2 : nœuds typés + arithmétique d'unités + éditeur nodal + tableau** (ADR-0025/0026, déclencheur acté ADR-0007 : connecteurs typés de l'éditeur nodal).
- **Phase E — desktop : espace CAO, arbre de projet, menu Fichier, rendu 3D riche, export croquis** (ADR-0027).

**Critères de sortie mesurables par phase** (détaillés dans chaque ADR) ; checklist de validation manuelle livrée au DÉBUT de chaque phase (gouvernance), regroupée en UNE checklist de fin de jalon (~08 janv 2027) après le checkpoint visuel « CAO lisible » (~mi-décembre). **Aucune phase sans ADR validé, aucune close sans capitalisation.**

**Hors périmètre du jalon 3 (explicitement, pour ne pas dériver)** : assemblages, mise en plan (drawings), révolution + chanfreins/arrondis (jalon 4 selon besoin), plans de travail arbitraires (l'événement `sketch.created` porte le champ, seule l'identité XY est consommée au jalon 3), multi-corps (un document = un corps), IFC, .blend, éditeur colorisé d'expressions (suspens ADR-0019, récidive), textures/glTF, CAM (jalon 4), arithmétique d'unités impliquant la vitesse (jalon 4/5).

## Risques transversaux du jalon 3 (documentés dès le lot)

1. **opencascade-rs immature** : le spike de la phase A tranche (wire→prism→STEP) ; si l'API nécessaire manque/casse, la bascule cxx ciblée est l'exception bornée de l'ADR-0004 — à refaire AVANT tout investissement en aval.
2. **vcpkg opencascade sur Windows CI** : temps de build (~30-60 min) amorti par cache ; si intractable, phase A livrée Linux/macOS d'abord et Windows à la suite (jamais de job silencieusement désactivé).
3. **Topological naming** : le jalon vit sous l'interdit D6 ADR-0024 ; la tentation « juste référencer cette face » sera permanente — tout contournement = refus au plan.
4. **Ampleur** : cinq phases denses ; l'ordre A→E suit les dépendances (l'extrusion consomme `ValeurCote` ; l'éditeur nodal consomme les types) ; chaque phase reste découpage en lots testables comme au jalon 2.
5. **Veille des bindings** : une montée OCCT (7.8/7.9) pendant le jalon = golden tests d'abord (ADR-0004), jamais en cours de lot.
