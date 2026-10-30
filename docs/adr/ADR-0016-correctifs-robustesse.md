# ADR-0016 : correctifs de robustesse — noyau, interop, démonstrateur (audits extérieurs)

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-17 — choix actés : A4 = plafond global 2 Gio réglable par l'appelant ; C1 = fil de travail std::thread + mpsc ; ordre d'implémentation A puis B puis C)
- **Date :** 2026-10-17
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 2 (après la phase C close), transverse (forge-session, forge-doc, forge-io, forge-values, omnifab-desktop)
- **Origine :** trois audits IA extérieurs du 2026-10-17 (Gemini, Claude, ChatGPT) ; chaque constat **vérifié dans le dépôt** sur `6f74f296` avant inclusion (registre complet : fiche architecture, « Dette connue — audits extérieurs »). Les faux positifs des audits (FFI, dpi SVG, rename Windows) sont exclus.

## Contexte

Les audits convergent sur des défauts réels dans trois zones : le contrat tout-ou-rien de la session (une commande refusée détruit la pile redo), la fiabilité de la sauvegarde (ordre `.bak`/`.tmp` dangereux, sauvegarde qui tue le redo en mémoire), et la fidélité des imports/exports (POLYLINE fermée, SVG après `Z`, export SVG sans métadonnées). S'y ajoutent des trous d'invariants du conteneur (noms de blobs, budget cumulé, snapshot non borné, migrations non branchées) et des défauts du démonstrateur (parses sur le fil UI, cache en échec re-parsé à chaque image, Cmd macOS, perte de travail silencieuse). Aucun n'est couvert par une ADR existante : ce sont des **défauts d'implémentation ou des durcissements**, à corriger **avant** la phase D (esquisse) et le jalon 3.

**Méthode imposée** : un point = un commit ; le test qui échoue est rédigé **avant** le correctif ; CI verte (issues relues, `get_commit(full_patch)` après chaque push) entre chaque point. Ordre proposé : A1→A8, puis B1→B4, puis C1→C5.

## Décision

### Lot A — noyau (forge-session, forge-doc, forge-values)

**A1 — `Session::execute` réellement transactionnel.** Réordonner : (1) appliquer les événem
ents aux projections (échec → `replay_applied()`, erreur, **rien d'autre n'a été touché — le redo est intact**) ; (2) déposer les blobs (échec → retirer via `Document::remove_blob` les blobs déjà déposés par CE plan, puis `replay_applied()`, erreur) ; (3) seulement alors `truncate_pending()` (l'abandon du redo devient légitime : la commande a réussi, ADR-0008) ; (4) `gc_blobs` (les nouvelles références existent déjà dans les projections après l'étape 1) ; (5) `append` + `applied`. Le commentaire de module (« un refus laisse le journal intact ») devient vrai, y compris pour la pile redo.
*Tests :* `SetInput` → `undo()` → `SetExpression` invalide → `can_redo()` vrai et `redo()` restaure l'état ; un plan à plusieurs blobs n'en laisse aucun au refus du second.

**A2 — `save_to_path` à ordre sûr.** (1) Écrire `projet.tmp` + `sync_all` (échec → erreur, **le fichier original est intact**, `.tmp` nettoyé) ; (2) si le projet existe, `std::fs::copy(projet → projet.bak)` (l'original reste en place à chaque étape) ; (3) `rename(tmp → projet)` (atomique ; `std::fs::rename` **remplace** la destination existante sur les trois OS — doc std, écarte le faux « échec Windows » de l'audit ChatGPT). Une seule génération de `.bak` (inchangé).
*Tests :* deux puis trois sauvegardes du même chemin (fixture tmp) ; échec d'écriture simulé → original présent et inchangé.

**A3 — projections rigoureuses + migrations branchées.** `champ_nombre` : `as_u64()` + `usize::try_from` (refus des négatifs, fractionnaires, NaN → erreur typée au lieu de `as usize` silencieux). `GeomProjection::apply` : vérifier `schema_version` des types connus (v2 d'un type connu → refus explicite). Brancher enfin `MigrationRegistry` : l'ouverture du desktop déclare les types connus (`values.input.set`, `values.expression.set`, `geom.traces.imported`, `geom.mesh.imported`) en version courante — `SchemaTooNew` (ADR-0001) redevient atteignable ; plus de lecture d'un événement v2 comme du v1.
*Tests :* événement forgé à nombr
e négatif/fractionnaire → erreur ; événement `schema_version: 2` → refus ; journal à version inconnue → `SchemaTooNew`.

**A4 — conteneur durci.** `container::read` : valider chaque nom de blob via `validate_blob_name` (l'archive forgée ne contourne plus l'invariant de `set_blob`) ; nom vide après `objects/` et doublons d'entrées → erreurs typées (plus d'écrasement silencieux) ; **plafond global** `MAX_TOTAL_BLOB_BYTES` (défaut **2 Gio** cumulés, nouvelle erreur `OF-DOC-0014`) ; suppression de la copie intégrale `bytes.to_vec()` (`Cursor` sur le slice emprunté).
*Tests :* noms invalides (`objects/../x`, séparateur Windows, vide), doublons, budget cumulé dépassé.

