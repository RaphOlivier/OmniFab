<!-- English translation (headers only) of docs/matrice-fonctionnalites.md — the French matrix is authoritative (see banner below). -->

# Feature × reference-software matrix — OmniFab

> **English translation — headers only.** The French version is authoritative: [`docs/matrice-fonctionnalites.md`](../matrice-fonctionnalites.md). Only this preamble, the section titles and the summary are translated; the ~277-row inventory itself is not (yet) translated. Translated as of 2026-10-26 (commit `403212f`) — the French original is updated continuously and may be ahead. In case of divergence, the French version prevails.

> **Role of this document:** an anti-omission guardrail and a long-term specification (the exhaustive inventory required in phase 0). It is NOT an immediate task list — every row is a capability that OmniFab's architecture must be able to host one day. Duplicates across software are merged; functions absent from all reference software are marked **★ novel** (OmniFab would go beyond).
>
> **Columns:** "Present in" = which reference software offer the function; "Target" = where it lands in OmniFab (crate / milestone).
>
> **Status:** v8 — enriched through 9 research cycles (~60 software tools documented). v6 additions: guided design assistants (§11). v7 additions: multi-referential object classification (BIM/OmniClass/Uniclass), parametric parts lists/BOM, openBIM interoperability (IFC, BCF, IDS, DSTV/NC1). v8 additions (2026-10-24): libraries track (automatic controller detection, "open reference" libraries on the OSM model, generated geometry of standard parts, glTF LODs) + ArtCraft/FullControl track (direct toolpath design, AI-agent control via MCP/CLI/JSON, live vector effects, PSD/PSB and XLSX round-trip, paper space/viewports, DWG reading, calibration specimens, parametric web generation). Still to be prioritized by milestone during phase 0.

## Sections

| § | Section (translated title) | French original |
|---|---|---|
| 1 | 2D vector drawing — ref. Inkscape, Graphite, FreeCAD (Sketcher) | [Dessin vectoriel 2D](../matrice-fonctionnalites.md#1-dessin-vectoriel-2d--réf-inkscape-graphite-freecad-sketcher) |
| 2 | Raster / painting — ref. GIMP, Krita, GMIC | [Raster / painting](../matrice-fonctionnalites.md#2-raster--painting--réf-gimp-krita-gmic) |
| 3 | Parametric CAD — ref. FreeCAD | [CAO paramétrique](../matrice-fonctionnalites.md#3-cao-paramétrique--réf-freecad) |
| 4 | 3D mesh — ref. Blender (via FabexCNC) | [Mesh 3D](../matrice-fonctionnalites.md#4-mesh-3d--réf-blender-via-fabexcnc) |
| 5 | CAM / toolpaths — ref. FabexCNC (BlenderCAM), KiriMoto, FreeCAD Path | [CAM / trajectoires](../matrice-fonctionnalites.md#5-cam--trajectoires--réf-fabexcnc-blendercam-kirimoto-freecad-path) |
| 6 | G-code simulation — ref. Camotics | [Simulation G-code](../matrice-fonctionnalites.md#6-simulation-g-code--réf-camotics) |
| 7 | Sending & machine control — ref. bCNC, UGS, gSender, LaserGRBL, LaserWeb | [Envoi & contrôle machine](../matrice-fonctionnalites.md#7-envoi--contrôle-machine--réf-bcnc-ugs-gsender-lasergrbl-laserweb) |
| 8 | Firmware / controllers — ref. GRBL, FluidNC, Marlin, LinuxCNC | [Firmware / contrôleurs](../matrice-fonctionnalites.md#8-firmware--contrôleurs--réf-grbl-fluidnc-marlin-linuxcnc) |
| 9 | Cross-cutting | [Transversal](../matrice-fonctionnalites.md#9-transversal) |
| 10 | Inventory extensions (v2 — research addition) | [Extensions de l'inventaire](../matrice-fonctionnalites.md#10-extensions-de-linventaire-v2--ajout-recherche) |
| 11 | Guided design assistants (v6 — addition) | [Assistants de conception guidée](../matrice-fonctionnalites.md#11-assistants-de-conception-guidée-v6--ajout) |
| — | Summary | [Bilan](../matrice-fonctionnalites.md#bilan) |

## Summary

- **≈ 277 features** catalogued (v8), across 10 domains + section 10 (extensions) — enriched through **9 research cycles** (~60 software tools). Cycle 8 adds **multi-referential object classification** (à la Archicad/Revit: OmniClass, Uniclass, UniFormat, ISO 12006-3), live-recalculated **parametric parts lists/BOM**, and openBIM interoperability (IFC import BIM→CAM, BCF/IDS exchange, DSTV/NC1 steel export). Cycle 9 merges the 2026-10-19 "libraries" track (automatic controller detection, "open reference" libraries on the OSM model, generated geometry of standard parts, glTF LODs) and the 2026-10-24 ArtCraft/FullControl track (direct toolpath design, AI-agent control, live vector effects, PSD/PSB and XLSX round-trip, paper space/viewports, DWG reading, calibration specimens, parametric web generation).
- **≈ 59 ★ rows** (functions absent or nearly absent from free software — OmniFab's unique value proposition): universal contextual help, parallel multi-machine operation, educational risk warning, dialect assistant, full G-code editor, nodal CAD→CAM bindings, free RS-232 DNC, modern LPT control, slice-based manufacturing "123D Make", demanding developer documentation; v3 additions: linkage simulation (SolveSpace), digitizer (bCNC), AI-based depth (UGS), job sheet, fluting/texture; v6 additions: nodal CAM chain (mods), **custom assistant editor**; v7 additions: **multi-referential classification + free BIM→CAM chain**; v8 additions: **direct toolpath design (FullControl)**, **"open reference" libraries + 3-tier architecture**, **automatic controller detection (zero-config)**, live vector effects, paper space/viewports, PSD/PSB round-trip, AI-agent control.
- ~~4 "to be specified" rows~~ → **resolved in v2**: V-engraving (F-Engrave/Vectric), plasma (PlasmaC/SheetCam — identifiable free gap), turning (Ezilathe/Ecam), 5 axes (3+2 positional first). Initial ambition levels proposed in §10.3, to be validated in phase 0.
- **22 additional free software tools** identified (§10.1), including Deepnest, FlatCAM, SolveSpace, InkCut, QCAD, CNCjs, mods (MIT CBA), BOSL2, WikiHouse, FullControl and the ArtCraft suite (VectorCraft, CADCraft, PhotoCraft, GridCraft); **27 commercial inspiration entries** (§10.2, behavior only), including SolidWorks, 3DEXPERIENCE CATIA, Onshape, Siemens NX, the construction specialists (PolyBoard, StairDesigner, Cadwork, Advance Steel/Tekla, MTpro) and the BIM world (Revit/ArchiCAD).
- **Next step (phase 0)**: prioritize by milestone, arbitrate wire EDM (outside the initial scope for now), validate ambition levels (fluting/texture = option; 5 axes = 3+2 positional first; embroidery and 3D printing = "exotic" processes validating the abstraction), and lay out in the architecture the **assistant system (§11)** and the **multi-referential classification (§9)** — two data systems built on forge-values and milestone 10.
