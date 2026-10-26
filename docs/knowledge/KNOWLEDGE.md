---
name: omnifab
display_name: "OmniFab"
description: "Projet OmniFab : logiciel libre en Rust (egui, wgpu, OpenCascade) de fabrication A→Z — CAO, CAM, G-code, pilotage machine. Stack, décisions et index des fiches du projet."
---

# OmniFab

Logiciel libre polyvalent qui remplace de A à Z les programmes nécessaires pour concevoir et fabriquer des objets : graphique 2D, CAO paramétrique, CAM multi-procédés, éditeur G-code, pilotage machine (multi-machines), painting raster, plugins.

**Stack validée :** Rust, egui (UI, plan B Slint sur mobile), wgpu (GPU), OpenCascade via bindings (CAO), planegcs (contraintes 2D), tokio (machine), GPL-3.0+.
**Plateformes :** Linux, Windows, Android, web (WASM) — un noyau unique, frontends multiples.
**Priorité de travail :** tranche horizontale de A à Z (cas d'école : fabriquer un meuble — CAO paramétrique → découpe/gravure 2D + raster → CAM 2,5D → LinuxCNC), puis élargissement. Premier assistant de conception à prototyper : **meuble/caisson** (référence PolyBoard), aligné sur le cas d'école. Tests machine : simulation d'abord, aucune machine physique au départ.

## Fiches du projet

- [État actuel](etat-actuel/KNOWLEDGE.md) — **relevé d'une page, mis à jour à chaque lot** : ce qui compile, état de la CI, dette ouverte. **À lire en premier** pour situer le projet.
- [Architecture](architecture/KNOWLEDGE.md) — structure des crates, six verrous techniques, roadmap en 11 jalons (gel post-jalon 4 en vigueur), règles plateformes/web.
- Références Sketcher/solveur (`references/KNOWLEDGE.md`, plateforme seulement) — documentation FreeCAD/planegcs intégrée (PDF solveur Abdullah Tahiri, pages wiki Snap/Move/Preferences/Workbench) : cadre connu pour l'application des contraintes, le drag et l'aimantation. *(non publié au miroir : docs externes, licences — ADR-0028)*
- [Conventions](conventions/KNOWLEDGE.md) — unités strictes, licences, i18n, concurrence, gouvernance open source.
- [Formats](formats/KNOWLEDGE.md) — matrice d'interopérabilité par domaine (SVG, STEP, glTF, G-code…).
- [Machines](machines/KNOWLEDGE.md) — contrôleurs (LinuxCNC prioritaire), multi-machines, machines maison, éditeur G-code.
- [Sécurité](securite/KNOWLEDGE.md) — modèle de menace (fichiers importés, réseau, machine physique, supply chain, plugins) et règles de sécurité par jalon ; principe : sécurité par défaut sans limiter fonctions ni performances, arbitrage explicite par ADR. Chaque règle porte son statut (en place / rédigé / prévu) et sa date.
- Économie de tokens (`economie-tokens/KNOWLEDGE.md`, plateforme seulement) — règles d'économie de contexte : prompt fondateur via la Knowledge, fiches ciblées, découpage des discussions, Vibe Code chirurgical. *(non publié au miroir : processus de travail avec l'assistant — ADR-0028)*
- Flux de travail mobile/sans PC (`flux-travail-mobile.md`, plateforme seulement) — tâches exigeant le PC (identifiables à l'avance), file de tâches sans PC, regroupement des validations manuelles en checkpoints, galerie Xvfb élargie. *(non publié au miroir — ADR-0028)*
- [Historique & versionnage](historique/KNOWLEDGE.md) — protocole de versionnage : n° de version + changelog sur chaque fiche, journal append-only des évolutions par fonction ([journal-fonctions.md](historique/journal-fonctions.md), consultation par grep sur l'ID) ; **toute modification d'une fiche incrémente sa version et ajoute une ligne à son Historique ; toute évolution d'une fonction ajoute une ligne au journal**.
- [Archives des discussions](archives/KNOWLEDGE.md) — résumés datés des discussions significatives (raisonnement et argumentaires, complément des fiches de décisions) ; à alimenter en fin de discussion.
- Prompt fondateur (`prompt-fondateur.md`, plateforme seulement) — vision 15 points + annexes, source de vérité du projet. Ne pas coller en entier dans une discussion (voir Économie de tokens) ; à relire uniquement pour les tâches touchant la vision globale ou lors des re-synchronisations canvas ↔ fichier. *(non publié dans le miroir knowledge : copie de référence au dépôt, `docs/prompt-fondateur.md` — ADR-0028)*

## Documents du projet

- **Matrice fonctionnalité × logiciel** — **v8** (2026-10-24), ≈ 277 fonctions, 9 cycles de recherche (~60 logiciels libres et commerciaux) ; §10 extensions (22 logiciels libres), §11 assistants de conception guidée. **Copie de référence : le dépôt GitHub fait foi — `docs/matrice-fonctionnalites.md` (commit 1368a70)** ; le canvas dédié a été **resynchronisé en v8 le 2026-10-24** dans sa discussion hôte. La directive d'intégration v7→v8 a été exécutée le 2026-10-24 (fiche directive supprimée après confirmation, l'analyse étant intégralement reportée dans la matrice v8).
- **Prompt fondateur** (canvas « atelier-universel-prompt ») — version vivante (vision 15 points + annexes) ; le fichier `prompt-fondateur.md` ci-dessous en est la copie de référence synchronisée (**v4, 2026-10-26** : DWG en lecture + PSD/PSB round-trip + broderie DST/JEF/PES + listes de logiciels harmonisées sur cette racine). Le dépôt contient `docs/prompt-fondateur.md` (document complet : partie A + annexes B1–B7, **resynchronisé en v4 le 2026-10-26, commit 3aa78a7**). Le canvas a été resynchronisé en v3 le 2026-10-24 — **dette ouverte : resynchroniser en v4 dans sa discussion hôte**. Resynchroniser ces trois exemplaires à chaque évolution.
- Référentiel des canvases (`canvases-referentiel.md`, plateforme seulement) — inventaire des canvases qui font foi, statut de synchronisation vers la Knowledge (copies de référence), protocole de resynchronisation et dettes en attente ; à consulter avant de chercher un canvas ou d'en créer un qui fait foi. Rappel : les canvases ne sont accessibles que dans leur discussion hôte, seule la Knowledge suit toutes les discussions. *(non publié au miroir — ADR-0028)*

## Logiciels de référence

**Noyau :** FreeCAD, Blender, Inkscape, Krita, GIMP, GMIC, Graphite, LinuxCNC (PlasmaC/QtPlasmaC), Camotics, bCNC, KiriMoto, LaserGRBL, Universal G-Code Sender, FluidNC, gSender, LaserWeb, FabexCNC, BlenderCAM, F-Engrave, Ezilathe. **Élargis (matrice §10.1, v8) :** Deepnest, SVGnest, FlatCAM, pcb2gcode, SolveSpace, OpenSCAD, InkCut, CNCjs, PrusaSlicer/Cura, MeshLab, QCAD, Ink/Stitch, PyCAM, mods (MIT CBA), BOSL2, WikiHouse, **FullControl** (design direct de trajectoires en G-code, GPL-3.0), **suite ArtCraft** (VectorCraft, CADCraft, PhotoCraft, GridCraft — libres en Rust natif, principes : agent-ready MCP/CLI/JSON, local-first, multi-backend GPU). **Inspirations commerciales (comportement uniquement, matrice §10.2) :** Vectric, SheetCam, Fusion 360, Mastercam, SolidWorks/SolidWorks CAM, 3DEXPERIENCE CATIA, Siemens NX, Onshape, Alibre, LightBurn, MeshCAM, Edgecam, PolyBoard, StairDesigner, SketchList, Cadwork/Dietrich's/sema, Advance Steel/Tekla, MTpro, Revit/ArchiCAD (classification BIM + BOM).

Exigence : inventaire exhaustif « fonctionnalité × logiciel » en phase 0 (garde-fou anti-oubli, pas liste de tâches immédiate) — réalisé en 9 cycles de recherche ; la matrice fait foi (v8, dépôt : `docs/matrice-fonctionnalites.md`).
