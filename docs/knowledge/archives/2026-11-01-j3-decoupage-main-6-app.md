# [J3] Découpage main.rs 6/6 (app.rs), dettes de sommaires — 2026-11-01

**Jalon dominant : J3** (transversal J0 signalé : dettes de sommaires réglées au fil). Suite directe de la discussion « Bump EN de la couche d'accueil » (phase A du jalon 3 confirmée intégralement faite, backlog des audits n°3/n°4 épuisé, 0 issue ouverte) ; départ tête `f55c936`.

## Thème

Agenda pré-phase B réglé en une discussion, 3 points : (1) étape 6/6 du découpage de `main.rs` — le chemin critique avant la phase B ; (2) dettes de sommaires en une passe mécanique (backfill de l'index des archives, statut des ADR-0029..0034) ; (3) ouverture de la phase B — renvoyée à la discussion suivante (budget absorbé par les points 1-2, résumé de transition produit).

## Le fil

### 1. Étape 6/6 : app.rs (test-first, le chemin critique)

- **Rouge constaté** : `311625f` sur `refactor/decoupage-main-6-app` (test `demoapp_vit_dans_le_module_app`) → [issue #595](https://github.com/RaphOlivier/OmniFab/issues/595) : E0433 exact, **une seule erreur, aucun impact collatéral** — le rouge visé.
- **Correctif `72a5226`** (10 fichiers, un commit) : `app.rs` nouveau module (3 502 lignes : `RenderState`, écrans `Ecran`/`Vue`, `ActionDestructrice`, `DecisionFermeture`+`garde_fermeture` (audit n°3 A3-06), `DemandeEchelle` (ADR-0017), `aide_du_code` (ADR-0019), `DemoApp` 51 champs/31 méthodes en `pub(crate)`, impl `eframe::App` privé, 67 tests du module) ; 14 blocs de tests du fil déplacés vers `travail.rs` (863 lignes, aucun import nouveau) ; `main.rs` réduit au lanceur (53 lignes, 12 modules) ; 7 fichiers retouchés aux imports uniquement (`crate::app::`).
- **[PR #596](https://github.com/RaphOlivier/OmniFab/pull/596)** : vert de branche sur le run PR — lint-test (fmt+clippy+tests+rustdoc) ✅, ubuntu ✅, macos ✅, aarch64 ✅ ; windows en cours à la fusion (non arbitre — patron établi sur 9 lots, recoupé au run main) ; les runs « cancelled » sont les push-runs superseded par concurrency, le run PR fait foi.
- **Fusion squash → main `3b35f5b`** (verdict complet dans le message de commit). Main vérifié après fusion : `main.rs` 53 lignes, `mod app` présent.
- **#595 fermée MANUELLEMENT** (commentaire + close completed) : « Referme » n'est pas un mot-clé de fermeture GitHub — la fermeture auto attendue n'a pas eu lieu.

### 2. Dettes de sommaires (J0, passe mécanique)

- **Backfill des 12 lignes d'index v18→v29** de la fiche archives (les fichiers d'archives 2026-10-31 existaient déjà au dépôt — ADR-0028, seul l'index était en retard) : fiche **v37**, dépôt `04681d5`, sommaire déjà resynchronisé (`2e66009`) — la dette comblée n'a pas été recréée.
- **Constat de retard du miroir privé** : la fiche archives privée traînait en **v13** (2026-10-26) — 23 versions sans que rien ne le signale ; resynchro en bloc v13→v37 (`9035511`), dette comblée au passage.
- **ADR-0029..0034 : ABSENTS du dépôt** (`docs/adr/` = 0000→0028 + TEMPLATE) → la dette EN de l'index ADR reste **OUVERTE**, consignée ici ; traduction reportée au prochain point dur (« clôture J0 ») — pas d'improvisation hors point dur.

### 3. Phase B (renvoyée)

Checklist de début de phase + lot 1 « croquis → solide » (ADR-0022/0024, designs validés le 2026-10-26) renvoyés à la discussion suivante. Rappel : la dette A3-08.e (formes_max réel — le binding 0.3.0 n'itère pas les sous-formes) est **assumée**, reportée au lot 2, non re-signalable en audit.

## Leçons

1. « Referme » n'est pas un mot-clé de fermeture GitHub (les mots-clés sont anglais) : fermer manuellement ou écrire le mot-clé EN dans le message de fusion.
2. Un miroir peut traîner sans symptôme : chaque « repush du miroir » se vérifie par roundtrip, jamais par présomption (écho v44.1 : un push annoncé dans une fiche n'est pas un push vérifié).
3. Un garde-fou de roundtrip qui échoue peut être un faux négatif du garde (chaîne cherchée mal formée) : relire le contenu réel avant de conclure à un échec de push — le v37 était correct dès le premier push.

## Fiches touchées

- [`preparation/decoupage-main-rs.md`](https://github.com/RaphOlivier/omnifab-prive/blob/main/knowledge/omnifab/preparation/decoupage-main-rs.md) **v2** (statut clôturé 6/6) — miroir privé `42cf208` ;
- [état-actuel](https://github.com/RaphOlivier/OmniFab/blob/main/docs/knowledge/etat-actuel/KNOWLEDGE.md) **v71** ;
- fiche archives **v37** (backfill) puis **v38** (présente entrée) ;
- [sommaire](https://github.com/RaphOlivier/OmniFab/blob/main/docs/sommaire.md) : ligne de la présente entrée (règle v7 — la dette comblée en `2e66009` n'est pas recréée).

## Suite proposée

« [J3] — Phase B lot 1 : croquis → solide (checklist + test-first) »
