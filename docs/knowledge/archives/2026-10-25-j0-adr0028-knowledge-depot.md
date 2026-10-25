# [J0] ADR-0028 : copies de référence de la Knowledge au dépôt (2026-10-25)

## Question posée par Raphaël

La Knowledge OmniFab vit uniquement sur la plateforme Mistral. Bonne idée d'en avoir des copies synchronisées sur GitHub (compréhension par d'autres IA/humains), ou trop lourd en tokens ?

## Analyse

- Volume mesuré : ~800 Ko / 66 fichiers, très inégalement publiables (fiches de décisions vs copies de docs externes FreeCAD vs notes de process personnelles).
- Le coût tokens n'est PAS le vrai sujet avec un bon protocole : initial ~300 Ko une fois ; récurrent limité aux fichiers touchés **si** le versionnage des fiches sert de signal de delta (bump de version = fichier touché, jamais de relecture complète). Ce qui serait lourd : relire/diffuser tout à chaque fois.
- Vrais risques : divergence des copies (une copie en retard = source de vérité fausse) ; contenu sensible dans un dépôt devenu **public** (notes franches des archives, habitudes de travail personnelles) ; gouvernance (décision de fond → ADR).
- Découvert en route : le dépôt est passé public — le tri à la source est d'autant plus important.
- Verrou technique découvert : le sandbox Vibe peut lire les fichiers locaux via le connecteur `file_system` → la synchro se fait à coût de tokens quasi nul (lecture locale → push direct, roundtrip md5 pour l'intégrité).

## Décision (choix de Raphaël : sous-ensemble filtré + ADR + première synchro immédiate)

ADR-0028 accepté : miroir `docs/knowledge/` 1:1 du sous-ensemble publiable ; la plateforme fait foi ; règle « bump de fiche ⇒ repush de la même fiche » ; exclusions documentées (references, preparation, fiches process Vibe, doublons docs/ déjà existants). Chaîne : `314c7b3` (ADR) → `37a884e` + `035edbe` (20 fichiers, ~286 Ko, roundtrip md5 vérifié dont architecture).

## Première application de la règle

Conventions fiche bumpée v2 (présente décision) → repoussée le jour même, avec l'index des archives.
