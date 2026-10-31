# [J3] Mineur forge-units A3-17 : constructeur faillible Length::try_millimeters

**Date :** 2026-10-31 (soir)
**PR :** #540 — **Fusion squash :** `bfa1045` (rouge `54ccfd8`, correctif `02f9139`)
**Jalon :** [J3] — correction des signalements de l'audit externe n°3 au vert

## Contexte

L'audit n°3 (signalement A3-17) relevait que `Length::from_millimeters` accepte NaN et ±∞ sans aucune garde : la garde de finitude était reportée sur chaque consommateur et rien ne l'ancrait dans forge-units. Verdict retenu : **documenter + constructeur faillible** — les gardes consommateurs existent déjà (A3-08 forge-cad, budgets forge-io), aucun consommateur à modifier ; forge-units est le newtype minimal du noyau (`#![deny(missing_docs)]`, aucune dépendance).

## Protocole test-first

- **Rouge** `54ccfd8` : test unique `constructeur_faillible_refuse_valeurs_non_fines` (NaN/+∞/-∞ → None, 0.0 → Some, 12.5 mm → Some(12.5)) ajouté à `mod tests` de `crates/forge-units/src/lib.rs`. Rouge de compilation **E0599 localisé au test** (méthode absente) : rouge légitime d'un **ajout d'API**, distinct du E0599 d'import manquant qui masque les assertions (leçon #534 du lot forge-doc).
- **Flocon de plateforme** : le run push #449 sur `54ccfd8` a échoué en 7 m 20 s **sans issue auto** — le mécanisme d'issues de ci.yml ne couvre que les étapes Verifications/Compilation/cross, un échec « Dépendances natives » n'en crée jamais. Aucun correctif poussé tant que le rouge n'était pas constaté.
- **Constat par PR** : PR draft #540 ouverte sur la tête rouge — le run `pull_request` (événement opened) re-teste le rouge, clippy tournant avec `--all-targets` attrape l'E0599 du test → **issue auto #539** « CI : echec lint-test @ 54ccfd8 ». Rouge constaté AVANT correctif, protocole respecté malgré le flocon.

## Correctifs (`02f9139`)

- `Length::try_millimeters(mm: f64) -> Option<Self>` : `Some` si la valeur est finie, `None` sur NaN/±∞ — garde de frontière pour toute longueur issue d'une donnée externe (saisie utilisateur, fichier importé).
- Contrat documenté sur `from_millimeters` : aucune garde, NaN/±∞ transitent tels quels — la garde de finitude vit aux frontières, dans la continuité des gardes A3-08 de forge-cad ; doc complète requise par `#![deny(missing_docs)]`.

## Leçons

- Un échec de run **sans issue auto** = flocon d'une étape non couverte par le mécanisme (Dépendances natives, runner mort) : jamais un constat de rouge, jamais une preuve de vert. Le run d'une PR ouverte re-teste la même tête et crée l'issue manquante.
- E0599 **méthode absente** dans un test d'ajout d'API = rouge légitime (l'API à créer), à distinguer de l'E0599 d'import manquant qui masque les assertions (leçon #534).
- clippy `--all-targets` compile les tests : les erreurs de compilation des tests sont constatées par l'étape Verifications (rapide), pas seulement par cargo test.

## Vérifications

- Vert de branche : check runs de la PR #540 (tête `02f9139`) — lint-test fmt+clippy+tests+rustdoc ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en cours à la fusion, non arbitre).
- Fusion squash → main `bfa1045` ; issue #539 fermée completed (motif : rouge test-first attendu), 0 issue ouverte.
- Run main ci.yml #453 sur `bfa1045` : verdict à relever (arbitre final) — sera consigné par la relève suivante.

---

*Fiche rédigée avec l'assistance de Vibe (IA) — relu et validée par Raphaël, mainteneur OmniFab (conventions v10-v15).*
