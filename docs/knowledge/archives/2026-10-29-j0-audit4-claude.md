# [J0] Audit externe n°4 (Claude, analyse statique) — 15 signalements A4 + vérification des correctifs de l'audit n°3, traitement et verdicts

**Date :** 2026-10-29 · **Auditeur :** Claude (analyse statique, ni cargo ni rustc) · **Format :** `docs/audit-protocole.md` §4 (numérotation A4-NN).
**SHA audité :** non disponible côté auditeur (zips sans `.git` ; la consigne contenait le littéral `<SHA>`). Deux états lus : zip n°1 (état v36) et zip n°2 (état v38, avec `examples/spike_phase_a.rs`, liens-md.yml activé, conventions v10) — aucun code de production changé entre les deux ; le zip n°2 correspond à main entre `4042858` (liens-md activé, v37) et `21dc112` (fusion du spike, v41.1). **Ancrage à rattacher aux SHA réels par Raphaël à l'archivage.**
**Limites déclarées par l'auditeur :** aucun test exécuté ; `third_party/planegcs` absent du zip (sous-module non auditable) ; tout constat non confirmé est marqué « non vérifié » par l'auditeur lui-même — les hypothèses de lecture (A4-02 limite wgpu, A4-04 statut 3, A4-05 use-after-free, A4-07 arc, A4-09 isolation Fluent) sont signalées comme telles.
**Couverture déclarée :** 25 705 lignes de `.rs` inventoriées, 13 crates ; non relu ligne à ligne : la plus grande partie de main.rs (UI), atelier_esquisse.rs (balayé), forge-session/esquisse.rs l.1150-1650, scene3d.rs, egui_adapteur.rs, command.rs, projection.rs ; ADR non relus hors 0013/0023.

## Vérification des correctifs de l'audit n°3 (§2 du rapport) — conforme au plan

Constat de l'auditeur : **aucun correctif de code A3 appliqué — exactement le plan assumé** (« correction au vert », branche courte + PR ; A3-01 est le premier en cours : PR draft #475). Recoupements :

- B1 (lock sans forge-cad) : **corrigé** ✓ — cohérent avec `d5b4b2c` (v27).
- B2 : resté réfuté, lock cohérent ✓.
- B3 : **confirmé ouvert** — et désormais dépassé par l'exécution : le spike v41 a constaté les aborts/segfaults C++ (A4-14).
- B4 (toolchain) : **corrigé** ✓ — `1.99.1` + rustfmt/clippy.
- B5 : **corrigé côté configuration** (`deny.toml` `3474f1d`) ; le job CI n'existait toujours pas → A4-12, **corrigé le jour même** (workflow dédié activé, décision Raphaël).
- A3-01..A3-18 « non corrigés » : attendu (backlog au vert) — pas de re-signalement, l'auditeur a vérifié au lieu de re-signaler (protocole respecté). **Convergence** : sa re-vérification d'A3-01 (gc_blobs purge les blobs des événements appliqués) confirme la nôtre (v41.2) ; A3-08.a est confirmé à l'exécution par le spike.
- D1-D5, D7 corrigés ✓ ; D6 ouvert avec A3-06 ✓.

## Verdicts du mainteneur (traités le 2026-10-29 — validés en bloc par Raphaël)

Protocole §5 : **corrigé** · **dette acceptée** · **refusé**. Aucun réfuté : chaque signalement est soit confirmé par nos propres constats, soit plausible en lecture statique et honnêtement marqué « non vérifié » par l'auditeur.

