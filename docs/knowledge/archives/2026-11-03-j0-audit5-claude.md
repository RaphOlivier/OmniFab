# [J0] Audit externe n°5 (Claude, analyse statique) — 8 signalements A5 + revérification des compléments A4-16..21, verdicts validés en bloc (0 réfuté)

**Date :** 2026-11-03 · **Auditeur :** Claude (analyse statique, ni cargo ni rustc — lecture seule grep/awk/sed) · **Format :** `docs/audit-protocole.md` §4 (numérotation A5-NN).
**SHA rattachés (2026-11-03, réception)** : zip = code de production **`c36cb15`** (PR #605 fusion squash, phase B lot 2, 2026-11-03 09:12 UTC) + état docs de la tête **`cd02d5d`** (fiche v73 `a6c4fb9`, matrice v9 `646fdb9`, protocole d'audit §5 amendé `39043d5` — ce qui explique la présence de l'exigence « registre de couverture » dans la consigne alors qu'aucun `coverage-ledger.json` n'existait encore). Le zip sans `.git` ne permettait pas à l'auditeur de lire le SHA ; le rattachement est établi par la réception (contenu v73 + lot 2 + protocole §5 = entre `a6c4fb9` et `cd02d5d`, code identique à `c36cb15`).
**Écart de capitalisation consigné :** les compléments A4-16..21 et les « 14 conseils d'architecture » cités par la consigne **n'existent dans aucun des deux dépôts** (l'audit n°4 archivé, [2026-10-29](2026-10-29-j0-audit4-claude.md), s'arrête à A4-15) — ils vivaient dans la seule consigne transmise à l'auditeur. Les définitions font désormais foi via la table §2 du présent rapport ; la consigne d'un audit futur doit joindre le rapport source ou son archive.

**Limites déclarées par l'auditeur :** aucun test exécuté, aucun signalement reproduit par exécution ; couverture nettement plus étroite que le protocole ne le demande (§7 du rapport : app.rs, atelier_esquisse.rs, ui/\*, travail.rs, forge-render/\*, forge-doc/\*, forge-gcs/\*, command.rs partiel, workflows hors ciblage, ADR-0022/0024 non relus) ; `third_party/planegcs` absent du zip, non auditable. **Premier registre de couverture créé à la réception :** `docs/knowledge/coverage-ledger.json` (le rapport n'en avait pas produit — le protocole §5 venait d'être adopté ; le registre est construit depuis les §1/§7 du rapport pour servir de point de départ à l'audit n°6).

## Verdicts du mainteneur (validés en bloc par Raphaël le 2026-11-03 — décision : 0 réfuté)

Protocole §6 : **corrigé** · **dette acceptée** · **refusé**. Recoupements du mainteneur : `profondeur_valide` absent du code (grep) ; `compte_instances` présent dans forge-cad/src/lib.rs et l'archive du lot 2 dit correctement « instances STEP » (seule la fiche v73 était imprécise) ; l'advisory zip GHSA-q7v9-x93c-xqv3 était déjà consigné comme mini-lot prioritaire planifié à la clôture du lot 2 ; la leçon n°5 du lot 2 (« la garde vit à la frontière, pas dans OCCT ») fait de A5-04 la brèche exacte de ce principe au rejeu.

