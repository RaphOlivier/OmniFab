# Démonstrateur omnifab-desktop — français, langue de référence.
# Toute l'interface vit ici (ADR-0010) : aucune chaîne d'interface dans
# le code Rust. Chaque widget interactif porte un help_id : sa page
# d'aide est le message du même nom, avec les attributs
# titre / description / exemple.

# --- Fenêtre ---

titre-fenetre = OmniFab — démonstrateur

# --- Barre du haut ---

top-annuler = Annuler
top-retablir = Rétablir
top-recalculer = Recalculer tout
top-enregistrer = Enregistrer
top-enregistrer-sous = Enregistrer sous…
top-ouvrir = Ouvrir
top-nouveau = Nouveau
top-bibliotheque = Bibliothèque

# --- Import / export (fichiers choisis par l'utilisateur, ADR-0014) ---

top-importer-svg = Importer SVG
top-importer-dxf = Importer DXF
top-importer-stl = Importer STL
top-exporter-svg = Exporter SVG
top-exporter-dxf = Exporter DXF
top-exporter-stl = Exporter STL

# --- Filtres des sélecteurs natifs (ADR-0014) ---

filtre-svg = Fichier SVG
filtre-dxf = Fichier DXF
filtre-stl = Maillage STL
filtre-projet = Projet OmniFab

# --- Écran d'accueil / bibliothèque (ADR-0014) ---

accueil-titre = Bibliothèque de projets
accueil-nouveau = Nouveau projet
accueil-ouvrir = Ouvrir un projet…
accueil-reprendre = Reprendre la session en cours
accueil-liste-titre = Projets du dossier
accueil-vide = Aucun projet enregistré pour l'instant : « Enregistrer sous… » range vos projets ici.
accueil-indisponible = Dossier de la bibliothèque introuvable : la liste est indisponible.
accueil-ouvrir-projet = Ouvrir
accueil-date-aujourdhui = aujourd’hui
accueil-date-hier = hier
accueil-date-jours = il y a {$n} jours
accueil-taille-octets = {$n} octets
accueil-taille-ko = {$n} Ko
accueil-taille-mio = {$n} Mio

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

viewport-titre = Viewport — rectangle de démonstration, tracés et maillage importés
viewport-erreur = Erreur : {$texte}

# --- Compteurs de géométrie importée ---

geom-traces-compteur = {$traces} tracés · {$sommets} sommets importés
geom-maillage-compteur = Maillage importé : {$triangles} triangles
viewport-wireframe-limite = Maillage trop dense pour le wireframe ({$triangles} triangles > {$limite}) : seules la boîte englobante et les statistiques sont affichées.

# --- Messages d'information ---

info-recalcul-ok = Toutes les valeurs sont à jour.
info-enregistre = Projet enregistré dans « {$chemin} ».
info-recharge = Projet ouvert : {$n} événements rejoués.
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
    .exemple = Importez un SVG puis cliquez : les tracés disparaissent du viewport. Le raccourci Ctrl+Z fait la même chose.

aide-top-retablir =
    .titre = Rétablir
    .description = Rétablit le dernier groupe d'actions annulé. Une nouvelle action après une annulation abandonne définitivement les actions défaites.
    .exemple = Après « Annuler », cliquez ou pressez Ctrl+Y : les tracés importés reviennent dans le viewport.

aide-top-importer-svg =
    .titre = Importer SVG
    .description = Ouvre le sélecteur de fichiers natif et convertit le SVG choisi en tracés 2D sous budgets de sécurité (une unité SVG = 1 mm). Les octets d'origine partent dans un blob du projet, et l'import est une action annulable : une transaction au journal.
    .exemple = Choisissez un SVG contenant un chemin M 0 0 L 100 0 L 0 100 Z : le viewport dessine le triangle en jaune et affiche les compteurs.

