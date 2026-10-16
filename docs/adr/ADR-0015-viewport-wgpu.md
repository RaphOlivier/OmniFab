# ADR-0015 — Phase C : viewport wgpu (crate forge-render)

- **Statut : ACCEPTÉ** (validé par Raphaël Olivier le 2026-10-15, tel quel)
- **Date : 2026-10-15**
- **Liens : ADR-0012 (découpage du jalon 2, phase C) ; ADR-0014 (phase B quick wins, appliqué et clos)**

## 1. Contexte

Le démonstrateur (jalon 2, phase B) dessine les tracés 2D et le wireframe STL avec le **painter egui** : toute la géométrie est re-triangulée côté CPU à chaque frame. Limites actées :

1. Plafond provisoire de 20 000 triangles (réglage du démonstrateur, amendement 2 de l'ADR-0014).
2. Pas de caméra : ni zoom, ni pan, ni orbite.
3. Pas de rendu 3D réel, alors que la roadmap vise la CAO 3D, les toolpaths et la simulation.
4. Le painter ne passe pas à l'échelle (maillages, toolpaths CAM) : c'est un plafond structurel, pas un réglage.

## 2. Décision

### 2.1 Crate `forge-render` (MPL-2.0)

Nouvelle crate du workspace : **cœur sans dépendance egui** (scène, caméras, buffers, pipelines wgpu, shaders WGSL). L'adaptateur egui (paint callback) vit dans la même crate derrière une feature `egui` — pas de crate séparée tant que la duplication n'existe pas (même règle que le report d'omnifab-ui, ADR-0010).

Dépendances autorisées : `wgpu`, `forge-geom` (Point2/Path2/Mesh), `glam` (déjà au workspace). Interdites : `forge-doc`, `forge-session` — la scène est alimentée par l'application, qui seule connaît la session (frontière noyau/UI préservée, principe directeur).

### 2.2 Bascule du desktop sur le renderer wgpu d'eframe

`omnifab-desktop` ajoute la feature `wgpu` d'eframe (glow conservé : les deux renderers coexistent, wgpu devient celui du desktop). C'est le seul chemin propre pour dessiner du wgpu **dans** la fenêtre egui : `egui::PaintCallback` + `egui_wgpu::CallbackTrait` (le callback reçoit le `CommandEncoder` et la texture cible). **Une seule version de wgpu dans le graphe : celle dictée par egui-wgpu** — jamais de version supplémentaire choisie librement.

`omnifab-android` est **inchangée** (renderer glow, eframe 0.29) : crates séparées, le retest Android sur eframe ≥ 0.31 reste la veille non bloquante déjà actée au jalon 0.

### 2.3 Deux étapes mesurables

- **C1 — pipeline 2D** : tracés (SVG/DXF) + grille de fond dessinés par wgpu ; caméra 2D (molette = zoom centré sur le curseur, glisser = pan) ; parité visuelle avec le painter actuel (mêmes couleurs, mêmes largeurs de trait).
- **C2 — wireframe 3D** : le wireframe STL migre du painter vers wgpu (lignes indexées, caméra orbite, projection perspective) ; **suppression du plafond 20 000 triangles** et du repli boîte englobante ; le message i18n de plafond est retiré.

### 2.4 Règles transversales

- **Unités** : newtypes forge-units jusqu'à la frontière du viewport ; conversion mm → coordonnées monde au dernier moment ; f32 au GPU (leçon egui f32 du jalon 1).
- **Wasm-friendly** : wgpu est la couche rendu de la cible web (jalon 9) ; aucun accès natif hors abstraction.
- **Shaders WGSL uniquement** (pas de GLSL) — un seul langage multi-OS.
- **Échec d'initialisation wgpu** : message i18n dans le viewport, **pas de repli painter** (complexité non justifiée) ; pas de nouveau code d'erreur en C1/C2 (une panne GPU est un état d'affichage, pas une erreur métier).
- **Aucune modification de ci.yml** ni du mécanisme d'issues CI.

## 3. Alternatives rejetées

| Alternative | Rejet |
|---|---|
| Garder le painter egui (statu quo) | Ne passe pas à l'échelle, pas de 3D, le plafond resterait |
| Deux surfaces (wgpu hors egui) | Conflits de présentation, fragile multi-OS, complexité maximale |
| Rendu en texture puis affichage egui | Copie complète par frame, pas de depth buffer, perf médiocre |
| vello | Réservé à forge-paint (raster 2D vectoriel, jalon 6) — pas un viewport 3D |
| Boucle winit maison | Abandon de l'UI egui existante (aide, i18n, widgets, bibliothèque) |

## 4. Critères de sortie (phase C close si tous tenus)

1. **Parité 2D** : tracés SVG/DXF affichés identiques au painter actuel (couleurs, largeurs), grille en plus.
2. **Caméra fluide** : zoom/pan sans à-coup perceptible sur les builds Linux et Windows de Raphaël (validation manuelle — la fluidité n'est pas testable en CI).
3. **3D sans plafond** : un STL de plus de 20 000 triangles s'affiche intégralement, sans repli.
4. **CI verte multi-OS** (fmt, clippy -D warnings, tests, rustdoc, --locked), aucune régression jalon 1 / phase B (undo/redo, bibliothèque, sélecteurs intacts).
5. **Capitalisation** : fiche architecture mise à jour, leçons CI reportées, fiches prep marquées OBSOLÈTES.

## 5. Risques et parades

- **Arbre de dépendances wgpu** (volumineux) : Cargo.lock à régénérer par Raphaël sur son laptop, procédure habituelle — feature ajoutée puis `cargo check -p omnifab-desktop`, commit du Cargo.toml + Cargo.lock, CI verte car wgpu non encore utilisé.
- **Temps de build CI** : wgpu + shaders → plafond de build à surveiller au premier ride (référence phase A : ~+7-8 min/job maximum toléré).
- **Versions couplées** : wgpu dicté par egui-wgpu 0.29 ; pas de montée isolée.
- **Backends** : Vulkan (Linux) / Metal (macOS) / DX12 (Windows) déjà couverts par la matrice CI des 3 OS.

## 6. Séquence d'exécution (après validation)

1. **Raphaël** : ajouter la feature `wgpu` `eframe` dans `apps/omnifab-desktop/Cargo.toml`, puis `cargo check -p omnifab-desktop`, puis commit + push (Cargo.lock à jour, CI verte : wgpu non encore utilisé). Commandes exactes fournies au moment venu.
2. **Vibe** : batch C1 (crate `forge-render` + intégration desktop + clés FTL et pages d'aide caméra), un seul commit, convergence CI.
3. **Validation manuelle C1** (parité + caméra fluide), puis batch C2 (wireframe 3D, suppression du plafond), convergence CI.
4. **Validation manuelle finale** → phase C close, capitalisation.
