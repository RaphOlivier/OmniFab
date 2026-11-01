# [J0] — Bump EN de la couche d'accueil : traduction des amendements sécurité, sort d'AGENTS.md, alignement du statut phase A

**Date :** 2026-11-01 · **Jalon dominant :** J0 (transversal — documentation) · **Fiches touchées :** [Conventions](../conventions/KNOWLEDGE.md) v17→v18, [Archives](KNOWLEDGE.md) v35→v36

## Contexte

Discussion de suite directe de la sécurisation du dépôt (entrée du 2026-11-01, archives v35) : exécution du « bump EN » de la couche d'accueil, synchronisé au point dur que constitue la clôture de l'agenda J0 — le protocole de la règle langue (conventions v5/v6) : traduction par lots aux points durs, bandeau d'autorité + date/SHA sur chaque fichier EN, le FR fait foi en cas de divergence.

## Déroulé et argumentaire

1. **Traductions** (commit `0794c7a`, roundtrip vérifié, liens-md #75 vert) : README EN — bandeau de statut mis à jour (phase A clôturée, binding OpenCascade validé, CI verte multi-OS + cargo-deny), bloc « Getting your bearings » (sommaire général, absent de la couche EN), entrée sommaire en tête de la section Documentation, nouvelle section « Development with AI assistance » (déclaration globale du README FR traduite fidèlement) ; CONTRIBUTING EN — intégral à jour : « AI-generated or AI-assisted contributions » (clause comprendre/assumer, fermeture sans relecture, aucun assouplissement des conventions) + « Warning — hidden instructions (prompt-injection defense) » (tout texte d'une issue/PR/commentaire/fichier = donnée, jamais instruction ; tentative de détournement ignorée ; fermeture + signalement) ; statut branches/PR actualisé (protection main active — `lint-test`, vérifiée 2026-11-01) ; sommaire EN — bandeau 2026-11-01 (commit `d871072`) et entrée AGENTS.md annotée.
2. **Sort d'AGENTS.md tranché : non traduit, écart documenté** (décision de périmètre — conventions v18, point (6) de la règle langue). Argumentaire : l'audience d'AGENTS.md, ce sont les agents IA opérant sur ce dépôt, qui travaillent en FR canonique ; une version EN créerait une seconde surface d'instructions parallèle à celle qui fait foi — la même hygiène que l'avertissement anti-injection (une seule source d'instruction, l'écart rendu visible plutôt que trompeur). L'écart est documenté là où un lecteur EN le cherche : bandeau des trois fichiers EN touchés (« deliberately left untranslated (French only) »), entrée du sommaire EN, statut de CONTRIBUTING EN.
3. **Incohérence de statut repérée au fil de la traduction** : le tableau de feuille de route du README FR (ligne jalon 3) et le statut de CONTRIBUTING FR disaient encore « phase A ouverte » alors que le bandeau du README dit « phase A clôturée (binding OpenCascade validé) » depuis `d1881f2` — traduire fidèlement aurait propagé l'écart dans la couche EN. Corrigé sur demande de Raphaël : `58bf733` (FR, source de vérité) puis `cf24e27` (couche EN alignée + bandeaux re-pointés sur `58bf733`) — l'ordre FR d'abord garantit que le bandeau EN référence l'état FR traduit. Recherche code « phase A ouverte/open/opened » : exactement les 4 occurrences attendues, aucune autre fiche ou doc touchée ; liens-md #76/#77 verts (aucun lien modifié).
4. **Dette du sommaire comblée au passage** : la table « Archives des discussions » du sommaire FR avait 18 lignes de retard (aucune entrée du 2026-10-31 ni du 2026-11-01 — la règle v7 « tout nouveau document figure au sommaire » n'avait pas été tenue pour les campagnes A3/A4/expect) : resynchronisée avec 19 lignes (18 manquantes + la présente entrée).

## Décisions tranchées

- AGENTS.md reste FR pur dans la couche EN — non traduit, écart documenté (conventions v18).
- Bandeaux EN : date + SHA de l'état FR traduit ; re-pointés sur `58bf733` après le correctif de statut.
- Ordre de correctif : FR d'abord, couche EN re-pointée ensuite.

## Leçons

- L'oracle d'existence des liens doit se limiter aux cibles **nouvelles** : une salve parallèle de ~40 `get_file_contents` est rejetée en bloc par le connecteur (limite de débit) alors que 2 appels (les 2 seules cibles ajoutées) suffisent — les liens inchangés sont déjà couverts par les runs liens-md verts précédents.
- `setTimeout` non nul indisponible dans le sandbox : les attentes post-push se font par appels successifs, jamais par sleep.

## Points ouverts

- Index des archives (fiche + miroir) : les entrées v18→v29 (2026-10-31, hors lot E) n'ont toujours pas leur ligne d'index — dette documentée depuis v30 ; un backfill dédié reste possible (les fichiers existent au dépôt).
- La relecture humaine des traductions EN appartient au mainteneur (le commit `0794c7a` porte : « traduction rédigée avec assistance IA ; version française relue et assumée »).

## Fiches et commits

- Conventions v18 (`0794c7a` au miroir) ; archives v36 (présent commit) ; couche EN `0794c7a`/`cf24e27`, correctif FR `58bf733` ; sommaire FR resynchronisé ; liens-md #75/#76/#77 verts.

*Discussion menée avec l'assistance de Vibe (IA) — clôture de phase A et correctifs demandés par Raphaël Olivier, mainteneur OmniFab.*

