# [J0] ADR-0040 gouvernance de l'ouverture multi-contributeurs : niveau 2 ouvert, examen du plan multi-IA, incident outillage du wrap

**Date :** 2026-11-07 · **Décision :** ADR-0040 « Gouvernance de l'ouverture multi-contributeurs » (poussé `a40bf4a`, propre `f7d490dd`) · **Fiches synchronisées :** preparation/ouverture-multi-contributeurs v2, conventions v19, economie-tokens v6 (§6.6) · **Suite :** ordre 4 ADR-0038 (A4-20, A4-19, A5-05).

## Contexte : la phase déclencheur est atteinte

La fiche preparation/ouverture-multi-contributeurs (v1, 2026-11-05) conditionnait l'ouverture du niveau 2 à un moment opportun précis : backlog audit n°5 épuisé + capitalisation publique soldée + 0 issue « CI : » ouverte + CI verte intégrale. La clôture de l'ordre 3 ADR-0037 (entrée du 2026-11-07) a fait tomber ce déclencheur : A5-04/A4-16 (ordre 1), A5-01/A4-17 (ordre 2), A5-02/A4-18/A5-03 (ordre 3) soldés, arbitre main vert intégral (CI #658). Raphaël a collé le plan multi-IA de Claude (2026-11-05, « Document de discussion rédigé par Claude, à étudier avec Mistral ») et demandé l'examen puis l'ADR.

## Examen du plan multi-IA de Claude (verdict : aucune incompatibilité)

Examen contre les six verrous, le gel post-jalon 4 et le protocole test-first : **aucune incompatibilité**. Plusieurs points sont déjà en place, plus strictement que proposé : AGENTS.md (FR pur, décision 2026-11-01), CONTRIBUTING.md public avec clause IA + avertissement anti-injection (2026-11-01), licence GPL-3.0+ tranchée depuis l'origine (le plan proposait de la « trancher tôt »), CI bloquante avec clippy `-D warnings`, rustdoc obligatoire et mécanisme d'issues « CI : », cargo-deny actif (A4-11/12), protection main active (vérifiée UI 2026-11-01), la série d'audits externes ancrés à un SHA = déjà la relecture croisée entre IA (le plan la suggérait comme innovation).

**Écartés dans l'ADR, avec motivations reprises dans l'ADR :**
- **Require PR global** : bloquerait les pushes docs du miroir Knowledge par le connecteur (coût pur, réfuté empiriquement `d1881f2`/`0562eff`).
- **CODEOWNERS-propriété par module** : un seul mainteneur arbitre ; la relecture croisée passe par les audits externes, pas par une propriété par dossier.
- **docs/JOURNAL.md** : le journal d'événements event sourcing + les fiches Knowledge font foi ; un journal parallèle deviendrait une seconde vérité (anti-pattern déjà évité pour la doc).
- **Rôles docs/agents/*.md** : les fiches Knowledge privées jouent ce rôle ; une seconde surface d'instructions contredirait l'hygiène anti-injection (même logique que « AGENTS.md FR pur »).
- **Automatisations IA déclenchées par GitHub** (axes F/G du plan : agent sur commentaire/étiquette, revue automatique, orchestrateur) : toute activation future = **ADR dédié** ; le mode semi-automatique reste la règle (Raphaël colle les instructions, la CI arbitre).

Le plan est archivé verbatim dans `preparation/plan-multi-ia-claude.md` (même modèle que la consigne de l'audit n°5) pour que l'ADR garde sa source.

## Les décisions actées (ADR-0040, validé par Raphaël)

- **Trois niveaux** : (1) audits externes ouverts (protocole public, zéro écriture sur le code) ; (2) pistes isolées **ouvert par cet ADR** ; (3) cœur forge-cad/OCCT **fermé** jusqu'au lot 3 branché + meuble du cas d'école bout en bout.
- **Arbitre unique** : Raphaël pour les fusions et verdicts — exécution délégable, décision non.
- **Branche + PR obligatoires** pour tout contributeur ; la CI arbitre technique : test-first, un commit par point, rouges prouvés, issues « CI : ».
- **Cargo.lock** : écriture autorisée aux contributeurs du niveau 2 (au minimum autres IA ; humains même règle), uniquement dans une PR, sous garde-fous CI `--locked` + cargo-deny + revue de l'arbitre. Le `workflow_dispatch` de régénération du lock = **ADR préalable obligatoire** (commit machine sur la supply chain).
- **Linux fortement préféré** (Windows/macOS documentés mais plus lourds).
- **Machine réelle sans validation humaine interdite**, noir sur blanc.

## Niveau 2 ouvert : les cinq pistes (issues #637-#641)

- **#637 devcontainer Codespaces** (bon premier PR d'un contributeur : prérequis build, premier build OCCT lourd puis prébuild en cache).
- **#638 traduction EN** : rattrapage de l'index ADR EN en retard (ADR-0035 à 0040) puis couche d'accueil — l'écart EN est documenté dans les bandeaux.
- **#639 DXF** : $INSUNITS et INSERT/blocs (restes du backlog audit n°2).
- **#640 corpus golden tests CC0** (tangency-labs/models, fiche veille-ecosysteme, méthode oracle).
- **#641 matrice** : veille et compléments sourcés.

## Incident outillage : le wrap des canaux de lecture (leçons → economie-tokens §6.6)

Deux canaux de lecture (connecteur `get_file_contents` ET `open_url`/raw) insèrent des retours à la ligne parasites dans les lignes longues ; le raw peut servir une version périmée de plusieurs minutes. Le jour même, cela a causé deux corruptions brièvement gravées :

1. **ADR-0040, liens relatifs** : le premier push (`a40bf4a`) portait des liens `../` depuis `docs/adr/` → runs liens-md #116/#117 ROUGES (les liens vers la racine du dépôt sont `../../`). Le correctif `1f336140` a été composé depuis une copie fetchée re-wrappée → 4 coupes mid-mot gravées (détecté par stats +8/-4). Rétabli depuis le texte source : `f7d490dd` (52 lignes). Runs liens-md #118/#119/#120 VERTS.
2. **Sommaire, rattrapage du tableau ADR** : la dette « tableau ADR s'arrête à 0028 » a été rattrapée (`3d15c8d9`, six lignes ADR-0035→0040), mais composé depuis une copie re-wrappée → 8 lignes fragmentées en 17 (stats +23/-8). Réparé depuis l'état exact du parent + les 6 lignes : `640af37c` (185 lignes, +8/-17).

Leçons actées (détail §6.6 de economie-tokens v6) : composer tout push depuis le texte source, jamais depuis une copie fetchée d'un fichier à lignes longues ; les fichiers à lignes courtes (< ~250 caractères) sont servis fidèlement ; **vérité après chaque push docs = stats de diff git** (+N/-0 attendu pour une insertion pure) ; liens-md ne crée pas d'issue → relever le run lui-même (fetch frais en variant un paramètre d'URL).

## Artefacts de wrap préexistants du sommaire : nettoyage acté

Des artefacts de wrap plus anciens (« chaqu/e fiche », noms d'entrées d'archives coupés, « la raisonnement ») préexistaient au 2026-11-07 et avaient été conservés volontairelement au commit `640af37c` (historiques, hors périmètre de la réparation). Décision de Raphaël à la capitalisation : **nettoyer**, en commit séparé, jointures vérifiables par stats de diff.

## Restes manuels (Raphaël)

- Supprimer la branche `fix/a5-02-a4-18-cercle-explicite` (fusionnée en `66c2eb8`) : https://github.com/RaphOlivier/OmniFab/branches
- Branches périmées repérées, à sa discrétion : `feat/phase-b-lot1`, `feat/phase-b-lot2`, `probe-tmp-a6`.