**A5 — snapshot borné.** Refuser un snapshot dont `at_seq` dépasse la dernière séquence réelle du journal (aujourd'hui le condensat couvre tout le journal et peut correspondre).
*Test :* snapshot forgé `at_seq = 999` sur un journal de 3 événements → refus.

**A6 — migration tout-ou-rien.** `migrate_events` construit la liste complète puis remplace `self.events` uniquement si toutes les migrations réussissent (plus d'état partiellement migré en mémoire).
*Test :* migration échouant au nième événement → journal inchangé.

**A7 — identifiants v7 horodatés par l'horloge injectable.** `Document::append` et le `txn` de `Session::execute` tirent l'horodatage du trait `Clock` (précision ms) pour construire l'UUIDv7 (`uuid::Builder::from_unix_timestamp_millis`), au lieu de `Uuid::now_v7()` (horloge système : non-déterminisme en test, comportement wasm32 incertain).
*Tests :* `FixedClock` → identifiants reproductibles ; deux appends → séquences monotones.

**A8 — invariants noms et finitude.** `SetInput`/`SetExpression` valident le nom contre le jeu du lexeur actuel (`[A-Za-z_]` puis `[A-Za-z0-9_]*`, à élargir en Unicode à l'implémentation de l'ADR-0006) : plus d'entrée créable mais jamais référençable. `ValueGraph::set_input` refuse les non-finis (réutilisation du garde de la revue point 5) : l'invariant tient au ni
veau du graphe, pas seulement de la commande.
*Tests :* nom avec espace/accent/chiffre initial refusé ; `set_input(NaN/±inf)` → erreur.

### Lot B — interop (forge-io)

**B1 — POLYLINE fermée.** Lire le drapeau de fermeture de `dxf::Polyline` (API exacte à vérifier sur docs.rs au moment du commit — drapeau 70 bit 1) et le passer à `pousse_polyligne` et `Path2::from_points` ; le bulge de fermeture s'applique alors au segment de fermeture (parité avec `LwPolyline`).
*Tests :* fixture `POLYLINE` fermée R12 → tracé fermé ; cas ouvert ; parité avec une `LWPOLYLINE` équivalente.

**B2 — SVG après `Z`.** `collecte` mémorise le point de départ du sous-tracé (dernier `MoveTo`) ; `Close` termine le sous-tracé **et** réinitialise le point courant sur ce départ (sémantique SVG) : les commandes après `Z` reprennent depuis le bon point, plus de segment perdu.
*Tests :* `M 0 0 L 10 0 L 10 10 Z L 5 5 L 7 7` → un tracé fermé [(0,0),(10,0),(10,10)] + un ouvert [(0,0),(5,5),(7,7)] (après transformation et inversion Y).

**B3 — export SVG exploitable hors OmniFab.** Calcul de la boîte englobante ; en-tête `width="{w}mm" height="{h}mm" viewBox="{minx} {miny} {w} {h}"` ; chaque `<path>` : `fill="none" stroke="#000000" stroke-width="0.1"`. Les coordonnées restent **exactes** (décision du round 4 intacte — la viewBox couvre les Y négatifs, aucune translation).
*Tests :* le texte exporté porte viewBox/width mm ; export → ré-import : mêmes dimensions ; un lecteur externe interprète à la bonne taille physique.

**B4 — STL binaire tolérant.** `est_binaire` : arithmétique en `u64` (`84 + 50 * compteur as u64` vs `len as u64`) → plus de débordement sur cible 32 bits (wasm32). Ordre de détection : binaire-exact → ASCII (`solid`) → **binaire tolérant** (`len ≥ 84 + 50n`, octets excédentaires ignorés) → erreur claire.
*Tests :* STL binaire avec bourrage final (fixture) importé ; compteur proche de `u32::MAX` sur un petit fichier → erreur propre, pas de panic.

### Lot C — démonstrateur (omnifab-des
ktop)

**C1 — cache d'échec + parses hors du fil UI.** Les caches deviennent des `Result` (clé = nom de blob) : un parse en échec est **mis en cache**, plus jamais re-parsé à chaque image (fini le freeze sur blob corrompu). Lectures de fichiers, parses et exports déplacés vers un fil de travail `std::thread` + canal `std::sync::mpsc` (pas tokio — le verrou n°4 le réserve au pilotage machine) : état « en cours » i18n, résultat appliqué au retour de canal, résultats obsolètes ignorés (identifiant de requête). Pas d'annulation au jalon 2.
*Tests :* comptage : un blob invalide n'est parsé qu'une fois ; résultat obsolète ignoré (fonction pure de la file de requêtes).

**C2 — raccourcis clavier corrects.** `i.modifiers.ctrl` → `i.modifiers.command` (Ctrl sous Windows/Linux, Cmd sous macOS) ; garde : aucun raccourci global quand un champ de texte a le focus (`ctx.wants_keyboard_input()`) — la saisie en cours n'est plus écrasée par Ctrl+Z ; l'annulation reste accessible par les boutons.
*Tests :* logique de garde en fonction pure ; Cmd testé à la main au prochain passage macOS (le job macOS ne fait que compiler).

**C3 — protection du travail.** Indicateur `dirty` (toute commande effective le pose, la sauvegarde le lève ; `*` dans le titre, clé FTL) ; « Nouveau » / « Ouvrir » / « Reprendre » demandent confirmation si `dirty` (fenêtre modale i18n + help_id). **Sauvegarde sans casser le redo** : nouvelle méthode `Document::to_bytes_prefix(applied)` — le fichier reste le journal linéaire visible (ADR-0008 inchangé) mais la session en mémoire n'est plus mutée : `undo → save → redo` fonctionne. Fermeture de fenêtre : interceptée — `ctx.input(|i| i.viewport().close_requested())` + `ViewportCommand::CancelClose` (egui 0.29, exemple `confirm_exit`), corrigé A3-06 (audit n°3).
*Tests :* `to_bytes_prefix` → relecture = journal appliqué attendu ; `undo → save → redo()` restaure ; `dirty` suit les commandes.

**C4 — messages d'état fidèles.** `executer` retourne `Result` : « import réussi » n'est posé que si la commande a réussi ; `self.error`/`
self.info` sont effacés au début de chaque action utilisateur (plus d'erreur rouge persistante après correction).
*Tests :* import d'un fichier invalide → pas de message de succès ; l'action suivante efface l'erreur.

**C5 — noms de fichiers.** `ajouter_suffixe` : suffixe ajouté seulement si l'extension actuelle (insensible à la casse) n'est pas parmi les extensions attendues ; `chemin_projet` ajoute `.omnifab` si manquant ; `bibliotheque::lister` compare l'extension sans casse.
*Tests :* « plan v1.5 » + filtre svg → « plan v1.5.svg » ; projet sans extension listé et ré-ouvrable.

## Alternatives rejetées

- **A1 par copies d'état candidat / dry-run sur les projections** : coûteux, complexifie le trait `Projection` ; le simple réordonnancement suffit (le rejeu rétablit déjà l'état des projections).
- **A2 conserver l'ordre actuel (`.bak` avant `.tmp`)** : le défaut même ; **rotation multi-générations de `.bak`** : confort, pas un défaut — hors périmètre.
- **C1 tokio dans le desktop** : le verrou n°4 réserve tokio au pilotage machine ; trois opérations bloquantes n'exigent pas un runtime async.
- **B2/B3 normaliser/recadrer les sous-tracés à l'export-import** : contredit la décision « coordonnées exactes » (round 4).
- **Un seul commit géant** : la règle un-point-un-commit + CI verte entre points réduit le risque de régression invisible (protocole de la revue externe).
- **Inclure les lots D/E** : D (segments typés, tolérance de discrétisation, chaînage, RapportImport, INSUNITS/INSERT/OCS, Fluent) exige des ADR dédiés jalon 3/4 ; E (licences, README, deny, SHA) est documentaire, sans ADR.

## Conséquences

- **Positives** : le contrat tout-ou-rien de la session est tenu (P1 des deux audits convergents) ; plus aucun chemin de projet vacable sur échec d'écriture ; imports fidèles (POLYLINE fermée, SVG après `Z`) ; export SVG interopérable ; fin du gel UI et du re-parse à chaque image ; macOS utilisable (Cmd) ; le travail non enregistré est protégé ; l'infrastructure d
e migrations (ADR-0001) devient effective.
- **Négatives / coûts acceptés** : un nouveau code d'erreur (OF-DOC-0014, budget cumulé) + codes pour doublons/noms invalides si non couverts (registre + CI bidirectionnelle) ; la garde `wants_keyboard_input` rend Ctrl+Z inopérant pendant une saisie (compensé par les boutons) ; une petite machine à états de requêtes dans le desktop (testée) ; la tolérance au bourrage STL élargit l'espace de fichiers acceptés (documenté dans le module).
- **Risques** : A1 touche le cœur du modèle — tests d'abord, relecture `get_commit(full_patch)` ; l'API de fermeture de `dxf::Polyline` est à vérifier sur docs.rs avant le commit B1 ; `ctx.on_request_exit` à vérifier dans eframe 0.29 ; aucun changement de schéma d'événement (A3 durcit la tolérance, ne migre rien).
- **Dépendances** : aucune nouvelle crate (uuid et std suffisent).
- **Hors périmètre, renvoyés aux ADR D1/D2/D3 et au lot E** : segments typés / tolérance de discrétisation / chaînage de contours (prérequis jalon 4 CAM), `RapportImport` + `$INSUNITS` + INSERT/OCS (thème unités, avec le mini-ADR STL), isolation Fluent, soudure de sommets STL, licences manquants, README/CONTRIBUTING, exécution réelle de cargo-deny, épinglage SHA des actions.
