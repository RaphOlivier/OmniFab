[J3] — Ouverture du jalon 3 : lot d'ADR ADR-0022..0027 validé en une session

Session du 2026-10-25. Demande de Raphaël : « lance le jalon 3 ». Le lot d'ADR d'ouverture avait été préparé le 2026-10-22 (ADR-0022 découpage en cinq phases, ADR-0023 forge-cad/OCCT, ADR-0024 croquis→solide + feature tree + topological naming borné, ADR-0025 valeurs typées phase 2, ADR-0026 éditeur nodal + tableau, ADR-0027 desktop CAO) — la gouvernance du 2026-10-19 exige une validation en UNE session avant exécution libre.

Digest : 4 axes posés par ask_user_question, 4 tranchés.

1. **Features solides** : additive + soustractive retenues (recommandation suivie) — le cas d'école meuble exige poches et perçages ; la révolution n'y sert pas, suspens jalon 4. Sans débat.
2. **Arithmétique d'unités** : avec dérivés Area/Volume (recommandation suivie) — répond en entier à la demande « arithmétique d'unités » du 2026-10-21 (surfaces de panneaux). Sans débat.
3. **Rendu 3D** : option médiane depth_stencil + ombrage simple + MSAA 4× (recommandation suivie) — cible le checkpoint « CAO lisible » de mi-décembre ; matériaux paramétriques complets et textures hors périmètre.
4. **.FCStd** : **seul axe où Raphaël s'écarte de la recommandation** — il choisit l'inclusion dès la phase A alors que le suspens était recommandé (STEP suffit au cas d'école). Argument implicite : la valeur d'interopérabilité FreeCAD immédiate prime sur la focalisation du lot. Acté tel quel : ADR-0023 D4 amendé (lecture du ZIP Document.xml + BREP, budgets OF-CAD-*, lecture seule au jalon 3) ; le risque d'ampleur de la phase A est assumé et documenté au lot (risque transversal n°4).

Conséquences : jalon 3 OUVERT (cinq phases A→E) ; le prep porte « tranché » ; fiche architecture passée en v2. Prochaines étapes : prérequis locaux de Raphaël (deps OCCT + cargo add opencascade-rs + Cargo.lock), puis spike wire→prisme→STEP en ouverture de phase A, ADR au dépôt avec le premier lot.

Fiches touchées : architecture/KNOWLEDGE.md (v2, entrée ADR-0022..0027, roadmap jalon 3), preparation/ADR-0022-lot-jalon-3.md (digest tranché, D4 amendé), la présente archive (v3).
