# [J0] Protocole d'audit du dépôt — gouvernance des audits externes réguliers (2026-10-26)

Date : 2026-10-26. Jalon : transversal (gouvernance).

## Contexte

Raphaël souhaite faire **régulièrement** auditer le dépôt par d'autres IA (l'audit n°2 du 2026-10-25 a montré la valeur de l'exercice, mais aussi son bruit : signalements portant sur des choix délibérés couverts par des ADR, ou de la dette déjà inventoriée). Question posée : que préparer pour faciliter la re-vérification ?

## Décision

Création d'un **protocole d'audit public** : `docs/audit-protocole.md` (commits `6c942bc`, `d6eb7d2` correctifs de liens racine, `ee37fa6` formulation §4), lié depuis la section Documentation du README. Contenu :

1. **Cadrage** : audit ancré à un SHA ; documents de référence à lire avant de signaler (AGENTS, CONTRIBUTING, architecture, registre OF-*, ADR, état actuel) ; automatismes d'abord (fmt/clippy/tests/liens-md), l'audit porte sur ce que les outils ne voient pas.
2. **Conventions délibérées** : liste anti-faux-positifs (unités strictes, licences par zone, vendoring planegcs figé, expect admis en build.rs/tests/invariants CI, unsafe FFI seulement, fenêtre de convergence du Cargo.lock).
3. **Dette assumée** : pointeur vers la fiche état actuel — ne pas re-signaler sauf aggravation.
4. **Format de rapport** : signalements numérotés A<n>-<nn>, fichier:ligne, sévérité, classe (bug/violation/risque/suggestion), preuve reproductible, test de régression proposé ; sans ligne ni scénario = retourné « à préciser ».
5. **Traitement** : chaque signalement reçoit un verdict explicite (corrigé test-first / dette acceptée datée / refusé motivé) ; rapports archivés dans `docs/knowledge/archives/` avec SHA et verdicts, pour que les audits suivants **vérifient les corrections au lieu de tout re-signaler**.
6. **Modèle de consigne** prêt à copier pour l'auditeur IA (une phrase à compléter : SHA + périmètre).

## Leçons

- Le protocole lui-même a failli contenir deux liens relatifs cassés (AGENTS.md/CONTRIBUTING.md vus depuis `docs/`) — attrapés au relire avant activation du workflow liens-md : confirmation de la valeur du contrôle.
- Règle qui en découle pour les audits futurs : le rapport d'audit n°2 n'avait pas de SHA d'ancrage explicite ; désormais exigé.

## Liens

- Fiches touchées : `etat-actuel` (v9), la présente entrée d'archives (index v9), README (section Documentation).
- Docs au dépôt : `docs/audit-protocole.md` + README.
