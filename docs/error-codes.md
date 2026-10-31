# Registre central des codes d'erreur (ADR-0003)

> **Source unique de vérité** pour tous les codes `OF-<DOMAINE>-<NNNN>`.
>
> Règles :
> - chaque variante d'erreur du noyau porte un code **stable** : jamais renommé, jamais réutilisé ;
> - un code absent de ce registre fait échouer la CI : chaque crate concernée embarque un test
>   qui vérifie la présence de ses codes dans ce fichier ;
> - un code n'apparaît qu'une seule fois (pas de doublon inter-crates) ;
> - le message d'exécution, la page d'aide intégrée (`help_id`), le manuel et le site web
>   dérivent tous de ce registre — la colonne « Aide intégrée » sera reliée aux pages d'aide
>   dès le jalon 1 (système d'aide).

Droits de domaine attribués : `VAL` (forge-values), `DOC` (forge-doc), `SES` (forge-session), `GEO` (forge-geom), `IO` (forge-io), `I18` (forge-i18n), `GCS` (forge-gcs), `CAD` (forge-cad), `WORK` (omnifab-desktop).
Réservés : `UNT` (forge-units), `CAM`, `GCO`, `LNK`, `PNT` —
à attribuer à la création de leur crate, via un ADR si la frontière bouge.

## forge-values (VAL) — moteur de liaisons

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-VAL-0001 | Cycle dans le graphe de liaisons | Une expression référence, directement ou transitivement, la valeur qu'elle définit | Supprimer l'une des références circulaires |
| OF-VAL-0002 | Référence à une valeur inconnue | Faute de frappe dans un nom, ou valeur non encore créée | Corriger le nom, ou créer la valeur avant l'expression |
| OF-VAL-0003 | Expression invalide | Syntaxe hors du sous-ensemble déterministe accepté (cf. rustdoc de forge-values) | Corriger la syntaxe |
| OF-VAL-0004 | Erreur d'évaluation | Division par zéro, racine carrée d'un nombre négatif, résultat non fini (NaN ou infini) | Corriger les données d'entrée ou l'expression |
| OF-VAL-0005 | Valeur inconnue | Lecture d'un nom absent du graphe | Vérifier le nom demandé |
| OF-VAL-0006 | Valeur d'entrée non finie | Écriture d'un NaN ou d'un infini par set_input | Corriger la valeur d'entrée |
| OF-VAL-0007 | Incohérence d'unités à la définition | Expression ou entrée déclarée d'une unité incompatible avec ses références (ex. `deg` lisant du `mm`) | Corriger l'unité déclarée |
| OF-VAL-0008 | Alias réservé | Création sous un préfixe système (`esquisse.`, tableaux) ou écrasement d'un nœud d'une autre origine | Choisir un autre nom |
| OF-VAL-0009 | Alias référencé | Suppression `Simple` demandée alors que des formules ou cotes lisent encore cet alias | Utiliser Cascade ou Remplacer, ou corriger d'abord les références |
| OF-VAL-0011 | Chaîne de dépendances trop profonde | Le coût d'évaluation (profondeur d'AST plus sauts de dépendances) dépasse la borne de 1 024 | Restructurer les formules intermédiaires |

Code réservé, non encore attribué : OF-VAL-0010 (table dimensionnelle, design de phase D — ADR-0025/0026).

