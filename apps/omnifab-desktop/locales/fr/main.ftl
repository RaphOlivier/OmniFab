# Démonstrateur omnifab-desktop — français, langue de référence.
# Toute l'interface vit ici (ADR-0010) : aucune chaîne d'interface dans
# le code Rust. Chaque widget interactif porte un help_id : sa page
# d'aide est le message du même nom, avec les attributs
# titre / description / exemple.

# --- Fenêtre ---

titre-fenetre = OmniFab — démonstrateur
titre-fenetre-modifie = OmniFab — démonstrateur *

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

# --- Panneau des liaisons (ADR-0019) ---

liaisons-titre = Liaisons
liaisons-nouvel-alias = + Nouvel alias
liaisons-nouveau-titre = Nouvel alias
liaisons-nouveau-nom = Nom
liaisons-nouveau-expression = Expression pilotée
liaisons-nouveau-texte = Texte de l'expression :
liaisons-nouveau-valeur = Valeur :
liaisons-nouveau-unite = Unité
liaisons-unite-aucune = sans unité
liaisons-nouveau-poser = Poser
liaisons-nouveau-annuler = Annuler
liaisons-supprimer = Supprimer
liaisons-suppression-titre = Supprimer l'alias
liaisons-suppression-texte = {$n} formules lisent encore cet alias :
liaisons-suppression-cascade = Supprimer aussi ces formules ({$n})
liaisons-suppression-remplacer = Remplacer la référence par :
liaisons-suppression-remplacer-bouton = Remplacer
liaisons-suppression-annuler = Annuler
liaisons-suppression-aucun-candidat = Aucun autre alias disponible pour le remplacement.

# --- Dialogue de cote de l'atelier (ADR-0019) ---

cote-titre = Cote de distance
cote-fixe = Valeur fixe
cote-expression = Expression pilotée
cote-texte = Texte de l'expression :
cote-alias = Alias (préfixe {$prefixe})
cote-poser = Poser
cote-annuler = Annuler

# --- Journal des événements ---

