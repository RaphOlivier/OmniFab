---
name: formats
description: "Formats d'interopérabilité OmniFab : vectoriel (SVG/DXF/PDF), CAO (STEP/IGES/BREP/FCStd), maillage (STL/3MF/glTF/blend), images (PNG/ORA/kra/psd/xcf/GMIC), G-code multi-dialectes, projet natif ZIP."
---
**Version :** v1 (2026-10-25) — versionnage mis en place (fiche historique/KNOWLEDGE.md)

# Formats — interopérabilité OmniFab

Priorité aux formats ouverts/standard. Imports testés contre des fichiers réels produits par les logiciels de référence.

**Jalons d'arrivée (ADR-0009, 2026-10-14 — découpage par dépendance technique ; périmètre jalon 1 = minimum strict acté par Raphaël)** : SVG/DXF/STL → **jalon 1** (forge-io : usvg, crate `dxf`/spike `acadrust`, parseur maison STL ; forge-geom minimal) ; OBJ/CSV/3MF → jalons suivants (même architecture, pas d'ADR, guide d'extension) ; STEP/IGES/BREP/.FCStd → **jalon 3** (lecteur B-rep OCCT de forge-cad + golden tests ADR-0004 ; .FCStd = ZIP + `.brp`, plus simple que STEP une fois OCCT branché) ; G-code → **jalon 4** (crate forge-gcode + dialectes).

**Placement à l’import SVG/DXF (décision de Raphaël, 2026-10-16)** : coordonnées **exactes** du fichier — zéro translation ni centrage (fidélité, ré-export identique à l’origine) ; SVG inversé en Y (origine haut-gauche), DXF Y montant. Les origines des logiciels de référence diffèrent (Inkscape : coin du dessin à l’origine, géométrie en quadrant positif ; FreeCAD : pièce centrée sur l’origine) → des imports de sources différentes atterrissent à des places différentes : comportement attendu, pas un bug (validé C1 round 6). Un placement alternatif (centrage/cadrage) se fera uniquement sur demande explicite.

## Limites actuelles des importateurs (audits 2026-10-17, vérifiées au code — registre complet dans la fiche architecture « Dette connue »)

- **DXF** : convention 1 unité = 1 mm, z ignoré ; `$INSUNITS` non lu (pouces importés comme mm, ×25,4 silencieux) ; blocs/INSERT ignorés (périmètre ADR-0009 minimal — mais beaucoup de DXF réels portent leur géométrie en blocs) ; OCS/extrusion ignoré (normale (0,0,−1) = miroir) ; POLYLINE fermée importée ouverte (bug, lot B1) ; spline invalide ignorée en silence ; seuil de bulge 1e-12 trop fin (rayon ~1e9, perte de précision).
- **SVG** : convention 1 px = 1 mm actée (dpi 25,4, `83675c6`, validée — toute assertion d’import « ×3,78 » est fausse) ; après `Z` le point courant n’est pas réinitialisé au départ du sous-tracé → segments perdus (bug, lot B2) ; usvg en f32 (~1e-4 mm à 1 m) ; budgets appliqués après le parse complet (explosion `<use>` non bornée en amont) ; export minimal sans `width`/`height`/`viewBox` → 0,26 mm/unité dans tout lecteur externe (lot B3).
- **STL** : format sans unités (Blender exporte en m, FreeCAD en mm — échelle à l'import actée : dialogue de confirmation + champ `echelle` persisté dans l'événement `geom.mesh.imported` v2, ADR-0017) ; octets de bourrage non tolérés ; pas de soudure de sommets (3 par triangle) ; précision f32 inhérente au format.
- **Évolutions candidates** : `RapportImport` affiché à l’utilisateur (entités ignorées par type, unité détectée, avertissements — plus d’abandon silencieux) ; segments typés + discrétisation à tolérance + chaînage de contours (prérequis G2/G3, jalon 4 CAM).
| Domaine | Formats | Logiciel de référence |
|---|---|---|
| Vectoriel 2D | SVG, DXF, DWG (lecture), PDF (import) | Inkscape, FreeCAD TechDraw ; CADCraft (DWG R13–2018 en natif) |
| CAO solide | STEP, IGES, BREP, .FCStd (import) | FreeCAD |
| BIM (import) | IFC — maquettes : pièces/sous-modèles réutilisables pour la fabrication (chaîne BIM → CAM) | Revit, Archicad (exporteurs) |
| Maillage 3D | STL, 3MF, OBJ, PLY, glTF, .blend (import) | Blender, FreeCAD |
| Image raster | PNG, JPEG, TIFF, WebP, EXR, .psd/.xcf (import), .psb | GIMP, Krita |
| Document image calques | OpenRaster (.ora), .kra ; **PSD/PSB round-trip avec préservation des calques** | Krita, GIMP ; PhotoCraft (round-trip complet PSD/PSB en natif) |
| Filtres | .gmic | GMIC |
| Graphes nodaux | Conception type Graphite / Blender nodes | Graphite, Blender |
| Fabrication | G-code GRBL/FluidNC/LinuxCNC, toolpaths, DSTV/NC1 (pièces acier → CNC) | LinuxCNC, bCNC, UGS, gSender ; Tekla/Advance Steel (DSTV) |
| Simulation usinage | Rejouage G-code 3D | Camotics, BlenderCAM |
| Laser / plasma / couteau | G-code GRBL, paramètres laser/plasma, commandes tangentielles | LaserGRBL, LaserWeb, FabexCNC |