## forge-doc (DOC) — conteneur et journal d'événements

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-DOC-0001 | Version du conteneur non supportée | Projet écrit par une version plus récente d'OmniFab | Mettre à jour le logiciel |
| OF-DOC-0002 | Format de conteneur inconnu | Le manifeste n'annonce pas `omnifab-document` | Vérifier que le fichier est bien un projet OmniFab |
| OF-DOC-0003 | Journal corrompu | Ligne non conforme dans `events.jl` | Restaurer une copie saine du projet |
| OF-DOC-0004 | Séquence d'événements incohérente | Journal altéré, trou de séquence | Restaurer une copie saine du projet |
| OF-DOC-0005 | Version de schéma trop récente | Événements écrits par une version plus récente du logiciel | Mettre à jour le logiciel |
| OF-DOC-0006 | Migration manquante | Étape de migration non enregistrée pour ce type/version (bug logiciel) | Compléter le registre de migrations |
| OF-DOC-0007 | Entrée absente du conteneur | `manifest.json` ou `events.jl` absent du ZIP | Vérifier l'intégrité du fichier projet |
| OF-DOC-0008 | Erreur d'archive ZIP | Archive endommagée ou illisible | Restaurer une copie saine du projet |
| OF-DOC-0009 | Erreur de sérialisation | JSON invalide dans le conteneur | Restaurer une copie saine du projet |
| OF-DOC-0010 | Erreur d'entrée/sortie | Échec de lecture/écriture fichier (feature `native`) | Vérifier chemins et permissions |
| OF-DOC-0011 | Entrée trop volumineuse | Une entrée (manifeste, journal, snapshot, blob) dépasse la taille maximale acceptée | Fichier projet anormal — restaurer une copie saine |
| OF-DOC-0012 | Trop d'entrées dans le conteneur | L'archive dépasse le nombre maximal d'entrées (garde-fou « bombe ZIP ») | Fichier projet anormal — restaurer une copie saine |
| OF-DOC-0013 | Nom de blob invalide | Nom vide, traversée de répertoire (`..`), chemin absolu, séparateur Windows ou octet nul | Corriger le nom du blob |
| OF-DOC-0014 | Blobs cumulés trop volumineux | La somme des objets binaires dépasse le plafond global du chargement (garde-fou « bombe ZIP » cumulative) | Fichier projet anormal — restaurer une copie saine |
| OF-DOC-0015 | Entrée en doublon dans le conteneur | L'archive contient deux entrées de même nom (archive forgée) | Fichier projet anormal — restaurer une copie saine |
| OF-DOC-0016 | Fichier projet trop volumineux | Le fichier .omnifab dépasse le budget d'octets du chargement avant même d'être lu (audit n°3 A3-07) | Fichier anormal — vérifier le fichier |

## forge-session (SES) — session, commandes et undo/redo

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-SES-0001 | Commande refusée à la planification | Données invalides : nom vide ou hors du jeu du lexeur, expression vide, nombre non fini, format d'import non supporté | Corriger la commande |
| OF-SES-0002 | Rejet par une projection | L'événement viole une règle du domaine (ex. expression référençant une valeur inconnue) | Corriger la commande ; le journal est intact |
| OF-SES-0003 | Rien à annuler | Undo demandé alors que tout est déjà annulé | — |
| OF-SES-0004 | Rien à rétablir | Redo demandé sans événement défait en attente | — |
| OF-SES-0005 | Données d'événement invalides | Journal altéré : champ manquant ou de type inattendu pour un type connu | Restaurer une copie saine du projet |
| OF-SES-0006 | Blob refusé par le document | Nom de blob invalide (OF-DOC-0013) ou taille hors bornes (OF-DOC-0011) au dépôt d'une commande | Corriger le nom ou le contenu du blob |

## forge-geom (GEO) — géométrie minimale

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-GEO-0001 | Indice de sommet hors du maillage | Triangle référençant un sommet absent | Corriger le maillage source |
| OF-GEO-0002 | Coordonnée non finie | NaN ou infini dans une coordonnée | Corriger les données d'entrée |

