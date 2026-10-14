# ADR-0008 : Crate forge-session — session, commandes et undo/redo (ouverture du jalon 1)

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-14)
- **Date :** 2026-10-14
- **Décideurs :** Raphaël Olivier

## Contexte

Le jalon 0 est clos (verdict egui validé sur Android, revue externe intégrée, CI verte). Le jalon 1 vise : noyau projet + **undo** + import/export SVG/DXF/STL + i18n Fluent + `help_id`. La présente ADR en est l'ADR d'ouverture : elle tranche la conception de la crate **session/commandes** (point d'architecture nº1).

Aujourd'hui, le démonstrateur `apps/omnifab-desktop` porte lui-même :

- la **projection** (état reconstruit par rejeu : `DemoApp::apply` sur le graphe forge-values) ;
- la **validation avant écriture** (`DemoApp::record` : un événement invalide ne pollue jamais le journal) ;
- le **rejeu** à l'ouverture (`rebuild_from_journal`).

Cette logique est induplicable proprement pour l'application Android, l'undo n'existe pas encore, et l'UI mélange intentions métier et persistance. La frontière naturelle est actée en phase 0 : **forge-doc = persistance/rejeu bruts, session = intentions utilisateur + undo/redo**.

Contraintes : verrous nº1 (conteneur ZIP) et nº2 (event sourcing, undo illimité) ; ADR-0001 (journal JSON Lines, types stables, snapshots **non autoritaires**) ; ADR-0003 (erreurs typées `OF-<DOM>-<NNNN>`, registre CI) ; wasm-friendly ; zéro dépendance UI ; rustdoc obligatoire sur l'API publique.

## Décision

Nous créons la crate **`crates/forge-session`** (MPL-2.0), qui possède le `Document` forge-doc et les projections par domaine, expose une couche de **commandes** (une intention utilisateur = une commande typée), et implémente **undo/redo par curseur sur le journal**.

1. **Frontière.** forge-doc reste agnostique des domaines (« bytes ↔ événements ») ; forge-session est la seule couche qui connaît les noms d'événements métier, leur validation et l'état projeté. Les applications (desktop, Android) ne touchent plus `Document` directement : elles construisent des commandes et lisent les projections.

2. **Commandes.** Chaque intention est une struct Rust (ex. `SetInput`, `SetExpression`, plus tard `ImportSvg`). Une commande se **planifie contre l'état courant** : `plan(&état) -> Result<Transaction, SessionError>`, produisant 1..n `NewEvent` partageant un identifiant de transaction (`NewEvent::in_txn`, déjà en place — schéma A de la revue). `Session::execute` applique d'abord les événements aux projections ; si une projection refuse, **rien n'est écrit au journal**. Généralisation du `record()` du démonstrateur : un journal chargé se rejoue toujours, une commande échouée n'y laisse aucune trace.

3. **Projections par domaine, enregistrables.** Trait `Projection { fn apply(&mut self, &Event) -> Result<(), …>; }`. Au jalon 1, la seule projection enregistrée est le graphe de liaisons forge-values. Les types d'événements inconnus sont tolérés et listés (compatibilité avant : journal écrit par une version plus récente), comme dans le démonstrateur. Les domaines futurs (esquisse, CAO, CAM) enregistreront leurs projections sans modifier la crate.

4. **Undo/redo par curseur (en mémoire, dans la session).**
   - La session tient un curseur `applied` = nombre d'événements projetés. Les événements défaits restent en mémoire au-delà du curseur : le **redo** les re-projette.
   - **Undo** : le curseur recule d'un groupe (une transaction entière, ou un événement isolé) ; l'état est reconstruit par **rejeu de `events[0..applied]`** dans chaque projection. Coût accepté au jalon 1 (journaux courts) ; optimisation suspendue (checkpoints incrémentaux).
   - **Nouvelle commande après undo** : les événements défaits sont **abandonnés** (retirés du journal en mémoire), comme dans tout éditeur : une nouvelle action vide la pile redo. Les blobs devenus orphelins sont ramassés (un blob survit s'il est référencé par un événement vivant).
   - **Sauvegarde** : le fichier contient le journal linéaire visible `events[0..applied]`. Un projet sur disque se rejoue toujours dans l'ordre, sans événements « futurs ». À la réouverture, la pile d'undo est vide (comportement standard : l'historique d'undo n'est pas persisté).
   - Granularité : **une commande = un groupe undo**, quel que soit son nombre d'événements (transaction).

5. **API publique (esquisse).** `Session::new/open/save` ; `execute(cmd)` ; `undo()/redo()` ; `can_undo()/can_redo()` ; accesseurs aux projections (ex. `graph()`). Toute l'API est rustdoc-ée et testée en CI ; feature `native` (fichiers) isolée comme dans forge-doc.

6. **Erreurs.** `SessionError` (thiserror, ADR-0003), codes `OF-SES-*` au registre `docs/error-codes.md`, vérification bidirectionnelle par CI comme pour DOC/VAL (rejet de commande par validation, rejet par projection, blob orphelin, etc. — numéros définitifs à l'implémentation).

7. **Portée du jalon 1 hors ADR.** L'import/export (forge-io, SVG/DXF/STL) fera l'objet d'un ADR propre — il se branchera sur forge-session par des commandes produisant événements + blobs. Fluent et `help_id` sont des conventions à appliquer, pas des décisions structurantes.

## Alternatives rejetées

- **Statu quo (projection dans omnifab-desktop)** : impossible à partager avec Android ; l'UI mélangerait intentions et persistance ; l'undo n'aurait pas de place naturelle.
- **Commandes dans forge-doc** : forge-doc deviendrait connaisseur des domaines métier (noms d'événements, validation) ; la frontière et le versionnement indépendant des crates le défendent.
- **Deux crates (session + commandes séparées)** : pas de second consommateur aujourd'hui ; frontière modifiable par ADR plus tard si un besoin réel (scripting/batch) apparaît.
- **Undo par événements compensatoires (append-only pur)** : chaque commande devrait définir son inverse, et le rejeu devrait comprendre les sémantiques de compensation — une vieille version ignorant un nouveau type compensatoire reprojeterait un état **faux** (casse la compatibilité avant) ; le journal gonflerait indéfiniment. Les actions métier « sémantiques » (ex. supprimer une feature) restent de vrais événements et sortent de ce cadre.
- **Snapshots autoritaires pour l'undo** : violerait ADR-0001 (snapshots = accélérateurs non autoritaires, jamais source de vérité).
- **Persistance du curseur/pile d'undo dans le projet** : état caché, divergence entre appareils, fichier contenant des événements « futurs » que l'utilisateur croit disparus — débogage impossible à la main.

## Conséquences

- **Positives** : undo/redo identiques sur desktop et Android ; journal sur disque toujours linéaire et rejouable ; commande invalide = zéro trace ; frontière forge-doc/forge-session testable ; les domaines futurs se branchent sans toucher au noyau.
- **Négatives / coûts acceptés** : une couche d'indirection de plus ; rejeu complet à chaque undo au jalon 1 ; historique redo perdu dès qu'une nouvelle action suit un undo (standard, à documenter pour l'utilisateur) ; l'abandon d'événements défaits retire ces actions de l'audit du fichier (le journal sur disque reflète l'historique *retenu*, pas toutes les tentatives).
- **Décisions futures suspendues** : checkpoints incrémentaux (undo sans rejeu complet) ; journal intégral optionnel (audit) ; commandes scriptables par lot ; compaction du journal.
