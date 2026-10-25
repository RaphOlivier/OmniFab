# 2026-10-19 — Gouvernance, ADR-0019 et vision bibliothèques

Discussion de planification jalon 2 (suite) + série de questions de fond. Points clés :

## Gouvernance des validations
- **Validations au fil de l'eau** (demande de Raphaël) : soumettre chaque décision mûre immédiatement pendant que le travail non dépendant continue — plus d'attente séquentielle. Acté dans Conventions (point 8). Première application : ADR-0019 validé le jour même, 4 axes sur 4.

## ADR-0019 — moteur de liaisons forge-values (phase E, jalon 2) : ACCEPTÉ
Périmètre : (1) noyau + **panneau de liaisons desktop** (le panneau paramètres devient le panneau liaisons) ; (2) sémantique tableur : **chaque cellule de tableau = un nœud du graphe** référencé par alias, cycle = OF-VAL-0001 ; (3) édition = champ simple + erreurs i18n (éditeur riche jalon 3) ; (4) **cotes d'esquisse pilotées par expressions** intégrées (le solveur reçoit la valeur, jamais l'expression). Fichier prep : `preparation/ADR-0019-liaisons-phase-E.md`.

## Pourquoi le développement est long / réflexions longues
- Un prompt crée une démo ; OmniFab reconstruit l'équivalent de ~55 logiciels (des décennies) : correctitude (tolérance 1e-6), robustesse (fichiers réels pièges), irréversibilité (verrous), industrialisation (CI, i18n, ADR). Le temps réel = découvrir ce qui manque, pas écrire.
- Réflexions de plusieurs heures = raisonnement explicite sur questions ouvertes + auto-vérification ; maximisé par les conflits de contraintes et l'irréversibilité du projet. Réfléchir longtemps ≠ lenteur : profondeur dépensée au bon endroit.
- Stack jugée **fitness au projet**, pas optimale en absolu : Rust/OCCT/planegcs/egui approuvés ; divergences honnêtes exprimées (scope plus timide, réflexe SQLite, réflexe Qt) mais tenues pour raisonnables — egui validé empiriquement (PoC Android), Qt aurait coûté FFI C++, wasm fragile, LGPL. ZIP vs SQLite : garde le ZIP (avantages lisibilité/blobs/diff ; l'atomicité manquante déjà payée via lot A ADR-0016).

## Interopérabilité FreeCAD
- Import `.FCStd` : géométrie toujours, recette paramétrique partiellement (même solveur planegcs), **historique d'interaction jamais** — vrai dans tous les sens, aucune exception industrielle.
- **Export vers FreeCAD** : seule direction permettant reconstruction quasi-complète de l'arborescence (journal d'événements → objets via API scriptable) ; importateur = module côté FreeCAD, rien exigé du nôtre.

## Réécriture OCCT/planegcs en Rust ?
- **planegcs : oui à terme** (~10 kloc, déjà derrière le trait `Solveur2D`, parité par croquis dorés).
- **OCCT : non** (1 M+ lignes, décennies de cas limites ; Fornjot en pause = leçon). Stratégie : surveillance + contributions amont ; les frontières étanches (ADR-0004/0013) gardent l'option ouverte pour toujours.

## Vision bibliothèques (série de décisions, fiche Formats §Bibliothèques)
1. **Détection automatique des paramètres machine** (fiche Machines) : sonde de bauds, bannières, empreintes protocolaires → identification contrôleur via base communautaire ; jamais de mouvement pendant la sonde ; profil → validation → profil maison.
2. **Bibliothèques = référence ouverte du marché** (modèle OSM) : infrastructure partagée consommable par des tiers sans OmniFab ; format autonome documenté, triple accès (fichiers/API/crate), gouvernance neutre, identifiants stables, ODbL comme levier.
3. **Architecture à 3 niveaux** : (a) **géométrie générée depuis les valeurs** pour les pièces normées (visserie, IPE, quincaillerie normée — générateurs paramétriques sur forge-values, zéro stockage, cohérence par construction) ; (b) **données descriptives embarquées** dans l'installation (ontologie à relations typées : compatibilité = requête sur le graphe, ex. pas de vis partagé ; assemblages compositionnels) ; (c) **géométrie téléchargée à la demande** (glTF LOD + STEP/3MF, cache local) pour le non-normé. Règle : si une norme décrit la pièce → générée ; sinon → téléchargée.

## Sincérité sur les limites de l'IA (questions de Raphaël)
- Fichiers perdus = sandbox éphémère par conception (résolu : prep dans knowledge/) ; corruption au push = intermédiaires défaillants (résolu : relecture du commit réel) ; code à l'aveugle = pas de compilateur dans le sandbox (contenu : CI oracle + gardes ; complète : boucle locale).

## Canaux de travail et préférences de Raphaël
- **Trois canaux complémentaires** : Vibe Work (orchestrateur), Vibe Code VS Code (boucle locale compile/corrige/test — à installer), Vibe Code Web/Android (agent cloud sur dépôt → PR relue ; cas d'usage : convergence CI mécanique à distance, corrections textuelles/accents, bugs rapportés).
- **Règle Raphaël : fournir des prompts prêts à coller** pour Vibe Code quand une tâche s'y prête (prompt chirurgical complet, fichiers/lignes cibles). **Ne plus conseiller sur l'abonnement Pro** — décision personnelle, ne plus en parler.
- Règles anti-quota Vibe Code : jamais « analyse le projet », une tâche = une session, cible fichier/fonction.

## En attente (reporté au résumé de transition)
- Resynchronisation canvas « atelier-universel-prompt » (points 5 et 9 enrichis dans le fichier, pas le canvas).
- Matrice v7→v8 : nouvelles fonctions (détection auto, base ouverte, génération paramétrique, LOD) + logiciels de référence à rattacher.
