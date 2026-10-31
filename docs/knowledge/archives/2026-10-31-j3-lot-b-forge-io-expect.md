# [J3] Lot B forge-io expect : 11 messages d'invariant, zéro changement de comportement

**Date :** 2026-10-31 (soir) — **Jalon dominant :** J3 — **Statut :** clôturé (PR #561 fusion squash → main `279b7d9`).

## Contexte

Troisième lot de la campagne expect (fiche `preparation/patchs-expect.md`) : les 11 sites `expect()` de production de forge-io sont des invariants de garde — le lot ne touche QUE les messages, aucun chemin d'exécution n'est modifié, aucun rouge test-first n'était possible (cas analogue aux invariants du lot A).

## Sites (11)

- `stl.rs` ×3 : `.expect("indice validé à la construction")` → `.expect("invariant : indices validés par Mesh::from_parts")` (export — les triangles ne référencent que les sommets du maillage) ;
- `stl.rs` ×3 : `.expect("quatre octets")` → `.expect("invariant : tranche [80..84] garantie par la garde len ≥ 84")` (est_binaire, est_binaire_tolerant, import_binaire — la garde `len < 84` précède chaque lecture) ;
- `stl.rs` ×1 : `lire_f32` → `.expect("invariant : offset borné par est_binaire_tolerant (len ≥ 84 + 50n)")` (lecture sur des blocs de 50 octets annoncés par le compteur) ;
- `svg.rs` ×2 : `.expect("écriture en mémoire infaillible")` → `.expect("invariant : write! dans String infaillible hors OOM")` (write! dans un `String` en mémoire) ;
- `dxf.rs` ×2 : `.expect("au moins deux sommets")` → `.expect("invariant : garde len < 2 → continue ci-dessus")` (export — la garde `points.len() < 2 → continue` précède).

## rustfmt

La leçon du lot C s'applique sans reprise : les chaînes de plus de 60 caractères APRÈS le récepteur passent en forme éclatée (les 4 sites compteur/lire_f32 et les 2 sites dxf), les 5 autres restent une ligne (maillon ≤ 60, ligne ≤ 100) — CI verte du premier coup.

## Verdict

Vert de branche INTÉGRAL sur la tête `7d205b2` : lint-test (fmt+clippy+tests+rustdoc) ✅, ubuntu ✅, macos ✅, windows ✅ (vert AVANT la fusion — premier lot de la campagne à fusionner sur preuve intégrale), aarch64 ✅, APK ✅. Fusion squash → main `279b7d9` ; run main ci.yml à relever (arbitre final). 0 issue auto : comportement inchangé, aucun rouge possible.

## Leçons

- Un lot « messages uniquement » ne déclenche aucun mécanisme d'issue — la preuve du vert passe par les check runs, pas par l'absence d'issues.
- Le préfixe « invariant : » dit OÙ vit la garde (conventions expect v15) : chaque message cite la garde qui précède (`len < 84`, `len ≥ 84 + 50n`, `len < 2 → continue`).

## Fiches touchées

etat-actuel v63, journal-fonctions inchangé (messages, pas une fonction), archives v28 (cette entrée).

*rédigé avec l'assistance de Vibe (IA) — relu et validé par Raphaël, mainteneur OmniFab (conventions v10-v15).*
