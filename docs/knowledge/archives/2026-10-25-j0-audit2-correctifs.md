# [J0] Traitement de l'audit externe n°2 — 5 correctifs de robustesse poussés (2026-10-25)

## Contexte

Raphaël a transmis un rapport d'audit n°2 (IA externe, analyse statique) sur le dépôt, en précisant : rapport fait sur une version **antérieure**, une discussion jalon 3 est déjà ouverte. Traitement après re-vérification de chaque constat sur la tête `4409a65` : le rapport est fiable, ses points critiques exacts. Les correctifs sont des points de robustesse → **pré-approuvés** (gouvernance 2026-10-19), un point = un commit, tests de régression inclus (test d'échec et correctif dans le même commit, faute de cargo local — la CI validera).

## Décisions tranchées (au fil de l'eau, micro-décisions)

1. **N1 (noms en chemin)** : le lexeur accepte le point **entre deux segments** (`piece.largeur`, `esquisse.<alias>`) — un point suivi d'autre chose qu'un début d'identifiant est une erreur typée OF-VAL-0003 (« a.5 » invalide). Les cotes `esquisse.*` deviennent lisibles par les formules utilisateur (le défaut que `verifie_nom_chemin` créait sans le lexeur).
2. **N2 (substitution)** : `remplacer_identifiants` étend la bordure d'identifiant au point (remplacer « a » ne touche plus `piece.a` ni les décimales `1.5`) et épargne les appels de fonction (« min( ») ; les alias nommés comme des fonctions sont refusés au plan (commande, OF-SES-0001) — pas de nouveau code d'erreur, `forge_values::est_nom_de_fonction` exporté pour ça.
3. **Récursion non bornée** : profondeur max 256 dans le parseur (parenthèses ET chaînes de négations), `collecte_deps_transitifs` rendu itératif. Résidu accepté : `chemin_vers` reste récursif (borné par le nombre de nœuds, atteignable uniquement par un journal construit commande par commande) — à revoir si un import crée des chaînes de nœuds.
4. **N7 (export non atomique)** : `ecris_fichier` du desktop écrit `<cible>.tmp` + `sync_all` + rename, temporaire nettoyé à l'échec.
5. **N9 (SVG plat)** : chaque dimension de la boîte exportée plancher à `MARGE_BOITE_PLATE` = 0,1 mm (épaisseur du trait) — plus de `height="0mm"`.

## Chaîne (tous sur main)

`d152080` (N1 : expr.rs + tests/noms_chemins.rs) → `e7c4f9f` (N2 : expr.rs, lib.rs, command.rs, tests values+session) → `e4e491a` (récursion : expr.rs, lib.rs) → `628d709` (N7 : main.rs ecris_fichier + test) → `4315374` (N9 : forge-io svg.rs + test).

## CI

Rouge attendue et **inchangée dans sa cause** : `Cargo.lock` périmé depuis le lot 1 phase A (issues #311-#313 sur 4409a65) ; les issues #314-#317 (mes commits) portent toutes le même « cannot update the lock file --locked ». **Rien ne compile tant que le lock n'est pas poussé** (build local de Raphaël) — mes commits seront validés par ce même passage. Aucune issue fermée (aucune convergence possible sans compile).

## Restes de l'audit n°2 (backlog proposé, non traités)

- Discrétisation à nombre fixe (64 segments) → tolérance/flèche : renvoyé aux ADR D1/D2 (segments typés Path2, jalon 3 phase B).
- DXF : $INSUNITS, INSERT/blocs, extrusion, RapportImport (ADR-0016 D2), chaînage de contours, LWPOLYLINE à l'export — jalon 3 phase B.
- N3 (suppression « Remplacer » crate-level non atomique) : faible (validation complète avant modification), hygiène clone-then-swap à faire sauter occasion.
- N5 (suppression d'entités/contraintes), N6 (validations dégénérées), N4 (solveur à état historique) : à traiter avec l'esquisse du jalon 3.
- N8 (catch_unwind + TryRecvError dans le fil de travail) : à faire avec le prochain lot desktop.
- Hygiène build : rust-toolchain.toml, rust-version, LICENSE manquants dans certaines crates, expect() en production — à proposer en lot documentaire (ADR déjà prévu pour la doc).
