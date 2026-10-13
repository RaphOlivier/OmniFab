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

Droits de domaine attribués : `VAL` (forge-values), `DOC` (forge-doc).
Réservés : `UNT` (forge-units), `GEO`, `CAD`, `CAM`, `GCO`, `LNK`, `IO`, `PNT` —
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
