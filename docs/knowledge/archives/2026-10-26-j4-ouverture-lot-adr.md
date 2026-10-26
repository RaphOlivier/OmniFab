# 2026-10-26 — [J4] Ouverture du jalon 4 : lot d'ADR ADR-0029..0034 validé en une session (sans PC)

Session « sans PC » du 2026-10-26 (Raphaël sans accès à son PC) : deux blocs de décisions.

## Bloc 1 — Conventions `expect()` (audit n°3) tranchées

1. `build.rs` : `expect` admis (cargo garantit l'environnement).
2. Invariants internes : `expect` uniquement avec commentaire « invariant : … », sinon erreur typée ; clippy `expect_used` avec échappatoires datées (activation à la CI verte).
3. Mutex empoisonné (egui_adapteur) : `into_inner` assumé — l'UI survit à une panique déjà survenue (règle d'or du projet) ; refonte seulement si contention réelle.
4. Graine de démo main.rs : reportée au découpage main.rs (étape demo, plan existant).
5. `formes_max` (A3-08) : budget réel contrôlé à l'import STEP (compteur de shapes, code OF-CAD).

Argumentaire : la règle commune est « une panique en production n'est acceptable que si l'échec est prouvé impossible et documenté au point d'usage » ; tout le reste passe par les erreurs typées OF-* (ADR-0003).

## Bloc 2 — Lot d'ADR du jalon 4 préparé et VALIDÉ (digest 4/4, même session)

Fiche : `preparation/lot-jalon-4.md` (porte « VALIDÉ ») ; à recopier au dépôt `docs/adr/ADR-0029..0034` au premier lot de phase A.

- **ADR-0029** découpage A→E : A forge-gcode + langage déclaratif, B forge-cam 2,5D, C post-procs + multi-procédés 2D, D simulateur, E desktop CAM + éditeur. Hors périmètre : pilotage (jalon 5), raster/heightmap (jalon 6), tournage/3D/4-5 axes, nesting, DSTV/NC1, broderie, tranches.
- **ADR-0030** forge-gcode MPL-2.0 : parseur RS-274 maison budgété, IR de programme, **langage déclaratif de contrôleur YAML versionné** (une description alimente colorisation, autocomplétion, post-proc, simulateur — conseil fort fiche Machines), dialectes linuxcnc + grbl, writer déterministe, wasm-friendly.
- **ADR-0031** forge-cam : événements `cam.*` (patron `cad.*` : added/updated à identifiant conservé, removed non cassant), opérations Contournage/Pochage/Perçage avec `ValeurCote`, **toolpaths = état dérivé jamais sérialisé**, MoteurCam injecté (forge-session indépendante), `Speed`/`RotationalSpeed` (ADR-0025 prolongé), budgets OF-CAM-*.
- **ADR-0032** post-processeurs 100 % déclaratifs (zéro code par contrôleur, nouveau dialecte = YAML + golden tests sans ADR) ; découpe (kerf, lead-in/out) + gravure V sur `Path2`.
- **ADR-0033** simulateur consommant l'IR forge-gcode (pas les toolpaths internes — ce qui est simulé = ce qui est envoyé) : D1 trajectoires obligatoire, D2 heightmap 2,5D **avec porte de sortie** (spike à seuil, descopée vers le jalon 5 sans rouvrir les ADR si échec) ; **destination actée : collisions contre montages/fixations modélisés en 3D (étau à mort mobile, bridage) — design dédié avant implémentation** ; Camotics embarqué rejeté (natif, non wasm, licences).
- **ADR-0034** espace CAM (arbre usinage = ordre du journal, drag = événement `cam.operation.moved`), éditeur G-code dialecté avec documentation en ligne et **aperçu de trajectoire au curseur**, « Recalculer tout » obligatoire avant export (verrou ADR-0002 appliqué à l'UX), export = ce qui est simulé.

Digest tranché par Raphaël : (1) 2,5D + découpe/gravure 2D ; (2) simulateur D1+D2 avec porte de sortie + exigence montages 3D à terme ; (3) langage déclaratif dès le jalon 4 ; (4) clôture au G-code prêt + simulé, gel des jalons 5-10 intact.

Argumentaire général : le jalon 4 ne dépend d'aucun résultat du spike OCCT (le CAM consomme la tessellation et les croquis) — d'où la validation possible sans attendre la fin du jalon 3 ; la phase A (forge-gcode) est même indépendante du jalon 3 et exécutable en parallèle. Le simulateur consomme l'IR G-code et non les toolpaths internes pour prouver le post-processeur lui-même. Le gel reste lisible : rien ne touche forge-link/tokio/machines.

## Leçons / points de méthode

- Le patron « lot d'ADR préparé + digest 4 axes validé en une session » (inauguré au jalon 3) fonctionne aussi sans PC : la session a tenu en un échange d'ask questions + un fichier prep.
- Exigence destination exprimée par Raphaël pendant le digest (collisions montages 3D) : intégrée à l'ADR comme destination écrite avec design dédié avant implémentation — pas gonflée dans le périmètre du jalon.
