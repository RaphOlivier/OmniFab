---
name: machines
description: "Communication machine OmniFab : LinuxCNC prioritaire puis GRBL/FluidNC/Marlin, multi-machines parallèle, liaisons port série (USB + RS-232/DNC pour machines anciennes) et port parallèle LPT (step/dir), machines maison Arduino, alertes de sécurité pédagogiques, éditeur G-code multi-dialectes, assistant de profils, description déclarative de contrôleurs."
---
**Version :** v1 (2026-10-25) — versionnage mis en place (fiche historique/KNOWLEDGE.md)

# Machines — communication et contrôle OmniFab

## Priorités

- **Contrôleur prioritaire : LinuxCNC** (multi-axes, industriel, changeur d'outils), puis dialectes GRBL / FluidNC / Marlin.
- **Tests forge-link : simulation d'abord** (décision 2026-10-10) — aucune machine physique au départ ; le simulateur de rejouage G-code et la cible web suffisent à valider l'architecture et l'abstraction des contrôleurs ; les machines réelles (et les liaisons physiques RS-232/LPT) seront priorisées plus tard.
- **Procédé pilote : fraisage CNC 2,5D**, puis multi-axes, tournage, couteau, plasma, laser (découpe + gravure), fraisage 3D, broderie (Ink/Stitch en référence libre), fabrication par tranches « 123D Make » (générateur de gabarits : tranches empilées/enchevêtrées/courbes/radiales/panneaux pliés — outil abandonné par Autodesk, aucun équivalent libre complet).
- **Machines maison** : plusieurs firmwares, G-code maison, Arduino et plusieurs moteurs — prises en charge via l'abstraction (axes multiples, options, extensions) + assistant de création de profils machines et de dialectes G-code maison.

## Fonctions

- DRO, jog, palpage/sonde, broche/laser/plasma, changeur d'outils, retour d'état, journalisation.
- **Multi-machines en parallèle** : file de travaux par machine, supervision simultanée. (Pas de file d'attente « en série » : le besoin exprimé concernait les ports physiques, pas l'ordonnancement des jobs.)
- **Port série (USB + RS-232)** : USB pour contrôleurs modernes ; RS-232/DNC drip-feed pour machines anciennes (Fanuc/Mazak/HAAS…) — transfert et upload/download de programmes, envoi du G-code au fil de l'exécution (RAM limitée des contrôleurs vintage), bauds/parité/bits/handshake configurables. Quasi absent du libre — atout différenciant.
- **Port parallèle LPT** : pilotage direct de machines pas-à-pas classiques par signaux step/dir sur les broches (comme le driver parport de LinuxCNC et Mach3 historiquement). Exige un noyau Linux temps réel et un LPT natif ou une carte PCIe ; les adaptateurs USB-parallèle sont inutilisables pour le step/dir (latence). Option long terme.
- **Sécurité pédagogique pendant le contrôle** : alertes qui expliquent les risques de l'action en cours (collision, courses, broche en rotation, feed hold, e-stop distant) avant lancement et pendant l'exécution ; tout contournement demande une confirmation explicite.
- **Éditeur G-code intégré** : colorisation syntaxique + autocomplétion référencées sur le dialecte du contrôleur cible ; aides à la modification (documentation des commandes en ligne, détection d'erreurs, aperçu de la trajectoire au curseur) ; simulateur d'usinage 3D.
  - Référence « design direct » : **FullControl** (GPL-3.0, voir index racine § Logiciels de référence) — conception paramétrique de trajectoires directement en G-code, contrôle ligne à ligne (vitesse, extrusion, accélération, couches non planes) ; inspiration pour un futur mode de conception au-dessus de l'éditeur, en complément du pipeline CAO→CAM.

## Architecture (conseil fort)

- Définir un **langage déclaratif de description de contrôleur** (fichier YAML/DSL : commandes, réponses d'état, axes, limites). Une seule description alimente autocomplétion, colorisation, post-processeur, simulateur et l'assistant de profils maison — quatre fonctionnalités pour le prix d'une.
- **Deux transports dès le départ** : série natif (serialport + tokio) et WebSocket (pont local pour la cible web ; Web Serial API non universelle — Chrome/Edge desktop, Android récent seulement).
- Spécification du protocole machine (états) : LinuxCNC d'abord, à rédiger.

## Intégration LinuxCNC — deux modes (décision 2026-10-09)

Axis n'est qu'une GUI parmi d'autres : l'INI désigne l'interface ([DISPLAY] DISPLAY=axis | touchy | gmoccapy | gscreen | qtvcp qtdragon | qtplasmac…). Le contrôleur temps réel et la GUI communiquent via NML (C++ + bindings Python). Conséquences pour forge-link :

- **Mode GUI natif** : `DISPLAY=omnifab` dans l'INI, liaison NML via FFI Rust (cxx). OmniFab remplace Axis sur le PC LinuxCNC : CAO + CAM + pilotage en un seul écran ; le temps réel (step/dir, LPT, Mesa, THC) et le HAL restent à LinuxCNC. Pré-requis : exécution sur la machine LinuxCNC.
- **Mode distant** : via `linuxcncrsh` (interface réseau en mode texte, utilisable à la place d'une GUI ou en plus). Chemin le plus simple pour l'abstraction forge-link, compatible Android/web. Mise en garde : mots de passe transmis en clair — réseau de confiance ou tunnel uniquement.
- Ces deux modes valident l'architecture : le « dialecte LinuxCNC » de forge-link n'est pas un protocole texte ad hoc mais la couche NML/linuxcncrsh. À arbitrer en phase 0 (GUI native = gros travail FFI mais expérience intégrale ; distant = plus rapide, latence réseau en contrepartie).

## Détection automatique des paramètres de communication (vision Raphaël, 2026-10-19)

Objectif : connecter une machine en retirant le maximum d'étapes de configuration ; identifier automatiquement le contrôleur (type, marque, modèle), en déduire les paramètres de communication. Cadre proposé (à cadrer par ADR au jalon 5, greffé sur le langage déclaratif de contrôleurs) :
- **Sonde à échelle de bauds** (débits courants, chaînes neutres) + lecture des **bannières** (`Grbl x.y`, `Marlin x`, invites Fanuc/HAAS…).
- **Empreinte protocolaire** : signature des réponses par dialecte (`?`, `M115`, invites) → identification par correspondance avec la **base communautaire de contrôleurs**.
- **Sécurité** : la sonde n'envoie jamais de commande de mouvement ; toute écriture de config machine derrière confirmation explicite (fiche Sécurité).
- Résultat : profil proposé → validation utilisateur → sauvegardé comme profil maison. Base de contrôleurs alimentée par Internet + communauté (cf. bibliothèques, jalon 10, et fiche Formats §bibliothèques de données).

## Bibliothèques de pré-réglages

Matériaux (avances, vitesses de coupe, propriétés physiques réelles), types et marques de machines courantes (hobby + industriel, caractéristiques constructeurs), normes ISO (visserie, formats…). Sources : bases libres existantes, contributions communautaires, normes ISO ; données sourcées, citées, sous licence ouverte (voir fiche Conventions).

## Historique

- v1 (2026-10-25) : mise en place du versionnage — n° de version en tête + cette section (protocole : fiche historique).
