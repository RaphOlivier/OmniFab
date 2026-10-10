# ADR-0003 : Stratégie d'erreurs typées

- **Statut : accepté**
- **Date : 2026-10-10**
- **Décideurs : Raphaël Olivier**

## Contexte

La maintenabilité de premier ordre exige des « messages d'erreur exploitables, codes d'erreur typés, jamais de panique silencieuse ». Il faut trancher le mécanisme concret : types d'erreur, codes stables, et articulation avec l'aide intégrée (help_id).

## Décision

Chaque crate définit une énumération d'erreur Rust par domaine (via **thiserror**), chaque variante porte un **code stable documenté** au format `OF-<DOMAINE>-<NNNN>` (ex. `OF-VAL-0001` : cycle dans le graphe de liaisons ; `OF-CAM-0142` : outil hors zone de travail). Les codes sont **attribués par le registre central** `docs/error-codes.md` (une ligne par code : signification, cause probable, action corrective, page d'aide liée) — la CI refuse un code non documenté ou dupliqué (test génératif qui parcourt les enums). Le registre est la **source unique** alimentant : le message d'exécution, la page d'aide intégrée (via help_id), le manuel et le site web. Les erreurs de bordure (I/O, dépendances) sont enveloppées avec contexte en couche la plus externe ; `anyhow` est autorisé **uniquement dans les binaires d'exemple et les tests**, jamais dans les crates du noyau. La panique est réservée aux violations d'invariants internes documentées comme « bugs du programme » (et aboutit à un rapport structuré).

## Alternatives rejetées

- **anyhow partout** : erreurs opaques, non testables, non documentables — rejette l'exigence du prompt fondateur.
- **Codes numériques bruts (errno maison)** : pas de typage compile-time, collisions faciles.
- **Chaînes d'erreur localisées directement** : l'i18n (Fluent) exige des codes stables indépendants de la langue ; la traduction se fait à l'affichage, clé = code.

## Conséquences

- **Positives** : chaque panne est explicable à l'utilisateur via l'aide intégrée ; testable (matching sur les codes) ; registre documenté en amont des développeurs externes.
- **Négatives / coûts acceptés** : discipline de tenue du registre (outillée par la CI) ; verbosité des enums.
- **Décisions futures suspendues** : format du rapport de panique structuré ; export des erreurs vers la télémétrie (optionnelle, locale).
