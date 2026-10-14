# ADR-0009 : forge-io — import/export du jalon 1 (SVG, DXF, STL)

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-14)
- **Date :** 2026-10-14
- **Décideurs :** Raphaël Olivier

## Contexte

Le jalon 1 (ouvert par l'ADR-0008) inclut l'import/export. La fiche Formats liste ~20 formats à terme ; le périmètre du jalon 1 a été arbitré le 2026-10-14 : **minimum strict SVG/DXF/STL**, découpage des autres formats **par dépendance technique** (pas par facilité). Choix validé : STEP/.FCStd attendent le lecteur B-rep OCCT de forge-cad (jalon 3, golden tests ADR-0004) ; G-code attend forge-gcode (jalon 4) ; OBJ/CSV/3MF aux jalons suivants sans changer l'architecture.

Contraintes : ADR-0008 (l'orchestration passe par des commandes forge-session) ; fiche Sécurité (section sécurité obligatoire, budgets par parseur, SVG assaini, parsing hors fil UI, fuzzing) ; verrou 5 (unités strictes) ; ADR-0001 (gros objets = blobs, jamais dans les événements) ; aucune crate de géométrie n'existe encore (forge-doc/units/values seulement).

## Décision

1. **Crate `forge-io`** (MPL-2.0, wasm-friendly, `#![forbid(unsafe_code)]`) : pure **octets ↔ géométrie**, zéro dépendance à forge-doc, forge-session ou l'UI. L'orchestration (événements, blobs, undo) reste dans forge-session via des commandes (`ImportSvg`, `ExportStl`, …) : forge-io reste testable en isolation et réutilisable (batch, web).

2. **`forge-geom` minimal créé au jalon 1** : types 2D (segments ligne/arc/Bézier quadratique-cubique, polylignes, calques) + maillage indexé (sommets/triangles). Pas de NURBS ni OCCT (jalon 3). Unités strictes à la frontière : tout devient `Length`/`Angle` forge-units après conversion (unités utilisateur SVG, `$INSUNITS` DXF, convention mm du STL).

3. **Formats du jalon 1 : SVG, DXF, STL**, import + export chacun :
   - **SVG : `usvg`** (Linebender/resvg, Apache-2.0/MIT). Couvre la spec statique, ignore scripts/animations/événements **par conception** (assainissement natif conforme à la fiche Sécurité), aucune ressource externe chargée (options offline), résout références/`use`/texte en arbre simple ; réutilisable par forge-paint/vello aux jalons 2 et 6. On extrait les seuls éléments géométriques → forge-geom.
   - **DXF : candidat principal crate `dxf`** (lecture/écriture DXF+DXB), alternative `acadrust` (plus récent, couvre aussi DWG — non requis). Choix définitif par **spike sur corpus réel** à l'implémentation. Entités géométriques (LINE, ARC, CIRCLE, LWPOLYLINE, SPLINE basique) → forge-geom, calques préservés.
   - **STL : parseur maison** (binaire + ASCII, ~100 lignes) : budget de ressources intégré dès le parseur — aucune crate STL (stl_io, nom_stl, pk_stl) n'applique nos bornes d'allocation. Export binaire uniquement.

4. **API** : registre de formats par extension (`import(bytes, budget) -> Result<GeomDocument, IoError>`, `export(&GeomDocument) -> Result<Vec<u8>, IoError>`). Erreurs `IoError`, codes `OF-IO-*` au registre `docs/error-codes.md`, vérification CI bidirectionnelle (ADR-0003). La géométrie importée vit en **blob** référencé par un événement (ADR-0001) ; jamais de géométrie dans les événements.

5. **Section sécurité** (fiche Sécurité, coûts déclarés — tous négligeables) :
   - budget de ressources par parseur : nombre d'éléments/sommets, allocation totale, profondeur XML max — dépassement = erreur typée `OF-IO-*` (coût : néant) ;
   - usvg sans DTD ni entités externes (XXE), aucune ressource réseau ; `.svgz` (gzip) : budget de décompression avant parse ;
   - parsing **hors fil UI** (l'app appelle forge-io sur un worker) : un fichier corrompu ne gèle jamais l'UI ni le pilotage (verrou 4) ;
   - cibles **fuzz** déclarées (cargo-fuzz : svg, dxf, stl) dès l'implémentation, alimentées par le corpus de fichiers réels.

6. **Tests** (verrou « imports testés contre fichiers réels ») : fixtures produites par Inkscape (SVG), LibreCAD/FreeCAD/QCAD (DXF), Blender/PrusaSlicer (STL) ; aller-retours import → export → import ; violation de budget = erreur typée testée ; commande d'import échouée = journal intact (ADR-0008).

7. **Roadmap des formats** (actée 2026-10-14, découpage par dépendance) : OBJ/CSV/3MF aux jalons suivants — ajouter un format n'exige **pas** d'ADR (même architecture, guide d'extension « comment ajouter un format »), le choix de bibliothèque significatif reste documenté ; **STEP + .FCStd au jalon 3** avec forge-cad/OCCT (le `.FCStd` est un ZIP + fichiers `.brp` : plus simple que STEP une fois OCCT branché) ; **G-code au jalon 4** avec forge-gcode/dialectes.

## Alternatives rejetées

- **Tout au jalon 1** (STEP/.FCStd/G-code inclus) : STEP/.FCStd exigent le lecteur B-rep OCCT (forge-cad, jalon 3) — sans lui, soit OCCT tiré en avance (dette, golden tests non prêts), soit maillage lossy (le paramétrique est détruit, l'import est à refaire). G-code sans forge-gcode/éditeur/simulateur (jalon 4) n'a rien à alimenter.
- **forge-io dépendant de forge-doc** (écrire les événements directement) : briserait la frontière ADR-0008 — les commandes orchestrent ; forge-io doit rester pur et réutilisable.
- **Parseur SVG maison** (quick-xml direct) : la spec SVG dépasse 800 pages et les fichiers réels débordent de cas tordus ; usvg résout références et texte, ignore les scripts par conception, et est éprouvé par la suite de tests resvg (~1600 tests SVG→PNG).
- **Parseur DXF maison** : format énorme (groupes de codes, versions R12→2018) ; les crates existantes l'ont déjà absorbé ; le budget s'applique au-dessus (comptage d'entités et garde d'allocation après parse).
- **Crates STL** : aucune n'applique nos budgets d'allocation ; le format est trivial.
- **Géométrie dans les événements** : violerait ADR-0001 (gros objets = blobs référencés).
- **`f64` nus** à la frontière : violerait le verrou 5.

## Conséquences

- **Positives** : pipeline complet fichier → géométrie → projet (événements + blobs) → export dès le jalon 1, sur une surface 100 % Rust mémoire-sûre ; usvg réutilisable aux jalons 2 (esquisse) et 6 (paint) ; budgets sécurité posés dès le premier parseur, modèle réutilisable par tous les formats suivants ; l'ajout d'un format n'exige pas d'ADR.
- **Négatives / coûts acceptés** : usvg est une dépendance non triviale (justifiée par la couverture de spec et l'assainissement) ; la crate `dxf` a une maintenance modeste (mitigation : spike `acadrust`, remplacement borné possible — l'API forge-io ne bouge pas) ; STL ASCII malformés rejetés en tout-ou-rien au jalon 1 (pas de tolérance ligne à ligne).
- **Décisions futures suspendues** : DWG (si `acadrust` adopté) ; SPLINE/NURBS complètes au DXF (avec forge-geom jalon 3) ; PDF vectoriel ; stratégies de fusion de calques/transformations à l'import ; glisser-déposer et boîtes de dialogue (UI, hors forge-io).
