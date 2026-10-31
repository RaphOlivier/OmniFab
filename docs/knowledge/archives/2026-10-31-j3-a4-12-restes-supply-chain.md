---
name: archives-2026-10-31-j3-a4-12-restes-supply-chain
description: "Hygiène A4-12 restes : dédoublonnage des issues auto (spam reproduit puis corrigé avec preuve expérimentale), épinglage SHA des actions GitHub, cargo-apk --locked 0.10.0, APK de CI nommé debug. PR #553, squash 52e0b8f."
---

# [J3] Hygiène A4-12 restes — chaîne d'approvisionnement de la CI

**Date :** 2026-10-31 (soir) · **PR :** [#553](https://github.com/RaphOlivier/OmniFab/pull/553) · **Fusion squash :** [`52e0b8f`](https://github.com/RaphOlivier/OmniFab/commit/52e0b8ffe26395fde94111007507e6e0d1bd4dca) · **Spéc :** audit n°4 (`preparation/backlog-audit4.md`, miroir privé) ; fiche sécurité v3 → v4.

## Contexte — le reste de la spéc A4-12

Le workflow cargo-deny dédié avait été activé dès le traitement de l'audit (v45). Restaient quatre points de la spéc A4-12 : (1) cargo-apk installé SANS version dans android.yml — la CI pouvait casser du jour au lendemain sur une montée de version ; (2) les actions GitHub référencées par tag majeur (un tag peut être déplacé) ; (3) l'APK de CI produit par un keystore debug mais sans la mention « debug » au nom du fichier (fiche sécurité v3) ; (4) le spam d'issues auto : un échec par job et par run crée une issue à chaque fois — jusqu'à 2 issues pour UN seul commit rompu quand une PR est ouverte (run push + run pull_request sur le merge ref), 8 issues pour un commit rouge à l'époque du lot A4-01/02/03 (v46).

## Protocole test-first — le mécanisme d'issues est du code, il se teste

1. **Rouge `35cf370`** : mod de test `tests_a4_12_dedoublonnage` au formatage volontairement cassé (fmt échoue, tout compile). Le run push ci.yml #480 crée l'issue [#552](https://github.com/RaphOlivier/OmniFab/issues/552) ; la PR draft #553 ouverte ENSUITE (ordre délibéré : le run PR annule le run push de la même tête par concurrency — laisser l'issue n°1 se créer d'abord) fait échouer le run pull_request #481 qui crée [#554](https://github.com/RaphOlivier/OmniFab/issues/554) sur le merge ref `9fd4cfc` : **le spam est reproduit — 2 issues pour un seul échec**.
2. **Correctif `d9f141b`** (fmt toujours cassé) : avant chaque `gh issue create`, `MOTIF="CI : echec <job>"` puis recherche d'une issue ouverte par motif de titre (`gh issue list --state open --search "in:title ..."` en JSON) → si trouvée, `gh issue comment` avec le rapport, sinon création comme avant. Appliqué aux 6 sites (ci.yml ×3 : lint-test, build par OS, check-aarch64 ; android.yml build-apk ; galerie.yml captures ; cargo-lock.yml regen-lock).
3. **Preuve expérimentale** : le run PR #483 (merge ref `27e6f03`, [run 41231443513](https://github.com/RaphOlivier/OmniFab/actions/runs/41231443513)) échoue sur le même fmt — et **commente** [#554](https://github.com/RaphOlivier/OmniFab/issues/554) au lieu de créer une 3e issue. 2 issues ouvertes, inchangées. 0 nouvelle issue.
4. **Reprise `0c5112c`** : à la relecture intégrale, la ligne `MOTIF=` avait hérité de l'indentation de l'ancienne ligne `gh issue create` (20 espaces au lieu de 10) — bash l'acceptait (le commentaire a bien été posté) mais la relecture exige une indentation propre ; remise à 10 espaces, sans changement de comportement.
5. **Commit final `3fcfae4`/`0c5112c`** : retrait du mod de test (fmt réparé) + épinglage SHA + cargo-apk + APK debug.

## Correctifs

- **Dédoublonnage** (ci-dessus) : une issue ouverte par motif = (workflow, job) ; le rapport de chaque échec ultérieur s'ajoute en commentaire de l'issue existante. Le motif du titre inclut l'OS du job build — deux OS en échec = deux issues distinctes, conformément à la spec « une issue par motif ».
- **Épinglage SHA** (6 workflows) : checkout v5 → `fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09`, rust-toolchain stable → `b725eda525362e7cf34af9a7243cac1e3f01a7ec`, rust-cache v2 → `d1527c8f67ef9af357290475d4af4b0fab8f12e0`, cache v4 → `0057852bfaa89a56745cba8c7296529d2fc39830`, upload-artifact v4 → `ea165f8d65b6e75b540449e92b4886f43607fa02`, cargo-deny-action v2 → `3c6349835b2b7b196a839186cb8b78e02f7b5f25` ; chaque ligne porte le tag majeur en commentaire (SHA résolu par get_commit au dépôt amont — voie hors quota API publique).
- **cargo-apk** : `cargo install cargo-apk --locked --version 0.10.0` (crates.io fait foi — docs.rs 2026-10-17, la page releases GitHub montre 0.9.7) + clé de cache versionnée (bump de clé = réinstallation propre à la prochaine montée).
- **APK debug** : step de renommage `omnifab-android-debug.apk` + artifact `omnifab-android-apk-debug` — le keystore jetable à mot de passe public est désormais identifiable au nom (fiche sécurité v4).

## Verdict

Vert intégral sur la tête `0c5112c` (run PR #487 / [41231895823](https://github.com/RaphOlivier/OmniFab/actions/runs/41231895823)) : lint-test (fmt + clippy + tests + rustdoc) ✅, build ubuntu ✅, build windows ✅, build macos ✅, check aarch64 ✅, build APK ✅ (cargo-apk 0.10.0 installé --locked), cargo-deny ✅ (#15). Fusion squash → main `52e0b8f` ; issues [#552](https://github.com/RaphOlivier/OmniFab/issues/552)/[#554](https://github.com/RaphOlivier/OmniFab/issues/554) fermées completed avec motifs différenciés, 0 ouverte. Runs main sur `52e0b8f` à relever (ci.yml, android, cargo-deny, galerie — arbitre final).

## Leçons

1. **Le mécanisme d'issues est du code** : il se teste test-first — reproduire le spam AVANT de le corriger, preuve expérimentale par commentaire (pas seulement « le code semble bon »).
2. **Ordre des constats sous concurrency** : le run PR et le run push partagent le groupe `ci-<head_ref>` → le plus récent annule l'ancien ; pour constater DEUX issues (le spam), laisser le run push créer la première AVANT d'ouvrir la PR.
3. **Le SHA du merge ref diffère de la tête** : les issues du run pull_request portent le sha court du merge — c'est le motif de job (in:title) qui déduplique, pas le sha.
4. **Remplacement d'ancre sans indentation de tête** : remplacer une ligne sans son indentation fait hériter la première ligne insérée de l'indentation de la ligne remplacée (20 espaces au lieu de 10) — bash l'a toléré, la relecture intégrale l'a rattrapé au commit suivant ; toujours découper l'ancre ligne complète (indentation incluse).
5. **Résolution de versions hors quota** : `get_commit(dépôt tiers, tag)` résout les tags en SHA complets ; pour les crates, crates.io/docs.rs fait foi sur la page releases GitHub (cargo-apk : 0.10.0 vs 0.9.7).
6. **lint-test est le plus long des check runs** (tests workspace + golden OCCT) : le vert des builds/APK/cargo-deny arrive avant son verdict — ne pas fusionner sur le vert partiel, poller les check runs jusqu'au verdict fmt.

## Statuts mis à jour

- Fiche sécurité v4 : APK nommé debug, actions épinglées SHA, cargo-apk épinglé 0.10.0, issues dédoublonnées ; premier verdict cargo-deny en production relevé (run main #9 SUCCESS).
- etat-actuel v60 ; archives v25 (cette entrée). Journal des fonctions : inchangé (workflows — pas une fonction du logiciel).

*rédigé avec l'assistance de Vibe (IA) — relu et validé par Raphaël, mainteneur OmniFab (conventions v10-v15).*