aide-top-importer-dxf =
    .titre = Importer DXF
    .description = Ouvre le sélecteur de fichiers natif et convertit le DXF choisi (entités LINE, LWPOLYLINE, POLYLINE, CIRCLE et ARC ; une unité = 1 mm) en tracés courants — une action annulable, comme l'import SVG.
    .exemple = Choisissez un DXF contenant une polyligne fermée : la forme apparaît en jaune et le compteur affiche tracés et sommets.

aide-top-importer-stl =
    .titre = Importer STL
    .description = Ouvre le sélecteur de fichiers natif et convertit le STL choisi (binaire ou ASCII, lu en millimètres) en maillage courant : octets d'origine dans un blob, compteurs au journal. Le maillage se dessine en wireframe cyan dans le viewport.
    .exemple = Importez un petit STL : le wireframe cyan apparaît par-dessus le rectangle, et le compteur « Maillage importé » affiche le nombre de triangles.

aide-top-exporter-svg =
    .titre = Exporter SVG
    .description = Ouvre le sélecteur et écrit les tracés courants dans le fichier choisi : les octets d'origine sont relus depuis leur blob puis re-sérialisés (une unité = 1 mm). L'export ne change pas l'état du projet : rien au journal.
    .exemple = Après un import, cliquez et choisissez un emplacement : le fichier SVG y est écrit, lisible par n'importe quel visualiseur.

aide-top-exporter-dxf =
    .titre = Exporter DXF
    .description = Ouvre le sélecteur et écrit les tracés courants au format DXF, un segment par entité LINE (les tracés fermés sont refermés). Comme tout export, l'action n'est pas journalisée.
    .exemple = Importez un SVG puis exportez en DXF : le fichier contient le même dessin en entités LINE.

aide-top-exporter-stl =
    .titre = Exporter STL
    .description = Ouvre le sélecteur et écrit le maillage courant au format STL binaire (millimètres, normales recalculées). Le bouton est actif seulement si un maillage a été importé.
    .exemple = Après un import STL, cliquez et choisissez un emplacement : le fichier STL y est écrit, lisible par n'importe quel visualiseur.

aide-top-recalculer =
    .titre = Recalculer tout
    .description = Recalcule toutes les valeurs du projet à partir des dimensions saisies et des expressions. Utile après un changement de largeur ou de hauteur pour vérifier la diagonale.
    .exemple = Saisissez 800 pour la largeur et 600 pour la hauteur, puis cliquez : la diagonale affichée vaut 1000 mm.

