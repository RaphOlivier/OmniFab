[J3] — Lot 1 de la phase A : ADR-0022..0027 au dépôt, crate forge-cad (OCCT système), registre OF-CAD, CI OCCT 3 OS — commit 4409a65, 2026-10-25

## Ce qui a été poussé (un seul commit, 14 fichiers)

- **6 ADR au dépôt** (`docs/adr/ADR-0022..0027-*.md`) : découpe du lot préparé du 2026-10-22 par ancres, en-têtes « ACCEPTÉ le 2026-10-25 », digest 4 axes intégré à ADR-0022 (+ risques transversaux), remplacements « selon digest » → décisions tranchées (Extrusion additive+soustractive dans 0022 ; rendu depth_stencil+ombrage simple+MSAA 4× dans 0027 D3) ; aucun résidu.
- **Crate `forge-cad` (MPL-2.0)** : `opencascade 0.3` SANS features par défaut (sinon OCCT 7.8.1 statique vendue — interdit D1) → liaison dynamique vers OCCT système ≥ 7.8 ; module interne `occt` seule frontière (Forme : boite/cylindre_centre/prisme/soustraire/unir/tesseller/STEP/BREP) ; API publique `Solide`/`Maillage`/`CadBudgets`/`step::`/`brep::` en `Length` strictes ; volumes par tessellation signée (exact plans, ~1e-4 courbes à tol 0,001) — VolumeProperties exacte reportée lot 2 ; budgets vérifiés avant OCCT (OF-CAD-0004) ; golden tests ×10 (volumes exacts, STEP/BREP aller-retour, budget, STEP invalide, codes stables, registre).
- **Registre** : `CAD` attribué à forge-cad (4 codes OF-CAD-0001..0004), retiré des réservés.
- **CI** : Linux → `ubuntu-26.04` (OCCT 7.9 via apt `libocct-*` + cmake ; 24.04 = 7.6.3 insuffisant) ; macOS brew opencascade ; Windows vcpkg opencascade (statique — déviation D1 documentée) + cache actions/cache@v4 sur `installed/` ; `DEP_OCCT_ROOT` partout ; check-aarch64 exclut forge-cad (OCCT non cross-installable). Mécanisme d'issues intact.
- Vérification post-push par get_commit full_patch : 14 fichiers, sondes UTF-8/`//!`/digest toutes présentes (leçon push_files appliquée).

## État CI (rouge ATTENDUE)

Issues #311 (check-aarch64) et #312 (build macos) : unique cause = `Cargo.lock` périmé (`--locked` + nouvelles deps). Les autres jobs suivront le même motif. **Seule action possible : Raphaël régénère le lock localement** (prérequis + `cargo build -p forge-cad`, commit+push du Cargo.lock) — la CI ne peut pas converger avant. Checklist de validation manuelle de la phase A livrée (`preparation/phaseA3-design-checklist.md`) : prérequis apt OCCT ≥ 7.8 (alerte si Kubuntu 24.04 = 7.6), ldd liaison dynamique, 10 golden tests, CI verte 3 OS (Windows vcpkg ~30-60 min au premier build, caché ensuite).

## Leçons

- run_typescript : budget 60 s par appel → surveiller la CI par petits rounds (sleep ≤ 30 s), jamais une longue boucle.
- Découpe des ADR par ancres en sandbox (read_file prep + write_file) : fiable, backticks littéraux préservés — le contenu ne JAMAIS transiter par un template literal.
- Nommer les ADR dépôt avec slug dès l'écriture (adr-0022.md renommé en ADR-0022-decoupage-jalon-3.md).

## Suite

Lot 2 phase A : import .FCStd (décision Raphaël, contre recommandation — ZIP Document.xml + BREP), volume exact VolumeProperties, BREP binaires ; puis phases B→E selon ADR-0022. Checkpoint visuel « CAO lisible » ~mi-décembre.
