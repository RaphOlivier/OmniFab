# [J0] Étude d'intégration Tangency Labs : B-light pour le noyau CAO, matrice v9, fiche veille-écosystème — 2026-11-03

**Jalon dominant : J0** (transversal — touche le jalon 3 par les règles de conception de forge-cad).

## Contexte et fil

Demande de Raphaël : étudier l'intégration du compte GitHub [tangency-labs](https://github.com/tangency-labs) (« CAO, FEM et bibliothèques, tout en Rust ») dans OmniFab — le moteur, les bibliothèques, ou plus. La demande rappelait la vision fondatrice : chaîne cohérente BIM → pièce → élément → stratégie de fraisage, FAO/contrôle machine, et base de données **paramétrique** (murs, quincaillerie, visserie, circuits électroniques).

## Analyse (canvas « etude-tangency-labs-omnifab », discussion hôte)

- **Inventaire** : solidcore (noyau B-rep pur Rust, port du SMLib de NVIDIA, Apache-2.0, sûr/déterministe/wasm, difftest C++ à 1e-9, mais API expérimentale et **aucun traducteur STEP**) ; solidcraft (CAO paramétrique complète, 13,3k ★, produit pas brique, 75/76 pièces à l'oracle FreeCAD) ; fieldcraft (FEM/CFD de bout en bout, pas de crates publiées) ; pcbcraft (EDA KiCad-compatible, GPL-3.0, 5 commits) ; corpus `models` CC0. Licences toutes compatibles GPL-3.0+.
- **Argumentaire central** : l'event sourcing d'OmniFab rend une future bascule de noyau peu coûteuse côté document (la géométrie ne vit jamais dans le journal — ADR-0011) ; le vrai risque est la fuite de concepts OCCT dans le feature tree du jalon 3, qui se joue **pendant** la phase A/B. Le multi-noyau prématuré coûterait cher (tests combinatoires, plus petit dénominateur commun, CI doublée, abstraction spéculative).
- **Lecture stratégique** : la famille Craft occupe le même espace que la suite ArtCraft (matrice §10.1) — mêmes principes MCP/CLI/JSON, local-first, Rust pur. Aucune CAM chez Tangency : confirmation du positionnement unique d'OmniFab (la seule chaîne complète jusqu'à la machine). Le BIM (IFC) reste un chantier OmniFab propre.

## Décisions (Raphaël, 2026-11-03)

1. **B-light** pour le noyau CAO (OCCT confirmé + règles de conception en phase A + trait extrait plus tard si critère rempli) — ADR-0035.
2. Golden tests avec le corpus CC0 Tangency + méthode oracle, adoptés au jalon 3.
3. Matrice v8 → **v9** (domaines de vision FEM/PCB : oui tous les deux ; mise à jour du dépôt immédiate).
4. Fiche **veille-ecosysteme** créée (critères de bascule solidcore, répertoire de veille).

## Effets

- **Dépôt** : matrice v9 poussée sur main (commit `646fdb9`) — §10.1 +4 lignes (SolidCraft, solidcore, FieldCraft, PCBCraft), §10.4 nouveau (domaines de vision), en-tête/bilan mis à jour (10 cycles, ~64 logiciels, 26 libres en §10.1). Push via `create_or_update_file` après deux 503 du serveur MCP sur `push_files` (leçon : repli opération par opération).
- **Knowledge** : ADR-0035 prep (`preparation/ADR-0035-solidcore-veille-regles-noyau.md`), fiche `veille-ecosysteme` v1, architecture v7 (entrée ADR-0035), index racine mis à jour.

## Points ouverts / dettes

- **Resynchroniser le canvas matrice en v9** dans sa discussion hôte (le dépôt fait foi ; même dette que le pattern v7→v8).
- **Prompt fondateur** : l'inscription Simulation/Électronique comme domaines de vision appelle une resynchronisation v4 → v5 (canvas + `docs/prompt-fondateur.md`) à traiter dans la discussion hôte du prompt — décision de Raphaël requise sur la formulation des 15 points.
- ADR-0035 à recopier au dépôt `docs/adr/` au premier lot du jalon 3 touchant forge-cad (même dette que ADR-0029..0034).
- Titre proposé pour la discussion suivante si découpage : *[J3] Phase B jalon 3 — règles de conception du module occt (ADR-0035)*.

## Suite de la discussion (2026-11-03) — arbitrage stratégique + outillage du processus

- **SolidCraft vs OmniFab** : tranché — **continuer OmniFab** ; l'overlap se limite au noyau CAO (CAM, pilotage, BIM, bibliothèque paramétrique non couverts chez Tangency) ; contribution amont **opportuniste uniquement** (bugs, corpus, docs), jamais un pivot. Consigné dans la fiche veille-ecosysteme v3 (« Position stratégique face à la famille Craft »).
- **Trois dépôts process évalués** (aucune brique produit, aucune dépendance Cargo) : **cloudflare/security-audit-skill** (MIT) **ADOPTÉ** — `docs/audit-protocole.md` amendé d'un §5 « Registre de couverture et réfutation » (commit `39043d5`) : coverage-ledger, chasse isolée + critique de couverture, réfutation par vérificateur frais, verdicts d'auditeur confirmé/à valider/réfuté distincts des verdicts du mainteneur, audits additifs ; l'audit n°5 utilisera la méthode. **obra/superpowers** (MIT) **picoré** : 2 concepts → fiche economie-tokens §7 (plan « lisible par un junior enthousiaste », sous-agents par lot) ; installation en bloc rejetée (~70 % de recouvrement avec le protocole). **pbakaus/impeccable** (Apache-2.0) **écarté** : cible web DOM/CSS vs egui desktop ; note jalon 9 (idée transposable : règles déterministes de qualité en CI).
- Leçons de méthode de la session : repli `create_or_update_file` quand `push_files` renvoie 503 ; rapatriement unique + remplacements programmatiques vérifiés par ancres pour les gros fichiers du dépôt (matrice 52 Ko → jamais passée intégralement en contexte, protocole anti-saturation respecté).
