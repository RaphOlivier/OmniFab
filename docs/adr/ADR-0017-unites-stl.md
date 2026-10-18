# ADR-0017 : Unités des maillages STL à l'import

- **Statut :** accepté
- **Date :** 2026-10-18
- **Décideurs :** Raphaël Olivier (design entériné le 2026-10-17 — validation C2 round 5 ; validé pour implémentation le 2026-10-18)

## Contexte

Le format STL ne porte aucune unité : les coordonnées y sont des nombres sans dimension. Blender exporte en mètres, FreeCAD en millimètres ; OmniFab lit et écrit des millimètres (newtypes forge-units, tolérance 1e-6 mm). Un STL exporté par Blender s'affiche donc mille fois trop petit dans le démonstrateur. Le round 5 de la validation C2 (2026-10-17) a acté le principe : détection automatique avec confirmation explicite à l'import.

## Décision

À l'import d'un maillage STL, si la plus grande dimension du modèle (le plus grand côté de la boîte englobante, en millimètres) est strictement positive et inférieure à 1,0 mm, l'application propose un choix explicite : « Mètres (× 1000) », « Millimètres (tel quel) » ou « Annuler l'import ». Le facteur d'échelle est persisté dans l'événement `geom.mesh.imported` schéma v2 (champ `echelle`, nombre) : le blob conserve les octets bruts et la relecture les re-parse, le facteur doit donc vivre dans l'événement pour survivre au rejeu (ADR-0001/0011). Les journaux v1 sont migrés à l'ouverture par le registre de migrations : `echelle` est posé à 1,0, comportement inchangé. L'échelle est appliquée au maillage re-parsé pour le rendu (cache) et pour l'export STL, qui écrit toujours des millimètres. Une échelle non finie ou non positive est refusée au plan de la commande (`CommandRejected`, OF-SES-0001) : aucun nouveau code d'erreur, aucune nouvelle dépendance.

## Alternatives rejetées

- **Unité par réglage global du logiciel** : l'unité est une propriété du fichier importé, pas de l'application — deux fichiers de la même session peuvent venir de logiciels différents.
- **Détection automatique sans confirmation** : un vrai modèle minuscule (bijou, micro-mécanique) serait mis à l'échelle en silence — destruction silencieuse de données, contraire aux principes du projet.
- **Unité dans le manifeste du conteneur ZIP** : l'échelle appartient à l'événement d'import — le manifeste est par document, l'historique mélangerait des origines différentes après undo/redo.
- **Mise à l'échelle des octets du blob** : les octets d'origine sont sacrés (ADR-0009/0011) — ré-exporter doit redonner le fichier d'origine ; le facteur est une donnée d'événement, pas une transformation du blob.

## Conséquences

- Un faux positif (vrai modèle minuscule) reste importable tel quel : le dialogue ne bloque jamais, « Millimètres » importe sans changement.
- Le seuil de 1,0 mm est une constante du démonstrateur (`SEUIL_METRES`), réglable sans ADR ; le futur `RapportImport` (lot D de l'ADR-0016, décision D2) généralisera la détection par format (DXF `$INSUNITS`, SVG…) et pourra reprendre ce mécanisme.
- Les journaux v1 existants migrent sans perte (échelle 1,0), migration testée en CI ; `SchemaTooNew` reste en vigueur pour les versions inconnues.
- L'export STL écrit toujours des millimètres : un modèle importé en mètres puis ré-exporté est converti, les octets d'origine restant intacts dans le projet.
