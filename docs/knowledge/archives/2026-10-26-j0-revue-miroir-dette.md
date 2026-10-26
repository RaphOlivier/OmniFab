# [J0] Revue externe du miroir Knowledge : statuts datés, clause de reprise ADR-0028, gel post-jalon 4

Date : 2026-10-26. Jalon dominant : J0 (gouvernance).

## Contexte

Après la publication du miroir `docs/knowledge/` (ADR-0028), Raphaël a fait relire le
dossier par un autre assistant (Claude). Sept points remontés ; chaque affirmation
factuelle a été re-vérifiée avant réponse (recherche `ci.yml`, ADR-0016 A4,
`Cargo.toml` des crates, dates de passage public).

## Constats vérifiés et traités

- **cargo-deny « en place » alors que le job CI est en commentaire** : VRAI —
  contradiction interne de la fiche sécurité → **fiche sécurité v2** : chaque règle
  porte désormais statut (en place / rédigé / prévu) et date ; cargo-deny reclassé
  « rédigé, inactif — à activer jalon 1 ».
- **Plafond blob « 1 GiB » sans le cumulé 2 GiB (ADR-0016 A4)** : VRAI → corrigé dans
  la fiche sécurité.
- **tokio (et rayon) dans les verrous mais dans aucun `Cargo.toml`** : VRAI — décisions
  de conception cibles des jalons 4/5, pas des dépendances actuelles → annotation de
  statut dans la fiche architecture (vérifié 2026-10-26).
- **Contradiction « privé »/« public » dans la fiche conventions** : VRAI — ligne
  historique « privé pendant le développement » vs « le dépôt étant public » →
  **fiche conventions v3** : statut PUBLIC depuis 2026-10-22 rectifié, la condition
  initiale (traduction EN d'abord) notée comme dette ouverte.
- **`architecture/KNOWLEDGE.md` ≈ 149 Ko, journal de validations plus que fiche** :
  VRAI — reconnu dette documentaire (point 4 de la fiche état actuel) ; le découpage
  (noyau stable ~10-15 Ko + historique vers archives) est **différé à un lot
  dédié** — il exige une relecture complète du fichier et ne devait pas bloquer les
  correctifs d'affirmations fausses.
- **Liens cassés du miroir** (racine pointe des fiches non publiées) : VRAI →
  annotations « *(non publié au miroir — ADR-0028)* » à la source (racine), lisibles
  des deux côtés ; le contrôle de liens Markdown en CI reste en dette (attend CI
  verte).
- **Fiche « état actuel » manquante** : VRAI → créée (`etat-actuel/KNOWLEDGE.md` v1),
  ajoutée au sous-ensemble miroité et à l'index racine ; protocole : relevé d'une
  page mis à jour à chaque fin de lot.

## Nuances apportées à la critique

- **« Deux sources de vérité »** : la critique confond le **code** (dépôt fait foi,
  sans ambiguïté) et la **Knowledge** (plateforme fait foi, dépôt = copie de
  référence) ; le risque de perte de plateforme est précisément ce que le miroir
  couvre.
- **Inversion des rôles (dépôt = référence, plateforme = copie)** : rejetée —
  voir décision ci-dessous.

## Décisions

1. **ADR-0028 amendé (2026-10-26)** : clause « reprise après perte de plateforme » —
   plateforme fait foi tant qu'accessible ; si elle disparaît, `docs/knowledge/`
   devient la référence de fait (perte maximale = la session en cours) ;
   l'inversion des rôles est rejetée pour trois raisons : coût (~5-8× par édition —
   aucune édition chirurgicale GitHub, réémission du fichier complet à chaque push),
   risque de troncature ~64 Ko (2 incidents réels de push), équivalence de risque
   (le miroir couvre déjà le scénario redouté, l'inversion n'ajoute que du coût).
   Choix explicite de Raphaël après explication des deux options.
2. **Gel de roadmap post-jalon 4** (décision Raphaël) : jalons 5-10 gelés tant que le
   meuble du cas d'école n'est pas fabriqué de bout en bout ; inscrit dans la fiche
   architecture v3 (les jalons restent listés comme garde-fou anti-oubli, aucun
   design ni code) ; levée du gel = décision explicite notée dans la fiche.
3. Préférences personnelles → `AGENTS.md`/fiche non publiée : accepté en principe,
   non encore traité (dette outillage).

## Liens

- Fiches touchées : `etat-actuel/KNOWLEDGE.md` (création), `securite` (v2),
  `conventions` (v3), `architecture` (v3), racine `KNOWLEDGE.md` (annotations),
  `README.md` (parcours).
- ADR-0028 (`docs/adr/` + copie `preparation/`) : amendement clause de reprise.
