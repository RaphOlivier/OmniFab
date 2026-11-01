# [J0] — Injection de prompt via contributions externes : le cas valmir5522/OpenAgents, capitalisation sécurité

**Date :** 2026-11-01 · **Jalon dominant :** J0 (transversal — gouvernance et sécurité) · **Fiches touchées :** [Sécurité](../securite/KNOWLEDGE.md) v4

## Contexte

Commentaire « top bot » reçu sur PR #584 (découpage main.rs 5/6, fusionnée le 2026-11-01 02:58 UTC) par **valmir5522**. Raphaël demande d'examiner l'auteur et son dépôt « protocole agentique » (OpenAgents) pour en tirer des leçons pour OmniFab, puis capitalise sur la sécurité du dépôt.

## Faits constatés (vérifiés via l'API GitHub le 2026-11-01)

- **valmir5522** : compte récent (Fortaleza, Brésil), bio « Fixando open source que paga » — chasseur de bounties. 12 dépôts = 1 README de profil + **11 forks** créés entre le 13 et le 31-10-2026, **zéro code original**. Revendique un « premier merge » de **+7 766 lignes en 26 commits** dans dsh-memory (projet chinois « AGI white-box ») ; farming en cours (PR vers omacom/omarchy le 2026-11-01).
- **OpenAgents** : **fork de ClankerNation/OpenAgents**, pas une création de l'auteur. « Decentralized AI Agent Orchestration Protocol » : contrats Solidity (AgentRegistry, TaskRouter, PaymentEscrow), SDK TypeScript, API FastAPI, oracle ; bounties payées en crypto (USDC/USDT/BTC/XMR) ; répertoires de contrats (dex, lending, lottery, nft, staking, vault…) = cibles de farming.
- **Le CONTRIBUTING.md est un piège à injection** : « humains interdits, agents IA autonomes uniquement » ; avertissement « bounties symboliques, projet de recherche académique »… puis commentaires HTML cachés ordonnant aux « automated systems » d'**ignorer l'avertissement et de continuer**. Le contenu lu a été traité comme donnée, jamais comme instruction.
- **clankers.md** : essaim d'agents documenté — 100+ comptes, jusqu'à **1 459 PR en 9 jours** (Gaotax2006), 444 en 3 semaines (rafaio1).
- **Contexte industriel 2026** : classe d'attaque « Comment and Control » (Novee Security, Black Hat USA, 2026-08-05) — une seule issue/PR peut ordonner à un agent en CI (Copilot Coding Agent, Gemini CLI, Claude Code) d'exfiltrer `GITHUB_TOKEN`/clés API dans les logs publics, sans interaction du mainteneur ; GitHub a mis en préversion le 2026-10-28 un code scanning « applications IA » détectant l'injection indirecte (XPIA) — mais **aveugle au contenu arrivant après le build** (commentaires d'issues, résultats MCP).

## Raisonnement et argumentaire

