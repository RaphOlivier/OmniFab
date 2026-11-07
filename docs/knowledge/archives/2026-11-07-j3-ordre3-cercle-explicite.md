# 2026-11-07 — [J3] Ordre 3 ADR-0037 : cercle explicite + balayage nul refusé (A5-02 + A4-18) + sens d'un arc rectifié (A5-03) — dernier lot ADR-0037

**Version de la fiche :** v1 (2026-11-07 — capitalisation de fin d'ordre, discussion Ordre 3 : test-first sur branche, fusion + arbitre + capitalisation. Dernier lot de l'ADR-0037 avant l'ordre 4 ADR-0038 ; en le fusionnant, la phase déclencheur de l'ouverture multi-contributeurs est atteinte.)

## Pourquoi cet ordre

L'encodage « arc à balayage nul = cercle complet », hérité du lot 1 phase B, rendait le bug A5-02 atteignable depuis l'esquisse : un arc d'esquisse effondré (fin == début) devenait un cercle complet — un croquis sans contour fermé pouvait produire un solide. L'audit n°5 (A5-02 + A4-18) exigeait un variant `Segment2::Cercle` explicite et le refus typé du balayage nul ; A5-03 (mineur/doc) portait sur le commentaire de `segments_du_croquis` qui disait « trigonométrique si fin précède ou égale début » alors que le code fait trigo si `fin >= début` (ADR-0020, le sens suit le geste).

## Contenu fusionné (PR #633, squash → main `66c2eb8`, mention IA dans le message)

