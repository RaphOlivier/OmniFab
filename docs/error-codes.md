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

Droits de domaine attribués : `VAL` (forge-values), `DOC` (forge-doc), `SES` (forge-session), `GEO` (forge-geom), `IO` (forge-io), `I18` (forge-i18n).
Réservés : `UNT` (forge-units), `CAD`, `CAM`, `GCO`, `LNK`, `PNT` —
à attribuer à la création de leur crate, via un ADR si la frontière bouge.

## forge-values (VAL) — moteur de liaisons

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-VAL-0001 | Cycle dans le graphe de liaisons | Une expression référence, directement ou transitivement, la valeur qu'elle définit | Supprimer l'une des références circulaires |
| OF-VAL-0002 | Référence à une valeur inconnue | Faute de frappe dans un nom, ou valeur non encore créée | Corriger le nom, ou créer la valeur avant l'expression |
| OF-VAL-0003 | Expression invalide | Syntaxe hors du sous-ensemble déterministe accepté (cf. rustdoc de forge-values) | Corriger la syntaxe |
| OF-VAL-0004 | Erreur d'évaluation | Division par zéro, racine carrée d'un nombre négatif, résultat non fini (NaN ou infini) | Corriger les données d'entrée ou l'expression |
| OF-VAL-0005 | Valeur inconnue | Lecture d'un nom absent du graphe | Vérifier le nom demandé |

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

## forge-session (SES) — session, commandes et undo/redo

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-SES-0001 | Commande refusée à la planification | Données invalides : nom vide, expression vide, nombre non fini | Corriger la commande |
| OF-SES-0002 | Rejet par une projection | L'événement viole une règle du domaine (ex. expression référençant une valeur inconnue) | Corriger la commande ; le journal est intact |
| OF-SES-0003 | Rien à annuler | Undo demandé alors que tout est déjà annulé | — |
| OF-SES-0004 | Rien à rétablir | Redo demandé sans événement défait en attente | — |
| OF-SES-0005 | Données d'événement invalides | Journal altéré : champ manquant ou de type inattendu pour un type connu | Restaurer une copie saine du projet |

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
