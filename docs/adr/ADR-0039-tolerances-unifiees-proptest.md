# ADR-0039 : tolérances unifiées et proptest ciblé (audit n°5, ordre 4 — conseils 1 et 2)

- **Statut : accepté** (validé par Raphaël Olivier le 2026-11-03 — choix actés : forge-limites rejeté, conseil 9 arbitré ; proptest sur 4 cibles, la quatrième au-delà du backlog, conforme au conseil 2 ; cargo-fuzz hors périmètre)
- **Date :** 2026-11-03
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 3 phase B, transverse (forge-geom, forge-units, forge-io, forge-values, forge-session, forge-cad, forge-render, CI)
- **Origine :** audit extérieur n°5 (Claude, 2026-11-03, code `c36cb15`) — A5-07 (conseil 1) et conseil 2, ordre 4 du backlog audit n°5 ; constats recoupés dans le dépôt avant inclusion (registre de couverture `coverage-ledger.json`)

## Contexte

- **A5-07** : trois politiques de tolérance coexistaient dans `valider_contour` — réglé par ADR-0036 (ordre 1, non rouvert ici) — et des epsilons absolus en dur restent dispersés hors forge-units : `dxf.rs` (1e-12 ×6 : bulge lu comme nul, corde dégénérée, splines), `segments.rs` (1e-12 : discrétisation ; les sites de balayage nul disparaîtront avec ADR-0037), `esquisse.rs:78` (`SEUIL_GRAINES_CONFONDUES_MM` 1e-6), `renderer.rs:238` (1e-6), `scene.rs`/`camera*.rs` (1e-9, planchers numériques). Les littéraux des tests (cas dégénérés A4-16, assertions d'angles) construisent des cas limites : ils ne sont pas des politiques.
- **Conseil 1** : « une politique de tolérances unique, relative à l'échelle, dans forge-geom [...]. Type Tolerances (linéaire, angulaire, relative) dérivé de l'étendue du dessin, prédicats robustes pour intersections et orientation (crate robust ou arithmétique adaptative), interdiction des littéraux d'epsilon ailleurs (lint ou grep en CI) » — le rapport lui-même assèche la consigne : « vaut la peine, en priorité avant l'extrusion réelle. Commencer petit ».
- **Conseil 2** : « fuzzing et tests de propriétés sur les parseurs et le journal : DXF, SVG, STL, expressions, journal. proptest en CI avec budget de temps fixe, cargo-fuzz en nocturne. Chaque gel ou abort des audits 3 et 4 (A3-02, A4-01, A4-03) se serait trouvé en une nuit » — meilleur rapport valeur/coût selon l'auditeur.
- **Conseil 9** (réservé à l'ordre 4 par la consigne) : crate `forge-limites` sans dépendances pour un type `Limites` unique (octets, entités, profondeur, magnitude) — motive par le risque de dérive d'une borne de magnitude « dupliquée à dessein ».
- **Timing** : ADR-0036 (prédicats exacts `robust`, aire/périmètre) et ADR-0037 (soudure `TOLERANCE_MM`) posent les fondations ; l'implémentation du présent ADR vient après celles des ordres 1 à 3 au checkpoint PC — le « big-bang » `forge-geom::tolerance` rejeté par ADR-0036 devient opportun une fois `valider_contour` fixé.

**Méthode imposée** (backlog audit n°5) : branche courte + PR, test d'abord (rouge constaté avant, vert après), un commit par lot, CI verte entre chacun.

## Décision

### D1 — module `forge-geom::tolerance` : une maison pour les epsilons

Constantes nommées (dans `forge-geom::tolerance`, l'une des deux maisons autorisées de littéraux) :

- `TOLERANCE_MM` = 1e-6 (déjà dans forge-geom, re-exporté par le module) ;
- `RATIO_NUL` = 1e-12 — rapport sans dimension au-dessous duquel on lit zéro (bulge, demi-angle de discrétisation) ;
- `PLANCHER_F64` = 1e-9 — garde numérique d'algorithme, pas une tolérance géométrique (plancher de pas de grille, limites de zoom) ;
- `TOLERANCE_RAD` (1e-9) reste dans forge-units — tolérance angulaire, inchangée.

Prédicats :

- `confondus(a, b)` — distance hypot < `TOLERANCE_MM` ;
- `colineaire(a, b, c)` — prédicat exact via la crate `robust` (orientation nulle) : les décisions combinatoires sont exactes (ADR-0036), pas floutées ;
- `aire_negligeable(aire, perimetre)` — `aire.abs() < TOLERANCE_MM * perimetre` (politique relative ADR-0036, un seul endroit).

Mapping des sites audités (à l'implémentation, sur le code d'après ADR-0037) :

- forge-io `dxf.rs` : `bulge.abs() <= 1e-12` → `RATIO_NUL` ; `corde <= 1e-12` (en mm !) → `TOLERANCE_MM` ; sites splines/points de contrôle → sémantique par site (`RATIO_NUL` ou `TOLERANCE_MM` selon la dimension), recensés dans le commit ;
- forge-geom `segments.rs` : demi-angle de discrétisation → `RATIO_NUL` ; les sites de balayage nul ne sont pas figés ici : ADR-0037 les supprime (`Segment2::Cercle`, OF-GEO-0007) ;
- forge-session `esquisse.rs` : `SEUIL_GRAINES_CONFONDUES_MM` → `tolerance::confondus` (même valeur 1e-6, une seule source) ;
- forge-render `renderer.rs:238` : `longueur < 1e-6` → `TOLERANCE_MM` ;
- forge-render `scene.rs`, `camera*.rs` : planchers numériques 1e-9 → `PLANCHER_F64`.

`valider_contour` (orientation, `segments_se_touchent`, `EPS_AIRE`) n'est pas rouvert : la zone est celle d'ADR-0036 (ordre 1) — l'epsilon d'aire disparaît au profit de signes exacts `robust`.

*Tests :* parité comportementale à la migration (chaque site converti garde ses tests verts — mêmes valeurs, source unique) ; prédicats testés seuls (`confondus` à la limite 1e-6, `colineaire` exact sur triplets pathologiques, `aire_negligeable` aux deux échelles extrêmes de A4-16).

### D2 — tripwire CI : plus de littéral `1e-` hors des deux maisons

Étape du job `lint-test` de ci.yml : échec si `git grep -nE '1e-[0-9]'` trouve une occurrence dans `crates/*/src/*.rs` ou `apps/*/src/*.rs`, hors `crates/forge-units/` et `crates/forge-geom/` (et hors fichiers de tests). Le message d'échec pointe vers `forge-geom::tolerance`. Assumé dans l'ADR et en commentaire du workflow : c'est un détecteur, pas une forteresse — l'écriture `1.0e-9` y échappe ; la convention et la revue de code ferment la brèche, un lint dédié pourra durcir plus tard. Le rendu est **converti** (pas d'allowlist forge-render) : trois lignes, une règle simple.

*Tests :* l'étape est testée par elle-même — rouge constaté sur un littéral introduit en exemple, vert au retrait.

### D3 — proptest ciblé, budget de temps fixe

- Nouvelle **dev-dépendance `proptest`** (épinglée) pour forge-values, forge-session, forge-cad et forge-io — édition du lock au checkpoint PC (comme `robust`, ADR-0036).
- **Budget de temps fixe** : 256 cas par test, graine déterministe (config par crate), fichiers `proptest-regressions` committés — un échec en CI se reproduit localement, le shrinking donne le contre-exemple minimal.
- **Quatre cibles** (arbitrage Raphaël du 2026-11-03 : la quatrième au-delà du backlog, le conseil 2 vise aussi les parseurs) :
  1. **expr (forge-values)** : génération d'expressions (nichages, unicode, bornes) — jamais de panic ; dépassements `LONGUEUR_MAX_EXPRESSION`/`MAX_NOEUDS_AST` refusés typés (OF-VAL-0003/0011 — lignage A3-02/A4-01) ; les expressions valides évaluent comme l'oracle (évaluateur récursif naïf de test) ;
  2. **apply/journal (forge-session)** : événements forgés (champs absurdes, ids négatifs, versions de schéma inconnues, types inconnus) — jamais de panic ; refus = rien appliqué (transactionnalité ADR-0016 A1) ; types inconnus tolérés et comptés (A4-03/A4-20) ;
  3. **valider_contour (forge-cad)** : contours aléatoires (2-20 points, échelles mêlées, dégénérés : confondus, alignés, aire nulle) — jamais de panic, refus typés ; les cas A4-16 deviennent des **propriétés** : tout contour dont aire/périmètre < `TOLERANCE_MM` est refusé quelle que soit l'échelle absolue ; tout contour simple fermé dans les bornes de magnitude est accepté ;
  4. **parseurs forge-io (SVG/DXF/STL)** : octets arbitraires — jamais de panic, que des `Result` typés (OF-IO-*), budgets respectés (un octet de plus que le budget = refus, jamais une lecture).
- Les propriétés ne remplacent pas les tests existants : elles ciblent la classe panic/abort/gel des audits 3 et 4.

### D4 — conseil 9 : `forge-limites` rejeté (arbitrage)

- La duplication redoutée n'existe pas : `MAGNITUDE_MAX_MM` vit une seule fois (forge-geom, A4-13b) et forge-session y accède par dépendance — vérifié au dépôt le 2026-11-03, aucune copie forge-session.
- Les budgets vivent là où ils s'appliquent — par parseur, publics et réglables par l'appelant (verrou ADR-0009 : « la sécurité ne doit pas brider les fonctions ») — et ADR-0038 D2 vient de faire des défauts des parseurs la source unique de la pré-lecture desktop.
- Une crate de plus (workspace, licences, CI) pour un gain nul aujourd'hui : **rejeté**. Signal de re-évaluation inscrit pour l'audit 6 : toute borne dupliquée entre crates rouvre la question.

### Registre des codes

Aucun nouveau code : l'ordre 4 ne crée ni erreur ni frontière. OF-VAL-0003/0011, OF-SES-0001/0002/0005, OF-IO-* et OF-GEO-* existants couvrent tous les refus visés.

## Alternatives rejetées

- **Type `Tolerances` complet dérivé de l'étendue du dessin** (conseil 1 intégral) : le rapport lui-même dit « commencer petit » ; les cas relatifs connus sont couverts (aire/périmètre ADR-0036, soudure ADR-0037) ; extension du module par ADR le jour où un cas l'exigera.
- **cargo-fuzz en nocturne** (conseil 2 intégral) : chaîne nightly + libfuzzer, CI nocturne à construire — reste la dette prévue ADR-0009, hors ordre 4.
- **Allowlist forge-render** dans le tripwire : convertir coûte trois lignes ; une exception de plus dilue la règle.
- **Lint clippy dédié** (interdiction des littéraux) : le grep suffit au stade actuel ; durcir si l'évasion `1.0e-9` devient réelle.
- **crate `forge-limites`** (conseil 9) : voir D4.
- **proptest sur le conteneur forge-doc** : hors périmètre — la couche a déjà les budgets ADR-0016 et le fuzz prévu ADR-0009 ; l'ordre 4 reste ciblé.

## Implémentation (checkpoint PC)

Après les ordres 1 à 3 (l'ADR s'applique au code d'après ADR-0037 — `Segment2::Cercle`). Branche courte + PR, test d'abord, un commit par lot : D1 (module + migration des sites, un commit par crate touchée si plus lisible), D2 (tripwire CI, rouge/vert sur lui-même), D3 (proptest, un commit par cible), D4 = rien à implémenter. Édition du lock : `proptest` en dev-dépendances. CI verte entre chacun. Recomptage `expect` au même passage.