## forge-io (IO) — import/export

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-IO-0001 | Entrée trop volumineuse | Fichier dépassant le budget d'octets du parseur | Vérifier le fichier, ou ajuster le budget |
| OF-IO-0002 | STL binaire invalide | Structure incohérente (garde pour les extensions du lecteur) | Vérifier l'intégrité du fichier |
| OF-IO-0003 | Budget de triangles dépassé | Maillage au-delà du budget du parseur, ou export de plus de `u32::MAX` triangles | Ajuster le budget ou simplifier le maillage |
| OF-IO-0004 | STL ASCII invalide | Syntaxe hors du sous-ensemble accepté, ou octets non UTF-8 | Vérifier le fichier d'entrée |
| OF-IO-0005 | Format non reconnu | Ni STL binaire cohérent, ni STL ASCII | Vérifier le format du fichier |
| OF-IO-0006 | Géométrie invalide à l'import | Coordonnée non finie dans les données importées | Vérifier le fichier d'entrée |
| OF-IO-0007 | SVG invalide | Document refusé par le parseur usvg (syntaxe, XML malformé) | Vérifier le fichier d'entrée |
| OF-IO-0008 | Budget vectoriel dépassé | Tracés ou sommets au-delà des budgets du parseur SVG/DXF | Ajuster le budget ou simplifier le document |
| OF-IO-0009 | DXF invalide | Document refusé par le parseur dxf | Vérifier le fichier d'entrée |

## forge-i18n (I18) — internationalisation Fluent et aide intégrée

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-I18N-0001 | Clé absente du catalogue | Message absent du FTL, ou message sans valeur | Ajouter la clé au FTL embarqué |
| OF-I18N-0002 | Ressource FTL invalide | Syntaxe refusée par le parseur, message en doublon, ou identifiant de langue invalide | Corriger le fichier FTL |
| OF-I18N-0003 | Page d'aide incomplète | Attribut `titre`, `description` ou `exemple` manquant | Compléter la page dans le FTL |
| OF-I18N-0004 | Erreur de mise en forme | Argument manquant ou inconnu dans un pattern | Vérifier les arguments passés à la clé |
## forge-gcs (GCS) — solveur de contraintes 2D (planegcs via FFI)

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-GCS-0001 | Identifiant inconnu | Point, ligne, cercle, arc, paramètre ou contrainte hors des identifiants émis par le solveur | Vérifier l'identifiant utilisé |
| OF-GCS-0002 | Échec de résolution du croquis | Croquis conflictuel ou sans solution pour l'algorithme choisi | Corriger les contraintes ou changer d'algorithme |
| OF-GCS-0003 | Reproductibilité non garantie | L'arène de paramètres n'a pas maintenu des adresses croissantes (FreeCAD #33399) | Réduire le croquis ou signaler le bug |
| OF-GCS-0004 | Appel invalide à l'ABI C | Handle nul, valeur non finie, ou exception C++ rattrapée dans le shim | Corriger les paramètres d'appel (bug logiciel sinon) |
| OF-GCS-0005 | Solution invalide | Résolution réussie mais géométrie résultante invalide (statut SuccessfulSolutionInvalid de planegcs) | Corriger les contraintes ou les valeurs de départ |

## forge-cad (CAD) — CAO paramétrique (noyau OCCT, ADR-0023)

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-CAD-0001 | Échec d'une opération OCCT | Construction, booléen, tessellation ou écriture refusés par le noyau CAO | Simplifier la géométrie ou vérifier l'installation OCCT (liaison dynamique, version >= 7.8) |
| OF-CAD-0002 | Forme nulle, invalide ou entrée invalide | Dimensions non finies ou <= 0, contour dégénéré, booléen sans résultat, ou import non solide | Corriger la géométrie ou les entrées source |
| OF-CAD-0003 | Fichier CAO illisible | STEP ou BREP refusé par le lecteur OCCT | Vérifier le fichier d'entrée |
| OF-CAD-0004 | Budget de ressources dépassé | Fichier dépassant les budgets d'octets ou de formes du parseur (garde-fou ADR-0009) | Vérifier le fichier, ou ajuster les budgets |

## omnifab-desktop (WORK) — fil de travail de l'interface

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-WORK-0001 | Travail terminé en échec inopiné | Panique du fil de travail pendant un import, un export ou une résolution (bug logiciel, audit n°3 N8) | Signaler le bug ; le fil survit et le travail suivant est traité |
