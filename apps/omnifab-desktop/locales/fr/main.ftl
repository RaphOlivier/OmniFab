# Démonstrateur omnifab-desktop — français, langue de référence.
# Toute l'interface vit ici (ADR-0010) : aucune chaîne d'interface dans
# le code Rust. Chaque widget interactif porte un help_id : sa page
# d'aide est le message du même nom, avec les attributs
# titre / description / exemple.

# --- Fenêtre ---

titre-fenetre = OmniFab — démonstrateur jalon 1

# --- Barre du haut ---

top-annuler = Annuler
top-retablir = Rétablir
top-recalculer = Recalculer tout
top-enregistrer = Enregistrer
top-ouvrir = Ouvrir
top-nouveau = Nouveau

# --- Import / export (chemins de démonstration, ADR-0011) ---

top-importer-svg = Importer SVG
top-importer-dxf = Importer DXF
top-importer-stl = Importer STL
top-exporter-svg = Exporter SVG
top-exporter-dxf = Exporter DXF
top-exporter-stl = Exporter STL

# --- Panneau des paramètres ---

params-titre = Paramètres
params-largeur = Largeur
params-hauteur = Hauteur
params-expression = Expression de la diagonale :
params-diagonale = Diagonale : {$valeur}

# --- Journal des événements ---

