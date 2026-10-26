# ADR-0019 — Moteur de liaisons forge-values (jalon 2, phase E)

- **Statut : accepté** — décisions validées par Raphaël Olivier le 2026-10-19 (digest, 4 axes sur 4) ; **design détaillé validé le 2026-10-20** (digest 4 axes + amendement suppression + `sketch.constraint.updated`) — exécution libre selon la gouvernance (critère de sortie : CI verte + capitalisation). Les axes validés ne se rouvrent pas sans demande explicite.
- **Date :** 2026-10-19 (décisions) / 2026-10-20 (design)
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 2, phase E (forge-values, forge-session, omnifab-desktop)

## Décisions tranchées (2026-10-19)

1. **Périmètre : noyau + panneau de liaisons desktop.** La phase E livre le moteur complet ET l'interface dans le démonstrateur : alias nommés, formules, valeurs calculées, erreurs traduites (i18n) ; le panneau de paramètres actuel (width_mm/height_mm/diagonale) devient le panneau de liaisons. *Rejeté* : noyau seul, interface reportée au jalon 3.
2. **Sémantique tableur (exigence fondateur ADR-0012) : chaque cellule d'un tableau = un nœud du graphe**, référencé par alias dans les formules ; recalculs en cascade par évaluation paresseuse (ADR-0002) ; cycle = erreur typée OF-VAL-0001. *Rejeté* : le tableau comme unique formule retournant un tableau (ne permet pas la référence par cellule).
3. **Édition des formules : champ simple + erreurs claires** dès la phase E ; éditeur riche (colorisation, autocomplétion des alias) reporté au jalon 3 avec l'éditeur nodal. *Rejeté* : éditeur riche dès la phase E (périmètre gonflé sans valeur immédiate).
4. **Cotes d'esquisse pilotées par expressions forge-values : intégrées à la phase E** (plan acté ADR-0013 : « cotes = paramètres fixes de l'arène, phase E : pilotage forge-values »). Une cote d'esquisse peut être une expression ; le solveur reçoit la valeur calculée, jamais l'expression (frontière forge-values → forge-gcs inchangée). *Rejeté* : report au jalon 3.

## Bases déjà actées (ne pas re-trancher)

- ADR-0002 : évaluation paresseuse, marquage dirty, pull + tri topologique, expressions déterministes ; « recalculer tout » obligatoire avant usinage (à porter au jalon CAM).
- ADR-0006 : grammaire des expressions (identifiants Unicode, `^` associatif à droite, `si(cond; a; b)` paresseux, profondeur 64 → OF-VAL-0003, espaces de noms par point).
- ADR-0007 phase 1 : `unit: Option<Unit>` par nœud, cohérence à la définition (OF-VAL-0006) ; les événements portant des expressions passent en schema_version 2 (grammaire v1 incluse, migration vide).
- Verrou n°6 : un moteur unique — alias et nodal sont deux vues du même graphe, zéro incohérence.

## Design détaillé (validé le 2026-10-20)

Ce design couvre le « reste à designer » : schémas d'événements v2 + migrations, namespaces d'alias, cotes d'esquisse pilotées, panneau de liaisons, erreurs, checklist.

### 1. Schémas d'événements v2 + migrations

#### `values.input.set` v1 → v2 (ADR-0007 phase 1 : métadonnée d'unité)
```json
{ "name": "largeur", "value": 800.0, "unit": "mm" }
```
- `unit` **optionnel** (absent = sans unité) ; valeurs acceptées : les codes d'unités de forge-units (`mm`, `deg`, … — pas de symbole composé en phase E).
- Migration v1→v2 : **vide** (champ absent = `None`), même patron que `geom.mesh.imported`.

#### `values.expression.set` v1 → v2
```json
{ "name": "diagonale", "expression": "sqrt(...)","unit": null }
```
- `unit` optionnel, déclaré à la définition ; cohérence vérifiée seulement à la définition (ADR-0007 phase 1) : une expression d'unité `mm` référencée par une expression déclarée `deg` → refus. Le graphe ne fait **pas** d'arithmétique d'unités (phase 2 = jalon 3).

#### `sketch.constraint.added` v1 → v2 (cote pilotée, décision 4)
```json
{ "contrainte": { "type": "Distance", "p1": 0, "p2": 1,
    "valeur": { "type": "fixe", "mm": 100.0 } } }
{ "contrainte": { "type": "Distance", "p1": 0, "p2": 1,
    "valeur": { "type": "expression", "texte": "largeur / 2", "alias": "esquisse.demi-largeur" } } }
```
- `valeur` devient **polymorphe** : `fixe` (valeur en mm, comportement v1) ou `expression` (texte + alias obligatoire).
- Migration v1→v2 : `{ "valeur": 100.0 }` → `{ "valeur": { "type": "fixe", "mm": 100.0 } }`.
- Le solveur reçoit **la valeur calculée** (résolue à la demande de résolution, sur le fil de travail), jamais l'expression — frontière forge-values → forge-gcs inchangée (ADR-0013/0018). Une cote expression dont le nœud est en erreur → la résolution échoue avec l'erreur forge-values traduite.
- Autres contraintes (Horizontale, etc.) inchangées en v2.

