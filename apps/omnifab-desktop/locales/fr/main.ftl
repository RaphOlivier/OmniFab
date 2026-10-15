# Démonstrateur omnifab-desktop — français, langue de référence.
# Toute l'interface vit ici (ADR-0010) : aucune chaîne d'interface dans
# le code Rust. Chaque widget interactif porte un help_id : sa page
# d'aide est le message du même nom, avec les attributs
# titre / description / exemple.

# --- Fenêtre ---

titre-fenetre = OmniFab — démonstrateur jalon 0

# --- Barre du haut ---

top-recalculer = Recalculer tout
top-enregistrer = Enregistrer
top-ouvrir = Ouvrir
top-nouveau = Nouveau

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

viewport-titre = Viewport (vide au jalon 0 — rectangle de démonstration)
viewport-erreur = Erreur : {$texte}

# --- Messages d'information ---

info-recalcul-ok = Toutes les valeurs sont à jour.
info-enregistre = Projet enregistré dans « {$chemin} ».
info-recharge = Projet rechargé : {$n} événements rejoués.

# --- Fenêtre d'aide ---

aide-titre-fenetre = Aide — {$titre}
aide-section-exemple = Exemple :
aide-page-introuvable = Aide introuvable pour « {$identifiant} ».

# --- Pages d'aide (help_id : message sans valeur, attributs
#     titre / description / exemple — ADR-0010) ---

aide-top-recalculer =
    .titre = Recalculer tout
    .description = Recalcule toutes les valeurs du projet à partir des dimensions saisies et des expressions. Utile après un changement de largeur ou de hauteur pour vérifier la diagonale.
    .exemple = Saisissez 800 pour la largeur et 600 pour la hauteur, puis cliquez : la diagonale affichée vaut 1000 mm.

aide-top-enregistrer =
    .titre = Enregistrer
    .description = Enregistre le projet dans le fichier de démonstration du répertoire courant, au format conteneur OmniFab (ZIP : manifeste + journal d'événements).
    .exemple = Cliquez : le fichier demo-jalon0.omnifab apparaît dans le répertoire courant, prêt à être rouvert par « Ouvrir ».

aide-top-ouvrir =
    .titre = Ouvrir
    .description = Ouvre le projet de démonstration du répertoire courant et reconstruit l'état complet par rejeu de son journal (event sourcing) : dimensions, expression, journal.
    .exemple = Après « Enregistrer », redémarrez l'application puis cliquez : la largeur, la hauteur et la diagonale reviennent à l'état enregistré.

aide-top-nouveau =
    .titre = Nouveau
    .description = Remet le démonstrateur à l'état initial : journal vidé, dimensions 800 × 600 mm, expression de diagonale par défaut.
    .exemple = Modifiez des valeurs, ajoutez des événements, puis cliquez : tout revient à l'état de départ.

aide-params-titre =
    .titre = Paramètres
    .description = Panneau des dimensions du rectangle de démonstration. Chaque modification devient un événement du journal : l'état se reconstruit par rejeu.
    .exemple = Agrandissez la largeur : une ligne demo.value.input_set apparaît en tête du journal, avec son horodatage.

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
    .description = Journal des événements du projet : chaque action utilisateur y est inscrite, et l'état entier se reconstruit par rejeu de ce journal.
    .exemple = Changez la largeur : la ligne demo.value.input_set la plus récente s'affiche en tête, avec son numéro, son horodatage et son acteur.

aide-viewport-titre =
    .titre = Viewport
    .description = Zone d'affichage, vide au jalon 0 : un rectangle aux dimensions saisies y est dessiné. Les messages d'erreur et de confirmation s'affichent ici.
    .exemple = Saisissez une expression invalide puis validez : le message d'erreur s'affiche en rouge dans le viewport, sans écrire au journal.
