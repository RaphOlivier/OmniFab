# 2026-11-04 — [J3] Mini-lot zip 9.3.0 : advisory GHSA-q7v9-x93c-xqv3 traité en priorité sécurité

**Version de la fiche :** v1 (2026-11-05 — rédigée a posteriori dans la discussion Ordre 1 : dette de capitalisation identifiée à la v75 de l'état-actuel, soldée ici.)

## Pourquoi ce lot avant l'ordre 1

L'audit n°5 (2026-11-03) avait validé en bloc un ordre de travail : gardes « avant branchement OCCT » d'abord (ordres 1 à 3), mineurs ensuite. L'advisory zip GHSA-q7v9-x93c-xqv3 (DoS à l'extraction, surface forge-doc) a été inséré AVANT l'ordre 1 : décision de sécurité (advisory actif contre dette connue), lot rapide. Hors arbitre ci.yml (workflow cargo-deny dédié, sans issues auto) : un run rouge cargo-deny ne conclut jamais au rouge du projet.

## Déroulement

- Branche `dep/zip-9` : Cargo.toml zip 2.4.2 → 9.3.0 (default-features = false, deflate), 3 ignores RUSTSEC-2026-0367..0369 retirés de deny.toml. Les 2 premiers commits sans lock : rouges `--locked` attendus, issues #617-#621 fermées avec motif.
- Cargo.lock régénéré en local par Raphaël (`19ee596`, checkpoint PC — le lock ne se génère pas depuis l'interface GitHub).
- Rupture API zip 9 corrigée sur la même branche : `21327dd` (`ZipFile<'a, R>` générique sur read_bounded/bounded_strings, `name()` faillible — container.rs), `d03031e` (`InvalidArchive(Cow<'static, str>)` → `"archive".into()` — tests/error_codes.rs).
- async-executor 1.14.0 → 1.14.1 (`b899008`) : yank du registry détecté ENTRE DEUX RUNS VERTS par `yanked = "deny"` — corrigé par `cargo update -p`, pas un ignore (l'ignore clipboard-win@5.4.1 reste, à monter à la prochaine passe).
- Fusion squash PR #616 → main `c24efaa` ; vert de branche intégral `b899008` (7 checks, macOS 30 m 32 s) ; arbitre main VERT SUBSTANTIEL — macOS bloqué par la capacité GitHub (« job not acquired by Runner », « capacity constraints macOS arm64 », run « queued » ~4 h puis 2 re-rats) : orthogonal, l'arbre était déjà macOS-vert via la PR.

## Leçons

1. Deux check runs « build APK » par SHA (run push annulé par la concurrency + run PR) — trier par statut, jamais dédupliquer par nom de job.
2. Yank registry → `cargo update -p`, jamais un ignore.
3. `cargo deny check` en local = diagnostic fiable (cargo-deny 0.20.2 installé chez Raphaël, même version que l'action CI).
4. ci.yml sur main resté « queued » ~4 h → cancel + re-run.
5. Quota api.github.com aléatoire (IP partagée) — connecteur github_app d'abord, page HTML en secours, advisory-db via raw.githubusercontent.com.
6. Les scheduled_tasks n'ont pas déclenché ce jour-là (2 rappels créés, 0 parti) — préférer les pings manuels.
7. Le champ resource.text donne le contenu d'un fichier du dépôt dans le sandbox : éditer/pousser sans retranscrire (patron container.rs de ce lot).

## Liens

Fiche état-actuel v75 puis v76 (texte intégral de l'époque dans la ligne Version v75, historique Git du miroir) ; PR #616 ; issues #617-#621 ; ADR-0036 (ordre des correctifs) ; backlog `preparation/backlog-audit5.md` (miroir privé).