#### `sketch.constraint.updated` (nouveau type, v1 — validé 2026-10-20)
```json
{ "contrainte": { "type": "Distance", "p1": 0, "p2": 1,
    "valeur": { "type": "expression", "texte": "hauteur / 3", "alias": "esquisse.tiers" } }
```
- Remplace **à id conservé** la contrainte portée par l'événement `sketch.constraint.added` de même index séquentiel (le journal reste append-only : l'événement `updated` porte la contrainte complète, la projection remplace). Servira au mode « Remplacer » quand une cote référence l'alias supprimé, et au jalon 3 (édition de contraintes).

#### `values.alias.removed` (nouveau type, v1) — modes validés 2026-10-20
```json
{ "name": "ancien" }
```
- Supprimer un alias = commande (undoable), **en trois modes** (amendement Raphaël 2026-10-20) : `Simple` (refus OF-VAL-0009 si référencé), `Cascade` (supprime l'alias ET récursivement chaque formule utilisateur qui le référence), `Remplacer { par }` (substitue l'alias par un autre alias existant dans toutes les formules et cotes — via `values.expression.set` re-émis et `sketch.constraint.updated` pour les cotes — puis supprime). Refus si la substitution crée un cycle ou si l'alias de remplacement n'existe pas. Un seul txn = un seul undo. Cotes système : non supprimables directement.

### 2. Namespaces d'alias

