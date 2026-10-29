# Vérification des dépendances : cargo-deny (revue point 13)

Le fichier `deny.toml` à la racine configure [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) :

- **licences** : liste blanche des licences acceptées (noyau GPL-3.0+ / crates
  MPL-2.0, dépendances MIT/Apache-2.0 et transitives) ;
- **advisories** : vulnérabilités connues (RustSec) et paquets *yanked* refusés ;
- **sources** : registre crates.io uniquement, aucun dépôt git inconnu.

## En local

```bash
cargo install cargo-deny
cargo deny check
```

## Intégration CI (activée le 2026-10-29)

Le contrôle est actif via le workflow dédié
[`.github/workflows/cargo-deny.yml`](../.github/workflows/cargo-deny.yml)
(décision Raphaël, audit n°4 A4-12) : déclenché sur push main et PR touchant
`Cargo.lock`/`deny.toml`/le workflow, + `workflow_dispatch`. **`ci.yml` reste
inchangé** et ce workflow ne crée jamais d'issue — un échec est visible dans le run.

```yaml
name: cargo-deny
on:
  push:
    branches: [main]
    paths: ["Cargo.lock", "deny.toml", ".github/workflows/cargo-deny.yml"]
  pull_request:
    paths: ["Cargo.lock", "deny.toml", ".github/workflows/cargo-deny.yml"]
  workflow_dispatch:
jobs:
  deny:
    name: cargo-deny (licences + advisories + sources)
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v5
      - uses: EmbarkStudios/cargo-deny-action@v2
```

Note : l'action lit `deny.toml` à la racine par défaut ; les advisories sont
vérifiées sur les versions exactement verrouillées par le `Cargo.lock` versionné.