journal-titre = Journal d'événements
journal-compteur = {$n} événements · conteneur ZIP · JSON Lines
journal-types-ignores = Types ignorés (journal d'une version plus récente ?) :

# --- Viewport ---

viewport-titre = Viewport — rectangle de démonstration, tracés et maillage importés
viewport-erreur = Erreur : {$texte}
viewport-wgpu-indisponible = Rendu wgpu indisponible : la grille, les tracés et le wireframe ne sont pas dessinés (moteur wgpu requis, ADR-0015).
viewport-vue-2d = Vue 2D
viewport-vue-3d = Vue 3D

# --- Esquisse (croquis contraint, ADR-0018) ---

esquisse-titre = Esquisse
esquisse-outil-selection = Sélection
esquisse-outil-point = Point
esquisse-outil-ligne = Ligne
esquisse-outil-cercle = Cercle
esquisse-outil-arc = Arc
esquisse-contraintes = Contraintes
esquisse-contrainte-horizontale = Horizontale
esquisse-contrainte-verticale = Verticale
esquisse-contrainte-point-fixe = Fixer le point
esquisse-contrainte-distance = Distance
esquisse-dof = Degrés de liberté : {$n}
esquisse-redondances = Contraintes redondantes : {$n}

# --- Compteurs de géométrie importée ---

geom-traces-compteur = {$traces} tracés · {$sommets} sommets importés
geom-maillage-compteur = Maillage importé : {$triangles} triangles

# --- Messages d'information ---

info-recalcul-ok = Toutes les valeurs sont à jour.
info-enregistre = Projet enregistré dans « {$chemin} ».
info-recharge = Projet ouvert : {$n} événements rejoués.
info-import-traces = Import réussi : {$traces} tracés · {$sommets} sommets.
info-import-maillage = Import réussi : {$triangles} triangles.
info-export = Exporté dans « {$chemin} ».
info-travail-en-cours = Traitement du fichier en cours…

# --- Confirmation (protection du travail, ADR-0016 C3) ---

confirmation-titre = Travail non enregistré
confirmation-texte = Le projet comporte des modifications non enregistrées : continuer les perdra définitivement.
confirmation-confirmer = Continuer
confirmation-annuler = Annuler

# --- Unités du maillage importé (ADR-0017) ---
echelle-titre = Unités du modèle
echelle-texte = La plus grande dimension du modèle mesure {$dimension} mm : il ressemble à un export en mètres (Blender, par exemple). Dans quelle unité l'importer ?
echelle-metres = Mètres (× 1000)
echelle-millimetres = Millimètres (tel quel)
echelle-annuler = Annuler l'import

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
    .description = Ouvre le sélecteur de fichiers natif et convertit le STL choisi (binaire ou ASCII, lu en millimètres) en maillage courant : octets d'origine dans un blob, compteurs au journal. Le maillage se dessine en wireframe cyan dans les deux vues du viewport, en projection de dessus en vue 2D.
    .exemple = Importez un petit STL puis basculez en vue 3D : le wireframe cyan apparaît en volume, et le compteur « Maillage importé » affiche le nombre de triangles.

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

aide-liaisons-titre =
    .titre = Liaisons
    .description = Panneau du moteur de liaisons (ADR-0019) : une ligne par alias du projet — entrée à valeur saisie ou expression pilotée, avec son unité. Les cotes de l'esquisse vivent sous le préfixe réservé « esquisse. » et s'y lisent sans se modifier.
    .exemple = Ouvrez un projet vierge : les alias largeur, hauteur et diagonale y apparaissent ; changez la largeur et la diagonale se recalcule au prochain recalcul.

aide-liaisons-valeur =
    .titre = Valeur d'entrée
    .description = Valeur saisie de l'alias, en millimètres ou degrés selon l'unité déclarée. L'événement part à la fin du geste : le journal reste fidèle aux actions de l'utilisateur.
    .exemple = Faites glisser la valeur de largeur jusqu'à 500 : une ligne value.input_set apparaît en tête du journal, avec son horodatage.

aide-liaisons-expression =
    .titre = Expression pilotée
    .description = Texte de l'expression de l'alias, dans le sous-ensemble déterministe accepté par forge-values. La saisie s'engage à la touche Entrée ; le résultat calculé s'affiche à côté en petit.
    .exemple = Remplacez l'expression de la diagonale par sqrt(largeur * largeur) : la diagonale vaut alors exactement la largeur.

aide-liaisons-unite =
    .titre = Unité
    .description = Unité déclarée de l'alias : sans unité, millimètres ou degrés. La changer réémet la définition et la cohérence des expressions est revérifiée (OF-VAL-0007).
    .exemple = Déclarez un angle en degrés puis écrivez une expression qui le lit : une incohérence d'unités affiche le badge d'erreur en clair sous la ligne.

aide-liaisons-supprimer =
    .titre = Supprimer l'alias
    .description = Supprime l'alias de la ligne. Un alias encore lu par des formules ouvre le dialogue de suppression : cascade, remplacement ou annulation (ADR-0019).
    .exemple = Supprimez la hauteur alors que la diagonale la lit : le dialogue propose de supprimer aussi la formule ou de remplacer la référence.

aide-liaisons-nouvel-alias =
    .titre = + Nouvel alias
    .description = Ouvre le dialogue de création d'un alias : entrée à valeur saisie ou expression pilotée, avec son unité. L'alias n'existe qu'au clic sur « Poser » : jamais d'événement avant.
    .exemple = Créez l'alias « epaisseur » en entrée millimétrique : il apparaît dans le panneau et peut servir dans une expression.

aide-liaisons-nouveau-nom =
    .titre = Nom
    .description = Nom de l'alias à créer : pas de point (les chemins sépareraient les espaces de noms) et pas de préfixe réservé « esquisse. » (OF-VAL-0008).
    .exemple = Saisissez « epaisseur » : l'alias se pose ; saisissez « esquisse.x » : le refus s'affiche et le dialogue reste ouvert.

aide-liaisons-nouveau-expression =
    .titre = Expression pilotée
    .description = Bascule l'alias à créer en expression pilotée : sa valeur se calcule depuis d'autres alias au lieu d'être saisie.
    .exemple = Cochez puis écrivez « largeur / 2 » : l'alias posé vaut la moitié de la largeur à chaque recalcul.

aide-liaisons-nouveau-texte =
    .titre = Texte de l'expression
    .description = Texte de l'expression du nouvel alias, dans le sous-ensemble déterministe accepté par forge-values : les noms disponibles sont ceux des autres alias du panneau.
    .exemple = Écrivez « sqrt(largeur * largeur + hauteur * hauteur) » : l'alias posé vaut la diagonale du rectangle.

aide-liaisons-nouveau-valeur =
    .titre = Valeur
    .description = Valeur initiale saisie de l'alias à créer, dans son unité déclarée.
    .exemple = Saisissez 12 puis posez : l'alias démarre à 12 et chaque changement devient un événement du journal.

aide-liaisons-nouveau-unite =
    .titre = Unité
    .description = Unité du nouvel alias : sans unité, millimètres ou degrés. La cohérence avec les expressions qui le liront est revérifiée à chaque définition (OF-VAL-0007).
    .exemple = Posez un alias en millimètres puis lisez-le depuis une expression en degrés : le refus s'affiche en clair.

aide-liaisons-nouveau-poser =
    .titre = Poser
    .description = Crée l'alias et ferme le dialogue. Un refus — nom invalide, expression invalide, incohérence d'unités — laisse le dialogue ouvert, la saisie reste corrigeable.
    .exemple = Saisissez un nom libre puis cliquez : l'alias apparaît dans le panneau et une ligne value.input_set ou value.expression_set part au journal.

aide-liaisons-nouveau-annuler =
    .titre = Annuler
    .description = Ferme le dialogue sans rien créer : aucun événement, le journal ne bouge pas.
    .exemple = Ouvrez la création puis annulez : le panneau des liaisons reste exactement comme avant.

aide-liaisons-suppression-cascade =
    .titre = Supprimer aussi ces formules
    .description = Supprime l'alias et toutes les formules qui le lisent, d'un seul groupe d'actions annulable. Désactivé si une référence est un nœud système : les cotes « esquisse. » ne se suppriment pas depuis le panneau.
    .exemple = Supprimez la largeur alors que diagonale la lit : la diagonale disparaît avec elle, et Ctrl+Z rétablit le tout.

aide-liaisons-suppression-remplacer =
    .titre = Remplacer la référence par
    .description = Liste des alias candidats au remplacement : les formules qui lisent l'alias supprimé liront l'alias choisi, unités revérifiées à la définition (OF-VAL-0007).
    .exemple = Remplacez les références de « hauteur » par « largeur » : les formules qui la lisaient affichent la largeur.

aide-liaisons-suppression-remplacer-bouton =
    .titre = Remplacer
    .description = Applique le remplacement choisi puis supprime l'alias : un seul groupe d'actions annulable.
    .exemple = Choisissez un candidat puis cliquez : l'alias disparaît, les formules survivent, Ctrl+Z rétablit le tout.

aide-liaisons-suppression-annuler =
    .titre = Annuler
    .description = Ferme le dialogue sans supprimer : l'alias et ses formules restent intacts.
    .exemple = Ouvrez la suppression puis annulez : rien ne change dans le panneau ni dans le journal.

aide-journal-titre =
    .titre = Journal d'événements
    .description = Journal des événements du projet : chaque action utilisateur y est inscrite, et l'état entier se reconstruit par rejeu de ce journal. Les imports y portent la référence du blob et leurs compteurs, jamais la géométrie.
    .exemple = Changez la largeur : la ligne value.input_set la plus récente s'affiche en tête, avec son numéro, son horodatage et son acteur.

aide-viewport-titre =
    .titre = Viewport
    .description = Zone d'affichage à deux vues (ADR-0015, phase C2). Vue 2D : le rectangle de démonstration aux dimensions saisies, la grille de fond, les tracés importés (SVG/DXF) en jaune et le maillage STL en wireframe cyan ; la molette zoome vers le curseur, un glissement déplace la vue — sauf sur une poignée du croquis en mode esquisse, où il déplace le point (ADR-0018 D4). Vue 3D : rectangle et tracés posés sur le plan z = 0, maillage STL en wireframe cyan ; la molette zoome, un glissement fait orbiter la caméra. Les messages d'erreur et de confirmation s'affichent ici.
    .exemple = Importez un SVG puis un petit STL : le triangle jaune et le wireframe cyan se dessinent en vue 2D, le wireframe en volume en vue 3D, avec les compteurs dessous.

aide-viewport-camera =
    .titre = Caméra du viewport
    .description = Vue 2D : la molette zoome sur le point du dessin sous le curseur, un glissement (bouton maintenu) déplace la vue — sauf sur une poignée du croquis en mode esquisse, où il déplace le point en aperçu (ADR-0018 D4). Vue 3D : la molette zoome vers la cible, un glissement fait orbiter la caméra (azimut et élévation). La caméra se réajuste automatiquement quand la géométrie change : import, annulation, rétablissement ou ouverture d'un projet.
    .exemple = Importez un SVG, éloignez la vue à la molette, puis annulez l'import : la vue se réajuste pour réafficher le rectangle de démonstration.

aide-viewport-vue-2d =
    .titre = Vue 2D
    .description = Vue de dessus du plan de travail : grille de fond, rectangle de démonstration, tracés importés (SVG/DXF) en jaune et maillage STL en wireframe cyan. La molette zoome sur le point du dessin sous le curseur, un glissement déplace la vue (ADR-0015) — sauf sur une poignée du croquis en mode esquisse, où il déplace le point (ADR-0018 D4).
    .exemple = Importez un SVG : le triangle jaune se dessine sur la grille ; zoomez à la molette, déplacez la vue en glissant.

aide-viewport-vue-3d =
    .titre = Vue 3D
    .description = Vue en volume de la scêne : rectangle et tracés posés sur le plan z = 0, grille au sol, maillage STL en wireframe cyan — arêtes dédupliquées, sans plafond d'affichage. La molette zoome, un glissement fait orbiter la caméra (ADR-0015, phase C2).
    .exemple = Importez un STL puis cliquez Vue 3D : le wireframe apparaît en volume ; glissez pour orbiter autour de la scêne, zoomez à la molette.

aide-geom-compteur =
    .titre = Compteur de géométrie importée
    .description = Compteurs de la géométrie courante : tracés et sommets pour le vectoriel (SVG/DXF), triangles pour le maillage STL — dessiné en wireframe dans les deux vues (projection de dessus en 2D), sans plafond d'affichage (phase C2).
    .exemple = Importez un SVG de deux rectangles : la ligne indique 2 tracés · 8 sommets importés.

aide-confirmation-perte =
    .titre = Travail non enregistré
    .description = Confirmation demandée avant une action destructive (nouveau projet, ouverture, reprise) quand des modifications non enregistrées existent : les continuer abandonne définitivement — « Enregistrer sous… » les met à l'abri d'abord.
    .exemple = Modifiez la largeur puis cliquez « Nouveau » : la fenêtre demande confirmation ; « Annuler » ramène au travail en cours, rien n'est perdu.

aide-echelle-metres =
    .titre = Mètres (× 1000)
    .description = Les coordonnées du modèle sont multipliées par 1000 : un export en mètres (Blender, par exemple) retrouve sa taille en millimètres. Le facteur d'échelle est enregistré dans le projet avec l'import, et l'export STL écrit toujours des millimètres.
    .exemple = Importez un STL Blender : si le modèle est minuscule, le dialogue des unités s'ouvre ; choisissez « Mètres (× 1000) » — le wireframe retrouve sa taille en vue 2D comme en vue 3D.

aide-echelle-millimetres =
    .titre = Millimètres (tel quel)
    .description = Le modèle est importé sans changement d'échelle : ses coordonnées sont déjà en millimètres. Le facteur 1 est enregistré dans le projet avec l'import.
    .exemple = Importez un STL FreeCAD : si le dialogue des unités s'ouvre à tort, choisissez « Millimètres (tel quel) » — le modèle garde exactement ses dimensions.

aide-echelle-annuler =
    .titre = Annuler l'import
    .description = Abandonne l'import : rien n'est écrit au journal, le projet et le fichier d'origine ne sont pas modifiés.
    .exemple = Changez d'avis dans le dialogue des unités : « Annuler l'import » ferme la fenêtre, aucun maillage n'est importé.

aide-esquisse-titre =
    .titre = Esquisse
    .description = Outils du croquis contraint de la vue 2D (ADR-0018) : les entités posées vivent dans la session et se résolvent au fil de travail — les positions calculées ne sont jamais enregistrées dans le projet.
    .exemple = Choisissez l'outil Point puis cliquez dans le viewport 2D : un point orange apparaît et le croquis se résout en arrière-plan.

aide-esquisse-outil-selection =
    .titre = Outil Sélection
    .description = Outil par défaut : un clic sélectionne le point du croquis le plus proche (sa poignée passe au marqueur distinct), un clic ailleurs vide la sélection. Un glissement depuis une poignée déplace le point : l'aperçu suit le curseur sans rien écrire au journal et la commande MovePoint pose la graine au relâchement (ADR-0018 D4). Un glissement du fond déplace la vue, la molette zoome.
    .exemple = Posez un point avec l'outil Point, revenez à Sélection et cliquez-le : la poignée se dessine. Glissez-la : le point suit le curseur, se pose au relâchement et le croquis se résout en arrière-plan.

aide-esquisse-outil-point =
    .titre = Outil Point
    .description = Pose un point libre du croquis contraint à l'endroit cliqué : la commande passe par la session (annulable), la position saisie sert de graine au solveur (ADR-0018).
    .exemple = Activez Point puis cliquez dans le viewport 2D : le point se pose ; annulez-le avec Ctrl+Z puis rétablissez-le avec Ctrl+Y.

aide-esquisse-outil-ligne =
    .titre = Outil Ligne
    .description = Trace une ligne du croquis en deux clics : le premier pose le point d'origine, le second l'extrémité — chaque clic s'accroche à la grille et aux points existants, les commandes passent par la session (annulables, ADR-0018).
    .exemple = Activez Ligne, cliquez près d'un point existant pour en partir, puis cliquez l'extrémité : la ligne se pose ; annulez avec Ctrl+Z, rétablissez avec Ctrl+Y.

aide-esquisse-outil-cercle =
    .titre = Outil Cercle
    .description = Trace un cercle du croquis en deux clics : le premier pose le centre, le second donne le rayon — le second clic ne pose pas de point, la commande passe par la session (ADR-0018).
    .exemple = Activez Cercle, cliquez le centre puis un point du viewport 2D : le cercle orange apparaît ; Ctrl+Z l'annule, Ctrl+Y le rétablit.

aide-esquisse-outil-arc =
    .titre = Outil Arc
    .description = Trace un arc du croquis en trois clics : le centre, le point de début (qui fixe le rayon), puis la fin — le sens de balayage suit le déplacement du curseur autour du centre, trigonométrique ou horaire (ADR-0020), les commandes passent par la session (ADR-0018).
    .exemple = Activez Arc, cliquez le centre, un point du cercle, tournez autour du centre dans le sens voulu, puis cliquez la fin : l'arc suit le geste ; Ctrl+Z annule le dernier ajout, Ctrl+Y le rétablit.

aide-esquisse-contraintes =
    .titre = Contraintes
    .description = Contraintes du croquis (ADR-0018) : posez une contrainte sur la sélection courante — une ligne pour Horizontale ou Verticale, un point pour Fixer, deux points pour Distance. Chaque contrainte devient un événement du journal (annulable), le solveur recalcule au fil de travail.
    .exemple = Sélectionnez une ligne puis cliquez Horizontale : elle s'aligne au prochain recalcul ; Ctrl+Z annule la contrainte.

aide-esquisse-contrainte-horizontale =
    .titre = Contrainte Horizontale
    .description = Force la ligne sélectionnée à l'horizontale. Le bouton est actif seulement avec une ligne en sélection ; la commande AddConstraint passe par la session (ADR-0018).
    .exemple = Tracez une ligne avec l'outil Ligne, sélectionnez-la en outil Sélection, cliquez : la ligne devient horizontale au prochain recalcul.

aide-esquisse-contrainte-verticale =
    .titre = Contrainte Verticale
    .description = Force la ligne sélectionnée à la verticale. Le bouton est actif seulement avec une ligne en sélection ; la commande AddConstraint passe par la session (ADR-0018).
    .exemple = Tracez une ligne, sélectionnez-la en outil Sélection, cliquez : la ligne devient verticale au prochain recalcul.

aide-esquisse-contrainte-point-fixe =
    .titre = Fixer le point
    .description = Ancre le point sélectionné à sa position courante : le croquis gagne sa référence au monde (ADR-0018). Le bouton est actif seulement avec un point en sélection.
    .exemple = Posez un point, sélectionnez-le, cliquez : un rectangle contraint fixé à ce coin tombe à zéro degré de liberté.

aide-esquisse-contrainte-distance =
    .titre = Contrainte Distance
    .description = Ouvre le dialogue de cote pour les deux points sélectionnés (ADR-0019) : valeur fixe en millimètres ou expression pilotée par un alias sous le préfixe réservé « esquisse. ». La contrainte n'est posée qu'au clic sur « Poser » : un refus laisse le dialogue ouvert. Le bouton est actif seulement avec deux points en sélection.
    .exemple = Sélectionnez deux points du croquis puis cliquez : le dialogue propose la distance courante ; posez une expression « largeur / 2 » sous l'alias « demi » et la cote pilotée s'impose au prochain recalcul.

aide-cote-fixe =
    .titre = Valeur fixe
    .description = Cote de distance à valeur fixe, en millimètres : la distance entre les deux points s'impose au solveur, indépendamment du moteur de liaisons.
    .exemple = Posez une cote fixe de 50 : la distance entre les deux points vaut 50 mm au prochain recalcul.

aide-cote-mm =
    .titre = Distance
    .description = Valeur fixe de la cote en millimètres, préremplie avec la distance courante entre les deux points sélectionnés.
    .exemple = Sélectionnez deux points à 30 mm : le dialogue propose 30 ; changez à 50 puis posez.

aide-cote-expression =
    .titre = Expression pilotée
    .description = Cote pilotée par le moteur de liaisons : la distance entre les deux points se calcule depuis une expression, et se re-fige quand les alias qu'elle lit changent (ADR-0019).
    .exemple = Posez une cote d'expression « largeur / 2 » : changez la largeur dans le panneau des liaisons et la cote se re-fige à la nouvelle valeur.

aide-cote-texte =
    .titre = Texte de l'expression
    .description = Texte de l'expression pilotant la cote, dans le sous-ensemble déterministe accepté par forge-values : les noms disponibles sont les alias du panneau des liaisons.
    .exemple = Écrivez « largeur / 2 » : la cote vaudra la moitié de la largeur, recalculée à chaque changement.

aide-cote-alias =
    .titre = Alias
    .description = Nom de la cote sous le préfixe réservé « esquisse. » : l'alias apparaît en lecture seule dans le panneau des liaisons et peut servir dans d'autres expressions.
    .exemple = Saisissez « demi » : la cote vit sous « esquisse.demi », visible dans le panneau des liaisons.

aide-cote-poser =
    .titre = Poser
    .description = Pose la contrainte Distance et ferme le dialogue. Un refus — expression invalide, alias déjà pris — laisse le dialogue ouvert, la saisie reste corrigeable.
    .exemple = Remplissez l'expression et l'alias puis cliquez : la contrainte part au journal et la cote s'impose au prochain recalcul.

aide-cote-annuler =
    .titre = Annuler
    .description = Ferme le dialogue sans poser la contrainte : aucun événement, le croquis ne bouge pas.
    .exemple = Ouvrez le dialogue puis annulez : le croquis reste exactement comme avant.

# --- Pages d'aide des codes d'erreur du moteur de liaisons
#     (ADR-0019 : badge rouge du panneau, clic droit) ---

aide-of-val-0001 =
    .titre = Cycle dans le graphe (OF-VAL-0001)
    .description = Le moteur de liaisons a détecté un cycle : une expression ramène à son propre alias, directement ou à travers d'autres formules. Le chemin du cycle accompagne le code.
    .exemple = Écrivez que « a » lit « b » puis que « b » lit « a » : le refus affiche le cycle et le graphe reste à l'état précédent.

aide-of-val-0002 =
    .titre = Valeur inconnue (OF-VAL-0002)
    .description = Une expression référence un nom absent du graphe des liaisons : vérifiez l'orthographe du nom cité, ou créez l'alias d'abord.
    .exemple = Écrivez « largeur * 2 » alors que « largeur » n'existe pas encore : le refus cite le nom inconnu.

aide-of-val-0003 =
    .titre = Expression invalide (OF-VAL-0003)
    .description = Le texte sort du sous-ensemble déterministe accepté : opérateurs arithmétiques, fonctions usuelles et noms d'alias seulement — pas d'appel arbitraire, pas de syntaxe libre.
    .exemple = Écrivez « largeur & hauteur » : l'esperluette n'est pas un opérateur accepté, le refus explique la raison.

aide-of-val-0004 =
    .titre = Erreur d'évaluation (OF-VAL-0004)
    .description = L'expression est syntaxiquement valide mais son évaluation échoue : division par zéro, racine d'un nombre négatif, ou résultat non fini.
    .exemple = Écrivez « 1 / (largeur - largeur) » : la division par zéro refuse la définition, le nœud garde sa valeur précédente.

aide-of-val-0005 =
    .titre = Valeur inconnue (OF-VAL-0005)
    .description = Lecture d'une valeur absente du graphe des liaisons : le nom demandé n'existe pas — l'état demandé est incohérent avec le journal.
    .exemple = Demandez la valeur d'un alias jamais posé : le refus cite le nom inconnu.

aide-of-val-0006 =
    .titre = Valeur non finie (OF-VAL-0006)
    .description = Une valeur d'entrée doit être un nombre fini : l'infini et les indéterminés ne vivent pas dans le graphe des liaisons.
    .exemple = Une saisie qui deviendrait infinie est refusée : le nom de l'entrée accompagne le code.

aide-of-val-0007 =
    .titre = Incohérence d'unités (OF-VAL-0007)
    .description = Une expression lit une valeur d'une unité incompatible avec l'unité déclarée du nœud : millimètres, degrés ou sans unité doivent s'accorder.
    .exemple = Déclarez un alias en degrés puis écrivez qu'il lit une longueur en millimètres : le refus cite les deux unités.

aide-of-val-0008 =
    .titre = Alias réservé (OF-VAL-0008)
    .description = Le nom demandé vit sous un préfixe réservé du système, comme « esquisse. » pour les cotes du croquis : ces alias se posent par l'atelier, jamais à la main.
    .exemple = Créez un alias nommé « esquisse.demi » depuis le panneau des liaisons : le refus explique le préfixe réservé.

aide-of-val-0009 =
    .titre = Alias référencé (OF-VAL-0009)
    .description = Suppression « Simple » refusée : des formules lisent encore cet alias. Le dialogue de suppression propose la cascade ou le remplacement.
    .exemple = Supprimez la hauteur alors que la diagonale la lit : le dialogue propose les deux modes au lieu de la suppression simple.