| ID | Sév. | Constat (résumé) | Verdict | Traitement |
|---|---|---|---|---|
| A4-01 | majeur | Boucle `x += pas` non bornée des grilles 2D/3D : gel + OOM dès ~2e12 mm (arrondi f64, l'addition n'avance plus) | **dette acceptée → correction au vert** | Inséré **après A3-02/03** (décision Raphaël 2026-10-29, même classe « fichier forgé ») ; correctif retenu : itération entière `x = (k0 + k)·pas` + `MAX_LIGNES` (~4096) + `pas <= 0`/non fini refusé en 3D + borne de magnitude forge-geom (cf. A4-13b) |
| A4-02 | majeur | Wireframe STL reconstruit à chaque frame (`scene_vue_3d` à chaque draw) : O(triangles)/frame CPU, ~288 Mo de tampons au budget max (4 M triangles) > limite GPU 256 Mio ; vue 2D pire | **dette acceptée → correction au vert** | Idem A4-01 ; le correctif « cache à clé d'édition » est **déjà designé** (phase E, ADR-0027 « tampons à clé d'édition », patron `camera_cle`) — à anticiper en lot robustesse + plafond/décimation du wireframe + refus typé plutôt que crash |
| A4-03 | majeur | Liste `unknown` des projections : `Vec::contains` O(n²) sur journal forgé (64 MiB, ~centaines de milliers de types distincts), reRejoué à chaque ouverture/undo/redo dans 3 projections | **dette acceptée → correction au vert** | Idem A4-01 ; HashSet + plafond (ex. 256 types retenus, le reste compté) |
| A4-04 | mineur | Statut solveur non mappé (valeur 3 éventuelle affichée « AbiInvalide/exception C++ », trompeur) ; `copier()` tronque à 32 tags sans indicateur | **dette acceptée** (mineurs) | À vérifier au premier contact planegcs (sous-module absent du zip) ; champ `tronque` à `GcsDiagnostic` (non cassant) |
| A4-05 | hygiène | Shim : arène munmap-ée avant `systeme` (ordre de destruction inverse) ; commentaire budget faux (2^20 doubles = 8 Mio/1 048 576 paramètres, pas ~1 Mo/131 072) | **dette acceptée** (mineurs) | Commentaire faux confirmé par arithmétique ; correction triviale (déclarer `params` avant `systeme`) au lot hygiène ; use-after-free théorique (non vérifié, sous-module absent) |
| A4-06 | hygiène | `build.rs` forge-gcs-sys : vcpkg activé par seule variable d'env (triplet x64-windows quelle que soit la cible) ; `rerun-if-changed` incomplet (ni CMakeLists ni planegcs) | **dette acceptée** (mineurs) | Lot hygiène : conditionner à TARGET windows + rerun élargis |
| A4-07 | mineur | Arc : le balayage affiché (signé, sens horaire possible) peut différer de celui du solveur (planegcs = CCW start→end) — **non vérifié** | **dette acceptée** (mineurs) | À trancher par le test proposé (arc 270°→90° doit passer par 0°) ; historique jalon 2 : arcs à balayage signé suivant le geste (migration v2) — le risque est réel, non prouvé |
| A4-08 | hygiène | `#![forbid(unsafe_code)]` absent de 7 crates | **dette acceptée** (mineurs) | Nuance : la convention écrite est « unsafe seulement aux frontières FFI », pas forbid() partout ; forbid ajoutable aux crates sans unsafe (forge-cad, forge-render, forge-session, forge-values, omnifab-desktop), forge-gcs/native.rs reste la frontière FFI attendue (unsafe contrôlé) |
| A4-09 | hygiène | Doc forge-i18n « sans marques d'isolation » possiblement fausse (isolation Fluent FSI/PDI) — **non vérifié** | **dette acceptée** (mineurs) | Contrôle visuel galerie + test `traduction_avec("avec-argument", 12)` sans U+2068/U+2069 ; sinon `set_use_isolating(false)` ou corriger la doc |
| A4-10 | mineur | Tests i18n : 15 clés utilisées absentes de `CLES`/`HELP_IDS` (14 + page aide) — CI resterait verte, l'UI paniquerait si une clé disparaissait du FTL | **dette acceptée** (mineurs) | Aucun bug actuel (recoupement auditeur : 107 clés, 0 absente du FTL, 0 doublon) ; ajouter les 15 entrées ou dériver d'une table unique partagée avec le code |
| A4-11 | hygiène | Commentaire `deny.toml` périmé : « liaison dynamique obligatoire = obligation LGPL satisfaite » alors qu'OCCT est builtin **statique** (ADR-0023 D1 amendé) et planegcs/shim statiques (ADR-0013) | **corrigé (doc) le 2026-10-29** | Commentaire réécrit : conformité LGPL par **distribution des sources recompilables** de l'ensemble GPL-3.0+ — **reformulation validée explicitement par Raphaël** (licences = décision à validation explicite) |
| A4-12 | mineur | cargo-deny : aucun job CI (la fiche sécurité v2 affirme « job en commentaire dans ci.yml » — **faux**) ; chaîne APK non épinglée (cargo-apk sans --locked, cache « v1 » jamais invalidé, actions non épinglées par SHA, keystore debug mot de passe public) ; issues auto non dédoublonnées | **corrigé (doc + outillage) le 2026-10-29** | **Workflow dédié `cargo-deny.yml` activé** (décision Raphaël) ; fiche sécurité corrigée (v3 : le « job en commentaire » n'a jamais existé, APK debug noté) ; reste en hygiène supply chain : épinglage cargo-apk `--locked`, actions par SHA, nom « debug » de l'APK, dédoublonnage des issues auto |
| A4-13 | mineurs divers | (a) facteur NaN empoisonne définitivement la caméra (clamp propage NaN) ; (b) magnitude non bornée (1e300 accepté — cf. A4-01) ; (c) `From<usize>` → `as i64` troncature théorique | **dette acceptée** (mineurs) | (a) ignorer facteur non fini/≤0, test `zoom_au_curseur(NaN)` caméra inchangée ; (b) borne de magnitude commune forge-geom règle A4-01 en amont ; (c) trivial |
| A4-14 | majeur | Cas dégénérés du prisme : **abort C++ non rattrapable** (`StdFail_NotDone` points confondus, constaté cas 1) ; `Solide::prisme` sans validation ; l'exemple sans argument abat le processus au cas 1 → cas 2-4 et booléen vide jamais exécutés | **confirmé par exécution** (spike v41 : abort cas 1, **segfault cas 2**, hauteur négative acceptée sans garde) → correctif déjà inscrit **A3-08.a** | Validation amont du pont cxx (≥ 3 points distincts, aire > 0, hauteur > 0 strict, dimensions finies, `OF-CAD-0002`) — leçon v41 : « OCCT n'apporte AUCUNE garde » ; **bloquant avant branchement UI du prisme (phase B, ADR-0024)** ; ordre des cas de l'exemple (fusionné sur main `21dc112`) à corriger dans le même lot (booléen vide d'abord, ou cas dégénérés sur argument explicite) |
| A4-15 | hygiène | Témoins du spike (`spike_phase_a.step`/`.brep`) écrits au répertoire courant, non ignorés par git | **corrigé (outillage) le 2026-10-29** | `.gitignore` + `*.step`/`*.brep` ; `#![allow(clippy::expect_used)]` daté à prévoir dans l'exemple au moment du lot E |