**Ajouts 2026-10-23 (analyse ArtCraft/FullControl — intégrés dans la matrice v8 au dépôt le 2026-10-24)** : **DWG en lecture** (R13–2018) — précédent récent CADCraft (Rust natif, licence permissive : crate réutilisable à évaluer au moment venu ; quasi absent du libre, QCAD/FreeCAD passent par des convertisseurs), jalon à décider (candidat élargissement forge-io après STEP/.FCStd du jalon 3) ; **PSD/PSB round-trip avec préservation des calques** (précédent PhotoCraft), jalon 6 painting raster.

## Classification et nomenclatures

- **Classification multi-référentielle des objets** : chaque élément porte des codes de plusieurs référentiels simultanément (OmniClass, Uniclass, UniFormat, normes mécaniques ISO, bibliothèques maison) ; la disponibilité des propriétés est conditionnée par la classification (modèle Archicad/Revit). C'est le lien entre assistants (valeurs normées héritées par classe), bibliothèques du jalon 10 et interopérabilité.
- **Nomenclatures/BOM paramétriques** : recalculées en direct depuis le modèle, export tableur.
- **openBIM** (interopérabilité avec les outils commerciaux, noté long terme) : BCF (sujets/incidents ancrés sur le modèle), IDS (exigences d'information lisibles machine).

## Bibliothèques de données (matériaux, visserie, quincaillerie, moteurs, électronique…) — principes actés, schéma à cadrer au jalon 10 (vision Raphaël, 2026-10-19)

Périmètre : matériaux (propriétés physiques, avances/vitesses), visserie et standards, quincaillerie, moteurs, circuits électroniques — paramètres, dimensions ET relations entre éléments. Principes proposés :
- **Données, pas code** : bibliothèques = fichiers versionnés, lisibles, diffables (même philosophie que les assistants déclaratifs et profils contrôleurs).
- **Ontologie à relations typées** : identifiants stables par entité ET par caractéristique normalisée (ex. `Filetage_M20x2` référencé par la vis, l'écrou, le mitigeur, l'embout de douche) — la compatibilité (« quels éléments s'adaptent ? ») devient une requête sur le graphe ; squelette = classification multi-référentielle déjà actée. C'est le prérequis des assistants de conception (valeurs normées héritées par classe) et des nomenclatures/BOM.
- **Provenance et licence par enregistrement** (ODbL/CC0/CC-BY, jamais GPL pour des données — fiche Conventions) ; normes ISO via tables légalement publiées, sources citées.
- **Alimentation communautaire** : pipeline de contribution avec modération, relecture des sources, badges de confiance (donnée constructeur citée > contribution non vérifiée) ; base de contrôleurs machines incluse (cf. fiche Machines, détection automatique).
- **Récupération** : hors ligne par défaut (lot embarqué, application pleinement fonctionnelle), synchronisation en ligne optionnelle ; requêtes par classification, propriété physique ou relation.
- **Bibliothèques comme référence ouverte du marché (décision Raphaël, 2026-10-19)** : les bibliothèques ne sont pas un atout interne d'OmniFab mais une **infrastructure de données partagée pour l'industrie** — accessibles et utilisables par des tiers (autres utilisateurs, autres logiciels) comme base de données de référence gratuite. Modèle : OpenStreetMap des données de fabrication. Conséquences de conception : (a) format d'échange **autonome**, documenté et versionné publiquement, indépendant du format projet OmniFab ; (b) accès triple — fichiers téléchargeables, API de requête, crate/bibliothèque de lecture réutilisable (MPL/Apache) ; (c) gouvernance neutre (dépôt dédié, contribution tierce possible sans utiliser OmniFab) ; (d) identifiants et schéma **stables et publiés** (les tiers doivent pouvoir dépendre dessus) ; (e) ODbL = levier : le share-alike sur les données incite la contribution à la base commune plutôt que le fork fermé — exactement la dynamique OSM. Objectif explicite : devenir la référence gratuite du domaine (matériaux, visserie, quincaillerie, moteurs, électronique, contrôleurs machines).
- **Architecture à trois niveaux : données vs géométrie (décisions Raphaël, 2026-10-19)** : les entrées référencent aussi des **assemblages compositionnels** (moteur, valve, vérin = sous-entrées + relations) et peuvent porter des **modèles 2D/3D liés**. Les niveaux : **(1) généré depuis les données** — pour les pièces entièrement décrites par une norme (visserie, profilés IPE/HEA/UPN/cornières, écrous, rondelles, arbres, câbles/tuyaux…), la géométrie n'est pas stockée mais **générée depuis les valeurs de la couche 1** par des générateurs paramétriques (templates déclaratifs branchés sur forge-values — même philosophie que les assistants) : zéro volume additionnel, cohérence données↔géométrie par construction, licences limpides (dimensions normées publiées, modèle non copié), déclinaison infinie des tailles ; **règle de décision publiable : si une norme décrit la pièce → générée ; sinon → modèle téléchargé** (un générateur = un coût de qualité, prioriser les familles à fort usage ; modèle constructeur téléchargeable reste optionnel pour le visuel) ; **(2) bibliothèque descriptive embarquée** (couche données) : propriétés, normes, relations, références vers modèles par identifiant stable `geo:…` — volumétrie Ko, **intégrée au fichier d'installation**, requêtable hors ligne dès le premier lancement ; **(3) bibliothèque géométrique téléchargée** (dépôt séparé, volumétrie Mo/Go : glTF léger par LOD pour la prévisualisation, STEP/3MF pour la fabrication) — **téléchargement à la demande** par article ou lot thématique + cache local. Les trois niveaux forment une seule base logique liée par identifiants stables ; ne jamais mixer géométrie et données dans un même dépôt (tuerait l'adoption tierce : la couche données doit rester adoptable seule). Cohérence avec le verrou : la géométrie est toujours du contenu référencé, jamais inline (même philosophie que blobs hors journal).

## Projet natif OmniFab

Conteneur ZIP : manifeste JSON versionné + un fichier par objet + blobs binaires (maillages, images). Historique (event sourcing) et paramètres machine enregistrés dedans. Champs version + outil de migration dès le jour 1. Schéma détaillé du manifeste : à préciser en phase 0.

## Historique

- v1 (2026-10-25) : mise en place du versionnage — n° de version en tête + cette section (protocole : fiche historique).
