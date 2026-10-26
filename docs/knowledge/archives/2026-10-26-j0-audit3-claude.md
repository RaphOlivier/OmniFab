# [J0] Audit externe n°3 (Claude, analyse statique) — 18 signalements A3 + 5 bloquants build, traitement et verdicts

**Date :** 2026-10-26 · **Auditeur :** Claude (analyse statique, sans compilation ni exécution) · **Format :** `docs/audit-protocole.md` §4 (numérotation A3-NN).
**SHA audité :** non disponible côté auditeur (zip sans `.git`) ; état documenté : tête `ee37fa6` (fiche état actuel v9) — tête réelle au moment du traitement : `e273cc6`. Chaque signalement à re-ancrer au SHA réel lors de la correction en code.
**Limites déclarées par l'auditeur :** ni cargo ni rustc — aucun test exécuté ; les constats non confirmables sont marqués « non vérifié » par l'auditeur lui-même.
**Couverture déclarée :** lecture ligne à ligne de forge-units, forge-geom, forge-cad, forge-doc, forge-values (expr.rs, graphe), forge-session (lib, command, projection), forge-io (stl, dxf, svg), forge-gcs (native.rs), desktop (fichiers, bibliothèque, fil de travail, boucle de retours) ; sondage (panics, indexations, casts) sur esquisse.rs, atelier_esquisse.rs, forge-render, forge-i18n, omnifab-android ; non lu : shim C++ et third_party.

## Verdicts du mainteneur (traités le 2026-10-26, sans PC — la CI reste rouge, aucun correctif de code n'est poussable)

Protocole `docs/audit-protocole.md` §5 : chaque signalement reçoit un verdict — **corrigé** · **dette acceptée** (fiche état actuel, date) · **refusé** (motif). Règle AGENTS en vigueur : aucun code métier sur main tant que la CI est rouge ; les correctifs de code partent en branche courte + PR au retour du vert.