### Incohérences de fiches signalées (§5 du rapport)

| Constat | Verdict | Traitement |
|---|---|---|
| état-actuel : en-tête « Date du relevé : 2026-10-26 », section CI « PARTIEL sur `415925b` », dette n°1 « push du Cargo.lock bloque » — tous périmés (v37+) | **confirmé, corrigé** (v42 le 2026-10-29) | Corps de la fiche rafraîchi : date 2026-10-29, section CI (verte #312/`5e8b6f1`, `21dc112` à relever), dette n°1 remplacée par les branches de correction ; statut jalon 3 (phase A clôturée) |
| état-actuel : « 44 `expect()` sur 16 fichiers » vs 39/10 comptés | **écart à clarifier** | L'inventaire v6 datait de la tête `89d5c62` (avant les commits du 26-28) ; à recompter au moment du lot A/E (noté dans la fiche et le backlog) |
| sécurité v2 : « job cargo-deny en commentaire dans ci.yml » | **confirmé, corrigé** (v3) | Le job n'a jamais existé ; workflow dédié activé, statuts réalignés |
| deny.toml / ADR-0023 : `docs/occt-dynamique.md` cité comme caduc mais absent du dépôt | **accepté** (doc) | À écrire noir sur blanc (rejet formel de l'OCCT dynamique) au prochain passage doc ; cohérent |
| fiches v35/v36 (camcraft, upstream #234) appuyées sur `preparation/` hors dépôt | **par conception** | `preparation/` est exclu du miroir public (ADR-0028) — pas un écart ; les originaux vivent dans la Knowledge privée (omnifab-prive) |
| liens-md.yml : fin du script (message d'échec, code de sortie) non relue | **à vérifier** | Prochaine veille |

**Décisions Raphaël (2026-10-29)** : (1) verdicts validés en bloc ; (2) **A4-01/02/03 insérés après A3-02/03** (même classe « fichier forgé », gels/plantages à l'ouverture) ; (3) **workflow cargo-deny activé immédiatement** (workflow dédié, `ci.yml` inchangé) ; (4) **reformulation licence validée** : conformité LGPL par distribution des sources recompilables + GPL-3.0+ racine (A4-11).

**Ordre intégré retenu** (A3 + A4, protocole branche courte + PR, test d'abord rouge/vert, un commit par point) : 1. A3-01 (en cours, PR #475) ; 2. A3-02/03 ; 3. **A4-01/02/03** ; 4. A3-06/07 + N8 (perte de travail) ; 5. A3-04/05 (fiabilité import) ; 6. A3-08 (leçons spike intégrées + A4-14 + ordre des cas de l'exemple + A4-15) — avant tout branchement UI du prisme (phase B) ; 7. mineurs A3-09..18 + hygiène A4-04..A4-10, A4-13 (A4-05/06 avec le shim/build.rs).

**Bonne prise de l'audit** : conformité exemplaire au protocole (vérification des correctifs du n°3 au lieu de re-signaler, §2 entier ; auto-marquage des « non vérifié » ; recoupements chiffrés — 107 clés i18n, 39 expect, 330 `#[test]`) ; l'auditeur a su distinguer preuve et hypothèse. Les écarts de comptage (44/16 vs 39/10 expect ; « 72 tests OK » fiche v34 vs 330 `#[test]` comptés — vraisemblablement un sous-ensemble par crate, ambigu) sont à lever par re-dénombrement au moment des lots concernés, pas par mémoire.

---

## Rapport intégral (verbatim, tel que soumis par l'auditeur)

> [J0] — Audit externe n°4 (Claude, analyse statique) : 15 signalements A4 + vérification des correctifs de l'audit n°3
>
> Date : 2026-10-29 · Auditeur : Claude (analyse statique, ni cargo ni rustc) · Format : docs/audit-protocole.md §4 (numérotation A4-NN)
>
> SHA audité : non disponible côté auditeur (zips sans .git ; la consigne contenait le littéral <SHA>). État documenté : fiche état actuel v38 (2026-10-28), CI verte sur 5e8b6f1. À rattacher au SHA réel à l'archivage.
> Deux états ont été lus : (1) zip OmniFab-main_4_ (état v36), (2) zip OmniFab-main_1_ (état v38). Entre les deux, aucun code de production n'a changé. Différences : crates/forge-cad/examples/spike_phase_a.rs (nouveau, 171 lignes), liens-md.yml (activé sur push *.md), conventions v10, etat-actuel v38. Tous les signalements ci-dessous valent pour les deux états.
> Limites : aucun test exécuté ; third_party/planegcs (sous-module) absent du zip, donc non auditable. Ce qui n'est pas confirmé est marqué « non vérifié ».
>
> == 1. INVENTAIRE (25 705 lignes de .rs au dernier état ; 25 534 au premier) ==
>
> Workspace : 13 membres (le périmètre du n°3 en listait 9) : omnifab-android, omnifab-desktop, forge-cad, forge-doc, forge-gcs, forge-gcs-sys, forge-geom, forge-i18n, forge-io, forge-render, forge-session, forge-units, forge-values.
> - omnifab-desktop : main.rs 5232 (production jusqu'à la ligne 3081, tests dès 3082), atelier_esquisse.rs 1488, bibliotheque.rs 158, aide.rs 129, i18n.rs 229, fichiers.rs 116, esquisse.rs 26 (+ locales/fr/main.ftl 546 lignes).
> - forge-session : esquisse.rs 2000, command.rs 749, lib.rs 517, projection.rs 409 (+ tests).
> - forge-values 1472, forge-doc ~1300, forge-io ~1460 (dxf 590, svg 426, stl 325), forge-geom 285, forge-units 287.
> - forge-cad : lib 304, occt 104, erreur 47, tests/golden 142, + examples/spike_phase_a.rs 171 (NOUVEAU).
> - forge-gcs : native 275, lib 281 ; forge-gcs-sys : build.rs 132, lib 29 (jamais cité au périmètre du n°3).
> - forge-render : renderer 387, camera3d 344, scene3d 291, scene 230, egui_adapteur 190, camera 183, lib 36, trace2d.wgsl 38.
> - forge-i18n : catalogue 225, lib 110, erreur 57. omnifab-android : lib.rs 201.
> - Shim C++ : gcs_c_api.cpp 829, gcs_c_api.h 197, CMakeLists.txt 45.
> Nouveau ou modifié depuis le n°3 : forge-cad (OCCT 7.8.1 builtin, Debug pour Forme, échappement rustdoc [J3], exemple spike), rust-toolchain.toml (1.99.1), deny.toml, Cargo.lock (forge-cad, occt-sys, opencascade 0.3.0), ci.yml/android.yml/liens-md.yml, ADR-0016/0019 poussés, ADR-0023 amendé, fiches v10→v38.
> Écarts de comptage : 39 expect( en production sur 10 fichiers (la fiche dit 44 sur 16) ; 0 unwrap() en production ; 330 #[test] (la fiche v34 dit « 72 tests OK » sur Linux : ambigu, non recoupé).
>
> == 2. VÉRIFICATION DES CORRECTIFS DU N°3 (par lecture) ==
>
> Constat global : AUCUN correctif de code A3 n'est appliqué (conforme à la fiche : « correction au vert »).
> - B1 lock sans forge-cad : CORRIGÉ (forge-cad, occt-sys 7.8.1, opencascade 0.3.0 présents dans Cargo.lock).
> - B2 : réfuté au n°3, cohérent avec le lock.
> - B3 : OUVERT (risque comportemental OCCT) ; le spike confirme désormais des aborts C++ (voir A4-14 et A3-08).
> - B4 : CORRIGÉ (rust-toolchain.toml channel 1.99.1 + rustfmt, clippy).
> - B5 : CORRIGÉ côté configuration (deny.toml : private workspace, LGPL-2.1/GPL-3.0 autorisées). Le job CI cargo-deny N'EXISTE TOUJOURS PAS (voir A4-12).
> - A3-01 : NON CORRIGÉ. forge-session/src/lib.rs:237,267,428 : gc_blobs ne conserve que les références de l'état courant.
> - A3-02 : NON CORRIGÉ. forge-values/src/expr.rs:233-310 : PROFONDEUR_MAX ne couvre que parenthèses, arguments, négations ; les boucles + - * / ne sont pas bornées.
> - A3-03 : NON CORRIGÉ (aucune borne de chaîne de dépendances trouvée par grep ; non relu en détail).
> - A3-04 : NON CORRIGÉ. forge-io/src/svg.rs:126 : texte.find("viewBox").
> - A3-05 : NON CORRIGÉ. dxf.rs : aucune occurrence de paper / is_visible / layer.
> - A3-06, A3-07 : NON CORRIGÉS. Aucun close_requested/CancelClose ; std::fs::read à main.rs:263 et 288.
> - A3-08 a-e : NON CORRIGÉ. forge-cad/src/lib.rs:63-129 renvoie Ok inconditionnel ; formes_max fictif ; temp_dir() sans 0o600 ni garde Drop. Aggravation : voir A4-14.
> - A3-09/10 : NON CORRIGÉS (set_input_unite sans revalidation des dépendants). A3-11 : NON CORRIGÉ (forge-doc/src/lib.rs:150-165, now_unix()*1000, octets aléatoires nuls). A3-13/18 : NON CORRIGÉS (container.rs:43). A3-14 : NON CORRIGÉ (stl.rs:60-77). A3-17 : NON CORRIGÉ (Length::from_millimeters accepte NaN/inf). N8 (catch_unwind) : NON CORRIGÉ.
> - D1-D5, D7 (docs) : CORRIGÉS. D6 : ouvert (lié à A3-06).
>
> == 3. SIGNALEMENTS A4 ==
>
> --- A4-01 · Grille 2D/3D : boucle x += pas non bornée, gel + OOM ---
> Fichier:ligne : crates/forge-render/src/scene.rs:126-145 ; crates/forge-render/src/scene3d.rs:172-191
> Sévérité : majeur · Classe : bug de robustesse (fichier forgé ou coordonnées extrêmes)
> Preuve (calcul, non exécuté) : ajouter_grille avance par x += pas dans while x <= max_x. Quand |x| >= 2^52 * pas, l'addition n'avance plus (l'espacement des f64 dépasse le pas). Seule la finitude des coordonnées est validée (forge-geom/src/lib.rs:37-40 ; forge-session esquisse.rs raison_finie) ; aucune borne de magnitude. Scénario : importer un tracé, ou poser un point d'esquisse, vers 2e12 mm. Le cadrage automatique (main.rs:2536) donne echelle = ECHELLE_MAX = 1e6, donc pas = 2^-13 ≈ 1,2e-4 mm ; x + pas retombe sur x (arrondi pair) ; la boucle ne termine plus et Vec<Segment2D> grossit jusqu'à l'OOM. Scene3D::ajouter_grille boucle aussi sans fin si pas < 0 (API publique non validée).
> Correctif : calculer n = ((max - min) / pas).ceil(), refuser au-delà de MAX_LIGNES (~4096), itérer sur un entier k avec x = (k0 + k) * pas ; rejeter pas <= 0 ou non fini en 3D ; idéalement borner la magnitude des coordonnées dans forge-geom.
> Test : ajouter_grille(Boite2{min_x:2e12, max_x:2e12+1e-3, min_y:0, max_y:1e-3}, 1e6, 80.0, ..) rend en temps fini avec segments.len() <= 2*MAX_LIGNES ; Scene3D::ajouter_grille(.., pas=-1.0, ..) ne boucle pas.
>
> --- A4-02 · Maillage STL : wireframe reconstruit à chaque frame, au-delà des limites GPU ---
> Fichier:ligne : apps/omnifab-desktop/src/main.rs:1764-1782, 1834-1837, 2689 ; crates/forge-render/src/scene3d.rs:102-138 ; crates/forge-render/src/renderer.rs:185-228
> Sévérité : majeur · Classe : bug / risque non documenté (ADR-0015 ne documente que le tampon recréé à chaque draw)
> Preuve : StlBudgets::default autorise 4 000 000 triangles (stl.rs:49). La vue 3D appelle scene_vue_3d à chaque frame (main.rs:2689), donc Scene3D::depuis_maillage (HashSet d'arêtes sur tout le maillage) puis lignes() : ~6 M arêtes x 2 sommets x 24 octets ≈ 288 Mo, au-dessus des 256 Mio par défaut de max_buffer_size (aucun required_limits dans le code). Une erreur de validation wgpu est, je crois, fatale par défaut (non vérifié). La vue 2D (ajouter_maillage_2d) émet 3 segments par triangle sans dédoublonnage : ~12 M Segment2D et 72 M sommets (~1,7 Go) par frame. Le coût CPU par frame est de toute façon O(triangles).
> Correctif : mettre la scène en cache avec une clé d'édition (le patron existe : camera_cle) ; plafonner le wireframe (décimation ou contour seul) au-dessus d'un seuil ; refus typé plutôt que crash.
> Test : maillage de 500 000 triangles, scene_vue_3d appelée deux fois sans reconstruction ; nombre de sommets envoyés <= plafond.
>
> --- A4-03 · Liste unknown des projections : O(n²) sur journal forgé ---
> Fichier:ligne : crates/forge-session/src/esquisse.rs:1984-1985 ; crates/forge-session/src/projection.rs:138-139 et 268-269
> Sévérité : majeur (gel à l'ouverture ; même classe qu'A3-02) · Classe : bug de robustesse
> Preuve : if !self.unknown.contains(&event.event_type) { self.unknown.push(..) } sur un Vec<String>. Le journal admet 64 Mio (MAX_JOURNAL_BYTES), soit plusieurs centaines de milliers d'événements de types tous distincts. Chaque rejeu (ouverture, et chaque undo/redo via replay_applied) refait ce travail dans trois projections.
> Correctif : HashSet plus plafond (ex. 256 types retenus, le reste compté).
> Test : journal de 200 000 types d'événement distincts rejoué en moins de N secondes.
>
> --- A4-04 · Statut solveur inconnu mal traduit ; troncature silencieuse du diagnostic ---
> Fichier:ligne : crates/forge-gcs/src/native.rs:198-207 ; third_party/planegcs-shim/gcs_c_api.cpp:138-150
> Sévérité : mineur · Classe : risque à documenter
> Preuve : gcs_resoudre renvoie le GCS::SolveStatus brut. S'il existe une valeur 3 dans l'extraction (SuccessfulSolutionInvalid chez FreeCAD ; non vérifié, sous-module absent), Rust l'affiche « AbiInvalide ... exception C++ rattrapée », trompeur ; le header ne documente que 0, 1, 2. Par ailleurs copier() tronque à GCS_MAX_TAGS = 32 sans indicateur : au-delà de 32 contraintes conflictuelles ou redondantes, le reste est perdu sans signal.
> Correctif : mapper explicitement ce statut vers une erreur typée ; ajouter un champ tronque à GcsDiagnostic / Diagnostic.
> Test : 40 contraintes redondantes, Diagnostic.tronque == true.
>
> --- A4-05 · Shim : ordre de destruction et commentaire de budget faux ---
> Fichier:ligne : third_party/planegcs-shim/gcs_c_api.cpp:205-217 et 125-128
> Sévérité : hygiène · Classe : risque / doc
> Preuve : struct GcsSysteme déclare GCS::System systeme AVANT Arena params ; la destruction suit l'ordre inverse, donc l'arène (munmap) est libérée avant systeme. Si un destructeur de contrainte déréférençait un double*, ce serait un use-after-free (probablement non ; non vérifié, sous-module absent). Le commentaire « ~1 Mo de plage réservée (131 072 paramètres) » est faux : 1u << 20 doubles = 8 Mio = 1 048 576 paramètres.
> Correctif : déclarer params avant systeme ; corriger le commentaire.
>
> --- A4-06 · build.rs de forge-gcs-sys : vcpkg activé selon l'environnement, rebuild incomplet ---
> Fichier:ligne : crates/forge-gcs-sys/build.rs:65-76, 89-101, 127-131
> Sévérité : hygiène · Classe : risque
> Preuve : VCPKG_INSTALLATION_ROOT suffit à activer le toolchain vcpkg avec le triplet par défaut x64-windows, quelle que soit la plateforme cible (les runners GitHub Linux exportent cette variable : non vérifié). rerun-if-changed ne couvre ni CMakeLists.txt ni le sous-module planegcs : les modifier ne relance pas le build.
> Correctif : conditionner le bloc vcpkg à TARGET contenant « windows » ; ajouter rerun-if-changed (CMakeLists.txt) et rerun-if-env-changed (VCPKG_*).
>
> --- A4-07 · Arc : balayage affiché peut différer de celui du solveur (non vérifié) ---
> Fichier:ligne : apps/omnifab-desktop/src/atelier_esquisse.rs:1468-1478 et 403-410
> Sévérité : mineur · Classe : risque (non vérifié)
> Preuve : angles_arc utilise fin - debut brut (négatif = sens horaire) et balaie() produit des balayages négatifs ; planegcs traite un arc comme CCW de start à end. Un arc créé dans le sens horaire pourrait s'afficher du côté opposé à celui que contraint le solveur. Le comportement exact de addConstraintArcRules n'est pas lisible ici.
> Test proposé : arc 270° -> 90° ; le tracé affiché doit passer par 0°.
>
> --- A4-08 · #![forbid(unsafe_code)] absent de 7 crates ---
> Fichier:ligne : crates/forge-cad/src/lib.rs, forge-gcs-sys, forge-gcs, forge-render, forge-session, forge-values, apps/omnifab-desktop/src/main.rs
> Sévérité : hygiène · Classe : violation de convention (unsafe seulement aux frontières FFI)
> Preuve : seuls forge-doc, forge-geom, forge-i18n, forge-io, forge-units portent la garde. Le seul unsafe hors FFI scellée est #[unsafe(no_mangle)] d'android_main (point d'entrée FFI légitime). forge-gcs/native.rs est la frontière FFI attendue.
> Correctif : ajouter #![forbid(unsafe_code)] à forge-cad, forge-render, forge-session, forge-values, omnifab-desktop.
>
> --- A4-09 · forge-i18n : doc sur les marques d'isolation (non vérifié) ---
> Fichier:ligne : crates/forge-i18n/src/catalogue.rs:135-140
> Sévérité : hygiène · Classe : risque à documenter
> Preuve : la doc dit qu'Argument::Entier est formaté « sans marques d'isolation ». Si Fluent isole toutes les variables d'un message à plusieurs éléments (comportement de fluent-rs à confirmer), « { $n } événements » contiendra aussi FSI/PDI autour du nombre ; ni set_use_isolating(false) ni nettoyage n'existent côté desktop. Ces marques peuvent s'afficher en caractères manquants selon la police.
> Correctif : vérifier visuellement (galerie) ; sinon désactiver l'isolation ou corriger la doc. Test : traduction_avec("avec-argument", n=12) ne contient pas U+2068/U+2069.
>
> --- A4-10 · Tests d'i18n : listes de clés tenues à la main, incomplètes ---
> Fichier:ligne : apps/omnifab-desktop/src/i18n.rs:35-190
> Sévérité : mineur · Classe : suggestion (le texte() du desktop fait un expect)
> Preuve : recoupement fait : 107 clés statiques utilisées dans le code, 0 absente du FTL (191 messages, 0 doublon). Mais 14 clés utilisées (esquisse-contraintes-poses, contrainte-libelle-*) et la page aide-esquisse-contraintes-poses ne figurent pas dans CLES / HELP_IDS. Si l'une disparaissait du FTL, la CI resterait verte et l'UI paniquerait (main.rs:1061, expect).
> Correctif : ajouter ces 15 entrées, ou dériver la liste d'une table unique partagée avec le code.
>
> --- A4-11 · deny.toml : justification de licence périmée ---
> Fichier:ligne : deny.toml (commentaire de [licenses])
> Sévérité : hygiène · Classe : doc inexacte
> Preuve : le commentaire affirme « liaison dynamique obligatoire = obligation LGPL satisfaite ». Faux : planegcs et le shim sont liés en statique (build.rs:83-84, ADR-0013) et OCCT est builtin statique depuis l'amendement d'ADR-0023 (D1). La conformité repose sur la distribution des sources et la licence GPL-3.0+ de l'ensemble.
> Correctif : réécrire le commentaire. La décision de licence reste à validation explicite de Raphaël.
>
> --- A4-12 · CI : cargo-deny absent, fiche sécurité inexacte, chaîne APK non épinglée ---
> Fichier:ligne : .github/workflows/ci.yml (aucun job deny) ; docs/knowledge/securite/KNOWLEDGE.md:38 ; .github/workflows/android.yml
> Sévérité : mineur · Classe : doc inexacte + risque
> Preuve : la fiche sécurité (v2) dit « le job CI est en commentaire dans ci.yml » : il n'y en a aucun (grep deny : zéro occurrence dans .github/workflows ; la proposition est dans docs/ci-cargo-deny.md). android.yml : cargo install cargo-apk sans version ni --locked ; clé de cache « v1 » jamais invalidée ; actions non épinglées par SHA ; l'APK est signé avec un keystore de debug généré à la volée (mot de passe public « android »), alors que le sommaire parle d'APK signé. Chaque échec de push crée une issue (151 issues « CI : » fermées après le vert) : pas de dédoublonnage.
> Correctif : corriger la fiche, activer ou retirer la mention du job ; épingler cargo-apk (--locked) ; nommer l'APK « debug » ; dédoublonner les issues auto.
>
> --- A4-13 · Mineurs divers ---
> (a) crates/forge-render/src/camera.rs:37-43 et camera3d.rs:97-99 : un facteur NaN empoisonne la caméra définitivement (clamp propage NaN). Correctif : ignorer facteur non fini ou <= 0. Test : zoom_au_curseur(.., NaN) laisse la caméra inchangée.
> (b) forge-session esquisse.rs raison_finie / forge-geom : coordonnées de 1e300 acceptées (voir A4-01). Une borne de magnitude commune règlerait A4-01 en amont.
> (c) crates/forge-i18n/src/catalogue.rs:149-153 : From<usize> fait valeur as i64 (troncature théorique).
>
> --- A4-14 · Spike phase A : les cas dégénérés abattent le processus avant la fin ; A3-08.a confirmé ---
> Fichier:ligne : crates/forge-cad/examples/spike_phase_a.rs (en-tête et boucle des cas 1-4) ; crates/forge-cad/src/lib.rs:99-109 (Solide::prisme)
> Sévérité : majeur (A3-08.a passe de latent à constaté ; bloquant avant branchement UI du prisme) · Classe : bug + suggestion
> Preuve : le commentaire de l'exemple indique qu'un contour dégénéré provoque un abort C++ (StdFail_NotDone) qui « abat le process », « constaté cas 1 ». Solide::prisme n'a aucune validation (points confondus, moins de 3 points, hauteur nulle ou négative partent vers OCCT) ; un Result ne rattrape pas un abort. Dès que ADR-0024 branchera un croquis sur le prisme, un contour dégénéré fera planter l'application avec perte du travail non enregistré (A3-06 non corrigé). De plus, sans argument, l'exemple lance les cas 1 à 4 dans l'ordre : le cas 1 abat le processus, donc les cas 2 à 4 et la section 5 (booléen vide self - self, attendu par la fiche) ne s'exécutent jamais.
> Correctif : valider avant OCCT dans Solide::prisme (>= 3 points distincts, aire non nulle, dimensions finies et > 0) avec OF-CAD-0002 ; dans l'exemple, exécuter le booléen vide avant les cas dégénérés, ou ne lancer ceux-ci qu'avec un argument explicite.
> Test : prisme(contour de 4 points confondus, 12.0), prisme(2 points, 12.0), prisme(carré, 0.0), prisme(carré, -5.0) renvoient chacun Err(FormeInvalide) sans abort.
>
> --- A4-15 · Fichiers témoins du spike non ignorés par git ---
> Fichier:ligne : crates/forge-cad/examples/spike_phase_a.rs (écritures de spike_phase_a.step / .brep dans le répertoire courant) ; .gitignore
> Sévérité : hygiène
> Preuve : .gitignore n'a ni *.step ni *.brep ; lancer l'exemple à la racine du dépôt crée des fichiers à commiter par erreur. Autre point : l'exemple utilise .expect(..) ; quand clippy expect_used sera en deny (lot E) avec --all-targets, il faudra #![allow(clippy::expect_used)] daté dans ce fichier.
> Correctif : ajouter *.step et *.brep à .gitignore (ou écrire sous target/) ; prévoir l'allow daté.
>
> == 4. DETTE CONNUE : AGGRAVATIONS OU RÉGRESSIONS ==
> - expect() en production : 39 (8 dans egui_adapteur.rs sur le chemin de rendu : Renderer2D initialisé, verrou de frame). Déjà couvert par le lot A des patchs ; pas d'aggravation.
> - Aucun unwrap() en production. unsafe confiné aux frontières FFI (forge-gcs/native.rs, android_main).
> - Registre des codes OF-* : aucun code du code absent de docs/error-codes.md, et inversement.
> - A3-08.a : aggravation (voir A4-14).
>
> == 5. FICHES docs/knowledge/ : INCOHÉRENCES AVEC LE CODE ET ENTRE ELLES ==
> 1. etat-actuel v38 : l'en-tête garde « Date de ce relevé : 2026-10-26 » (l.9) et la dette n°1 « CI au vert : push du Cargo.lock — bloque toute validation compile » (l.34), alors que v37 annonce la CI verte. La section « État de la CI — PARTIEL sur 415925b » n'a pas été remplacée : contradiction interne de la fiche. La ligne « Version » est un bloc de plusieurs milliers de caractères (dernière version en premier) ; une version par ligne dans « Historique » serait plus lisible.
> 2. etat-actuel : « 44 expect() sur 16 fichiers » ; je compte 39 sur 10.
> 3. securite v2 : « job cargo-deny en commentaire dans ci.yml » est faux (A4-12).
> 4. deny.toml / ADR-0023 : voir A4-11 ; docs/occt-dynamique.md est cité comme caduc et n'existe pas dans le dépôt (cohérent, à écrire noir sur blanc).
> 5. Les fiches v36 (camcraft) et v35/v38 (contribution upstream #234) s'appuient sur des fiches preparation/ hors dépôt : non vérifiables ici.
> 6. liens-md.yml est activé sur push *.md : fin du script (message d'échec, code de sortie) non relue ; non vérifié.
>
> == 6. NON COUVERT ==
> - Lecture ligne à ligne non faite : la plus grande partie de main.rs (UI), atelier_esquisse.rs (balayé par motifs ; lu en détail : clics, sélection, arc), forge-session/esquisse.rs lignes 1150-1650 (passes de résolution), scene3d.rs et egui_adapteur.rs (balayage), command.rs et projection.rs (non relus depuis le n°3).
> - Non vérifiable ici : third_party/planegcs, comportement réel des statuts du solveur, wgpu/OCCT/eframe à l'exécution, tout ce qui demande de compiler. A4-02 (limite wgpu), A4-04 (statut 3), A4-05 (use-after-free), A4-07 (arc), A4-09 (isolation Fluent) sont des hypothèses de lecture, pas des preuves.
> - Docs : ADR non relus un par un hors ADR-0013/0023 ; fiches lues par recoupement ciblé, pas intégralement.
> - Tests : les spécifications de tests A3 sont dans preparation/ (hors zip) ; je n'ai pas pu vérifier qu'elles couvrent les correctifs.
>
> == 7. ORDRE DE TRAITEMENT PROPOSÉ ==
> 1. A4-14 / A3-08.a : valider contours et dimensions avant tout branchement du prisme à l'UI.
> 2. A4-01, A4-02, A4-03 : gels ou plantages à l'ouverture d'un fichier ; puis A3-01 (perte d'undo), A3-02/03, A3-06/07 + N8, A3-04/05.
> 3. A4-04 à A4-13, A4-15 en lot « hygiène ».
> Chaque correctif : test d'abord (rouge avant, vert après), un commit par point, branche courte + PR, verdict explicite par signalement (corrigé / dette acceptée / refusé) selon docs/audit-protocole.md §5.
