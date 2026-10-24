<!-- Source : canevas « OmniFab — Matrice fonctionnalité × logiciel de référence » (v7). Synchronisé depuis la phase 0 ; fait foi pour l'inventaire exhaustif des capacités (garde-fou anti-oubli). -->

# Matrice fonctionnalité × logiciel de référence — OmniFab

> **Rôle de ce document :** garde-fou anti-oubli et cahier des charges à long terme (l'inventaire exhaustif exigé en phase 0). Ce n'est PAS une liste de tâches immédiate — chaque ligne est une capacité que l'architecture d'OmniFab doit pouvoir accueillir un jour. Les doublons entre logiciels sont fusionnés ; les fonctions absentes de tous les logiciels de référence sont marquées **★ novateur** (OmniFab irait au-delà).
>
> **Colonnes :** « Présent dans » = quels logiciels de référence offrent la fonction ; « Cible » = où elle atterrit dans OmniFab (crate / jalon).
>
> **Statut :** v7 — enrichie par 8 cycles de recherche (~55 logiciels documentés). Nouveautés v6 : assistants de conception guidée (§11). Nouveautés v7 : classification multi-référentielle des objets (BIM/OmniClass/Uniclass), nomenclatures/BOM paramétriques, interopérabilité openBIM (IFC, BCF, IDS, DSTV/NC1). Reste à hiérarchiser par jalon pendant la phase 0.

## 1. Dessin vectoriel 2D — réf. Inkscape, Graphite, FreeCAD (Sketcher)

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Formes primitives, chemins de Bézier, édition de nœuds | Inkscape, Graphite | forge-paint (2D), jalon 1 |
| Calques, groupes, verrouillage | Inkscape | forge-paint, jalon 1 |
| Alignement / distribution, grilles et guides | Inkscape | forge-paint, jalon 2 |
| Opérations booléennes (union, différence, intersection) | Inkscape, Graphite | forge-geom, jalon 2 |
| Texte vectoriel, texte sur courbe, vectorisation | Inkscape | forge-paint, jalon 2 |
| Simplification / nettoyage de chemins | Inkscape, Graphite | forge-geom, jalon 2 |
| Clones dynamiques / motifs répétés | Inkscape | forge-paint + forge-values |
| Vectorisation d'image (trace bitmap) | Inkscape | forge-io + forge-paint, jalon 4 |
| Graphe nodal procédural 2D | Graphite ★ | forge-values (nodal), jalon 3 |
| Extensions / scripts | Inkscape | plugins WASM, jalon 8 |
| Snapping objet/axe/grille fin | Inkscape, FreeCAD | forge-paint, jalon 2 |
| Effets de chemin vivants (LPE) : non destructifs, empilables, éditables (offset, tressage, engrenages…) | Inkscape ★ rare ailleurs | forge-paint + event sourcing, jalon 3+ |
| Symboles et blocs réutilisables, avec bibliothèques prêtes (électrique, mécanique…) | Inkscape (symboles), QCAD (blocs) | forge-paint, jalon 2+ |
| Cotation technique : styles de cotes, flèches, tolérances, cotes d'ensemble/détail | QCAD ★ absent du vectoriel libre courant | forge-paint (mode technique), jalon 4+ |
| Hachures normalisées (ISO/ANSI) dans zones fermées | QCAD | forge-paint, jalon 4+ |
| Grille et vue isométriques (dessin technique 2D) | QCAD | forge-paint, option |
| Import DWG en plus du DXF ; conversion par lots | QCAD ★ dans le libre | forge-io, jalon 2 |
| DXF exporté avec cotation conservée | SolveSpace, QCAD | forge-io, jalon 2 |

## 2. Raster / painting — réf. GIMP, Krita, GMIC

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Calques : masques, modes de fusion, groupes, transparence | GIMP, Krita | forge-paint, jalon 6 |
| Pinceaux paramétriques (taille, opacité, dynamics, textures) | GIMP, Krita | forge-paint, jalon 6 |
| Stabilisateur de tracé, assistants (perspective, symétrie, grille) | Krita | forge-paint, jalon 6+ |
| Outils de retouche : clone, correcteur, patch | GIMP, Krita | forge-paint, jalon 6 |
| Sélections : lasso, baguette magique, sélection par chemin, masque rapide | GIMP, Krita | forge-paint, jalon 6 |
| Réglages colorimétriques : courbes, niveaux, LUT, ICC | GIMP, Krita, GMIC | forge-paint, jalon 6 |
| Pipeline de filtres enchaînés (réseau de traitements) | GMIC | forge-paint (filtres), jalon 6/8 |
| Filtres et effets étendus (centaines) | GMIC, GIMP | bindings GMIC, jalon 6+ |
| Animation par calques / timeline | Krita | option avancée (long terme) |
| Gestion HDR / EXR | Krita | forge-paint + forge-io |
| Texte, filtres de texte | GIMP | forge-paint, jalon 6 |
| Historique de retouches détaillé | GIMP, Krita | event sourcing (natif) |
| Scripts (Script-Fu, plugins Python) | GIMP, Krita | plugins WASM, jalon 8 |
| Plusieurs moteurs de pinceaux (9+), tags et favoris de pinceaux | Krita | forge-paint, jalon 6 |
| Symétrie multi-pinceaux, assistants perspective + fisheye, plusieurs stabilisateurs | Krita | forge-paint, jalon 6+ |
| Wrap-around : peinture de textures sans couture (tuilage vivant) | Krita | forge-paint, jalon 6+ |
| Onion skinning (décalques), animation image par image | Krita | forge-paint, option |
| Calques vectoriels et calques-fichier (références externes) | Krita | forge-paint, option |
| Outils BD : cases, halftone (trames), traits de contour | Krita | forge-paint, option |
| Pipelines de filtres personnalisés enregistrés (scripts .gmic) | GMIC | forge-paint (filtres), jalon 6/8 |

## 3. CAO paramétrique — réf. FreeCAD

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Esquisse contrainte (géométrie + contraintes, solveur) | FreeCAD | forge-cad + planegcs, jalon 2 |
| Fonctions : pad, pocket, révolution, lissage (loft), balayage | FreeCAD | forge-cad, jalon 3 |
| Arbre des fonctions (feature tree) éditable, recalcul automatique | FreeCAD | forge-cad + event sourcing, jalon 3 |
| Fillets, chanfreins, coques | FreeCAD | forge-cad, jalon 3 |
| Assemblages avec contraintes entre pièces | FreeCAD | forge-cad, jalon 3+ |
| Mise en plan : vues projetées, cotation, cartouche | FreeCAD (TechDraw) | forge-cad, jalon 3+ |
| Surfaces NURBS, reconstruction | FreeCAD | forge-geom, jalon 3+ |
| Tableur paramétrique lié aux fonctions | FreeCAD | forge-values (alias/expressions) |
| Bibliothèque de visserie normée (ISO) | FreeCAD (Fasteners) | jalon 10 (bibliothèques) |
| Tôlerie (plis, développé) | FreeCAD (Sheet Metal) | forge-cad, option avancée |
| Multicorps, matériaux, propriétés physiques (masse, densité) | FreeCAD | forge-cad, jalon 3 |
| Import/export STEP, IGES, BREP ; import .FCStd | FreeCAD | forge-io (OCCT), jalon 3 |
| Simulation FEM | FreeCAD | hors périmètre initial (option) |
| Extrusion hélicoïdale (filets, ressorts), motifs step & repeat booléens | SolveSpace ★ | forge-cad, jalon 3 |
| Assemblages 6 degrés de liberté + **simulation de linkages avec traçage de trajectoire** (cinématique) | SolveSpace ★ unique en libre | forge-cad, long terme |
| Mesures intégrées : distances, angles, aires, propriétés de masse | SolveSpace, FreeCAD | forge-cad, jalon 2 |
| Texte 3D extrudé (ShapeString) | FreeCAD | forge-cad, jalon 3 |
| CAO par script : modèle = code, entièrement reproductible | OpenSCAD | forge-values / scripting, à arbitrer |
| BIM / architecture (murs, niveaux, coupes) | FreeCAD (Arch) | hors périmètre initial (option) |
| Mise en plan riche : pages multiples, cartouches, vues de coupe/détail, export SVG/DXF/PDF | FreeCAD (TechDraw), QCAD (2D) | forge-cad, jalon 3+ |
| Import nuages de points et reconstruction de surfaces | MeshLab | forge-geom, option |
| Configurations multiples par fichier + tables de conception (design tables Excel), configurations dérivées | SolidWorks ★ profondeur rare en libre | forge-cad, jalon 3+ |
| Charpentes métalliques (weldments) : profils standards, gestion des coins, nomenclature de coupe (cut list) | SolidWorks | forge-cad, long terme |
| Outillage de moulage (mold tools : dépouilles, ligne de joint, plaque) | SolidWorks | hors périmètre initial (option) |
| GD&T / cotation fonctionnelle 3D (FTA, MBD) : annotations associatives, schémas de cotes, revue sur maquette | CATIA (FTA), SolidWorks (MBD) ★ libre quasi absent | forge-cad, long terme |
| Knowledgeware : paramètres, formules, règles proactives, checks de validation, réactions événementielles | CATIA ★ unique | forge-values (moteur de règles), long terme |
| Surfaces Class-A (FreeStyle) + rétro-ingénierie de nuages scannés (DSE/QSR, Power Fit) | CATIA ★ | forge-geom, très long terme |
| Modélisation par subdivision (formes organiques, « Imagine & Shape ») | CATIA ★ | option très long terme |
| Design génératif / optimisation topologique avec résultat B-Rep **éditable** + assistants de reconstruction (usinage, fonderie, nervures) | CATIA (Generative Design), SolidWorks Simulation | option très long terme |
| DMU : cinématique, simulation d'ajustage (fitting), revue de maquette numérique | CATIA (DMU) | forge-cad, option |
| Auto-constrainte IA : esquisse entièrement contrainte proposée automatiquement | Fusion 360 (AutoConstrain) ★ | forge-cad, long terme |
| Mises en plan automatiques prêtes à fabriquer depuis le modèle 3D | Fusion 360 (Automated Drawings) ★ | forge-cad, long terme |
| Édition sans historique (synchronous modeling) de modèles importés / B-rep neutres | Siemens NX ★ rare | forge-cad, long terme |
| Symboles de soudure, états de surface, bibliothèques de matériel standard intégré | Onshape, SolidWorks | forge-cad, jalon 10 |
| ECAD-MCAD : schéma/PCB → modèle 3D BRep vivant inséré dans l'assemblage mécanique | Fusion 360 (Electronics) ★ unique | forge-cad, option long terme |

## 4. Mesh 3D — réf. Blender (via FabexCNC)

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Import/export STL, 3MF, OBJ, PLY, glTF ; import .blend | Blender | forge-io, jalon 1 |
| Nettoyage de maillage (non-manifold, réparations) | Blender | forge-geom, jalon 4 |
| Transformations, snap 3D, mesure | Blender | forge-cad + forge-render, jalon 2 |
| Heightmap / relief → mesh | BlenderCAM | forge-cam + forge-paint, jalon 4 |
| Sculpt numérique | Blender | option très long terme |
| Animation / rendu photoréaliste | Blender | hors périmètre (viewport wgpu seulement) |
| Système de nodes (geometry nodes, shaders) | Blender | forge-values (graphe nodal), jalon 3 |
| Fermeture de trous et réparation dédiée des maillages | MeshLab | forge-geom, jalon 4 |
| Décimation (quadric edge collapse), remeshing iso-paramétrique, lissage/fairing | MeshLab | forge-geom, jalon 4+ |
| Analyses : volume, enveloppe convexe, voxelisation, courbure, mesures topologiques | MeshLab | forge-geom, jalon 4+ |
| Découpe par plan et split de surfaces | MeshLab | forge-geom, jalon 4+ |
| Reconstruction de surfaces depuis nuages de points | MeshLab | forge-geom, option |
| Peinture de couleurs/régions directement sur le mesh | MeshLab, PrusaSlicer | forge-paint (3D), option |

## 5. CAM / trajectoires — réf. FabexCNC (BlenderCAM), KiriMoto, FreeCAD Path

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Opérations 2,5D : poche (pocket), contour (profile), perçage, gravure | Fabex, KiriMoto, FreeCAD Path | forge-cam, jalon 4 |
| Ponts (tabs), rampes d'entrée, plongée hélicoïdale | Fabex, KiriMoto | forge-cam, jalon 4 |
| Stratégies 3D : lignes parallèles, contournage, waterline, rest machining | Fabex | forge-cam, jalon 4+ |
| Stratégies de finition 3D : scallop (pas constant), pencil (nettoyage des creux), flowline (suivant le flux de surface), spirale morphée | Fusion 360 ★ libre quasi absent | forge-cam, long terme |
| **Projection de trajectoire sur surface 3D** (gravure suivant la courbe Z d'un objet, wrap sur cylindre) | Fusion 360 (wrap), Aspire ★ libre absent | forge-cam, long terme |
| Ébauche adaptative/trochoïdale avec anti-collision porte-outil, réduction des coupes à vide | Fusion 360 ★ libre absent | forge-cam, long terme |
| Photo → gravure par lignes de profondeurs variables (PhotoVCarve) | Vectric Aspire ★ libre absent | forge-cam + forge-paint, option |
| Gravure diamant (drag, Quick Engrave) — marquage sans coupe | Vectric Aspire, InkCut (traceurs) | forge-cam, option |
| Fraisage 3D de surfaces (heightmaps incluses) | Fabex (bas-relief), KiriMoto | forge-cam, jalon 4+ |
| Fraisage CNC 3, 4, 5 axes | LinuxCNC (machine), CNC Toolkit ★ libre rare | forge-cam : 3 axes → 3+2 positionnel → simultané (long terme) |
| Tournage (tour numérique) | Ezilathe (DXF→G-code, filetages), CNCSimple 2.0 (outils motorisés), LinuxCNC lathe ; commerciaux : Ecam, CamBam | forge-cam, long terme |
| Gravure V (V-carving) | F-Engrave (libre, référence), Open Voronoi, RasterCarve ; commerciaux : Vectric VCarve/Aspire, Carbide Create | forge-cam, ambition initiale à définir (lignes médianes + profondeur selon angle) |
| Découpe au couteau (drag knife, commande tangentielle) | KiriMoto | forge-cam, jalon 4 |
| Découpe plasma (kerf, lead-in/out, THC, perçage+dwell) | PlasmaC/QtPlasmaC (LinuxCNC, cut charts, THC complet), dxf2gcode, gcodetools ; commerciaux : SheetCam (référence standard), Mach3 | forge-cam + forge-link (THC), ambition initiale à définir |
| Laser : découpe/gravure, puissance & vitesse par couleur/couche | LaserGRBL, LaserWeb, KiriMoto | forge-cam, jalon 4 |
| Image raster → G-code (gravure, halftone) | KiriMoto, LaserGRBL | forge-cam, jalon 4 |
| Découpage 3D→2D automatique + nesting (gabarits empilés) | KiriMoto, Deepnest (fusion lignes communes, part-in-part, images bitmap pour gravure), SVGnest (web) | forge-cam, jalon 4+ |
| Emboitements menuiserie (mortaises, interlocks) | Fabex (joinery) | forge-cam, jalon 4+ |
| Bas relief, intarsion | Fabex | forge-cam, option avancée |
| **Fabrication par tranches « 123D Make » : tranches empilées, tranches enchevêtrées (fentes), tranches courbes (conformes à la surface), tranches radiales, panneaux pliés, goupilles de positionnement, plans d'assemblage numérotés** | Slicer for Fusion 360 / 123D Make ★ abandonné par Autodesk, aucun équivalent libre | forge-cam (générateur de gabarits), à planifier |
| Broderie numérique (passeurs de fils, DST/JEF/PES) | Ink/Stitch ★ | forge-cam (procédé), option avancée |
| Bibliothèques : outils, machines, opérations (pré-réglages) | Fabex (presets), KiriMoto (tool library) | forge-cam + jalon 10 |
| Post-processeurs personnalisables par machine | Fabex, KiriMoto | forge-gcode, jalon 4 |
| Macros G-code de début / changement d'outil / fin | KiriMoto | forge-gcode, jalon 4 |
| Traitement par lots avec variations | GMIC ★ partiellement | jalon 10 |
| Fluting : gorges/rampes décoratives creusées sur surfaces et courbes | Vectric Aspire ★ libre absent | forge-cam, option |
| Toolpath « texture » : passes ondulées/aléatoires pour effet matière (bois brossé) | Vectric (Texture), Carbide Create ★ libre absent | forge-cam, option |
| Inlays V-carve complets : pièces mâle/femelle avec sur-épaisseur d'assemblage (flange), purges | F-Engrave, Vectric ★ | forge-cam, jalon 4+ |
| B-carving : gravure fontaine à fraise boule + poches fenêtrées | F-Engrave ★ | forge-cam, option |
| Texte sur arc : justifications, miroir/flip pour supports | F-Engrave | forge-cam + forge-paint, jalon 4 |
| Récupération des réglages depuis un G-code existant (reverse CAM) | F-Engrave ★ rare | forge-gcode (import), long terme |
| Surfacing du brut (défonceuse) + tournage de brut, surfacing rotatif | gSender ★ rare | forge-cam + forge-link, jalon 5 |
| Perçage avancé : peck, retract, dwell, profondeurs par passe | Vectric, bCNC | forge-cam, jalon 4 |
| Filetage au fraisage (thread milling), hélice | bCNC, Camotics | forge-cam, jalon 5+ |
| Rotary wrapping : job XY projeté sur axe rotatif A/B | Vectric (rotary), gSender | forge-cam, long terme |
| Modèles de trajectoires réutilisables + fusion + copie en réseau (array) | Vectric (templates/merged) ★ rare en libre | forge-cam + forge-values, jalon 4+ |
| Tuilage de trajectoires dépassant la course machine (Tile toolpaths) | Vectric, EstlCAM | forge-cam, long terme |
| Fiche de travail (job sheet) : outils, paramètres, photos, temps estimé | Vectric ★ rare | forge-cam (rapport), jalon 5 |
| Sélection de vecteurs par propriété (couleur, couche, longueur) pour les trajectoires | Vectric ★ | forge-cam, jalon 4+ |
| Wire EDM (électro-érosion fil) : glue stops, slug removal, lead-off | Edgecam ★ libre absent | hors périmètre initial — l'abstraction « procédés » doit pouvoir l'accueillir |
| Calques laser multiples (~30) avec réglages par calque + calques guide non gravés (T1/T2) | LightBurn ★ libre partiel | forge-cam (laser), jalon 4 |
| Calibration caméra laser + traçage depuis photo, repérage print & cut après impression | LightBurn, InkCut ★ | forge-cam + forge-link, long terme |
| Offsetting laser : offset intérieur/extérieur des courbes | LightBurn | forge-cam, jalon 4 |
| Vectorisation d'images avec nettoyage (whiskers, filigranes) + ordre optimal des trajets | LaserGRBL ★ | forge-io + forge-cam, jalon 4 |
| Broderie détaillée : points satin/E-stitch/ripple/bean/zigzag, lettrage sur chemin, export par lots, tri couleurs, édition au point, cutwork/redwork | Ink/Stitch ★ | forge-cam (procédé broderie), long terme |
| Impression 3D (procédé dépôt) : régions modifiées par mesh, supports peints/organiques, fuzzy skin, multi-matières | PrusaSlicer ★ | forge-cam (procédé dépôt), jalon 8 |
| Reconnaissance automatique de fonctions (AFR / feature-based machining) : poches, bossages, trous → opérations proposées automatiquement | SolidWorks CAM / CAMWorks, Siemens NX ★ libre quasi absent | forge-cam, long terme |
| Usinage fondé sur la connaissance (KBM) : base de règles d'entreprise (TechDB), capture et réutilisation des bonnes pratiques | SolidWorks CAM / CAMWorks ★ | forge-cam + jalon 10, long terme |
| **Usinage piloté par les tolérances (TBM)** : stratégies ajustées automatiquement d'après la cotation/GD&T du modèle | SolidWorks CAM ★ unique | forge-cam (liaison CAO→CAM), très long terme |
| Fraisage indexé 4/5 axes : codes rotation A/B/C générés entre les setups | SolidWorks CAM, CAMWorks | forge-cam (3+2), long terme |
| Setup virtuel d'assemblage : pièces/fixtures assignés dans le montage, trajectoires les évitant automatiquement | SolidWorks CAM | forge-cam, long terme |
| Finition accélérée : outils segment de cercle (tonneau) → pas de finition élargi | Mastercam (Accelerated Finishing) ★ | forge-cam, très long terme |
| Usinage multi-tâches mill-turn, sous-broches, simulation machine complète (virtual machine) | CAMWorks, Mastercam | forge-cam, très long terme |
| Swiss machining (tours à barres guidés) : C-axis, face/cross contour et perçage | Mastercam (Swiss) ★ | hors périmètre initial — noté pour l'abstraction procédés |
| Filaire EDM avancé : bibliothèques de fil, passes brutes/skim, types de coins, conicités | Mastercam (Wire) ★ | hors périmètre initial — noté pour l'abstraction procédés |
| Ébauche adaptative multi-axes haute vitesse (jusqu'à -60 % de temps d'usinage) | Siemens NX (Adaptive Milling), Fusion 360 | forge-cam, long terme |
| Tube milling : fraisage de tubes et formes creuses étroites, visualisation des angles d'attaque | Siemens NX ★ | hors périmètre initial — noté pour l'abstraction procédés |
| Adaptation du modèle aux états intermédiaires d'usinage (in-process) + modélisation des fixations | Siemens NX ★ | forge-cam, long terme |
| Gestionnaire de synchronisation multi-canaux pour mill-turn (synchronisation interactive) | Siemens NX (Sync Manager) | forge-cam, très long terme |
| **Chaîne CAM entièrement nodale** : source (PNG/SVG) → modules de calcul → machine, sans passer par la CAO | mods (MIT CBA) ★ | forge-values (graphe nodal), jalons 3-4 |

## 6. Simulation G-code — réf. Camotics

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Rejouage 3D du G-code (enlèvement de matière) | Camotics | forge-gcode (simulateur), jalon 4 |
| Détection de collisions / erreurs | Camotics | forge-gcode, jalon 4 |
| Aperçu de la trajectoire au curseur de l'éditeur | ★ aucun | forge-gcode (éditeur), jalon 4 |
| Pas-à-pas, coupe, avance rapide, stats temps | Camotics | forge-gcode, jalon 4 |
| Multi-outils / changement d'outil simulé | Camotics | forge-gcode, jalon 5+ |
| Table d'outils simulée (n°, diamètre, longueur) | Camotics | forge-gcode, jalon 5 |
| Export de la pièce usinée en STL (vérification CAO ↔ CFAO) | Camotics ★ rare | forge-gcode, jalon 5 |
| Simulation des O-codes (sous-programmes, boucles) | Camotics | forge-gcode, jalon 5 |

## 7. Envoi & contrôle machine — réf. bCNC, UGS, gSender, LaserGRBL, LaserWeb

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Connexion série USB + réseau (Ethernet/WiFi/IP) | tous | forge-link, jalon 5 |
| **Transfert RS-232 / DNC drip-feed** (machines anciennes Fanuc/Mazak/HAAS : envoi au fil de l'exécution, bauds/parité/handshake configurables, upload/download de programmes) | logiciels DNC commerciaux ★ libre quasi absent | forge-link, long terme |
| **Pilotage direct par port parallèle LPT** (signaux step/dir sur les broches, machines pas-à-pas classiques ; exige noyau Linux temps réel + LPT natif/PCIe, les adaptateurs USB-LPT sont inutilisables à cause de la latence) | LinuxCNC (parport), Mach3 historique | forge-link, option long terme |
| **Alerte pédagogique des risques pendant le contrôle** (avant lancement : collision, courses, broche en rotation ; pendant : feed hold, e-stop distant ; contournement = confirmation explicite) | ★ aucun | forge-link, jalon 5 |
| DRO temps réel (positions, états) | tous | forge-link, jalon 5 |
| Overrides feed / rapid / broche | UGS, gSender, bCNC | forge-link, jalon 5 |
| Jog boutons + clavier, continu/par pas | UGS, gSender, bCNC | forge-link, jalon 5 |
| Palpage : Z auto (touch plate), coins, centre, grille | gSender, bCNC, UGS | forge-link, jalon 5 |
| WCS multiples (G54–G59), mise à zéro par axe | gSender, bCNC, UGS | forge-link, jalon 5 |
| Macros personnalisées (boutons, code) | gSender, bCNC | forge-link, jalon 5 |
| Changement d'outil M6 : pause / ignorer / bloc de code | gSender, UGS | forge-link, jalon 5 |
| Reprise « start-from-line » après incident | gSender, UGS | forge-link, jalon 5 |
| Visualiseur 3D avec bounds/outline du job | gSender, UGS, LaserWeb | forge-link, jalon 5 |
| Estimation du temps d'usinage | UGS, Camotics | forge-link, jalon 5 |
| Console G-code (envoyé/reçu, journal) | tous | forge-link, jalon 5 |
| Lecture/édition des paramètres firmware ($$) | gSender, bCNC, LaserGRBL | forge-link, jalon 5 |
| Outils de calibration (squareness, pas moteurs) | gSender ★ | forge-link, jalon 5+ |
| Contrôle à distance depuis un 2e appareil (navigateur) | gSender, LaserWeb | mobile + web OmniFab, jalons 7/9 |
| **Multi-machines en parallèle (files par machine)** | ★ aucun | forge-link, jalon 5 |
| **Assistant de création de profils/dialectes maison** | ★ aucun | forge-link + forge-gcode, jalon 10 |
| Sécurité : feed hold, door, e-stop, alarmes | gSender, UGS, bCNC | forge-link, jalon 5 |
| Éditeur G-code intégré : colorisation + autocomplétion par dialecte | bCNC (basique) ★ aucun complet | forge-gcode, jalon 4 |
| Aide en ligne sur les commandes G/M | ★ aucun | help_id (aide intégrée) |
| Compensation de backlash (mesure + correction) | UGS, bCNC | forge-link, jalon 5 |
| Autolevel Z : grille de palpage + ajustement en vol (supports non plans) | bCNC, LaserGRBL | forge-link, jalon 5 |
| Numérisation par palpage (digitizer) : relevé de points, cartographie 3D | bCNC ★ rare | forge-link, long terme |
| Gravure d'images à profondeur variable détectée par IA → depth map | UGS ★ unique | forge-cam + forge-paint, long terme |
| Dialogue M6 interactif avec jog (changement d'outil guidé manuellement) | UGS | forge-link, jalon 5 |
| Visualiseur : jog au clic droit, zoom sur sélection, ligne courante surlignée dans l'éditeur | UGS ★ ensemble | forge-link, jalon 5 |
| Affichage en direct des états des broches E/S (pins) | gSender | forge-link, jalon 5 |
| Visualisation laser dédiée (rendu du faisceau et de la gravure) | gSender | forge-link, jalon 5 |
| Contrôle au gamepad | gSender ★ rare | forge-link, option |
| Connexions TCP/IP + firmwares étendus (grblHAL, TinyG, g2core, Smoothieware) | UGS, CNCjs | forge-link, jalon 5+ |
| Périphériques atelier : clavier dédié, LCD, tablette shopfloor, commandes vocales | CNCjs ★ | forge-link + web, long terme |
| Mini-éditeur 2D intégré au contrôleur (import SVG/DXF/PNG, arc fitting G2/G3) | UGS (designer) ★ | forge-paint + forge-gcode |

## 8. Firmware / contrôleurs — réf. GRBL, FluidNC, Marlin, LinuxCNC

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Dialecte GRBL (status reports, EEPROM) | GRBL | forge-gcode/link, jalon 5 |
| FluidNC : config YAML, WiFi natif | FluidNC | forge-link, jalon 5+ |
| Marlin (impression + CNC léger) | Marlin | forge-gcode, jalon 8+ |
| LinuxCNC : multi-axes, tool table, remap G-code, HAL | LinuxCNC | forge-link, jalon 5 (prioritaire) |
| **OmniFab comme GUI LinuxCNC (remplace Axis)** : mode GUI natif déclaré dans l'INI ([DISPLAY] DISPLAY=omnifab, liaison NML via FFI) OU pilotage distant via linuxcncrsh (réseau) ; le temps réel et le HAL restent à LinuxCNC | gmoccapy, QtDragon/QtVCP (GUI alternatives) ★ GUI non-Python récente inexistante | forge-link, option structurante |
| Changeur d'outils automatique | LinuxCNC ★ | forge-link + forge-cam, long terme |
| Machines maison : N moteurs, axes multiples, G-code custom | ★ aucun | assistant de profils, jalon 10 |
| Correction dynamique de vitesse (thc plasma), relais, capteurs | LinuxCNC ★ | option avancée |
| Remap complet des G/M-codes (Python), O-word (conditions, boucles), paramètres persistants, accès HAL depuis le G-code | LinuxCNC ★ profondeur unique | forge-gcode (comprendre/simuler), jalon 5+ |
| Rack de changement d'outils (iocontrol, toolchange rack) | LinuxCNC | forge-link, long terme |
| Web UI embarquée multi-configs YAML (ESP32) | FluidNC | forge-link (transport web), jalon 9 |

## 9. Transversal

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| **Aide contextuelle : clic droit → doc + exemples sur chaque option** | ★ aucun | help_id, jalon 1 |
| i18n complet (traductions) | Krita, GIMP, FreeCAD | Fluent, jalon 1 |
| Thèmes sombre/clair, UI personnalisable (dockers) | Blender, Krita | egui, jalon 2 |
| Plugins/extensions isolés | Blender, GIMP, Krita, Inkscape, FreeCAD | WASM, jalon 8 |
| Scripting avancé (Python…) | FreeCAD, Blender | à arbitrer (API Rust/WASM) |
| Undo illimité, historique détaillé | tous | event sourcing (natif) |
| Raccourcis clavier configurables | Blender, Krita | egui, jalon 2 |
| Formats natifs documentés publiquement | Blender, FreeCAD | ZIP + manifeste, jalon 1 |
| Biblio. de matériaux avec données réelles (avances/vitesses) | gSender (presets machine) ★ partiellement | jalon 10 |
| Biblio. de machines types/marques, normes ISO | ★ aucun | jalon 10 |
| Liaison de valeurs par alias ET nodal (Xpresso) | Blender (drivers), Cinema 4D ★ non-libre | forge-values, jalons 2–3 |
| **Documentation développeur de premier ordre** : rustdoc sur toute API publique (testé CI), ADR pour chaque décision, doc d'architecture vivante, guide « ajouter un procédé/dialecte/format », codes d'erreur typés | Blender (Python API docs), FreeCAD ★ rare à ce niveau dans le libre hobby | transversal, jalon 0 |
| Interface adaptée par plateforme (desktop dense / mobile menus) | ★ aucun | jalons 0/7 |
| Collaboration temps réel multi-utilisateurs, versioning avec branches et fusions | Onshape ★ desktop absent | event sourcing natif (+CRDT), option très long terme |
| **Classification multi-référentiel des objets** : chaque élément porte des codes de plusieurs référentiels simultanément (OmniClass/Uniclass/UniFormat, normes mécaniques, bibliothèques maison) ; disponibilité des propriétés conditionnée par la classification | Archicad (Classification Manager, ISO 12006-3), Revit (OmniClass + Assembly Code) ★ libre quasi absent | forge-cad (métadonnées) + forge-values, long terme |
| Nomenclatures/BOM paramétriques : extraits multi-catégories (quantités, longueurs, matériaux) recalculés en direct, export tableur | Revit (schedules), Archicad (Interactive Schedules), Advance Steel/Tekla | transversal (tout modèle), jalon 10 |
| Recherche/sélection d'éléments par classification + surcharges graphiques par classe | Archicad | forge-cad, long terme |
| **Import IFC (maquettes BIM)** : pièces/sous-modèles réutilisés pour la fabrication (chaîne BIM → CAM : BOM, listes de coupe, fichiers CNC) | Revit/ArchiCAD (export IFC), openBIM ★ CAM libre absent | forge-io, long terme |
| Échange BCF (sujets/incidents ancrés sur le modèle) et IDS (exigences d'information lisibles machine) avec les outils commerciaux | buildingSMART (openBIM) | hors périmètre initial — noté pour l'interopérabilité |
| **Export DSTV/NC1** : pièces acier vers machines CNC quel que soit le fabricant (interface neutre standard) | Tekla, Advance Steel ; standard DSTV/NC1 | forge-io (format), lié à l'assistant acier (§11) |

## 10. Extensions de l'inventaire (v2 — ajout recherche)

### 10.1 Logiciels libres supplémentaires à ajouter aux références

| Logiciel | Domaine | Apport pour OmniFab |
|---|---|---|
| **Deepnest** | Nesting 2D (laser/plasma/CNC) | Fusion des lignes communes (une arête coupée une seule fois), part-in-part, nesting d'images bitmap pour gravure ; référence pour forge-cam |
| **SVGnest** | Nesting 2D (web, open source) | Algorithme bin-packing irrégulier itératif ; validation web de l'approche |
| **FlatCAM** | Prototypage PCB (usinage d'isolement) | Gerber/Excellon → G-code, double face, perçage ; cas d'usage « machine spécialisée » pour l'abstraction procédés |
| **pcb2gcode** (+ KiCad) | PCB → G-code | Chaîne 100 % libre de l'électronique à l'usinage ; entrées/format Gerber |
| **SolveSpace** | CAO paramétrique contrainte (GPL) | Deuxième implémentation libre du solveur par contraintes ; regard critique sur planegcs ; export G-code intégré |
| **OpenSCAD** | CAO par script | Modèle « CAO programmable » ; inspiration pour forge-values / scripting |
| **InkCut** | Contrôle traceurs/découpeurs vinyle 2D (HPGL/DMPL) | Machine 2D tangentielle mature ; protocoles traceurs pour forge-link |
| **CNCjs** | Interface machine web (GRBL/Smoothie/Marlin/TinyG) | Architecture web + multi-contrôleurs ; concurrent libre de la cible jalon 9 |
| **PrusaSlicer / Cura** | Découpage impression 3D (fraisage par dépôt) | Procédé « impression » dans la liste des procédés ; formats 3MF/gcode amp |
| **MeshLab** | Traitement/analyse de maillages | Réparation, simplification, mesure — complément forge-geom |
| **QCAD** | CAO 2D DXF (GPL) | Référence 2D technique éprouvée, bibliothèque DXF robuste (dxflib) |
| **Ink/Stitch** | Broderie (extension Inkscape) | Preuve qu'un procédé « exotique » se greffe proprement sur un socle 2D |
| **PyCAM** | CAM 3 axes libre | Héritage d'idées toolpath ; état de l'art libre limité |
| **mods (MIT CBA)** | Plateforme de fabrication modulaire web (libre, navigateur) | Graphe de modules à connecter : entrée PNG/SVG → calcul → machine ; pilote fraiseuses PCB (Roland SRM/MDX), découpeurs vinyle (Cameo, Roland GX), lasers, plasma, jet d'eau, fil chaud, EDM fil ; **implémentation de référence de l'approche nodale de forge-values** — un procédé = une chaîne de modules, pas un logiciel monolithique |
| **BOSL2** (bibliothèque OpenSCAD) | Composants paramétriques par code (libre) | Attachements, visserie normée avec logement d'écrou, tubes, jointures (screw/snap/dovetail) — le pendant libre des « assistants » : des générateurs paramétriques réutilisables |
| **WikiHouse** | Système constructif open source | Gabarits CNC en contreplaqué emboîtables type puzzle, plans librement téléchargeables : preuve qu'un système constructif complet peut être open source |

### 10.2 Inspirations commerciales (comportement uniquement, jamais le code — compatible GPL)

| Logiciel | Domaine | À s'inspirer |
|---|---|---|
| **Vectric VCarve/Aspire** | Gravure V, relief | Référence absolue du V-carving et des inlays : limites de profondeur, nettoyage, stratégies |
| **SheetCam** | Plasma/CAM 2,5D | Référence plasma : kerf, lead-in/out, cut charts, post-processeurs configurables, nesting intégré |
| **Fusion 360** | CAO+CFAO intégrée | Intégration A→Z dans un seul doc, tournage, 3+2 positionnel en base ; modèle d'arbre de fonctions moderne |
| **Mastercam / hyperMILL** | CFAO industrielle 5 axes | Plafond de l'ambition multi-axes simultané (long terme, éventuellement hors périmètre) |
| **Mach3/Mach4** | Contrôle machine Windows | UX pilote très répandue ; dialecte à supporter à l'import |
| **Ecam / CamBam** | Tournage / CAM 2,5D | Approches tournage abordables ; CamBam historiquement bien aimé du hobby |
| **Carbide Create / Carbide Motion** | CNC hobby intégrée | Simplicité extrême du parcours débutant (chaîne conception→usinage en 3 clics) |
| **EstlCAM** | CAM hobby | Onboarding débutant, pré-réglages machine ; très populaire dans la communauté MPCNC |
| **Slicer / 123D Make** (Autodesk, abandonné) | Fabrication par tranches | 5 techniques de construction (empilées, enchevêtrées, courbes, radiales, panneaux pliés), goupilles, plans d'assemblage numérotés — reprise du flambeau libre |
| **LightBurn** | Laser découpe/gravure (référence du marché) | Gestion par calques avec réglages par calque, calibration caméra + traçage depuis photo, print & cut, offsetting in/out — la référence UX laser |
| **MeshCAM** | CAM « sans CAO » (STL → usinage) | Ébauche contour/parallèle, finition unifiée, supports/tabs intégrés — parcours débutant sans apprendre la CAO |
| **Edgecam** | CFAO industrielle | Wire EDM (glue stops, slug removal, lead-off) — inspiration « procédé hors périmètre initial » |
| **Mastercam** (précisions) | CFAO industrielle | Dynamic Milling, Curve/Scallop finishing, Blade Expert (aubes), hybride steep/shallow avec limite d'angle, ordre by depth/closest |
| **SolidWorks** | CAO 3D industrielle (standard de facto) | Configurations + design tables, weldments avec cut lists, tôlerie, moulage, GD&T/MBD, écosystème complet (PDM, Simulation, Electrical, Inspection) — le « niveau pro » de la CAO paramétrique |
| **SolidWorks CAM / CAMWorks** | CFAO intégrée dans la CAO | AFR (reconnaissance de fonctions), KBM (base de règles d'entreprise), TBM (usinage piloté par les tolérances), fraisage indexé 4/5 axes, setup d'assemblage, mill-turn |
| **3DEXPERIENCE CATIA** | CAO plafond (aéro/auto) | Knowledgeware (règles, checks, réactions), FTA 3D, surfaces Class-A + rétro-ingénierie nuages (DSE/QSR), subdivision (Imagine & Shape), design génératif B-Rep éditable, DMU/cinématique |
| **Onshape** | CAO cloud natif | Versioning intégré (branches, fusions, historique complet), collaboration temps réel, FeatureScript (fonctions personnalisées au cœur même de la régénération paramétrique), matériel standard intégré |
| **Siemens NX** | CAO/CFAO industrielle (plafond) | Synchronous Modeling (édition sans historique), Adaptive Milling (-60 %), tube milling, Synchronization Manager mill-turn multi-canaux, modèles d'états intermédiaires |
| **Alibre Design** | CAO paramétrique abordable | Tôlerie : développé vérifiable à tout instant + export découpe laser direct ; preuve qu'un CAO pro peut rester simple et bon marché |
| **Autodesk Fusion** (précisions) | CAO+CFAO+CAE+ECAD cloud | Automated Drawings, AutoConstrain IA (esquisse contrainte automatiquement), design génératif multi-jeux de contraintes, PCB intégré (BRep 3D vivant), préparation impression FFF, Machine Connect |
| **PolyBoard** (Atelier Bois) | Menuiserie — LA référence assistant | Caissons/tiroirs/portes ajoutés en un clic, quincaillerie système 32 mm, liste de débit + plans + coûts + fichiers CNC générés automatiquement — le modèle même de l'assistant guidé |
| **StairDesigner** | Escaliers | Paramétrique, contrôle de conformité (loi de Blondel, hauteurs de marche), optimisation des limons, sorties débit/plans/CNC |
| **SketchList 3D** | Menuiserie | Bibliothèques de meubles réutilisables, débit optimisé, rendus 3D |
| **Cadwork / Dietrich's / sema** | Charpente & construction bois | Fermes paramétriques, bibliothèque d'assemblages traditionnels (tenon-mortaise, queues d'aronde, éparris), éléments toiture/plancher générés, lien CNC |
| **Advance Steel / Tekla Structures** | Structures acier (BIM) | Connexions paramétriques intelligentes, plans d'atelier automatiques, fichiers NC/DSTV pour machines de découpe/perçage |
| **MTpro / item** (outils gratuits) | Profilés aluminium | Configurateur de cadres en quelques étapes : profilés normés, connecteurs, calcul et documentation d'assemblage |
| **Autodesk Revit / Graphisoft Archicad** | BIM (bâtiment) | Systèmes de classification multi-référentiels (OmniClass/Uniclass/UniFormat, ISO 12006-3), nomenclatures/BOM paramétriques recalculées en direct, propriétés pilotées par la classification, écosystème openBIM (IFC, BCF, IDS) — le modèle à suivre pour classifier ET interopérer |

**Règle légale rappelée :** on s'inspire du comportement, des workflows et des concepts documentés publiquement — jamais du code, des algorithmiques propriétaires ni des assets. Aucun EULA de ces produits ne doit être accepté dans le cadre du projet. Une rétro-ingénierie d'interopérabilité de formats reste légale dans les limites du droit applicable, mais n'est pas nécessaire ici : tous les formats cibles sont documentés (G-code, DXF, STEP…) ou produits par des logiciels libres.

### 10.3 Conséquences pour les 4 zones « à spécifier » (phase 0)

1. **Gravure V** — F-Engrave prouve que le libre sait le faire ; ambition initiale réaliste : V-carving texte/contours + inlays simples (paramètres angle de pointe, profondeur max, nettoyage).
2. **Plasma** — LinuxCNC PlasmaC/QtPlasmaC couvre le contrôle (THC, cut charts) ; il manque un **CAM plasma libre** complet : OmniFab peut combler ce trou (kerf, lead-in/out, ordre intérieur/extérieur, cut charts matériaux).
3. **Tournage** — Ezilathe (libre, simple) et Ecam (commercial) montrent la voie : profil DXF → passes d'ébauche/finition, filetage, outils motorisés (CNCSimple). Ambition initiale : tournage 2 axes + filetage, outils motorisés ensuite.
4. **5 axes** — quasi vide en libre (CNC Toolkit ancien, LinuxCNC = machine pas CAM ; côté commercial ~1200-24000 $/an). Trajectory : **3+2 positionnel d'abord** (renkinématique simple, réutilise les opérations 3 axes), simultané en option très long terme.

## 11. Assistants de conception guidée (v6 — ajout)

> **Concept :** un assistant = un formulaire guidé pas à pas (type d'objet → dimensions → matériau → options) qui génère automatiquement un **modèle paramétrique éditable**, une nomenclature, une liste de débit et des fichiers CNC. Chaque assistant est un **template déclaratif** (données + règles + valeurs préenregistrées), pas du code figé : il réutilise le socle OmniFab — forge-cad (arbre de fonctions), forge-values (paramètres, pré-réglages normés, checks), forge-cam (trajectoires), forge-io (sorties).

### 11.1 Système générique (le cœur de l'idée)

| Fonctionnalité | Présent dans | Cible OmniFab |
|---|---|---|
| Formulaire pas-à-pas → modèle 3D + plans + débit + CNC en un clic | PolyBoard, StairDesigner, SketchList 3D | forge-cad (wizards) + forge-cam, jalon 10 (avec les bibliothèques normées) |
| Valeurs préenregistrées par secteur et par norme : épaisseurs de panneaux standard, quincaillerie système 32 mm, sections de bois normalisées, pentes de toit, pas de marche (formule de Blondel), profils acier/alu normés | PolyBoard, StairDesigner (Blondel), Cadwork, Advance Steel | forge-values (bibliothèques de pré-réglages normés), jalon 10 |
| Contrôles de conformité intégrés (hauteur de marche, garde-corps, dépassements, limites dimensionnelles) | StairDesigner ★ rare | forge-values (checks), jalon 10 |
| Bibliothèques d'assemblages paramétriques : tenon-mortaise, queue d'aronde, épar, platines boulonnées, raidisseurs, connecteurs alu | Cadwork, BOSL2 (joiners), Advance Steel | forge-cad, long terme |
| Sorties d'assistant : modèle éditable, liste de débit optimisée, plans par pièce, coût estimé, 3D de présentation, fichiers CNC | PolyBoard (débit+plans+coûts+CNC), StairDesigner, WikiHouse (gabarits) | chaîne native OmniFab A→Z |
| **Éditeur d'assistants : création de ses propres guides** (template déclaratif + règles + pré-réglages, partageable, sans coder) | ★ aucun — chaque éditeur fige ses assistants | forge-values + forge-cam, jalon 10 ★ novateur |
| Assistant en tant que données : versionné, publiable dans les bibliothèques (comme un fichier) | ★ aucun | event sourcing + jalon 10 |

### 11.2 Assistants par domaine (liste extensible)

| Domaine | Contenu type de l'assistant | Références |
|---|---|---|
| Meubles / caissons / cuisines | caissons, tiroirs, portes, plans de travail, plinthes, fileurs, quincaillerie 32 mm, chants | PolyBoard, SketchList 3D |
| Escaliers | marches, contremarches, limons/crémaillères, poteaux, mains courantes, balustres ; contrôle Blondel | StairDesigner |
| Charpente bois / ossature | fermes paramétriques, assemblages traditionnels (tenon-mortaise, queue d'aronde, épar), chevrons, arêtiers, plans de charpente | Cadwork, Dietrich's, sema |
| Structures acier | profils, platines, raidisseurs, assemblages boulonnés/soudés, plans d'atelier, fichiers NC/DSTV | Advance Steel, Tekla Structures |
| Cadres en tubes / profilés alu | profilés normés, connecteurs, perçages, listes de coupe, documentation d'assemblage | MTpro (Bosch Rexroth), item |
| Tuyauterie / garde-corps tubes | parcours de tubes, coudes et raccords standards | SolidWorks Routing |
| Systèmes constructifs ouverts | gabarits CNC en contreplaqué emboîtables (type puzzle), pièces numérotées | WikiHouse ★ libre |

---

## Bilan

- **≈ 265 fonctionnalités** cataloguées (v7), réparties en 10 domaines + section 10 (extensions) — enrichies par **8 cycles de recherche** (~55 logiciels). Le cycle 8 ajoute la **classification multi-référentielle des objets** (à la Archicad/Revit : OmniClass, Uniclass, UniFormat, ISO 12006-3), les **nomenclatures/BOM paramétriques** recalculées en direct, et l'interopérabilité openBIM (import IFC BIM→CAM, échange BCF/IDS, export DSTV/NC1 acier).
- **≈ 50 lignes ★** (fonction absente ou quasi absente du libre — la proposition de valeur unique d'OmniFab) : aide contextuelle universelle, multi-machines parallèle, alerte pédagogique des risques, assistant de dialectes, éditeur G-code complet, liaisons nodales CAO→CAM, DNC RS-232 libre, pilotage LPT moderne, fabrication par tranches « 123D Make », documentation développeur exigeante ; ajouts v3 : simulation de linkages (SolveSpace), digitizer (bCNC), profondeur par IA (UGS), job sheet, fluting/texture ; ajouts v6 : chaîne CAM nodale (mods), **éditeur d'assistants personnalisés** ; ajouts v7 : **classification multi-référentielle + chaîne BIM→CAM libre**.
- ~~4 lignes « à spécifier »~~ → **résolu en v2** : gravure V (F-Engrave/Vectric), plasma (PlasmaC/SheetCam — trou libre identifiable), tournage (Ezilathe/Ecam), 5 axes (3+2 positionnel d'abord). Niveaux d'ambition initiale proposés §10.3, à valider en phase 0.
- **16 logiciels libres supplémentaires** identifiés (§10.1) dont Deepnest, FlatCAM, SolveSpace, InkCut, QCAD, CNCjs, mods (MIT CBA), BOSL2, WikiHouse ; **27 entrées d'inspirations commerciales** (§10.2, comportement uniquement) dont SolidWorks, 3DEXPERIENCE CATIA, Onshape, Siemens NX, les spécialistes par construction (PolyBoard, StairDesigner, Cadwork, Advance Steel/Tekla, MTpro) et le monde BIM (Revit/ArchiCAD).
- **Prochaine étape (phase 0)** : hiérarchiser par jalon, arbitrer le wire EDM (hors périmètre initial pour l'instant), valider les niveaux d'ambition (fluting/texture = option ; 5 axes = 3+2 positionnel d'abord ; broderie et impression 3D = procédés « exotiques » validant l'abstraction), et coucher dans l'architecture le **système d'assistants (§11)** et la **classification multi-référentielle (§9)** — deux systèmes de données qui s'appuient sur forge-values et le jalon 10.