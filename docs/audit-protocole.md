# Protocole d'audit du dépôt OmniFab

Ce document s'adresse à **tout auditeur externe** (IA ou humain) chargé de relire le code ou la documentation d'OmniFab. Il fixe le périmètre, les conventions à connaître avant de signaler, le format de rapport attendu et le traitement réservé à chaque signalement. Il rend les audits comparables d'une session à l'autre.

## 1. Cadrage de l'audit

- **Ancrer l'audit à un commit précis** : indiquer toujours le SHA examiné en tête du rapport. Un signalement sans SHA n'est pas exploitable.
- **Documents de référence à lire avant de signaler** (dans cet ordre) :
  1. [`AGENTS.md`](AGENTS.md) — règles de travail et interdits ;
  2. [`CONTRIBUTING.md`](CONTRIBUTING.md) — conventions non négociables ;
  3. [`docs/architecture.md`](architecture.md) — frontières entre crates ;
  4. [`docs/error-codes.md`](error-codes.md) — registre des codes `OF-*` ;
  5. [`docs/adr/`](adr/) — le *pourquoi* de chaque décision (un ADR prime sur l'intuition de l'auditeur : si un choix figure dans un ADR accepté, ce n'est pas un défaut) ;
  6. [`docs/knowledge/etat-actuel/KNOWLEDGE.md`](knowledge/etat-actuel/KNOWLEDGE.md) — dette connue et assumée (voir §3).
- **Automatismes d'abord** : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, et le workflow `liens-md.yml` s'il est actif. L'audit humain/IA porte sur ce que ces outils ne voient pas : logique, invariants, sécurité, cohérence inter-crates.

## 2. Conventions délibérées (ne pas signaler comme défauts)

Ces choix sont actés (ADR cités) — un signalement à leur sujet n'est utile que s'il apporte une **aggravation ou une régression**, pas s'il reformule la décision :

- **Unités strictes** : jamais de `f64` nu pour une grandeur physique (newtypes `Length`, tolérance 1e-6 mm) ; les `f64` aux frontières FFI/GPU sont documentés (ADR-0015 : le GPU reçoit des `f32`).
- **Codes d'erreur typés** (`thiserror`), registre `OF-*` stable : jamais renommé, jamais réutilisé (ADR-0003) ; pas d'`unwrap()` hors tests.
- **Licences par zone** : noyau GPL-3.0+ ; crates isolées MPL-2.0 ; `forge-gcs`/`forge-gcs-sys` LGPL-2.1+ (héritage planegcs, ADR-0013) ; **jamais GPL pour les données** (ODbL/CC0/CC-BY). La cohabitation est délibérée et vérifiée par cargo-deny (`deny.toml`).
- **Vendoring figé** : `third_party/planegcs` est un sous-module jamais modifié localement (ADR-0013) — le contenu du sous-module n'est pas auditable ici.
- **`expect()` en production** : inventaire en cours de résorption avec politique d'exception annotée ; admis à ce jour dans `build.rs`, dans les tests, et pour les invariants garantis par la CI (clés FTL embarquées). Le lot de conversion est planifié.
- **`unsafe`** : seulement aux frontières FFI (shim C de planegcs, bindings OCCT) — signaler tout `unsafe` ailleurs.
- **Wasm-friendly** : features `native`/`wasm`, aucun accès natif hors trait d'abstraction ; i18n Fluent sur toute chaîne d'interface ; `help_id` sur chaque widget.
- **CI et lock** : le build officiel est `--locked` ; une CI rouge « Cargo.lock périmé » pendant une fenêtre de convergence est une situation connue, pas un défaut d'audit.

## 3. Dette connue et assumée (ne pas re-signaler, sauf aggravation)

La liste à jour vit dans la fiche [état actuel](knowledge/etat-actuel/KNOWLEDGE.md) (« Dette ouverte »). Les principaux postes : backlog de l'audit n°2 (cf. [archives](knowledge/archives/2026-10-25-j0-audit2-correctifs.md) — discrétisation à tolérance, DXF `$INSUNITS`/blocs/extrusion, N3-N8), découpage de `apps/omnifab-desktop/src/main.rs`, `expect()` en production, traduction EN de la documentation. **Un signalement déjà couvert par cette liste doit être marqué comme tel** — l'auditeur qui la lit fait gagner à tous un cycle de triage.

## 4. Format de rapport attendu

Chaque signalement, numéroté (`A<n>-<nn>`, n = numéro d'audit) :

| Champ | Contenu |
|---|---|
| Fichier:ligne | au SHA audité |
| Sévérité | bloquant / majeur / mineur / hygiène |
| Classe | bug · violation de convention (laquelle) · risque à documenter · suggestion |
| Preuve | scénario d'entrée minimal qui déclenche (pas de « peut-être » : si pas reproductible, dire « non vérifié ») |
| Correctif proposé | esquisse + test de régression proposé (entrée/attendu) si bug |

Signalement **sans ligne ni scénario = classé « à préciser » et retourné**. Un faux positif documenté (le coût des relectures d'IA successives) : toujours vérifier dans les ADR avant de conclure.

## 5. Traitement réservé aux signalements

Engagement du mainteneur : chaque signalement recevra un verdict explicite — **corrigé** (branche courte + PR, test de régression obligatoire, rouge avant/vert après) · **dette acceptée** (ajoutée à la fiche état actuel avec date) · **refusé** (motif, généralement conflit avec un ADR). Les rapports d'audit sont archivés dans `docs/knowledge/archives/` avec leur SHA et leurs verdicts, ce qui permet aux audits suivants de vérifier les corrections plutôt que de tout re-signaler.

## 6. Modèle de consigne à donner à l'auditeur IA

> Tu audites le dépôt public OmniFab au commit `<SHA>`. Lis d'abord `docs/audit-protocole.md` et applique-le à la lettre : conventions délibérées (§2), dette assumée (§3), format de rapport (§4). Périmètre de cet audit : `<crates ou thématique>`. Ne signale que ce qui est actionnable, avec fichier:ligne et scénario. Classe chaque signalement et propose un test de régression pour les bugs. N'émets pas d'opinion sur les décisions couvertes par un ADR sans avoir lu l'ADR.
