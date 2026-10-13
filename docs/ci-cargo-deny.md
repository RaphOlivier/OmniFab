# Vérification des dépendances : cargo-deny (revue point 13)

Le fichier `deny.toml` à la racine configure [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) :

- **licences** : liste blanche des licences acceptées (noyau GPL-3.0+ / crates
  MPL-2.0, dépendances MIT/Apache-2.0 et transitives) ;
- **advisories** : vulnérabilités connues (RustSec) et paquets *yanked*
  refusés ;
- **sources** : registre crates.io uniquement, aucun dépôt git inconnu.

## En local

```bash
cargo install cargo-deny
cargo deny check
```

## Intégration CI (proposition, à activer par accord explicite)

Le job suivant peut être ajouté à `.github/workflows/ci.yml` (aucune
modification de ce fichier sans accord de Raphaël) :

```yaml
  cargo-deny:
    name: cargo-deny (licences + advisories)
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v5
      - uses: EmbarkStudios/cargo-deny-action@v2
```

Note : l'action lit `deny.toml` à la racine par défaut. À activer de
préférence **après** l'ajout du `Cargo.lock` au dépôt (point 12), pour que
les advisories soient vérifiées sur les versions exactement verrouillées.