journal-titre = Journal d'événements
journal-compteur = {$n} événements · conteneur ZIP · JSON Lines
journal-types-ignores = Types ignorés (journal d'une version plus récente ?) :

# --- Viewport ---

viewport-titre = Viewport — rectangle de démonstration et tracés importés
viewport-erreur = Erreur : {$texte}

# --- Compteurs de géométrie importée ---

geom-traces-compteur = {$traces} tracés · {$sommets} sommets importés
geom-maillage-compteur = Maillage importé : {$triangles} triangles

# --- Messages d'information ---

info-recalcul-ok = Toutes les valeurs sont à jour.
info-enregistre = Projet enregistré dans « {$chemin} ».
info-recharge = Projet rechargé : {$n} événements rejoués.
info-import-traces = Import réussi : {$traces} tracés · {$sommets} sommets.
info-import-maillage = Import réussi : {$triangles} triangles.
info-export = Exporté dans « {$chemin} ».

# --- Fenêtre d'aide ---

aide-titre-fenetre = Aide — {$titre}
aide-section-exemple = Exemple :
aide-page-introuvable = Aide introuvable pour « {$identifiant} ».

# --- Pages d'aide (help_id : message sans valeur, attributs
#     titre / description / exemple — ADR-0010) ---

aide-top-annuler =
    .titre = Annuler
    .description = Annule le dernier groupe d'actions (saisie, expression ou import) : l'état revient à ce qu'il était juste avant. Les actions annulées restent disponibles pour « Rétablir » jusqu'à la prochaine nouvelle action.
    .exemple = Importez demo-import.svg puis cliquez : les tracés disparaissent du viewport. Le raccourci Ctrl+Z fait la même chose.

aide-top-retablir =
    .titre = Rétablir
    .description = Rétablit le dernier groupe d'actions annulé. Une nouvelle action après une annulation abandonne définitivement les actions défaites.
    .exemple = Après « Annuler », cliquez ou pressez Ctrl+Y : les tracés importés reviennent dans le viewport.

aide-top-importer-svg =
    .titre = Importer SVG
    .description = Lit demo-import.svg dans le répertoire courant et le convertit en tracés 2D sous budgets de sécurité (une unité SVG = 1 mm). Les octets d'origine partent dans un blob du projet, et l'import est une action annulable : une transaction au journal.
    .exemple = Créez demo-import.svg avec un chemin M 0 0 L 100 0 L 0 100 Z, cliquez : le viewport dessine le triangle en jaune et affiche les compteurs.

aide-top-importer-dxf =
    .titre = Importer DXF
    .description = Lit demo-import.dxf dans le répertoire courant (entités LINE, LWPOLYLINE, POLYLINE, CIRCLE et ARC ; une unité = 1 mm) et en fait les tracés courants — une action annulable, comme l'import SVG.
    .exemple = Créez demo-import.dxf contenant une polyligne fermée, cliquez : la forme apparaît en jaune et le compteur affiche tracés et sommets.

aide-top-importer-stl =
    .titre = Importer STL
    .description = Lit demo-import.stl (binaire ou ASCII, lu en millimètres) et en fait le maillage courant : octets d'origine dans un blob, compteurs au journal. Le maillage n'est pas dessiné au jalon 1 — le rendu arrive au jalon 2.
    .exemple = Importez un petit STL, cliquez : le compteur « Maillage importé » affiche le nombre de triangles sous le viewport.

aide-top-exporter-svg =
    .titre = Exporter SVG
    .description = Écrit les tracés courants dans demo-export.svg : les octets d'origine sont relus depuis leur blob puis re-sérialisés (une unité = 1 mm). L'export ne change pas l'état du projet : rien au journal.
    .exemple = Après un import, cliquez : le fichier demo-export.svg apparaît dans le répertoire courant.

aide-top-exporter-dxf =
    .titre = Exporter DXF
    .description = Écrit les tracés courants dans demo-export.dxf, un segment par entité LINE (les tracés fermés sont refermés). Comme tout export, l'action n'est pas journalisée.
    .exemple = Importez demo-import.svg puis cliquez : demo-export.dxf contient le même dessin en entités LINE.

aide-top-exporter-stl =
    .titre = Exporter STL
    .description = Écrit le maillage courant dans demo-export.stl (binaire, en millimètres, normales recalculées). Le bouton est actif seulement si un maillage a été importé.
    .exemple = Après un import STL, cliquez : demo-export.stl apparaît dans le répertoire courant, lisible par n'importe quel visualiseur.

aide-top-recalculer =
    .titre = Recalculer tout
    .description = Recalcule toutes les valeurs du projet à partir des dimensions saisies et des expressions. Utile après un changement de largeur ou de hauteur pour vérifier la diagonale.
    .exemple = Saisissez 800 pour la largeur et 600 pour la hauteur, puis cliquez : la diagonale affichée vaut 1000 mm.

aide-top-enregistrer =
    .titre = Enregistrer
    .description = Enregistre le projet dans le fichier de démonstration du répertoire courant, au format conteneur OmniFab (ZIP : manifeste, journal d'événements et blobs de géométrie). Les actions annulées sont abandonnées.
    .exemple = Cliquez : le fichier demo-jalon0.omnifab apparaît dans le répertoire courant, prêt à être rouvert par « Ouvrir ».

aide-top-ouvrir =
    .titre = Ouvrir
    .description = Ouvre le projet de démonstration du répertoire courant et reconstruit l'état complet par rejeu de son journal (event sourcing) : dimensions, expression, journal et géométrie importée.
    .exemple = Après « Enregistrer », redémarrez l'application puis cliquez : la largeur, la hauteur et la diagonale reviennent à l'état enregistré.

aide-top-nouveau =
    .titre = Nouveau
    .description = Remet le démonstrateur à l'état initial : nouvelle session, journal vidé, dimensions 800 × 600 mm, expression de diagonale par défaut.
    .exemple = Modifiez des valeurs, importez des tracés, puis cliquez : tout revient à l'état de départ.

aide-params-titre =
    .titre = Paramètres
    .description = Panneau des dimensions du rectangle de démonstration. Chaque modification devient un événement du journal : l'état se reconstruit par rejeu.
    .exemple = Agrandissez la largeur : une ligne value.input_set apparaît en tête du journal, avec son horodatage.

aide-params-largeur =
    .titre = Largeur
    .description = Largeur du rectangle de démonstration, en millimètres. Un événement est enregistré à la fin du geste, pas pendant le glissement.
    .exemple = Faites glisser jusqu'à 500 : le viewport dessine un rectangle plus étroit et la diagonale se met à jour.

aide-params-hauteur =
    .titre = Hauteur
    .description = Hauteur du rectangle de démonstration, en millimètres. Un événement est enregistré à la fin du geste, pas pendant le glissement.
    .exemple = Saisissez 1000 : avec une largeur de 1000, le rectangle de démonstration devient carré.

aide-params-expression =
    .titre = Expression de la diagonale
    .description = Expression de la diagonale, dans le sous-ensemble déterministe accepté par forge-values. Les noms width_mm et height_mm sont disponibles ; validez avec la touche Entrée.
    .exemple = Remplacez l'expression par sqrt(width_mm * width_mm) : la diagonale affichée vaut alors exactement la largeur.

aide-journal-titre =
    .titre = Journal d'événements
    .description = Journal des événements du projet : chaque action utilisateur y est inscrite, et l'état entier se reconstruit par rejeu de ce journal. Les imports y portent la référence du blob et leurs compteurs, jamais la géométrie.
    .exemple = Changez la largeur : la ligne value.input_set la plus récente s'affiche en tête, avec son numéro, son horodatage et son acteur.

aide-viewport-titre =
    .titre = Viewport
    .description = Zone d'affichage : le rectangle de démonstration aux dimensions saisies, avec par-dessus les tracés importés (SVG/DXF) en jaune. Les messages d'erreur et de confirmation s'affichent ici ; le maillage STL attend le rendu 3D du jalon 2.
    .exemple = Importez demo-import.svg : les tracés se dessinent en jaune par-dessus le rectangle, avec leurs compteurs dessous ; une expression invalide affiche une erreur en rouge sans écrire au journal.

aide-geom-compteur =
    .titre = Compteur de géométrie importée
    .description = Compteurs de la géométrie courante : tracés et sommets pour le vectoriel (SVG/DXF), triangles pour le maillage STL. Les tracés sont dessinés en jaune dans le viewport ; le maillage attend le rendu 3D du jalon 2.
    .exemple = Importez un SVG de deux rectangles : la ligne indique 2 tracés · 8 sommets importés.