| ID | Sév. | Constat (résumé) | Verdict | Traitement |
|---|---|---|---|---|
| A5-04 | majeur | Le rejeu du journal passe la profondeur `Fixe{mm}` au moteur sans contrôle (validation uniquement à l'écriture dans command.rs) ; couleur et « à travers » idem — le scénario d'abort du spike avec le futur moteur OCCT | **dette acceptée → correction au vert, PRIORITAIRE avant lot 3** | Garde commune `profondeur_valide()` dans reconstruire/apply (OF-CAD-0005) + bornes couleur/« à travers » ; tests journal mm=0, −5, 1e300, rgba hors bornes. Fusionné avec A4-16 dans la même passe « gardes avant OCCT » |
| A4-16 | majeur | Epsilons absolus de valider_contour (`f64::EPSILON`, `EPS_AIRE` 1e-12) : triangle (0,0),(1000,0),(500,5e-7) accepté ; alignement aléatoire à 1e6 mm. **L'exemple « lame 1 mm × 1e-9 mm » de l'audit 4 était faux** (refusé d'emblée par le contrôle d'arêtes ≤ TOLERANCE_MM) | **confirmé → correction au vert, PRIORITAIRE avant lot 3** | Tolérances relatives (aire/longueur) via le futur module `forge-geom::tolerance` (A5-07) ; contre-exemple de l'auditeur en test de régression ; l'exemple faux reste documenté ici (le rapport d'audit 4 archivé n'est pas réécrit) |
| A5-01 + A4-17 | majeur | Chaînage glouton `contours_des_segments` : fusion de boucles partageant un sommet, embranchement → boucle non fermée, un seul contour retenu (trous/îlots ignorés), O(n²) sans plafond ; `forge-geom::contours()` inchangé depuis le lot 1 | **confirmé → correction au vert, avant lot 3** | Graphe d'extrémités (index spatial à tolérance), cycles degré 2 (refus typé au-delà), orientation + parité pour les trous, plafond d'entités — dans `forge-geom::contours`, suppression de la version forge-session ; tests : 2 carrés sécants en un sommet, carré + queue, carré avec îlot, désordre/inversions |
| A5-02 + A4-18 | majeur | Un arc d'esquisse effondré (fin == début) est extrudé comme cercle complet — le choix lot 1 « balayage nul = cercle complet » devient atteignable depuis l'esquisse | **confirmé → correction au vert, avant lot 3** | `Segment2::Cercle` explicite + Arc de balayage nul refusé (OF-GEO à inscrire au registre) ; test arc début=fin=30° → jamais un cercle |
| A5-03 | mineur / doc | Commentaire de `segments_du_croquis` (cad.rs:732-745) inversé vs code et vs ADR-0020 | **corrigé (doc) au lot contours** | Commentaire rectifié + test de non-régression sur les deux sens d'arc, dans la même branche que A5-01/02 |
| A4-20 | mineur | `TypesIgnores` : plafond testé avant insertion → au-delà de 256 types, un type déjà connu est recompté `supplementaires` | **confirmé → correction au vert** | Inversion de deux lignes (projection.rs et CadProjection) ; test > 256 types distincts puis re-création d'un type connu |
| A4-19 | mineur | `MAX_OCTETS_IMPORT = 1 Gio` uniforme ; `DemandeEchelle.octets` non relu | **dette acceptée** | Plafond par format à arbitrer (même passe que A5-05/06) |
| A5-05 | mineur | features : `Vec` sans plafond, `len() as u32` tronquant, feature() O(n), tout événement non-CAO invalide le corps (clé `edition`) | **dette acceptée** | `MAX_FEATURES` (OF-xxx) + index par id + invalidation ciblée ; test MAX+1 features → refus typé |
| A5-06 | mineur | `lire_brep` sans budget de formes (formes_max ne vaut que pour STEP) — la fiche v73 était imprécise, l'archive du lot 2 était exacte | **confirmé → correction au vert** | Budget BREP (plafond d'octets ou comptage de `TShape`) ou documentation explicite « STEP only » ; fiche v74 rectifiée à la réception |
| A5-07 | hygiène | 3 politiques de tolérance dans valider_contour + epsilons en dur partout (dxf, segments, esquisse, renderer) | **dette acceptée → conseil 1, avant extrusion réelle** | Module `forge-geom::tolerance` (colineaire, distinct, aire_negligeable) + grep CI interdisant `1e-` hors forge-units/forge-geom |
| A4-21 | partiel | 11 ignores deny datés/motivés sans échéance automatique ; zip toujours 2.x avec l'advisory GHSA-q7v9-x93c-xqv3 | **dette acceptée → mini-lot D'ABORD (décision Raphaël)** | Mini-lot zip ≥ 9.0.0 déjà planifié à la clôture du lot 2 — exécuté en premier ; puis échéance automatique d'âge des ignores |
| A5-08 | hygiène / doc | Ligne « Version » de la fiche état-actuel : ~91 000 caractères, 55 entrées empilées (v73→v25) | **corrigé (structure) à la réception** | Restructuration fiche v74 : pile supprimée, entrées v73..v64.1 déplacées en section Historique (qui fait foi), v64..v1 y figuraient déjà, texte ancien retrouvable dans l'historique Git |

