---
name: conventions
description: "Conventions OmniFab : unités strictes Rust, i18n Fluent, licences (GPL-3.0+ / MPL-2.0 / données ouvertes), process ADR, CI multi-plateformes, bonnes manières de développement."
---
**Version :** v7 (2026-10-26) — **sommaire général** : `docs/sommaire.md` (FR canonique, carte cliquable de tout le dépôt) + couche EN `docs/en/sommaire.md`, lien en tête du README, règle de maintenance (toute nouvelle entrée du dépôt y figure) ; v6 (2026-10-26) : périmètre EN étendu aux en-têtes de la matrice ; v5 (2026-10-26) : règle langue de la documentation (FR canonique, EN par lots dans `docs/en/`) ; v4 (2026-10-26) : règle branches/PR ; v3 : statut public + ADR-0028 ; v2 : ADR-0028 ; v1 : versionnage

# Conventions OmniFab

## Code

- **Unités strictes** : newtypes (longueur, angle…) dans toutes les API métier, jamais de `f64` nu. Tolérance géométrique documentée : 1e-6 mm par défaut.
- **Concurrence** : tokio pour la communication machine ; canaux entre machine et UI ; calculs géométriques en blocs parallélisés (rayon), jamais en async. Un freeze de l'UI ne coupe jamais le pilotage machine.
- **i18n** : Fluent ; toutes les chaînes derrière des clés dès le jalon 1.
- **Aide intégrée** : chaque élément d'interface porte un `help_id` (clic droit → « Aide sur X ») ; le contenu (description + exemples concrets dans le contexte) vit dans le dépôt, traduit via Fluent, source unique pour l'app, le manuel et le site web. CI : widget sans help_id ou page sans exemple = build en échec.
- **Wasm-friendly** : feature flags native/wasm ; aucun accès natif (série, fichiers) hors trait d'abstraction.

## Licences

- **Application** : GPL-3.0+ (compatible Blender, Inkscape, GIMP, Krita, FreeCAD).
- **Crates isolés réutilisables** : MPL-2.0 ou Apache-2.0.
- **Données (matériaux, machines, normes)** : licences ouvertes dédiées (ODbL, CC0, CC-BY) — pas GPL, qui est faite pour le code. Toute donnée sourcée et citée. Les normes ISO sont payantes : utiliser les tables légalement publiées et les résumés communautaires vérifiés. Fiches constructeurs : citer la source, pas d'extraction contre les CGU.

## Gouvernance