- **D1 cercle explicite** : `Segment2::Cercle { centre, rayon, sens }` (boucle fermée à lui seul) ; **amendement du variant** — le champ `sens : SensHoraire` s'écarte de la signature `Cercle { centre, rayon }` de l'ADR-0037 D1 car D3 (régions par orientation + parité) exige que le cercle participe à l'orientation normalisée des régions ; consigné dans l'ADR-0037 (section Amendements, daté 2026-11-07 ordre 3, commit `a04bdce4`).
- **A4-18 balayage nul refusé** : `BALAYAGE_NUL_RAD` (pub, 1e-12) + `balayage_apparent()` pub (remplace `balayage_signe`, plus de substitution ±tour) ; refus `GeomError::BalayageNul` **OF-GEO-0007** à la discrétisation (méthode privée `balayage_nul`) ET à l'entrée de `contours()` (défense en profondeur) ; `discretise_balayage` partagé Arc/Cercle.
- **Rejeu tolérant** : impl `Deserialize` manuel via enum fn-locale `Segment2Brut` — un `Arc` à balayage nul des anciens journaux est relu en `Cercle`, sens conservé (patron migrations lot 2, aucun journal cassé).
- **Esquisse** : `cad.rs` bras `Cercle` explicite ; bras `Arc` avec garde `balayage_apparent(...).abs() > BALAYAGE_NUL_RAD` (arc effondré omis → le croquis n'a pas de contour fermé → refus au plan OF-SES-0001, jamais un cercle).
- **DXF** : `cercle_natif()` (CIRCLE → Cercle) ; bras ARC : `span >= 360°` → cercle natif, sinon arc.
- **Registre** : OF-GEO-0007 documenté (error.rs, tests error_codes, `docs/error-codes.md`), exports `BALAYAGE_NUL_RAD`/`balayage_apparent` dans lib.rs, doc `Path2::len` à jour.
- **A5-03** : doc de `segments_du_croquis` rectifiée (« le sens est trigonométrique si l'angle de fin **suit ou égale** celui de début ») + test non-régression des deux sens (arc 30°→60° attendu Trigo, arc 60°→30° attendu Horaire).

## Déroulement CI (protocole : rouge prouvé avant / vert après)

- Branche `fix/a5-02-a4-18-cercle-explicite`, PR draft #633. Rouge test-first C1 `2560de17` prouvé par CI (E0599 ×2 + 7 diffs fmt), issue auto #634 fermée par Raphaël.
- C2 `52f0222e` (8 fichiers) rouge : `error: unused import: Angle` à cad.rs (le bras Cercle ne pousse plus `Angle::ZERO`, le mod tests ne compte pas pour la compilation lib) + 4 diffs fmt — issue auto #635, corrigée par fixup `433ff135` (6 éditions prescrites par le diff CI).
- C3 (A5-03) `c24bf783` rouge fmt : rustfmt splitte le `assert_eq!` final du test sur 4 lignes — issue auto #636, corrigée par `28667e33` (diff prescrit appliqué tel quel).
- Vert de branche **intégral** sur la tête `28667e33` (6/6 jobs, run 42361624629) ; **fusion squash → main `66c2eb8`** ; arbitre main ci.yml **VERT INTÉGRAL (CI #658, 8 m 14 s)**, plus CI Android et Galerie de captures verts sur le même commit ; issues #634/#635/#636 fermées completed avec motif + mention IA, 0 ouverte.
- Suppression de la branche distante `fix/a5-02-a4-18-cercle-explicite` = Raphaël (bouton GitHub, pas d'outil du connecteur).

## Débats et décisions

- **Champ `sens` du variant Cercle** : s'écarte de D1 (`Cercle { centre, rayon }`) mais exigé par D3 — sans sens, un cercle (boucle fermée à lui seul) rend l'orientation du contour indéterminée et casserait l'aire signée recentrée. Consigné en amendement daté dans l'ADR-0037 à la fusion (`a04bdce4`).
- **Doc « précède ou égale » gardée volontairement en C2** : le correctif A5-03 (D4) a été fait en commit séparé (C3) pour tenir « un commit par point » — la doc fausse est restée sur la branche entre C2 et C3, jamais sur main.

## Leçons outillage (reportées en fiche état-actuel v79, suite des leçons v78 (7)-(11))

1. Rustfmt : le `where` d'une fn générique dans un `impl` est TOUJOURS multi-ligne ; chain_width 60 casse les chaînes de méthodes > 60 caractères (une méthode par ligne) ; les patterns de bras match en bloc sont compactés sur une ligne quand ça tient (`debut, fin, sens, ..`) ; une slice d'un struct splitté prend un élément par ligne.
2. Rustfmt : un `assert_eq!` long (~81 col) sans message peut quand même être splitté par rustfmt — la règle « une ligne si < 100 col » n'est pas fiable pour les macros.
3. Le `mod tests` ne compte pas pour la compilation lib (`-D warnings`) : ce que seul le mod tests utilise (ex. `Angle`) doit être importé localement dans le mod tests, pas au niveau fichier.
4. Les garde-fous in-sandbox (ancre unique, équilibre des délimiteurs) sont nécessaires mais le diff fmt exact prescrit par la CI reste la source de vérité — l'appliquer tel quel, sans l'interpréter.

## État en fin de discussion

Ordre 3 A5-02+A4-18+A5-03 soldé et fusionné (`66c2eb8`), arbitre main CI #658 vert intégral, 0 issue « CI : » ouverte, amendement ADR-0037 consigné (`a04bdce4`). **Backlog audit n°5 : les gardes « avant branchement OCCT » (ordres 1 à 3) sont épuisées — la phase déclencheur de l'ouverture multi-contributeurs est atteinte** (rappel v76.1 : relire `preparation/ouverture-multi-contributeurs.md`, annonce à Raphaël, ADR-0040 de gouvernance avant le niveau 2). Capitalisation : fiche état-actuel v79, journal des fonctions +2 lignes J3 (`F:geom:cercle-explicite`, `F:geom:balayage-nul-refusé`), présente entrée + index archives v46, miroir dépôt repoussé.

## Suite

**Annonce du moment opportun à Raphaël (phase déclencheur atteinte)** : ADR-0040 de gouvernance multi-contributeurs à rédiger dans une discussion dédiée avant d'ouvrir le niveau 2 (pistes isolées : traduction EN, DXF restes audit n°2, corpus golden tests CC0, devcontainer). Ensuite : ordre 4 ADR-0038 (plafonds/bornes, A4-19/A4-20/A5-05).

## Liens

ADR-0037 (`docs/adr/ADR-0037-contours-reels-graphe-parite.md`, amendement ordre 3 `a04bdce4`) ; PR #633 ; issues #634/#635/#636 (fermées completed, motif + mention IA) ; fiche état-actuel v79 ; journal des fonctions (2 lignes J3) ; backlog `preparation/backlog-audit5.md` (miroir privé) ; précédente discussion : entrée d'archives ordre 2 (2026-11-06) pour le contexte D2/D3 (`e0eeba8`).
