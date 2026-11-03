# [J3] Phase B lot 2 : forge-session — croquis nommés, features d'extrusion, alias cad., dette A3-08.e — 2026-11-03

**Jalon dominant : J3.** Suite directe de la discussion « Phase B lot 1 » ; exécution du lot 2 des designs D3/D4 de l'ADR-0024 (§7, validés le 2026-10-26) : le journal de session gagne les croquis nommés et les features d'extrusion, le MoteurCad devient injectable, la dette A3-08.e est soldée. Départ main code `bd57c4c` / docs `435dc20` (capitalisation lot 1).

## Thème

La session porte la structure : les croquis deviennent des objets nommés (`sketch.created`), les événements d'esquisse savent à quel croquis ils s'appliquent (avec migrations des anciens journaux), et les features d'extrusion (`cad.feature.added`/`cad.feature.updated`) deviennent des commandes journalisées — le solidificateur quitte le fil UI pour un MoteurCad injectable, rejouable en déterminisme. L'alias de profondeur naît automatiquement dans la même transaction, la dette A3-08.e (budget formes_max réel sur les instances STEP) est soldée au passage.

## Le fil

### 1. Rouge test-first (le rouge attendu)

- Branche `feat/phase-b-lot2-forge-session` coupée depuis main ; les tests du lot 2 poussés avant l'implémentation : rouge constaté par l'issue auto [#599](https://github.com/RaphOlivier/OmniFab/issues/599) — **18 tests** en échec avant tout correctif.

### 2. Correctif `6e0fdf6` : le contenu du lot

- **`sketch.created` v1** : croquis nommés (journalisés), croquis actif, l'implicite 0 préservé — les journaux antérieurs sans croquis se rejouent à l'identique.
- **Champ croquis des événements d'esquisse** : entité v3, contrainte v3, déplacement v2, mise à jour v2 ; migrations des anciens journaux, rejeu tolérant.
- **`cad.feature.added` / `cad.feature.updated` v1** : extrusions — mode, profondeur fixe/expression/à travers, couleur ; identifiant conservé, changement de croquis autorisé (amendement du 2026-10-26).
- **Alias auto `cad.extrusion{id}.profondeur`** dans la même transaction (un seul groupe d'undo) ; préfixe `cad.` réservé (OF-VAL-0008).
- **MoteurCad injecté** : volumes analytiques du prisme ; enveloppe « à travers » = 1,1 × diagonale, jamais journalisée ; échec marqué OF-CAD-0005, le corps survivant ; rejeu déterministe.
- **Dette A3-08.e soldée** : budget formes_max réel sur les instances STEP (`#N =`), refus avant le noyau (OF-CAD-0004).
- Reprises de compilation dans le même commit : E0502 + E0631 + lock forge-geom + clippy + test v3 + fmt.

### 3. Correctif `f02b886` : rustdoc liens privés

- Le rustdoc sous `-D warnings` refuse les liens intra-doc vers les items non réexportés (`private_intra_doc_links`) — références réécrites sans crochets (leçon déjà connue v46, réappliquée).

### 4. PR, CI, fusion

- [PR #605](https://github.com/RaphOlivier/OmniFab/pull/605) (« [J3] — Phase B lot 2 : forge-session (sketch.created, features, commandes, dette A3-08.e) »), ready puis fusion squash → main `c36cb15` (2026-11-03, 09:12:58 UTC), base `435dc20`, pas de conflit.
- Vert de branche : CI #591 SUCCESS 2 m 58 s + re-run macOS validé (run 41520136873, terminé 06:05 UTC).
- Arbitre main ci.yml : `ci.yml #592` — SUCCESS, 8 m 45 s, **0 issue auto ouverte**.
- L'issue [#599](https://github.com/RaphOlivier/OmniFab/issues/599) (rouge test-first attendu, corrigé par `6e0fdf6`/`f02b886`) fermée avec motif.

## Écarts et dettes signalés

- **Aucun écart aux designs** : le lot exécute D3/D4 validés le 2026-10-26, l'amendement « changement de croquis autorisé » était déjà consigné — ADR-0024 inchangée.
- **cargo-deny sur main** : run rouge = advisory `zip` (GHSA-q7v9-x93c-xqv3, DoS à l'extraction, surface forge-doc) — hors arbitre ci.yml (workflow dédié, sans issues auto), distinct des 3 CVE zip 2.4.2 déjà ignorées et datées (2026-10-30) ; mini-lot prioritaire planifié (montée zip ≥ 9.0.0).
- Branches résiduelles à supprimer par Raphaël : `feat/phase-b-lot2-forge-session` (tête `f02b886`, fusionnée), `feat/phase-b-lot1-segments-types` (tête `e4fc2a0`, fusionnée).

## Leçons

1. Les reprises de compilation (E0502 — move du MoteurCad, E0631 — qualification d'impl) se succèdent en pelage d'oignon : le run s'arrête à la première erreur, chaque correctif révèle la couche suivante — fermer l'issue auto après chaque correctif pour débloquer le signalement suivant.
2. Le rustdoc sous `-D warnings` refuse les liens intra-doc vers les items privés (`private_intra_doc_links`) : écrire les références aux items non réexportés sans crochets — leçon déjà connue (v46), elle se réapplique à chaque nouvelle crate exposée.
3. L'enveloppe « à travers » (1,1 × diagonale) est une valeur d'exécution jamais journalisée : le journal ne porte que la décision, jamais les constantes du moteur — le rejeu reste déterministe.
4. L'alias système `cad.extrusion{id}.profondeur` vit dans la même transaction que la feature : un seul groupe d'undo, jamais d'alias orphelin — et le préfixe `cad.` réservé empêche toute collision avec les alias utilisateur (OF-VAL-0008).
5. Le budget formes_max se compte sur les instances STEP réelles (`#N =`) et se refuse avant l'entrée dans le noyau (OF-CAD-0004) : la garde vit à la frontière, pas dans OCCT.

## Fiches touchées

- [état-actuel](https://github.com/RaphOlivier/OmniFab/blob/main/docs/knowledge/etat-actuel/KNOWLEDGE.md) **v73** ;
- fiche archives **v40** (présente entrée) ;
- [journal-fonctions](https://github.com/RaphOlivier/OmniFab/blob/main/docs/knowledge/historique/journal-fonctions.md) : 6 lignes J3 (`c36cb15`) ;
- [sommaire](https://github.com/RaphOlivier/OmniFab/blob/main/docs/sommaire.md) : ligne de la présente entrée (règle v7).

## Suite proposée

« [J0] — Réception audit externe n°5 (protocole d'audit, application aux frontières de lots) + mini-lot prioritaire advisory zip ≥ 9.0.0 (GHSA-q7v9-x93c-xqv3) »