| ID | Sév. | Constat (résumé) | Verdict | Traitement |
|---|---|---|---|---|
| B1 | bloquant | `Cargo.lock` sans forge-cad (cause unique CI rouge) | **dette acceptée** (déjà planifiée) | Action bloquante n°1 : build local OCCT + push du lock (Raphaël) |
| B2 | bloquant ? | opencascade 0.3 « n'existerait pas » sur crates.io | **refusé (réfuté)** | Vérifié le 2026-10-26 : `opencascade 0.3.0` publiée le 2026-08-24 sur crates.io (LGPL-2.1), dépendances `cxx ^1`, `glam ^0.24` (conforme au projet), `opencascade-sys ^0.3` — la génération du lock n'est pas compromise par la version |
| B3 | bloquant | Signatures du module `occt` jamais compilées | **dette acceptée** | Risque ajouté à ADR-0023 (section Risques phase A, amendé le 2026-10-26) : le premier build local produira vraisemblablement un lot de corrections d'API ; ne pas conclure « phase A livrée » avant |
| B4 | bloquant | `rust-toolchain.toml` : channel stable flottant + clippy `-D warnings` | **dette acceptée** | Épinglage d'une version exacte (celle validée en local, ≥ 1.88 : let-chains) + `components = [rustfmt, clippy]` — instruction ajoutée au prep, exécution au checkpoint PC |
| B5 | bloquant | cargo-deny jamais exécuté ; `deny.toml` refuserait LGPL-2.1 (forge-gcs) et GPL-3.0 (apps), `exceptions = []` | **dette acceptée** | Correctif `deny.toml` rédigé au prep (licences LGPL-2.1/GPL-3.0 autorisées, `licenses.private` pour le workspace) — **validation explicite de Raphaël requise (licences = décision à validation explicite)** ; fiche sécurité déjà correcte depuis v2 (« rédigé, inactif ») |
| A3-01 | majeur | GC de blobs détruit les imports antérieurs : undo profond impossible (contrat ADR-0008 trahi) | **dette acceptée → correction au vert (priorité 1)** | Re-vérifié au code le 2026-10-26 (`gc_blobs` purge tout blob absent des références de l'état courant) : confirmé. Correctif proposé par l'auditeur retenu (conservation des blobs référencés par `events[..applied]`) + test `undo_profond_conserve_les_blobs_des_imports_anterieurs` |
| A3-02 | majeur | Chaînes d'opérateurs non bornées : abort de pile sur formule longue | **dette acceptée → correction au vert (priorité 2)** | Re-vérifié au code : `PROFONDEUR_MAX = 256` ne borne que l'imbrication (parenthèses/args/négations), pas les boucles `+`/`-`/`*`/`/` ; confirmé. Correctif retenu : borne de longueur de texte + compteur de nœuds au parseur (le journal peut être forgé) |
| A3-03 | majeur | Récursion proportionnelle à la chaîne de dépendances (évaluation) | **dette acceptée → correction au vert (priorité 2)** | Correctif retenu : profondeur max de chaîne contrôlée à `prepare_expression` (nouveau code OF-VAL-*) ou évaluation itérative topologique |
| A3-04 | majeur | `lis_viewbox` : mauvaise viewBox (symbol, commentaires, virgules) → géométrie décalée en silence | **dette acceptée → correction au vert (priorité 5)** | Correctif retenu : attribut viewBox de l'élément racine via roxmltree, virgules acceptées |
| A3-05 | majeur | DXF : espace papier, calques éteints/gelés, entités invisibles importés | **dette acceptée → correction au vert (priorité 5)** | À traiter avec le backlog DXF de l'audit 2 ($INSUNITS, INSERT/blocs) ; comptage des ignorés pour RapportImport (ADR-0016 D2) |
| A3-06 | majeur | Fermeture de fenêtre sans garde alors qu'eframe 0.29 expose `close_requested()`/`CancelClose` | **dette acceptée → correction au vert (priorité 4)** | API eframe 0.29 plausible mais non recompilée ici ; correctif + test `doit_bloquer_la_fermeture(dirty, confirmation)` ; corrige aussi le commentaire main.rs et la fiche (« limite documentée » → faux, D6) |
| A3-07 | majeur | Fichier lu en entier avant tout budget (`std::fs::read` avant `max_bytes`) — viole ADR-0009 | **dette acceptée → correction au vert (priorité 4)** | Correctif retenu : `metadata().len()` ou `File::take(max+1)` ; test `lire_borne` |
| A3-08 | majeur (latent) | forge-cad : API sans validation, `formes_max` fictif, import non typé, fichiers temporaires 0644 sans garde Drop | **dette acceptée → correction avant branchage UI (priorité 6)** | Latent tant que forge-cad n'est pas branché à l'UI ; à traiter avec B3 au premier build (les mêmes fichiers seront relus) |
| A3-09..A3-18 | mineur/hygiène | 10 signalements mineurs (nœud fantôme, dépendants non revalidés, UUIDv7 seconde-précision/compteur, tolérance ZIP silencieuse, entrées répertoire, STL binaire/ASCII CR, ressources externes SVG, `Length::from_millimeters` NaN, `validate_blob_name` trop large) | **dette acceptée → correction au vert (priorité 7)** | Backlog au prep ; A3-11 (UUID) re-vérifié partiellement (commentaire « précision ms » vs `now_unix()` en secondes à confirmer au correctif) |
| N8 (précision) | — | états exacts du fil de travail : pas de catch_unwind, send ignoré, compteur jamais décrémenté → « travail en cours » permanent | **dette acceptée** (déjà en backlog) | Précision intégrée à la spécif existante du test N8 (`preparation/specifs-tests-regression-audit2.md`) |
| D1 | doc | audit-protocole §2 : « vérifiée par cargo-deny » alors que le job est inactif | **corrigé** (doc) | Reformulé le 2026-10-26 : « vérification cargo-deny prévue (deny.toml rédigé, job CI à activer — B5) » |
| D2 | doc | audit-protocole §6 : « dépôt public » (passage public non tenu : traduction EN en file) | **corrigé** (doc) | « dépôt OmniFab » le 2026-10-26 |
| D3 | doc | AGENTS.md : « protection de branche active » inapplicable pendant la fenêtre rouge | **corrigé** (doc) | Reformulé : la protection (status check lint-test) est décrite comme à (re)prendre au vert ; en attendant la règle branches/PR s'applique par discipline |
| D4 | doc | AGENTS.md : « la copie de travail (qui fait foi) » contredit « GitHub = seule source de vérité » | **corrigé** (doc) | Aligné sur ADR-0028 : `docs/knowledge/` = copie de référence au dépôt ; le dépôt fait foi pour le code |
| D5 | doc | liens-md.yml : liens `%20` signalés à tort ; liens de référence non couverts | **corrigé** (outillage) | Workflow réécrit (2026-10-26) : décodage percent-encoding + couverture des définitions de référence ; toujours en `workflow_dispatch` seul |
| D6 | doc | main.rs/fiche : « fermeture non interceptable » (faux) | **dette acceptée** | main.rs = code : correction du commentaire en même temps qu'A3-06, en branche+PR au vert |
| D7 | doc | ADR-0023 : risques B2-B3 de la phase A absents | **corrigé** (doc) | Section « Risques phase A » ajoutée à l'ADR-0023 le 2026-10-26 (B3 retenu ; B2 réfuté avec preuve crates.io) |

**Ordre de traitement retenu** (conforme à la proposition de l'auditeur, ajustée) : 1. B4 + B5 (build/CI) puis B1 (lock, action Raphaël) ; 2. A3-01 (perte d'undo — le plus probable en usage réel) ; 3. A3-02/A3-03 (plantage sur fichier forgé) ; 4. A3-06/A3-07 + N8 (perte de travail) ; 5. A3-04/A3-05 (fiabilité import avant le CAM) ; 6. A3-08 (avant branchage UI) ; 7. mineurs A3-09..A3-18. Chaque correctif : test d'abord (rouge avant / vert après), un commit par point, branche courte + PR (CI verte requise).

**Bonne prise de l'audit :** conformité exemplaire au protocole (§4 : fichier:ligne, sévérité, preuve, test proposé ; self-reporting des « non vérifié » ; re-lecture de l'existant avant de signaler ; vérification des correctifs de l'audit 2 plutôt que re-signalement). Le seul réfuté (B2) était explicitement marqué « non vérifié, réseau désactivé » par l'auditeur.

---

## Rapport intégral (verbatim, tel que soumis par l'auditeur)

> Date : 2026-10-26 · Auditeur : Claude (analyse statique) · Format : docs/audit-protocole.md §4 (numérotation A3-NN)
>
> SHA audité : non disponible (le zip ne contient pas .git). État documenté : tête ee37fa6 (fiche état actuel v9). À rattacher au SHA réel lors de l'archivage.
> Limites : ni cargo ni rustc ici — aucun test exécuté. Chaque « preuve » est un scénario d'entrée plus une lecture du code ; ce qui n'a pas pu être confirmé est marqué non vérifié.
>
> Couverture. Lu ligne à ligne : forge-units, forge-geom, forge-cad, forge-doc, forge-values (expr.rs, graphe), forge-session (lib, command, projection), forge-io (stl, dxf, svg import), forge-gcs (native.rs), desktop (fichiers, bibliothèque, fil de travail, boucle de retours), documentation ajoutée. Sondage seulement (recherche de panics, indexations, casts) : esquisse.rs, atelier_esquisse.rs, forge-render, forge-i18n, omnifab-android. Non lu : shim C++ et third_party.
>
> ### 1. Synthèse
>
> | ID | Sév. | Zone | Constat |
> |---|---|---|---|
> | B1-B5 | bloquant | build/CI | Lock sans forge-cad, version opencascade à vérifier, toolchain flottante, cargo-deny jamais exécuté |
> | A3-01 | majeur | forge-session | Le GC de blobs détruit les imports antérieurs : undo profond impossible |
> | A3-02 | majeur | forge-values | Chaînes d'opérateurs non bornées : abort de pile sur formule longue |
> | A3-03 | majeur | forge-values | Récursion proportionnelle à la chaîne de dépendances |
> | A3-04 | majeur | forge-io SVG | lis_viewbox : mauvaise viewBox, géométrie décalée en silence |
> | A3-05 | majeur | forge-io DXF | Espace papier, calques éteints/gelés et entités invisibles importés |
> | A3-06 | majeur | desktop | Fermeture de fenêtre sans garde, alors qu'eframe 0.29 le permet |
> | A3-07 | majeur | desktop | Fichier lu en entier avant tout budget |
> | A3-08 | majeur (latent) | forge-cad | API sans validation, budget formes_max fictif, import non typé |
> | A3-09 à A3-16 | mineur/hygiène | divers | Voir §4 |
>
> ### 2. Bloquants de build (avant tout code métier)
>
> **B1 — Cargo.lock sans forge-cad.** Confirmé : 0 occurrence de forge-cad dans Cargo.lock. Toute commande --locked échoue ; c'est la cause unique de la CI rouge. Action : push du lock par Raphaël (déjà planifié).
>
> **B2 — opencascade = 0.3 (crates/forge-cad/Cargo.toml) : non vérifié.** Réseau désactivé ici. À ma connaissance, la dernière version publiée de la crate opencascade est 0.2.x. Si 0.3 n'existe pas sur crates.io, la génération du lock (action bloquante n°1) échouera, et une dépendance git est interdite par deny.toml (unknown-git = deny). À contrôler AVANT le build local : cargo search opencascade. Vérifier aussi que glam = 0.24 est la même version que celle exigée par le binding (DVec3 passé à Shape::cylinder).
>
> **B3 — Signatures du binding jamais compilées.** Shape::box_with_dimensions, Shape::cylinder, Solid::extrude_polygon, mesh_with_tolerance, write_all_step, read_step, write_brep_text, read_brep_text (occt.rs) n'ont jamais été compilées (CI rouge depuis 4409a65, pas de cargo côté Mistral). Prévoir que le premier build local produise un lot de corrections d'API ; ne pas conclure « phase A livrée » avant.
>
> **B4 — rust-toolchain.toml : channel = stable (flottant).** Avec clippy -D warnings en CI, une nouvelle version stable qui ajoute un lint rend la CI rouge sans changement de code. Épingler une version exacte (celle validée en local, ≥ 1.88 : le code utilise les let-chains) et ajouter components = [rustfmt, clippy].
>
> **B5 — cargo-deny : jamais exécuté, config probablement en échec.** grep sur .github/workflows : aucun job cargo-deny (seulement une proposition dans docs/ci-cargo-deny.md). Or docs/audit-protocole.md §2 écrit « vérifiée par cargo-deny » et la fiche sécurité « en place ». De plus deny.toml n'autorise ni LGPL-2.1-or-later (forge-gcs, forge-gcs-sys) ni GPL-3.0-or-later (apps), avec exceptions = [] : cargo deny check licenses échouerait sur les membres du workspace (sauf réglage licenses.private, absent), et sur opencascade (LGPL-2.1) en transitif. Action : corriger deny.toml, activer le job, ou corriger les fiches.
>
> ### 3. Constats majeurs
>
> **A3-01 — Le GC de blobs détruit les imports atteignables par undo.**
> Fichier:ligne : forge-session/src/lib.rs:237 et :267 ; projection.rs GeomProjection::referenced_blobs (~l.253). Sévérité : majeur. Classe : bug (perte de données, contrat ADR-0008 trahi).
> Preuve (par lecture, non exécutée) : 1. ImportTraces svg A → blob a (événement e1). 2. ImportTraces svg B → blob b (e2). Ici references = {a} + {b} : rien n'est purgé. 3. N'importe quelle commande, ex. SetInput x=1. referenced_blobs() ne renvoie que l'état COURANT des projections, soit {b} : gc_blobs retire a. 4. undo ×3 : geom.traces s().blob == a mais document().blob(a) == None. Le desktop affiche « blob absent du document » (main.rs:1645). La sauvegarde (to_bytes_prefix) n'embarque pas non plus a (absent en mémoire) : le fichier rouvert garde un e1 qui pointe vers un blob inexistant. Le commentaire de execute (« un undo peut revenir à cet état ») ne protège qu'UN niveau d'undo.
> Correctif : gc_blobs conserve tout blob référencé par un événement de doc.events()[..applied] (clé data.blob, comme le fait déjà to_bytes_prefix) ; seuls les blobs des événements tronqués (pile redo abandonnée) sont retirés. Test (forge-session/tests/session.rs) : undo_profond_conserve_les_blobs_des_imports_anterieurs — import A, import B, SetInput, 3×undo : attendu session.document().blob(&nom_a).is_some() et sauvegarde/relecture sans référence orpheline.
>
> **A3-02 — Chaînes d'opérateurs non bornées (abort de pile).**
> Fichier:ligne : forge-values/src/expr.rs:270-304 (boucles + - * /), :444-500 (evaluate), :503-520 (collect_references), destructeur récursif de Box<Expr> ; forge-session/src/command.rs SetExpression::plan (aucune longueur max). Sévérité : majeur. Classe : bug de robustesse ; régression partielle du correctif « récursion » de l'audit n°2.
> Preuve : la garde PROFONDEUR_MAX n'est incrémentée que par les parenthèses, arguments et négations. Une formule 1+1+1+…+1 de 200 000 termes (≈ 400 Ko) est lue par les boucles expression_int/term sans augmenter la profondeur ; l'arbre est un peigne gauche de 200 000 niveaux. collect_references, evaluate et le Drop descendent récursivement : débordement de pile = abort, non rattrapable. Via un journal forgé (événement value.expression_set), ouvrir le fichier plante l'application. Idem avec 1*1*… ou 1-1-…. Seuil exact non mesuré (non vérifié).
> Correctif : (1) longueur max du texte d'expression (ex. 4096 car.) dans SetExpression::plan ET dans expr::parse (le journal peut être forgé) ; (2) compteur de nœuds d'AST borné dans le parseur ; (3) option : évaluation et Drop itératifs. Test : dans std::thread::Builder::new().stack_size(2 * 1024 * 1024), parse(&format!("{}1", "1+".repeat(200_000)), ...) doit renvoyer Err(InvalidExpression) sans abort.
>
> **A3-03 — Récursion proportionnelle à la chaîne de dépendances.**
> Fichier:ligne : forge-values/src/lib.rs:729-805 (eval_node), :662-683 (chemin_vers, déjà reconnu comme résidu au 25/10 mais jugé « borné par le nombre de nœuds »). Sévérité : majeur. Classe : bug de robustesse.
> Preuve : un journal de ~20 000 événements value.expression_set chaînés (a1 = a0 + 1, a2 = a1 + 1, …) — quelques Mo seulement. À l'insertion i, chemin_vers (cycle) descend de i niveaux ; ensuite value(a20000) recurse via eval_node → evaluate → fermeture → eval_node. Sur un fil à 2 Mio (Android, fil de travail) le seuil se compte en milliers de niveaux. Coût de définition en O(n²). Seuil non vérifié.
> Correctif : profondeur maximale de chaîne (nouveau code OF-VAL-…) contrôlée à prepare_expression, ou évaluation itérative par ordre topologique (deps_transitifs est déjà itératif). Test : chaîne de 5 000 alias dans un fil à pile 2 Mio : Ok ou Err propre, jamais d'abort.
>
> **A3-04 — lis_viewbox : mauvaise viewBox, géométrie décalée en silence.**
> Fichier:ligne : forge-io/src/svg.rs:102-135. Sévérité : majeur. Classe : bug (pièce usinée au mauvais endroit, aucun message).
> Preuve (3 entrées) : 1. svg avec symbol viewBox avant la racine : pas de viewBox racine, mais texte.find('viewBox') trouve celle du symbol → décalage (50 mm, 50 mm) appliqué à tout l'import. 2. viewBox='-50,-50,100,100' (virgules : valide en SVG) : split_whitespace échoue → None → compensation perdue. 3. commentaire contenant viewBox avant la racine : même défaut qu'en 1.
> Correctif : lire l'attribut sur l'élément racine via roxmltree (déjà transitif d'usvg), accepter virgules et espaces ; ne jamais chercher dans le texte brut. Test : les trois SVG ci-dessus, assertion sur les coordonnées du premier point importé.
>
> **A3-05 — DXF : géométrie invisible importée.**
> Fichier:ligne : forge-io/src/dxf.rs:81-135 (aucune occurrence de paper/visible/layer/frozen dans le fichier). Sévérité : majeur (fonctionnel pour la FAO). Classe : bug / lacune non listée dans la dette (la dette ne couvre que $INSUNITS, blocs, extrusion, chaînage).
> Preuve : un DXF contenant une LINE d'espace papier (code 67 = 1), ou d'un calque éteint/gelé, ou invisible (code 60 = 1) produit un tracé importé, donc usinable alors que l'utilisateur ne la voit pas dans son logiciel de CAO ; les cadres/cartouches d'espace papier s'ajoutent à la géométrie du modèle. Non exécuté. Les noms exacts des champs de la crate dxf 0.6.2 (is_in_paper_space, visibilité, table des calques) sont à confirmer.
> Correctif : ignorer espace papier, entités invisibles, calques désactivés/gelés ; compter les ignorés pour le futur RapportImport (ADR-0016 D2). Test : trois DXF minimaux, un par cas, attendu 0 tracé.
>
> **A3-06 — Fermeture de fenêtre sans garde.**
> Fichier:ligne : apps/omnifab-desktop/src/main.rs:~3000-3010 (commentaire « non interceptable »). Sévérité : majeur. Classe : bug / affirmation fausse en doc ; H8 reste ouvert.
> Preuve : Cargo.lock fixe eframe 0.29.1. Cette version expose ctx.input(|i| i.viewport().close_requested()) et egui::ViewportCommand::CancelClose. grep : aucun usage dans le code. La croix, Alt+F4 ou Cmd+Q perdent donc le travail non enregistré sans question (le * du titre ne protège rien).
> Correctif : dans update, si close_requested() et self.dirty : envoyer CancelClose et ouvrir une confirmation (Enregistrer / Quitter sans enregistrer / Annuler). Corriger le commentaire et la fiche (« limite documentée »). Test : fonction pure doit_bloquer_la_fermeture(dirty, confirmation) -> bool, trois cas.
>
> **A3-07 — Fichier lu en entier avant tout budget.**
> Fichier:ligne : apps/omnifab-desktop/src/main.rs:263 et :288 (std::fs::read), et Document::open_from_path (forge-doc/src/lib.rs). Sévérité : majeur. Classe : bug (l'ADR-0009 promet une erreur typée « avant toute allocation proportionnelle »).
> Preuve : choisir par erreur un fichier de 8 Go dans le sélecteur d'import : std::fs::read alloue 8 Go avant que max_bytes soit consulté ; l'échec d'allocation abort le processus (non rattrapable), travail perdu. Non exécuté.
> Correctif : std::fs::metadata(&chemin)?.len() comparé au budget avant lecture, ou File::open(..)?.take(max + 1).read_to_end(..). Test : fonction lire_borne(chemin, max) sur un fichier plus grand que max : Err(TooLarge) sans lecture complète.
>
> **A3-08 — forge-cad : API sans validation (latent tant que non branché à l'UI).**
> Fichier:ligne : forge-cad/src/lib.rs:63-129, 136-190, 235-261, 285-311 ; occt.rs. Sévérité : majeur (latent). Classe : bug / risque.
> a) Result jamais en échec : boite, cylindre_centre, soustraire, unir renvoient toujours Ok malgré la doc « Erreurs : OF-CAD-0001 » ; CadError::FormeInvalide (OF-CAD-0002) n'est jamais produite (variante morte). Dimensions ≤ 0, NaN ou ∞ partent telles quelles vers OCCT (exception C++ ou forme nulle ; si le binding ne la rattrape pas : abort — non vérifié). Un booléen qui donne un solide vide n'est pas détecté. Tests : boite(0,10,5) et boite(NaN,…) → Err OF-CAD-0002 ; cube.soustraire(&cube_identique) → Err ou solide vide explicite.
> b) formes_max ne borne rien : importer renvoie toujours un seul Solide ; seul formes_max < 1 est testé. La doc « refusé AVANT le premier octet confié à OCCT » est fausse pour ce budget. Compter les solides après lecture ou retirer le champ.
> c) Import non typé : la forme lue est renvoyée comme Solide même si c'est une coque, une face ou un compound ouvert ; volume_mm3 renvoie alors une valeur sans sens, sans erreur. Vérifier le type (solide) sinon Err.
> d) tesseller / volume_mm3 : tolerance ≤ 0, NaN ou ∞ non validée (maillage non borné — non vérifié) ; indexation sommets[t[i] as usize] sans contrôle ; as u32 tronque en silence ; volume signé sans abs(). Valider, utiliser get() et u32::try_from.
> e) Fichiers temporaires : dans temp_dir() avec droits par défaut (0644 sous Linux : le projet CAO est lisible par les autres comptes pendant l'export), sans garde Drop (panic/abort OCCT = fichier laissé). OpenOptions::create_new(true) + mode 0o600 + garde RAII.
>
> ### 4. Constats mineurs et hygiène
>
> | ID | Fichier:ligne | Constat | Correctif / test |
> |---|---|---|---|
> | A3-09 | forge-values/src/lib.rs:220-232, 597-610 | set_expression* crée le nœud (Input 0.0) AVANT de valider l'expression ; échec → nœud fantôme (value(x) == Ok(0.0)). La session masque via rejeu, pas l'API de la crate. | Rollback si nœud neuf. Test : set_expression(nouveau, 1/) Err puis value(nouveau) = Err(UnknownValue). |
> | A3-10 | forge-values/src/lib.rs:189-215 | set_input_unite ne revalide pas les dépendants : a (mm), b = a*2 (mm), puis set_input a en angle accepté ; OF-VAL-0007 contourné. | Vérifier les dépendants directs. |
> | A3-11 | forge-doc/src/lib.rs:150-164 | UUIDv7 : now_unix() * 1000 (précision seconde, pas ms), octets aléatoires nuls, compteur remis à 0 à chaque chargement et saturé à 0x0FFF. Ids non uniques entre documents ; txn d'une commande égal à celui d'une commande d'une session précédente → groupe_start/groupe_end fusionnent deux commandes en un undo (certain sous wasm : SystemClock = 0 ; aussi sous FixedClock). | Test : doc FixedClock(1000), SetInput a, save, reload même horloge, SetInput b, undo : doit annuler b seul. Remplir les octets aléatoires hors horloge figée, ou repartir du max des ids existants. |
> | A3-12 | forge-doc/src/container.rs:200-217 | La tolérance « dernière ligne tronquée » est silencieuse (log::warn seul) et probablement inatteignable (le CRC32 du ZIP échoue avant — non vérifié) ; le commentaire « .bak garde l'archive intacte » n'est vrai que jusqu'à la 2e sauvegarde. | Remonter un indicateur recovered à l'UI, ou supprimer la tolérance. |
> | A3-13 | forge-doc/src/container.rs:243-244 | Une entrée répertoire objects/ (ajoutée par zip -r, 7-Zip) donne un nom de blob vide → OF-DOC-0013 : un projet re-compressé à la main est refusé. | Ignorer les entrées finissant par /. Test dédié. |
> | A3-14 | forge-io/src/stl.rs:60-77 | Ordre de détection : un STL binaire dont l'en-tête commence par solid ET portant du bourrage final (cas que la doc du module dit toléré) est routé vers ASCII → « octets non UTF-8 ». | Tester est_binaire_tolerant avant est_ascii (sans risque : un texte ASCII donne un compteur ≥ 0x09090909). Test : en-tête solid cube + 1 triangle + 10 octets → 1 triangle. |
> | A3-15 | forge-io/src/stl.rs:189-196 | STL ASCII : seul le premier mot de chaque ligne est lu ; fins de ligne CR seul ou facets sur une ligne → import « réussi » à 0 triangle, sans erreur. | Tokeniser globalement, ou refuser 0 triangle. Test CR seul. |
> | A3-16 | forge-io/src/svg.rs:1-20, 78-82 | Non vérifié : la doc affirme « aucune ressource externe, ni DTD ». À ma connaissance, usvg::Options::default() charge les href d'images vers des chemins locaux (sous Windows, un chemin UNC déclenche une connexion SMB à l'ouverture) et accepte les entités DTD internes. Les budgets max_paths/max_vertices ne s'appliquent qu'APRÈS la construction complète de l'arbre. | Résolveur d'images neutralisé (resolve_string → None) ; tester entités imbriquées et use en éventail (2^n) sous limite de temps/mémoire ; corriger la doc si besoin. |
> | A3-17 | forge-units/src/lib.rs:38 | Length::from_millimeters accepte NaN/∞ : la garde est reportée à chaque consommateur (oubliée dans forge-cad, A3-08). | Documenter, ou constructeur faillible. |
> | A3-18 | forge-doc/src/container.rs:41 | validate_blob_name refuse tout nom contenant .. (ex. a..b, légitime). | Tester par segment. |
>
> ### 5. Précisions sur la dette déjà connue (ne pas re-signaler, mais préciser)
>
> - N8 (catch_unwind) — états exacts. fil_de_travail (main.rs:462-466) n'attrape aucun panic. Si executer() panique, le fil meurt ; envoyer_travail (:1296-1303) ignore silencieusement l'Err du send : imports/exports n'ont plus aucun effet ni message ; travaux_en_attente n'est jamais décrémenté pour le travail en panne → message « travail en cours » permanent et repaint toutes les 100 ms à l'infini (:2963-2965). Correctif : catch_unwind(AssertUnwindSafe(..)) → Retour::Erreur ; détecter la déconnexion et relancer le fil ; test avec un travail qui panique volontairement.
> - Discrétisation. Chiffres : bulge de 180° sur r = 1000 mm → 16 segments, flèche ≈ 4,8 mm ; cercle 64 points, r = 50 mm → 0,06 mm (cohérent avec les fiches).
> - expect() en production : recompte rapide ≈ 39 lignes avant le premier bloc de tests, sur 10 fichiers (fiches : 44 sur 16) ; méthodes différentes, à recouper, non bloquant.
>
> ### 6. Revue des modifications de documentation
>
> Bien : AGENTS.md (préférences isolées des conventions), docs/audit-protocole.md (ancrage SHA, anti-faux-positifs, verdict par signalement), fiche etat-actuel (honnête sur la CI rouge et le gel jalons 5-10), découpage de la fiche architecture, LICENSE ×6, rust-version = 1.88, workflow liens-md en déclenchement manuel, branches/PR avant fusion.
> À corriger : D1 (audit-protocole §2 « vérifiée par cargo-deny » — faux) ; D2 (§6 « le dépôt public » — passage public non tenu) ; D3 (AGENTS.md : protection de branche « active » vs push direct docs — règle inapplicable à préciser) ; D4 (AGENTS.md : « copie de travail qui fait foi » vs « GitHub = seule source de vérité » — à trancher) ; D5 (liens-md.yml : %20 et liens de référence) ; D6 (main.rs « fermeture non interceptable » — faux, cf. A3-06) ; D7 (fiche forge-cad/ADR-0023 : ajouter B2-B3 comme risques ouverts de la phase A).
>
> ### 7. Vérification des correctifs de l'audit n°2 (lecture, non compilés)
>
> - N1 (point dans les noms) ✔ — expr.rs:170-185.
> - N2 (substitution d'alias) ✔ — lib.rs:809-846 (bordure étendue au point, appels de fonction épargnés).
> - Récursion bornée ◐ — parenthèses, arguments et négations bornés ; chaînes d'opérateurs et de dépendances non (A3-02, A3-03).
> - N7 (export atomique) ✔ — main.rs:578-598.
> - N9 (SVG plat) ✔ — constante MARGE_BOITE_PLATE présente ; application non relue en détail.
>
> ### 8. Ordre de traitement proposé
>
> 1. Build : B2 (version opencascade) puis B1 (lock) ; ensuite B4 et B5. Sans CI verte, aucun correctif ci-dessous n'est validable (règle AGENTS : ne pas empiler du code métier sur main rouge — préparer en branche).
> 2. A3-01 (perte d'undo) — le plus probable en usage réel.
> 3. A3-02 / A3-03 — plantage à l'ouverture d'un fichier forgé ; borne de longueur, de nœuds et de profondeur.
> 4. A3-06 / A3-07 — perte de travail et abort mémoire côté desktop ; avec N8.
> 5. A3-04 / A3-05 — fiabilité de l'import avant le CAM (jalon 4).
> 6. A3-08 — avant de brancher forge-cad à l'UI.
> 7. Mineurs A3-09 à A3-18 et corrections de fiches D1-D7.
>
> Chaque correctif : test d'abord (rouge avant, vert après), un commit par point, verdict explicite par signalement (corrigé / dette acceptée / refusé) selon docs/audit-protocole.md §5.
