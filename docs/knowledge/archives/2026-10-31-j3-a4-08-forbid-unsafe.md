# [J3] Hygiène A4-08 : forbid(unsafe_code) sur les crates cœur

*Discussion du 2026-10-31 (soir) — jalon 3, lot d'hygiène de l'audit n°4 (A4-08). PR #549, fusion squash → main `817551b`.*

## Signalement

L'audit n°4 notait que `forge-geom` porte déjà `#![deny(missing_docs)]` + `#![forbid(unsafe_code)]`, mais que les cinq autres crates cœur (forge-cad, forge-render, forge-session, forge-values, omnifab-desktop) n'interdisent pas le `unsafe` au compilateur — rien n'empêcherait une future contribution d'en introduire.

## Protocole adapté : la garde EST le test

Le rouge test-first habituel n'existe pas pour une garde : le commit qui AJOUTE `#![forbid(unsafe_code)]` aux 5 crates est le test lui-même — si du `unsafe` existait, la CI le refuserait (E0133) et l'issue auto ouverte serait le constat. C'est le cas « garde » déjà rencontré en A4-14 : la garde est posée sans rouge possible et le vert de la CI prouve l'absence, toutes cibles confondues.

## Enquête préliminaire : le balayage

Balayage exhaustif de toutes les racines et de tous les modules des 5 cibles (comptage du mot `unsafe` sur chaque fichier) :

- forge-cad : lib.rs, occt.rs, erreur.rs — 0 ;
- forge-render : lib.rs, camera.rs, camera3d.rs, renderer.rs, scene.rs, scene3d.rs, egui_adapteur.rs — 0 ;
- forge-session : lib.rs, command.rs, error.rs, esquisse.rs, projection.rs — 0 ;
- forge-values : lib.rs, expr.rs, error.rs — 0 ;
- omnifab-desktop : main.rs, aide.rs, atelier_esquisse.rs, bibliotheque.rs, esquisse.rs, fichiers.rs, i18n.rs — 0.

Verdict : **0 `unsafe` dans les 5 cibles** — la CI attendue était verte, et elle l'a été : l'absence est prouvée par le compilateur, toutes plateformes de la matrice.

## L'exemption FFI

Tout le `unsafe` du workspace vit à la frontière FFI de l'ADR-0013 :

- `forge-gcs/src/native.rs` (23 `unsafe`) : chaque appel est un pont ABI audité vers le shim C de planegcs (exceptions C++ déjà rattrapées dans le shim, codes d'erreur traduits en `GcsError` typée) ;
- `forge-gcs-sys` : crate `-sys` de la frontière, déclarations bindgen consommées par forge-gcs seul.

Ces deux crates restent **volontairement sans forbid** ; l'exemption est documentée dans le doc de module des deux fichiers (note « audit n°4, A4-08 »).

## Mise en œuvre et verdict

- forge-cad et forge-render : `#![forbid(unsafe_code)]` juste après le `#![deny(missing_docs)]` existant ;
- forge-session, forge-values, omnifab-desktop : attribut en tête de fichier (patron forge-geom) ;
- commit `7bdf6b8` sur la branche `fix/a4-08-forbid-unsafe`, PR #549 (draft → ready) ;
- check runs du run PR : lint-test fmt+clippy+tests+rustdoc ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en file, non arbitre) ;
- fusion squash → main `817551b` ; 0 issue auto ouverte — aucun rouge possible.

## Leçons

- Une garde compileur (forbid) se pose en un seul commit : le vert de la CI EST le verdict — pas besoin de fabriquer un rouge ;
- le balayage textuel préliminaire informe, mais le compilateur reste l'oracle : c'est lui qui prouve l'absence, cible par cible, plateforme par plateforme ;
- l'exemption FFI doit être écrite noir sur blanc dans les modules exemptés, sinon la prochaine relecture relance le débat.

*Fiche rédigée à l'issue du lot — suite : A4-09 (marques d'isolation Fluent), A4-10 (clés i18n manquantes), A4-12 restes.*
