# ADR-0027 — Desktop du jalon 3 : espace CAO, arbre de projet, menu Fichier, rendu 3D riche (phase E)

- **Statut : ACCEPTÉ** — lot ADR-0022..0027 validé par Raphaël Olivier le 2026-10-25.
- **Jalon : 3** — CAO paramétrique + éditeur nodal de valeurs.
- **Source :** préparation du 2026-10-22, digest de session du 2026-10-25 (4 axes tranchés, validation du lot).

**Objet : la vitrine du jalon — le checkpoint visuel « CAO lisible » (~mi-décembre) : un solide extrudé d'un croquis contraint, ombré, dans le viewport 3D, avec son arbre de construction.**

**Décisions** :
- **D1 — Espace de travail « CAO »** (interface fondateur : espaces commutables type Blender) : l'atelier actuel devient l'espace CAO — esquisse 2D, arbre de projet, rendu 3D cohabitent ; la commutation Vue 2D/3D reste. L'arbre de projet (panneau latéral) liste croquis, features et imports dans l'ordre du journal : sélection/surbrillance, double-clic = réédition (panneau de la feature : mode/profondeur ; le panneau liaisons suit l'alias), clic droit = aide + suppression. Les icônes/espaces 2D/Paint/CAM/Machine restent des entrées futures désactivées (pas de fausse promesse).
- **D2 — Menu « Fichier »** (décision Raphaël du 2026-10-21) : le bouton d'outils devient un menu « Fichier » (Nouveau/Ouvrir/Enregistrer/Enregistrer sous), l'accueil garde « Nouveau projet » seul ; aides complètes.
- **D3 — Rendu 3D riche (digest du 2026-10-25 : depth_stencil + ombrage directionnel simple + MSAA 4×)** (point d'architecture n°7 + demande du 2026-10-17) : **depth_stencil** (fin des ordres de dessin manuels 3D), **ombrage simple** (normales de tessellation + une lumière directionnelle + composante ambiante), **MSAA 4×**, tampons de sommets **réutilisés entre frames** (audit : la recréation par frame actuelle part), clé d'édition = ne re-tesselle QUE si le solide change ; matériaux simples par feature (couleur + composante spéculaire, réglés par feature `couleur` — paramètre UI, pas journalisé) ; le filaire reste superposable en option (existant). Pipeline TriangleList 3D dédié sur le moteur forge-render/wgpu existant, adaptateur egui commun (même moteur, un pipeline de plus — ADR-0015).
- **D4 — Export du croquis** (suspens ADR-0018 levé) : SVG/DXF des positions RÉSOLUES du croquis sélectionné (un rendu fichier, jamais du journal) via forge-io — la chaîne meuble a besoin des découpes en DXF dès maintenant.
- **D5 — Suppressions visibles** : les commandes `*.removed` (ADR-0024) branchées à l'arbre et au clic sur entité (touche Suppr), confirmation i18n, undo intact.

**Alternatives rejetées** : moteur de rendu séparé du forge-render (ADR-0015) ; textures/photorealisme (hors périmètre, jalon 6+ ; l'ombrage simple suffit au « CAO lisible ») ; re-tessellation par frame (le problème de l'audit) ; export du croquis au journal (un export n'est pas une intention de modification — hors journal, comme les exports existants).

**Critères de sortie** : le scénario de démonstration du checkpoint tenu — croquis contrait carré + cercle, extrusion, poche soustractive pilotée par une expression du panneau de liaisons, changement d'alias → solide régénéré ombré sans artefact d'ordre ; arbre complet avec suppression/réédition ; export DXF du croquis re-importable ; 60 fps sur le laptop IdeaPad (mesure rapide, pas un bench).