**Décisions Raphaël (2026-11-03)** : (1) verdicts validés en bloc, 0 réfuté ; (2) **mini-lot zip ≥ 9.0.0 (GHSA-q7v9-x93c-xqv3) d'abord** (sécurité, rapide), puis l'ordre §8 du rapport ; (3) capitalisation poussée le jour même (présente entrée, registre de couverture, fiche v74 restructurée, backlog `preparation/backlog-audit5.md` au miroir privé) ; (4) auditeur de l'audit n°5 : Claude, comme les audits 3 et 4.

**Ordre intégré retenu** (backlog détaillé : `preparation/backlog-audit5.md`, miroir privé) :
0. **Mini-lot zip ≥ 9.0.0** (advisory, sécurité) — décision Raphaël.
1. **A5-04 + A4-16** (gardes au rejeu + tolérances relatives) — avant de brancher OCCT (lot 3).
2. **A5-01 + A4-17 + A5-02 + A4-18 + A5-03** (contours en graphe dans forge-geom, `Cercle` explicite, doc du sens d'arc).
3. **A4-20** (inversion), **A4-19** (plafonds par format), **A5-05** (MAX_FEATURES), **A5-06** (budget BREP).
4. **A5-07** (tolérances unifiées, conseil 1) + proptest ciblé (expr, apply, valider_contour).
5. CI Windows forge-cad (réduction documentée ADR-0023 D8 à réviser — le lot 2 a branché forge-cad au flux vivant) ; échéance automatique des advisories ; fiche d'état générée par script.

**Bonne prise de l'audit** : format respecté (fichier:ligne, scénarios, auto-marquage « non vérifié par exécution ») ; correction honnête d'un faux exemple de l'audit 4 (A4-16) avec le vrai contre-exemple chiffré ; §7 « non couvert » assumé dès l'en-tête ; recoupements de fiches pertinents (§5). Le registre de couverture manquant est le seul écart au protocole §5 adopté le matin même — comblé à la réception. Les unités non lues (§7) sont inscrites au registre pour l'audit n°6.

---

## Rapport intégral (verbatim, tel que soumis par l'auditeur)

> # Audit n°5 d'OmniFab — analyse statique (partielle, voir §7)
>
> SHA : inconnu (zip sans .git). État documenté : KNOWLEDGE.md v73 (2026-11-03), main c36cb15 (PR #605, phase B lot 2). Ce n'est PAS v72/bd57c4c : le zip contient le lot 2. À rattacher au SHA réel à l'archivage.
> Méthode : lecture seule (bash : grep/awk/sed), aucun cargo/rustc. Tout ce qui n'a pas été exécuté est « non vérifié par exécution ». Couverture réelle : voir §7 — elle est nettement plus étroite que le §5 de la consigne.
>
> ## 1. Inventaire (écarts avec §2)
> - Workspace : 13 membres, inchangé (omnifab-android, omnifab-desktop, forge-cad/doc/gcs/gcs-sys/geom/i18n/io/render/session/units/values).
> - .rs : 95 fichiers, 32 666 lignes (v72 : 30 085, soit +2 581). Hors tests/ et examples/ : 24 385 lignes (inclut les modules #[cfg(test)] inline).
> - Nouveau : crates/forge-session/src/cad.rs (876, projection CAO, trait MoteurCad), crates/forge-session/tests/cad.rs (847, moteur de test MoteurPrismesAnalytiques).
> - Modifiés (taille v72 → v73) : forge-session/src/esquisse.rs 2 037 → 2 400 ; command.rs 759 → 1 018 ; forge-cad/src/lib.rs 625 → 668 (compte_instances, budget formes_max) ; forge-session/src/lib.rs (profondeurs_pour_cad, ~l.452).
> - Inchangés en taille : app.rs 3 501, session.rs (tests) 2 272, atelier_esquisse.rs 1 532, travail.rs 849, segments.rs 739, dxf.rs 766, forge-doc/lib.rs 756, viewport.rs 636.
> - Docs : 102 fichiers .md hors third_party (v72 : « 77+ »). ADR jusqu'à 0035 (ADR-0028, 0035 présents ; numérotation non contiguë, non vérifié si voulu).
> - Comptages (méthode awk : lignes avant le 1er #[cfg(test)] de chaque fichier ; non strictement comparable à l'audit 4) : expect() en production = 26 sur 9-10 fichiers (build.rs 6, stl.rs 7, scene.rs 2, scene3d.rs 2, dxf.rs 2, svg.rs 2, aide.rs 3, i18n.rs 1, main.rs 1) contre « 20 sur 8 » ; unwrap() en production = 0 ; unsafe réel seulement dans forge-gcs/src/native.rs (23 occurrences) ; android_main non recompté ; forbid(unsafe_code) présent dans 12 lib.rs/main.rs. Le +6 d'expect() est peut-être un artefact de méthode : à recompter avec clippy.
> - third_party/planegcs absent : non auditable.
>
> ## 2. Vérification des correctifs
> | ID | Verdict | Preuve |
> |---|---|---|
> | A4-16 | NON CORRIGÉ (et l'exemple de l'audit 4 était faux) | forge-cad/src/lib.rs:87 refuse déjà toute arête ≤ TOLERANCE_MM (1e-6) : la lame 1 mm × 1e-9 mm est donc refusée par le contrôle d'arêtes. Reste vrai : l:116 `aire.abs() < f64::EPSILON` absolu ; l:161 `EPS_AIRE = 1e-12` absolu. Contre-exemple : triangle (0,0),(1000,0),(500,5e-7) → arêtes ≈500, aire 2,5e-4 > epsilon, accepté alors que le sommet est à 5e-7 mm de la base (< tolérance). À 1e6 mm de coordonnées, les termes x1*y2 valent ~1e12 : l'erreur d'arrondi (~1e-4) dépasse l'epsilon, le test d'alignement est aléatoire. Non vérifié par exécution. |
> | A4-17 | PARTIEL | forge-geom/src/segments.rs:392 contours() inchangé (ordre d'entrée, un seul Contour2, ne contrôle pas la fermeture). Mais le chaînage réel passe maintenant par forge-session/src/cad.rs:840 contours_des_segments (glouton avec inversion). Pas de trous, pas de parité, seul le 1er contour fermé est utilisé (cad.rs:872) ; défauts : voir A5-01. |
> | A4-18 | NON CORRIGÉ, désormais atteignable depuis l'esquisse | segments.rs:211-216 : `positif <= 1e-12 → tour` ; cad.rs:787-789 : un arc d'esquisse avec angle_fin == angle_debut devient Arc trigo de balayage nul = cercle complet ; cad.rs:769 le cercle utilise le même encodage (debut=fin=0). |
> | A4-19 | NON CORRIGÉ | apps/omnifab-desktop/src/travail.rs:486 `MAX_OCTETS_IMPORT = 1 Gio`. DemandeEchelle.octets non relu. |
> | A4-20 | NON CORRIGÉ | forge-session/src/projection.rs:55 `if self.vus.len() >= MAX_TYPES_INCONNUS` testé avant `vus.insert` : au-delà de 256 types, un type déjà connu est recompté dans `supplementaires`. Même motif dans CadProjection (cad.rs, via TypesIgnores). |
> | A4-21 | PARTIEL | deny.toml:85 : 11 ignores, chacun daté 2026-10-29 et motivé (amélioration), mais aucune échéance ni vérification automatique d'âge ; zip toujours en « 2 » (la fiche v73 annonce un mini-lot zip ≥ 9.0.0). |
> | A3-02/03 | corrigé (constantes présentes ; logique non relue) | forge-values/src/lib.rs:52 LONGUEUR_MAX_EXPRESSION 4096 ; lib.rs:59 PROFONDEUR_MAX_CHAINE 1024 ; expr.rs:242 MAX_NOEUDS_AST 4096 ; OF-VAL-0011 error.rs:113. |
> | A3-15/16 | corrigé (en-tête relu) | svg.rs:67-69 resolve_string → None ; svg.rs:75+ refus des DTD internes. Limite : détection limitée à la chaîne ASCII `<!DOCTYPE` (non vérifié pour d'autres encodages/BOM : usvg ne lit pas UTF-16 de toute façon, non vérifié). |
> | A3-14 | partiel | stl.rs:73 et :142 est_binaire_tolerant présents ; 7 expect() d'invariant dans stl.rs (l.98-174, 322) dont la justification repose sur des gardes non relues. |
> | A3-17 | corrigé (existence) | forge-units/src/lib.rs:53 Length::try_millimeters (rejette les non finis). |
> | A4-07 | corrigé | forge-geom/src/lib.rs:37 MAGNITUDE_MAX_MM = 1e9, appliqué dans Point2/Point3::new (l.58-108). |
> | A4-08 | corrigé (compte) | forbid(unsafe_code) dans 12 fichiers. |
> | A3-08.e | corrigé pour STEP, PAS pour BREP | forge-cad/src/lib.rs:507-520 compte_instances ≤ formes_max pour STEP. Pour BREP (l.570-574) le budget n'est vérifié que `>= 1` : aucun refus par nombre de formes. La fiche v73 écrit « budget formes_max réel » sans restreindre à STEP. |
> | A3-01, A3-04/05, A3-09..13/18, A4-04/05/06, A4-09..15 | NON VÉRIFIÉ | non relus faute de budget de lecture. |
> Fait nouveau non couvert par un correctif : forge-doc/CI Windows exclut toujours forge-cad (ci.yml:153-159) ; aarch64 exclut aussi forge-cad (ci.yml:226).
>
> ## 3. Signalements A5-NN
> A5-01 — majeur / bug (non vérifié par exécution)
>  cad.rs:840-868 contours_des_segments. (a) La chaîne continue de consommer des segments même après avoir bouclé sur son départ : deux boucles partageant un sommet sont fusionnées en une seule chaîne « fermée » en huit ; (b) en présence d'un embranchement (carré + segment qui dépasse d'un coin), le glouton prend la première correspondance dans `restants` (ordre d'entités) et peut quitter la vraie boucle : la boucle n'est alors pas détectée comme fermée → « pas de contour fermé » (command.rs:879, lib.rs:422) ; (c) un seul contour fermé est retenu (cad.rs:872) : trous et îlots ignorés en silence ; (d) O(n²) (`remove(index)` + `position`) sans plafond sur le nombre d'entités ; (e) pas de lien avec valider_contour : la fermeture est jugée à 1e-6 mm par points_recolles alors qu'OCCT n'est protégé que par forge-cad/lib.rs:59.
>  Correctif : graphe d'extrémités (index spatial à tolérance), extraction de cycles (degré 2 requis ; refus typé si degré > 2), orientation + parité pour les trous, plafond d'entités. Placer cela dans forge-geom::contours (A4-17) et supprimer la version de forge-session.
>  Tests : carrés (0,0)-(1,1) et (1,1)-(2,2) → 2 contours ou refus typé, jamais 1 ; carré + queue de 1 mm au coin → 1 contour fermé ; carré avec carré intérieur → 1 contour + 1 trou ; entités dans le désordre et inversées.
> A5-02 — majeur / bug (A4-18 propagé)
>  cad.rs:787 + segments.rs:211. Arc d'esquisse effondré (fin == début, ou écart ≤ 1e-12 rad) → cercle complet, puis extrudé. Correctif : Segment2::Cercle explicite ; Arc de balayage nul refusé (OF-GEO-xxxx à ajouter au registre). Test : arc début=fin=30° → pas de contour fermé (ou refus), jamais un cercle de rayon r.
> A5-03 — mineur / doc inexacte
>  cad.rs:732-745 (doc de segments_du_croquis) dit « trigonométrique si l'angle de fin précède ou égale celui de début, horaire sinon » ; le code (l.787) fait l'inverse (trigo si fin >= début), ce qui est conforme à ADR-0020 (esquisse.rs:42 : fin peut précéder début, le sens suit le geste). Corriger le commentaire ; ajouter un test de non-régression sur les deux sens.
> A5-04 — majeur / risque (non vérifié par exécution)
>  cad.rs:385 et 406. reconstruire() passe au moteur la profondeur journalisée `Fixe{mm}` sans contrôle (fini, > 0, ≤ magnitude) : la validation vit seulement dans command.rs:864-895 (à l'écriture). Un journal rejoué (ancien, édité, plus récent) peut porter 0, négatif ou 1e300 ; avec le futur moteur OCCT du lot 3 c'est le scénario d'abort du spike. L'événement n'est pas non plus validé à l'application (apply, cad.rs:465-500 : id séquentiel vérifié, mais pas la valeur). Même remarque pour « à travers » : 1,1 × diagonale peut valoir 0 (corps dégénéré) ou dépasser 1e9. Correctif : garde commune `profondeur_valide()` appelée dans reconstruire (feature en échec OF-CAD-0005) ; mêmes bornes pour la couleur (rgba dans 0..=1, cad.rs couleur_de_journal). Tests : journal avec mm=0, -5, 1e300, rgba=[2,0,0,1] → feature en échec ou événement refusé ; « à travers » sur boîte de diagonale 0.
> A5-05 — mineur / risque
>  cad.rs:322, 465 : `self.features.len() as u32` (troncature silencieuse) ; features : Vec sans plafond, feature() en O(n) (cad.rs:~300), reconstruire() O(n) appels moteur à chaque édition (la clé de cache `edition` croît à chaque événement de tout type, cad.rs:~425 : tout événement non CAO invalide le corps). Un journal borné en octets peut contenir des dizaines de milliers de features. Correctif : plafond MAX_FEATURES (OF-xxx), index par id, invalidation ciblée. Test : journal de MAX+1 features → refus typé.
> A5-06 — mineur / risque
>  forge-cad/src/lib.rs:570-574 : lire_brep n'applique pas de budget de formes (voir §2 A3-08.e). Correctif : refuser/limiter le BREP au même titre (ex. plafond d'octets plus bas ou comptage de `TShape`), ou documenter que formes_max ne vaut que pour STEP.
> A5-07 — mineur / hygiène
>  forge-cad/lib.rs:59-148 : valider_contour contient 3 politiques de tolérance différentes (TOLERANCE_MM 1e-6, f64::EPSILON, EPS_AIRE 1e-12 « en mm² »), et le point d'orientation `EPS_AIRE` est une aire, pas une distance : le seuil « se touchent » dépend de la longueur des arêtes. Autres epsilon en dur hors forge-units : dxf.rs:323,345,449,455,505,520 (1e-12) ; segments.rs (1e-12 l.216,221 ; ecart_a_la_corde l.249) ; esquisse.rs:78 SEUIL_GRAINES_CONFONDUES_MM 1e-6 ; renderer.rs:238 ; camera*.rs/scene.rs 1e-9 (rendu, sans conséquence). Voir conseil n°1.
> A5-08 — hygiène / doc
>  KNOWLEDGE.md (etat-actuel) : la ligne « Version » fait ~94 000 caractères (ligne 5) et contient 21 occurrences de « **Version :** » (historique empilé v73 → …), alors que la consigne audit 4 parlait de « plusieurs milliers de caractères ». Elle n'est lisible ni par humain ni par modèle, et dépasse probablement la fenêtre utile d'un lecteur IA. Générer par script ou archiver en historique/ (déjà un dossier).
>
> ## 4. Dette connue : aggravations ou régressions
> - A3-08.e : soldée pour STEP seulement (A5-06).
> - Windows/forge-cad : inchangé ; maintenant que forge-cad est branché au flux croquis→solide (command.rs, lib.rs:422), l'écart de CI porte sur du code d'intégration vivant, pas un spike. Aggravation modérée.
> - expect() : voir §1 (26 vs 20, méthode différente).
> - A4-17/A4-18 : réaggravés par le lot 2 qui les rend atteignables via cad.rs.
>
> ## 5. Incohérences des fiches knowledge
> - Fiche v73 : « dette A3-08.e soldée (budget formes_max réel…) » → vrai pour STEP, faux/non précisé pour BREP.
> - Fiche v73 : « rejeu déterministe » des features : tient pour l'ordre et l'identifiant, mais cad.rs n'impose pas de bornes de valeurs au rejeu (A5-04).
> - Fiche : « Segment2 … balayage nul = cercle complet » (v72) est présenté comme choix ; son effet sur les arcs d'esquisse (A5-02) n'est pas documenté.
> - Commentaire cad.rs sur le sens d'arc inversé (A5-03).
> - Ligne Version (A5-08). Comptages manuels (tests, expect) : non recomptés ici.
> - audit-protocole.md §5 : adopte un « registre de couverture » (coverage-ledger.json) ; je n'en ai trouvé aucun dans le zip (non vérifié exhaustivement) : il est donc à créer à partir du §7 ci-dessous.
>
> ## 6. Bilan des conseils d'architecture
> 1 Tolérances uniques : PAS FAIT (A5-07, plusieurs epsilon absolus ; MAGNITUDE_MAX_MM est un début). Vaut la peine, en priorité avant l'extrusion réelle. Commencer petit : un module forge-geom::tolerance avec 3 fonctions (colineaire, distinct, aire_negligeable) + un grep CI interdisant `1e-` hors de forge-units/forge-geom.
> 2 Fuzz/proptest : NON VÉRIFIÉ (je n'ai pas cherché de proptest/cargo-fuzz dans les Cargo.toml). Commencer par proptest sur expr, journal (apply) et valider_contour (A4-16, A5-04).
> 3 OCCT hors processus : PARTIEL au mieux (MoteurCad est un trait injecté, cad.rs ; frontière déjà posée, sérialisation non). Bon choix de frontière ; pas de processus enfant avant que le lot 3 branche les booléens. Condition : le trait doit rester sans références OCCT, et reconstruire() doit être exécutable hors du fil UI (la fiche indique un « fil de travail CAO » : non relu).
> 4 Chaînage par graphe + trous + Cercle : PAS FAIT (A5-01/02). À faire avant le lot 3.
> 5 CI Windows forge-cad : PAS FAIT (ci.yml:153-159, 226).
> 6 ADR sûreté machine : non vérifié (je n'ai pas lu docs/knowledge/machines/ ni la liste complète des ADR ; ADR-0035 « solidcore-veille » n'est pas ce sujet d'après son nom). Toujours à écrire avant tout code de pilotage.
> 7 app.rs : PAS FAIT (3 501 lignes, inchangé) ; reste de la dette à planifier, pas bloquante.
> 8 Budget de coût par frame : non évalué.
> 9 Limites uniques : PARTIEL (MAGNITUDE_MAX_MM dans forge-geom ; budgets par format ailleurs : A4-19, A5-05/06).
> 10 Moins de prose : PAS FAIT (102 .md ; ligne de 94 000 caractères).
> 11 Montée egui/zip : PARTIEL (ignores datés/motivés ; pas d'échéance).
> 12-14 : non évalués (android.yml, bruit des issues) ; android.yml est limité par paths, ce qui répond partiellement au n°12.
> Caduc : aucun.
>
> ## 7. Non couvert (honnête)
> Non lus : app.rs (hors compte), atelier_esquisse.rs, ui/*, travail.rs (sauf l.486), esquisse.rs (sauf 40-80, 489-510, 1039-1060, 1600-1620), segments.rs (hors 195-260, 380-450), dxf.rs/svg.rs (hors en-têtes/expect), forge-render/*, forge-doc/*, forge-values/* (constantes seulement), forge-gcs/*, command.rs (profondeur 840-1020 en partie), spike_phase_a.rs, workflows (hors ci.yml ciblé), ADR-0022/0024, bornes inter-parseurs (octets/entités/profondeur/magnitude : non comparées). Aucun test n'a été exécuté ; aucun signalement ci-dessus n'est reproduit par exécution. Pas de coverage-ledger produit.
>
> ## 8. Ordre de traitement proposé
> 1. A5-04 (garde de profondeur au rejeu) + A4-16 (tolérance relative) — avant de brancher OCCT.
> 2. A5-01 + A4-17 + A5-02/A4-18 (contours en graphe dans forge-geom, Cercle explicite).
> 3. A4-20 (inversion de deux lignes dans TypesIgnores), A4-19 (plafond par format), A5-05/06 (plafonds features/BREP).
> 4. Tolérances unifiées (conseil 1) + proptest ciblé.
> 5. CI Windows forge-cad ; échéance automatique des advisories ; fiche d'état générée.