aide-top-enregistrer =
    .titre = Enregistrer
    .description = Enregistre le projet dans son fichier courant au format conteneur OmniFab (ZIP : manifeste, journal d'événements et blobs de géométrie). Sans fichier courant, « Enregistrer » ouvre le sélecteur de « Enregistrer sous… », qui propose le dossier de la bibliothèque. Les actions annulées sont abandonnées.
    .exemple = Après un premier « Enregistrer sous… » dans la bibliothèque, « Enregistrer » réécrit directement ce fichier ; il apparaît dans la liste de l'écran d'accueil.

aide-top-enregistrer-sous =
    .titre = Enregistrer sous…
    .description = Enregistre le projet dans un fichier .omnifab choisi dans le sélecteur natif : le dossier de la bibliothèque est proposé par défaut, et le fichier devient le projet courant — « Enregistrer » écrira dedans ensuite.
    .exemple = Enregistrez sous « caisse.omnifab » : le fichier apparaît dans la bibliothèque de l'écran d'accueil, avec sa date et sa taille.

aide-top-ouvrir =
    .titre = Ouvrir
    .description = Ouvre le sélecteur natif et recharge un projet .omnifab, où qu'il soit sur le disque : l'état complet se reconstruit par rejeu de son journal (event sourcing) — dimensions, expression, journal et géométrie importée.
    .exemple = Après « Enregistrer », redémarrez l'application puis ouvrez le fichier : la largeur, la hauteur et la diagonale reviennent à l'état enregistré.

aide-top-nouveau =
    .titre = Nouveau
    .description = Remet le démonstrateur à l'état initial : nouvelle session, journal vidé, dimensions 800 × 600 mm, expression de diagonale par défaut — et retour à l'atelier.
    .exemple = Modifiez des valeurs, importez des tracés, puis cliquez : tout revient à l'état de départ.

aide-top-bibliotheque =
    .titre = Bibliothèque
    .description = Retourne à l'écran d'accueil : nouveau projet, ouverture, reprise de la session en cours, et liste des projets du dossier de la bibliothèque. La session en cours reste en mémoire tant qu'aucune autre n'est démarrée.
    .exemple = Cliquez : l'accueil liste vos projets enregistrés, du plus récent au plus ancien ; « Reprendre la session en cours » ramène à l'atelier sans rien changer.

aide-accueil-titre =
    .titre = Bibliothèque de projets
    .description = Écran d'accueil du démonstrateur (ADR-0014) : la bibliothèque liste les fichiers .omnifab du dossier OmniFab/Projets de votre dossier personnel — visible et manipulable à la main.
    .exemple = Enregistrez un projet avec « Enregistrer sous… », puis rouvrez l'application : le fichier apparaît ici avec sa date et sa taille.

aide-accueil-nouveau =
    .titre = Nouveau projet
    .description = Démarre une session vierge : journal vidé, dimensions 800 × 600 mm, expression de diagonale par défaut, aucun fichier courant. La session précédente est remplacée — pensez à enregistrer avant.
    .exemple = Cliquez depuis l'accueil : l'atelier s'ouvre sur le rectangle de démonstration initial.

aide-accueil-ouvrir =
    .titre = Ouvrir un projet…
    .description = Ouvre le sélecteur natif de fichiers et charge un projet .omnifab, où qu'il soit sur le disque. L'état se reconstruit intégralement par rejeu du journal.
    .exemple = Cliquez et choisissez un projet enregistré ailleurs que dans la bibliothèque : il s'ouvre dans l'atelier et devient le projet courant.

aide-accueil-reprendre =
    .titre = Reprendre la session en cours
    .description = Retourne à l'atelier sans rien changer : la session en mémoire (celle d'avant l'ouverture de l'accueil) est conservée tant qu'aucun nouveau projet n'est démarré.
    .exemple = Cliquez « Bibliothèque » en pleine séance de travail, puis « Reprendre » : vous retrouvez l'atelier exactement comme vous l'avez quitté.

aide-accueil-liste =
    .titre = Projets du dossier
    .description = Liste des fichiers .omnifab du dossier de la bibliothèque, du plus récemment modifié au plus ancien, avec l'âge et la taille de chaque fichier. Un clic sur « Ouvrir » charge le projet.
    .exemple = Enregistrez deux projets : le dernier enregistré apparaît en tête de liste.

aide-accueil-projet =
    .titre = Projet de la bibliothèque
    .description = Un projet enregistré dans le dossier de la bibliothèque : son nom, son âge et sa taille sont lus depuis le disque. « Ouvrir » le charge et en fait le projet courant.
    .exemple = Cliquez « Ouvrir » sur le projet le plus récent : l'atelier s'ouvre avec son état complet, prêt à être modifié.

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
    .description = Zone d'affichage : le rectangle de démonstration aux dimensions saisies, les tracés importés (SVG/DXF) en jaune et le maillage STL en wireframe cyan — même mise à l'échelle pour les deux. Les messages d'erreur et de confirmation s'affichent ici.
    .exemple = Importez un SVG puis un petit STL : le triangle jaune et le wireframe cyan se dessinent ensemble au-dessus du rectangle, avec leurs compteurs dessous.

aide-geom-compteur =
    .titre = Compteur de géométrie importée
    .description = Compteurs de la géométrie courante : tracés et sommets pour le vectoriel (SVG/DXF), triangles pour le maillage STL. Au-delà du plafond d'affichage du wireframe, seule la boîte englobante du maillage est dessinée — le maillage reste importé et compté.
    .exemple = Importez un SVG de deux rectangles : la ligne indique 2 tracés · 8 sommets importés.
