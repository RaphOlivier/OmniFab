# ADR-0028 — Copies de référence de la Knowledge au dépôt

Date : 2026-10-25. Jalon : transversal (gouvernance).

## Contexte

La Knowledge OmniFab (fiches de décisions, archives de discussions, journal
des fonctions) vit uniquement sur la plateforme Mistral, côté Vibe Work.
Deux problèmes :

- **point de défaillance unique** : rien ne garantit la pérennité de ce
  contenu si l'accès à la plateforme change ;
- **incompréhensible hors plateforme** : un nouveau contributeur humain ou
  une autre IA qui clone le dépôt voit le code et les ADR, mais pas le
  *pourquoi* ni l'historique des décisions — exactement ce qui manque le
  plus à un nouveau venu.

Les matrice v8 et prompt fondateur ont déjà des copies de référence au
dépôt (`docs/`) avec un protocole de resynchronisation : ce précèdent
fonctionne, il est étendu au reste de la Knowledge.

## Décision

1. **Sous-ensemble publiable miroité sous `docs/knowledge/`**, structure
   1:1 avec la Knowledge de la plateforme : racine (`KNOWLEDGE.md`),
   `architecture/`, `conventions/`, `formats/`, `machines/`, `securite/`,
   `archives/`, `historique/` (journal des fonctions compris).
2. **Autorité** : la Knowledge de la plateforme **fait foi** ; le dépôt
   est une copie de référence publiable. Jamais l'inverse.
3. **Règle de synchronisation** : tout bump de version d'une fiche
   (signal : la section Historique de la fiche, ou une nouvelle ligne du
   journal append-only) ⇒ la même fiche est repoussée au dépôt dans la
   foulée, avec les fichiers déjà en cours de commit. Le delta est
   toujours identifiable sans relire le reste.
4. **Exclusions explicites** :
   - `prompt-fondateur.md` et matrice : déjà des copies de référence à
     `docs/` (protocole existant), pas de doublon ;
   - `references/` : documentation **externe** (wiki FreeCAD, PDFs) —
     licences et attribution à respecter, inutile de republier ;
   - `preparation/` : documents de travail, souvent marqués OBSOLÈTES
     (le dépôt fait foi) ;
   - `economie-tokens/`, `flux-travail-mobile.md`,
     `canvases-referentiel.md` : processus de travail avec Vibe, contenu
     personnel/prestataire — pas destiné à être publié.
5. **Intégrité** : toute synchronisation vérifie le contenu poussé
   (roundtrip d'empreinte) — leçon des incidents de push tronqué.

## Coût

- Initial : ~300 Ko poussés une fois (~21 fichiers) — négligeable.
- Récurrent : seuls les fichiers touchés d'une session sont repoussés
  (quelques Ko). La version des fiches rend le delta gratuit : aucun
  relecture complète, aucun diff coûteux.

## Alternatives rejetées

- **Miroir complet (800 Ko)** : publierait des copies de documentation
  externe (question de licence) et des notes de process personnelles ;
  le dépôt est destiné à devenir public — le tri doit être fait avant.
- **Aucune copie** : laisse le point de défaillance unique et le projet
  incompréhensible hors plateforme.
- **Le dépôt fait foi** : le travail se fait dans Vibe Work ; doubler
  chaque édition dans les deux sens doublerait les occasions de
  divergence au lieu de les réduire.

## Conséquences

- Une discipline de plus (une ligne par bump de fiche) — assumée
  comme le protocole matrice/prompt déjà en place.
- Le passage public du dépôt n'aura pas de tri rétroactif à faire :
  le sous-ensemble publié est déjà propre.
- Toute divergence constatée se résout dans un sens : le dépôt reprend
  la version de la plateforme.

## Statut

**ACCEPTÉ** — validé par Raphaël Olivier le 2026-10-25 (choix du
sous-ensemble filtré, première synchronisation immédiate).