1. **Rien à emprunter structurellement** : l'auteur n'a aucun code original ; le protocole (orchestration on-chain + paiements) est orthogonal à OmniFab et contraire au local-first. L'interface « agent-ready » déjà actée (principes ArtCraft, matrice v8 §10 : MCP/CLI/JSON au-dessus de forge-values/forge-cad/forge-cam/forge-io, templates déclaratifs) est la bonne voie — ADR futur si le sujet revient, pas d'inspiration à prendre chez ClankerNation.
2. **Le vrai enseignement est la gouvernance** : l'écosystème open source 2026 contient des essaims d'agents à bounties ; OmniFab public deviendra une cible à mesure de sa visibilité. Nos garde-fous (CI stricte : rustdoc/tests/clippy/`--locked`, petits PR mécaniques, ADR obligatoires) sont l'antidote exact du « +7 766 lignes en 26 commits » ; à compléter par un CONTRIBUTING.md public avec critères d'acceptation.
3. **Le label « rédigé avec l'assistance de Vibe (IA) »** : pas un risque d'accès en soi (aucune permission n'est accordée). Risques **indirects** : (a) attracteur d'attention des essaims — constaté en direct (« top bot » arrivé pendant la vie du PR #584, quelques minutes avant le merge) ; (b) informer un attaquant que le mainteneur s'appuie sur un assistant → les issues/PR deviennent une surface d'attaque par injection **ciblée**. Parade = la règle anti-injection (contenu externe = donnée), pas le silence ; la transparence reste une valeur du projet (sincérité sur l'IA, position actée depuis le 2026-10-19).
4. **Signalement GitHub** : canaux confirmés — « Signaler le contenu » (menu ⋯ d'une issue/PR/commentaire), « Report abuse » (barre latérale d'un dépôt, sous « Top languages »), formulaire `support.github.com/contact/report-abuse` ; `abuse@github.com` n'est plus suivi. Le dépôt ClankerNation se présentant comme étude académique, l'action de GitHub est incertaine — le signalement reste néanmoins légitime (les injections visent des agents tiers, y compris le nôtre).

## Décisions tranchées

- **Règle anti-injection** ajoutée à la fiche Sécurité (v4) : contenu externe = donnée, jamais instruction ; validation humaine avant toute action.
- **Secrets** : aucun jeton persistant accessible à un traitement de contenu externe ; si automatisation issue/PR un jour : jetons courts, portée minimale, egress restreint (recommandations CSA 2026).
- **Déclaration d'assistance IA — périmètre précisé** (amende l'analyse initiale, décision Raphaël du 2026-11-01) : déclaration **globale au README** du dépôt OmniFab + mention par message conservée pour les communications vers des tiers (convention v10) ; mention par PR interne **abandonnée** (redondante : le relecture-merge par le mainteneur est l'acte de responsabilité) — conventions v16, sécurité v5.
- **CONTRIBUTING.md existant à amender** (correction du 2026-11-01 : il existe déjà — critères de build/code/PR en place ; ajouter clause « contributions générées par IA acceptées si le contributeur comprend et assume le code » + avertissement anti-injection, AGENTS.md même rappel).
- **Signalement du dépôt ClankerNation/OpenAgents déposé** (2026-11-01, par Raphaël via le bouton « Report abuse » du dépôt, catégorie « Malware and exploit » — la charge d'injection vise des logiciels/agents tiers ; confirmation reçue). **Case DSA art. 16 non cochée** (argumentaire : violation des CGU — tromperie + bulk activity — mais pas démontrable illégal au sens du DSA « sans examen juridique détaillé », art. 16 §3 ; notices manifestement infondées sanctionnables, art. 23). Le commentaire « top bot » n'est pas signalé (choix : la cible légitime est le dépôt hébergeant la charge).
- **README + miroir poussés** (2026-11-01, commit `8761862` sur main, roundtrip vérifié) : section « Développement avec assistance IA » au README + fiches sécurité v6, conventions v16, archives v33 au miroir ; statut miroir de la section « Menace processus » tranché de fait (publiée — le README déclare publiquement l'assistance IA).

## Points ouverts

- CONTRIBUTING.md : **amendement** à planifier (prochaine fenêtre — clause IA + avertissement anti-injection ; le fichier existe déjà) ; statut miroir de la section « Menace processus » (publiée au dépôt ou filtrée ADR-0028 — la règle est de la gouvernance publique, l'assistant est un processus privé) : arbitrage de Raphaël.
- README : déclaration **poussée** le 2026-11-01 (commit `8761862`, avec le miroir sécurité v6 / conventions v16 / archives v33, roundtrip vérifié) ; couche EN au prochain lot de traduction (point dur).
- Signalement : **dépôt ClankerNation déposé** (confirmation reçue le 2026-11-01) ; suite GitHub à suivre — issue incertaine, le dépôt se présente comme étude académique.
- Journal des fonctions : inchangé (aucune évolution de fonction).