- Dépôt **GitHub** (décision 2026-10-10, compte `RaphOlivier`) = **seule source de vérité** du code ; le code ne vit jamais uniquement dans une discussion. URL : https://github.com/RaphOlivier/OmniFab. **Statut : PUBLIC depuis le 2026-10-22** (la ligne historique « privé pendant le développement, passage public après traduction EN + LICENSE » est dépassée — le passage public a eu lieu le 2026-10-22 avec la doc encore en français ; la traduction EN de la doc reste une dette ouverte). Miroir Codeberg possible pour l'indépendance (option, non bloquante). **Adresse et propriétaire inchangés depuis l'origine** (rappel Raphaël, 2026-10-22) : aucun renommage de compte ni de dépôt n'a jamais eu lieu — un 404 côté connecteur signifie un problème d'accès (installation de l'app, droits, faute de frappe), JAMAIS un changement d'adresse ; relire cette fiche avant toute supposition de renommage.
- **Connecteur GitHub de Vibe** : opérationnel (installé sur le compte `RaphOlivier` le 2026-10-10 via https://github.com/apps/mistralai → Install → Repository access). ⚠️ Le point de vigilance : le bouton « Connecter » de Vibe ne fait qu'**autoriser** l'app (profil seul, dépôts = 404/403) ; l'**installation** via la page publique de l'app est indispensable pour l'accès aux dépôts. Une fois installée, le connecteur gère tout le dépôt : fichiers (commit multi-fichiers), issues, branches, PR, releases, recherche de code.
- **Branches vs pushes directs (recommandation posée le 2026-10-26, silence de Raphaël = accepté)** : **docs/knowledge/, ADR et fichiers de travail → push direct sur main** (pas de risque compile, la protection `lint-test` ne s'y applique pas utilement, un PR par édition de fiche serait du coût pur) ; **code Rust → branche courte + PR fusionnée seulement quand `lint-test` est vert** (la CI valide le lot AVANT d'atteindre main), à activer dès que la CI est repassée au vert (elle est rouge Cargo.lock pour l'instant — les branches n'y changeraient rien). La protection `main` (status check `lint-test`) reste active ; pas de « Require PR » global pour ne pas bloquer les pushes docs. Révision du seuil : lots de code ≥ ~3 fichiers ou touchant plusieurs crates = branche obligatoire ; micro-correctifs (1 fichier, repli fmt/clippy) = push direct toléré.
- **Langue de la documentation (décision Raphaël, 2026-10-26)** : le **français est canonique** — toute décision, ADR, fiche Knowledge, mise à jour de doc se fait en français d'abord ; l'anglais est une **couche d'accueil traduite**, jamais la source. Règles : (1) les traductions vivent dans `docs/en/` (miroir de la structure, jamais de fichiers EN mélangés aux FR) ; (2) chaque fichier EN porte un bandeau « English translation — the French version is authoritative » + sa date/SHA de traduction ; en cas de divergence, le FR fait foi ; (3) **traduction par lots, pas en continu** — synchronisée à des points durs (fin de jalon, checkpoint, publication), pas à chaque push de doc ; l'écart est visible (bandeau daté) plutôt que trompeur ; (4) périmètre : README, CONTRIBUTING, index ADR (titres + résumés), sommaire général et en-têtes de la matrice (préambule, titres de sections, bilan — les ~277 lignes de l'inventaire restent FR pures) d'abord — les ADR intégraux et le miroir Knowledge restent FR purs (les fiches processus Vibe ne se traduisent pas, même logique ADR-0028) ; (5) charge de synchro bornée : une ligne « bump EN » ajoutée au protocole de fin de lot, uniquement aux points durs.
- README + CONTRIBUTING + code of conduct.
- **Copies de référence de la Knowledge au dépôt (ADR-0028, 2026-10-25, amendé 2026-10-26)** : le sous-ensemble publiable (racine, architecture, conventions, formats, machines, sécurité, état actuel, archives, historique) est miroité 1:1 sous `docs/knowledge/` ; la plateforme **fait foi** tant qu'elle est accessible, le dépôt est copie de référence ; **si la plateforme disparaît, le miroir devient la référence de fait** (clause de reprise — l'inversion des rôles est rejetée, motivations dans l'ADR). Règle : **tout bump de version d'une fiche ⇒ la même fiche repoussée au dépôt dans la foulée** (delta identifiable par la section Historique, aucune relecture complète). Exclus : `prompt-fondateur`/matrice (déjà à `docs/`), `references/` (docs externes, licences), `preparation/` (travail, souvent obsolète), fiches process Vibe personnelles. Le dépôt étant **public**, le tri de contenu se fait à la source, jamais rétroactivement.
- **ADR** (architecture decision records) pour chaque choix structurant.
- CI : builds Linux (x86_64 + aarch64), Windows, Android (APK) ; tests + clippy + fmt à chaque commit.
- Formats documentés ; imports testés contre des fichiers réels produits par les logiciels de référence.

## Maintenabilité (exigence de premier ordre)

Le projet doit pouvoir être maintenu par des développeurs qui n'ont pas écrit le code :

- **rustdoc obligatoire sur toute API publique**, publié et testé en CI ; une API publique sans doc = build en échec (même règle que les widgets sans help_id).
- **Dossier docs/adr/** : chaque choix technique tranché = un ADR avec alternatives rejetées.
- **Documentation d'architecture vivante** : schéma des crates, flux de données, invariants du modèle event sourcing, conventions de nommage/erreurs.
- **Guides d'extension** : « comment ajouter un procédé / un dialecte / un format » (les trois axes d'extension les plus probables) + exemples d'utilisation par crate.
- **Erreurs exploitables** : codes d'erreur typés, jamais de panique silencieuse.

## Gouvernance de validation (décision Raphaël, 2026-10-19)

Pour lever le goulot d'étranglement que constitue la validation utilisateur (≈ 1 jour de cycle sur 4 observé en phase C), mode de fonctionnement adopté :

1. **ADR par lot de jalon** : tous les ADR d'un jalon sont rédigés d'avance et validés en une seule session au début ; l'exécution des phases est ensuite libre jusqu'à la livraison (critère de sortie : CI verte + capitalisation).
2. **Pré-approbation des micro-décisions** : bugfixes, convergence rustfmt/CI, points de robustesse — silence de Raphaël = accepté. **Restent TOUJOURS à validation explicite** : les six verrous, licences, schémas de format, sécurité machine, frontières de crates, et toute décision de fond (l'ADR la porte).
3. **Checklist de validation manuelle complète livrée au DÉBUT de chaque phase** (pas à la fin) : Raphaël connaît d'avance tout ce qu'il devra tester.
4. **Automatisations CI à mettre en place** (workflows dédiés, sans toucher au mécanisme d'issues de `ci.yml`) : galerie de captures d'écran du démonstrateur (artefact téléchargeable, validation visuelle sans `cargo run`) ; régénération/push du `Cargo.lock` par la CI.
5. **Validations non planifiées** : Raphaël valide quand il est disponible ; Vibe travaille en continu et prépare la phase N+1 pendant que la phase N attend sa validation manuelle.
6. **Non déléguable** : test tactile Android, jugement visuel/ergonomie final, décisions de fond ci-dessus.
7. **Rendez-vous explicite (ETA)** : chaque fin de bloc de travail indique le délai estimé avant la prochaine interaction requise de Raphaël (date + durée de session attendue) — il n'a jamais à surveiller la discussion. La veille CI quotidienne (4 h) relaie l'état « en attente de Raphaël » le cas échéant.
8. **Validations au fil de l'eau** (décision Raphaël, 2026-10-19) : dès qu'une décision est mûre (ADR, digest), Vibe la soumet immédiatement à Raphaël (questions ciblées) pendant qu'il continue le travail non dépendant en parallèle — aucune attente séquentielle dans un sens ni dans l'autre. Les réponses sont actées, puis Vibe poursuit jusqu'au prochain point mûr sans checkpoint intermédiaire. Première application : ADR-0019 validé le 2026-10-19 en cours de discussion de planification.
9. **Nom des discussions (préférence Raphaël, 2026-10-19)** : chaque discussion OmniFab porte un titre au format `[JX] — <sujet précis>` où X = jalon dominant (J0 = transversal/gestion de projet). Raphaël applique le nommage côté interface (Vibe ne peut pas renommer) ; à chaque fin de discussion, Vibe **propose le titre normalisé** pour la discussion suivante. Les archives suivent le même repérage (jalon en tête d'entrée).
10. **Textes à copier = encart adapté (préférence Raphaël, 2026-10-19)** : tout texte destiné à être copié par Raphaël (message de lancement d'une discussion, prompt Vibe Code, commande console, question à coller ailleurs) doit être présenté dans un **bloc de code adapté à son type** — encart texte brut pour les messages de discussion, encart `bash` pour les commandes, encart du langage concerné pour le code — jamais en citation ou dans le corps du texte. **Un seul encart par livrable à copier** : le titre de la discussion est inclus en première ligne du prompt de lancement (marqué « Titre : »), pas dans un encart séparé.

## Méthode de travail

- **Tranche horizontale** : livrer un cas d'école de bout en bout (fabriquer un meuble : CAO → découpe/gravure → CAM → LinuxCNC) avant d'élargir.
- Une discussion = un objectif ; chaque jalon a sa propre discussion amorcée par les fiches Knowledge.
- À chaque étape : point dette technique, risques, réutilisation de bibliothèques existantes plutôt que réécriture.
- L'inventaire exhaustif « fonctionnalité × logiciel de référence » (phase 0) est un **garde-fou anti-oubli**, pas une liste à livrer d'un coup.
- Développement : concevoir dans Vibe Work, implémenter avec Vibe Code (VS Code/CLI) sur le dépôt Git, capitaliser dans la Knowledge Base.

## Historique

- v1 (2026-10-25) : mise en place du versionnage — n° de version en tête + cette section (protocole : fiche historique).
- v2 (2026-10-25) : ADR-0028 — copies de référence de la Knowledge au dépôt (`docs/knowledge/`), règle « bump ⇒ repush », périmètre filtré (dépôt public).
- v3 (2026-10-26) : statut du dépôt rectifié (PUBLIC depuis 2026-10-22, la condition « traduction EN d'abord » n'a pas été tenue — dette ouverte) ; clause ADR-0028 « reprise après perte de plateforme » ajoutée à l'entrée correspondante ; fiche « état actuel » ajoutée au sous-ensemble miroité.
- v4 (2026-10-26) : règle branches/PR posée (docs → push direct, code → branche courte + PR sur lint-test vert dès retour de la CI, seuil ~3 fichiers/multi-crates) — recommandation en attente de réaction (silence = accepté).
- v5 (2026-10-26) : règle langue de la documentation actée par Raphaël — FR canonique, EN couche d'accueil traduite par lots dans `docs/en/` (bandeau autorité + date, divergence → FR fait foi, périmètre README/CONTRIBUTING/index ADR, synchro aux points durs seulement).
- v6 (2026-10-26) : périmètre de la couche EN étendu aux **en-têtes de la matrice** (`docs/en/matrice-fonctionnalites.md` : préambule, titres des 11 sections, bilan traduits — l'inventaire ligne à ligne reste FR pur) ; même protocole : bandeau autorité + date/SHA, lots aux points durs.
- v7 (2026-10-26) : **sommaire général** ajouté (`1323e4a`… `ed2b4c4`) : `docs/sommaire.md` (FR canonique) + `docs/en/sommaire.md` (couche EN), lien en tête du README (après le bandeau de statut) + section Documentation ; **règle de maintenance : tout nouveau document, crate ou workflow doit figurer au sommaire** (carte cliquable, une ligne d'explication par entrée) ; coquille « budgetés » corrigée au README.