- **Un seul espace global** (verrou n°6 : alias et nodal = deux vues du même graphe), identifiants Unicode à points (grammaire ADR-0006 déjà en place).
- **Préfixes réservés** (système) : `esquisse.` (cotes d'esquisse auto-nommées). `param.`, `doc.`, `biblio.` réservés pour la suite (matrice §10) sans être émis en phase E.
- Un alias utilisateur **ne peut pas** créer de nœud sous un préfixe réservé ni en écraser un → erreur **OF-VAL-0008** « alias réservé ».
- Un alias utilisateur ne peut pas être nommé comme un nœud réservé existant ; collision alias/alias = renommage interdit en création (le nœud existe déjà → c'est une redéfinition, l'`ensure_node` actuel l'écrase silencieusement → **corrigé** : `set_input`/`set_expression` sur un nœud d'un autre genre que celui créé par l'usager = refus explicite, pas d'écrasement silencieux).
- Sémantique tableur (exigence ADR-0012) : **actée sans UI tableau en phase E** — chaque cellule future d'un tableau sera un nœud nommé `<tableau>.<colonne><ligne>` (ex. `decoupe.B3`), donc référençable par toute formule et cible du même recalcul en cascade ; un tableau réservera son préfixe à sa création via le même mécanisme OF-VAL-0008. L'UI tableau arrive au jalon 3 avec l'éditeur nodal.

### 3. Nouveaux codes d'erreur (registre)

| Code | Signification | Cause probable | Action corrective |
|---|---|---|---|
| OF-VAL-0007 | Incohérence d'unités à la définition | Expression ou entrée déclarée d'une unité incompatible avec ses références (ex. `deg` lisant du `mm`) | Corriger l'unité déclarée |
| OF-VAL-0008 | Alias réservé | Création/renommage sous un préfixe système (`esquisse.`, tableaux) ou écrasement d'un nœud d'un autre genre | Choisir un autre nom |
| OF-VAL-0009 | Alias référencé | Suppression `Simple` demandée alors que des formules ou cotes lisent encore cet alias | Utiliser Cascade/Remplacer, ou corriger d'abord les références |

OF-VAL-0001..0006 inchangés. Tests bidirectionnels registre (patron groupe B).

### 4. Commandes forge-session (nouvelles/évoluées)

- `SetInput` v2 : + `unit: Option<Unite>`.
- `SetExpression` v2 : + `unit: Option<Unite>` ; **refus si le nœud existe déjà comme nœud système** (OF-VAL-0008).
- `RemoveAlias` (nouveau) : planifie `values.alias.removed` selon le **mode** (Simple/Cascade/Remplacer — cf. §1) ; Cascade émet un `values.alias.removed` par alias supprimé, Remplacer ré-émet les `values.expression.set` substitués + `sketch.constraint.updated` pour les cotes concernées — tout en un txn. Refus OF-VAL-0009 en mode Simple si référencé ; refuse tout alias sous `esquisse.` (les cotes sont immuables comme leurs contraintes, seule la substitution via Remplacer les touche).
- `AddConstraint` (Distance) v2 : `valeur: ValeurCote::{Fixe, Expression}` ; le plan crée AUSSI le nœud `esquisse.<alias>` dans le graphe (deux événements, même txn = un seul undo) ; refus OF-VAL-0002/0003/0007 au plan si l'expression est invalide ou incohérente en unités (une cote = toujours des mm à la frontière du solveur → l'expression doit être d'unité `mm` ou sans unité).
- Suppression de contrainte : n'existe pas en phase E (phase D : ajout seulement) → la cote-expression ne se supprime pas non plus ; **RemoveAlias refuse un alias sous `esquisse.`**.

### 5. forge-values : changements du noyau

- `Node` porte `unit: Option<Unite>` (métadonnée, jamais état d'expression — ADR-0007).
- `set_input`/`set_expression` prennent l'unité ; vérif de cohérence **à la définition** : une expression déclarée d'unité U ne peut référencer que des nœuds d'unité U ou sans unité (et réciproquement un nœud sans unité peut tout lire) → OF-VAL-0007. Une cote d'esquisse (alias `esquisse.*`) est forcément `mm` (contrainte système à la création).
- `remove(name, mode)` : retire le nœud + arêtes selon le mode (cf. §1) ; refus OF-VAL-0009 en mode Simple si référencé.
- Finitude : inchangée (OF-VAL-0004 à l'évaluation, 0006 à l'écriture).

### 6. Panneau de liaisons desktop (remplace le panneau de paramètres)

- Liste des alias du graphe : nom (gras si calculé), valeur (DragValue pour une entrée ; champ texte simple pour une expression), unité (liste déroulante : sans unité/mm/deg), badge d'erreur i18n à la ligne concernée (message du registre traduit Fluent, clic droit → page d'aide du code).
- « + Nouvel alias » (nom, valeur ou expression, unité) ; suppression (poubelle) → si l'alias est référencé, **pop-up egui** (amendement Raphaël 2026-10-20) listant les formules concernées : « Supprimer aussi ces formules (N) » / « Remplacer par… » (choix parmi les autres alias) / « Annuler » ; suppression directe sinon.
- `width_mm`/`height_mm` deviennent `largeur`/`hauteur` (unité mm, entrées) ; `diagonale` (expression) — les clés FTL et le rectangle paramétrique suivent ; le projet vierge pose les trois événements comme aujourd'hui.
- Les cotes d'esquisse (`esquisse.*`) : lignes en lecture seule (valeur calculée), pas de suppression.
- L'expression d'une cote Distance se pose depuis l'atelier esquisse (sélection 2 points → contrainte Distance → dialogue cote : valeur fixe OU expression+alias).
- Tampon d'édition persistant par champ (leçon round 6 egui), `help_id` sur chaque widget, FTL complet, aucune chaîne dans le code.

### 7. Checklist de validation manuelle (livrée au début — gouvernance)

À exécuter sur `cargo run -p omnifab-desktop` après la phase E :
1. Panneau de liaisons : largeur/hauteur modifiables au glissement (tampon persistant), diagonale recalculée affichée.
2. Créer un alias `epaisseur` (entrée, mm) ; formule `diagonale + epaisseur` sur un nouvel alias `total` → cascade visible.
3. Erreur volontaire : alias `boucle = boucle + 1` → message OF-VAL-0001 en clair, rien au journal (undo intact).
4. Cycle traversé : `a = b`, `b = a` → refus/erreur claire, aucune valeur gelée.
5. Unité : alias `angle` (deg) dont l'expression lit `largeur` (mm) → OF-VAL-0007 affiché, la saisie reste corrigeable.
6. Suppression d'alias référencé (`total` lit `epaisseur` → poubelle sur `epaisseur`) → refus OF-VAL-0009 traduit ; suppression de `total` OK.
7. Alias sous préfixe réservé : créer `esquisse.mien` → refus OF-VAL-0008.
8. Cote pilotée : atelier esquisse, rectangle contraint, cote Distance en expression `largeur / 2` alias `esquisse.demi` → résolution : la cote vaut la moitié de la largeur courante ; changer `largeur` → re-résolution, la géométrie suit.
9. Undo/redo : chaque action ci-dessus s'annule/rétablit par groupe (une commande = un groupe), y compris la cote pilotée (contrainte + nœud ensemble).
10. Enregistrement/ouverture : fermer, rouvrir le projet → alias, expressions, unités et cotes pilotées intactes (journaux v2 relus, projets v1 ouverts sans changement de comportement).
11. Aide intégrée : clic droit sur chaque widget du panneau → page d'aide complète.

### 8. Découpage en lots (exécution libre après validation)

- **Lot 1** : forge-values (unités, remove, refus d'écrasement, OF-VAL-0007/0008/0009 + registre + tests bidirectionnels) + forge-session (SetInput/SetExpression v2 + migrations, RemoveAlias, AddConstraint Distance v2 + nœud `esquisse.*` + migration v1→v2) + tests.
- **Lot 2** : desktop — panneau de liaisons complet (FTL, aide, i18n erreurs), dialogue cote expression dans l'atelier, adaptation rectangle/démonstrateur, tests.
- **Lot 3** : convergence CI (protocole habituel : gardes, veille, oracle des logs, fermetures).

### Risques

- Arité des appels desktop (garde pré-push) ; egui DragValue/TextEdit (leçons rounds 4-6 : tampon persistant, engagement seulement si changement effectif) ; rustfmt (replis exacts de l'oracle) ; le JSON de `valeur` polymorphe doit rester déterministe au rejeu (serde tag interne).

---

*Historique du fichier : décisions et design vivaient en deux fiches distinctes dans la Knowledge de travail ; fusionnées au push au dépôt le 2026-10-26 (numéro unique = fichier unique).*
