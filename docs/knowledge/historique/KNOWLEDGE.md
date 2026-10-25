---
name: historique
description: "Versionnage de la Knowledge OmniFab : n° de version + changelog sur chaque fiche, journal append-only des évolutions par fonction (une ligne par événement, consultation par grep)."
---

# Historique & versionnage de la Knowledge

**Version :** v1 (2026-10-25)

Principe : **rien ne change sans laisser de trace**, à coût token minimal.

## 1. Versionnage des fiches (léger)

- Chaque fiche porte en tête : `**Version :** vX (AAAA-MM-JJ)`.
- Chaque fiche se termine par une section `## Historique` : une ligne datée par révision — `vX (AAAA-MM-JJ) : <nature du changement> (source : ADR-NNNN / discussion / commit)`.
- On modifie une fiche **en place** (pas de snapshot complet) : l'ancien texte est retrouvable via les ADR, archives de discussion et le dépôt Git qui citent la version concernée. Le changelog suffit comme historique.
- La version d'une fiche est incrémentée à **chaque** modification de contenu, même petite ; les corrections typographiques peuvent être regroupées.

## 2. Historique par fonction (journal append-only)

Chaque fonction (de la matrice ou du code) doit pouvoir retrouver son historique. Mécanisme : le fichier [journal-fonctions.md](journal-fonctions.md), **append-only**, une ligne par événement :

```
AAAA-MM-JJ | F:<id-fonction> | J<jalon> | <changement en une ligne> | source: <ADR-NNNN / commit / discussion>
```

- **ID de fonction** : slug stable `F:<domaine>:<nom-court>` (ex. `F:esquisse:contrainte-hor-ver`). Une fonction de la matrice garde le même ID pour toujours ; ne jamais renommer un ID, en créer un nouveau si besoin.
- Consultation sans consommer de tokens : **grep sur l'ID**, jamais lecture du fichier entier. Le journal peut grandir indéfiniment, seule la ligne recherchée est lue.
- Événement consigné : changement de statut (spécifié → implémenté → validé), révision de comportement, bug corrigé, API modifiée. Pas d'entrée pour une simple discussion sans effet.
- En fin de tâche, le protocole de capitalisation (skill omnifab) ajoute les lignes au journal pour chaque fonction touchée.

## 3. Décisions

Les ADR restent la source de versionnage des décisions (numérotation append-only, une révision = une nouvelle entrée D1/D2 dans l'ADR, jamais de réécriture). Le journal-fonctions et les changelogs de fiches **citent** toujours l'ADR ou la discussion concernée.
