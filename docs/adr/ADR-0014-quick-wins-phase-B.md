# ADR-0014 : phase B — sélecteur de fichiers, bibliothèque de projets, wireframe STL

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-15, à la condition — vérifiée
  et actée — que le sélecteur fonctionne facilement sur toutes les grandes distributions
  Linux, pas seulement KDE/Plasma : voir « Portabilité Linux »)
- **Date :** 2026-10-15
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 2, phase B (quick wins suspendus par l'ADR-0011, définis par l'ADR-0012)

## Contexte

La phase A est close (ADR-0013, CI verte multi-OS sur `4423563`) : planegcs tourne derrière
le trait `Solveur2D`. La phase B enchaîne sur les trois quick wins suspendus par l'ADR-0011 :
les import/export du démonstrateur utilisent encore des chemins de démonstration fixes,
le maillage STL importé n'est affiché que comme compteur, et il n'existe ni sélecteur de
fichiers ni bibliothèque de projets. Critère de sortie de l'ADR-0012 : « le démonstrateur
ouvre/enregistre des fichiers choisis par l'utilisateur et affiche le maillage importé. »

## Décision

**1. Sélecteur de fichiers : crate `rfd` 0.17, backend `xdg-portal` sous Linux.**

- `rfd` (double licence MIT/Apache-2.0, MSRV 1.88) est le standard de fait des dialogues
  de fichiers natifs pour egui. API **synchrone** (`FileDialog::pick_file()` /
  `save_file()`) appelée depuis les boutons : le dialogue est modal, la boucle egui reprend
  à sa fermeture — comportement usuel des dialogues natifs, acceptable au démonstrateur.
- Backend Linux : `xdg-portal` (`default-features = false, features = ["xdg-portal"]`) :
  **pur Rust** (D-Bus), aucune dépendance C de build → **aucune modification de la CI**
  (les jobs Linux et aarch64 restent tels quels ; a contrario le backend gtk3 exigerait
  `libgtk-3-dev` sur les jobs Linux et l'exclusion du desktop du job aarch64). Au runtime,
  xdg-desktop-portal est présent sur les distributions desktop modernes, KDE/Plasma inclus.
- Encapsulation : module `fichiers.rs` du desktop (`choisir_import`, `choisir_export`,
  `chemin_enregistrement`) — isolé pour le jalon 7 (Android : intent système en
  remplacement) et le jalon 9 (web : API async de rfd).
- Plan B documenté : si le portal dysfonctionne sur un poste réel, bascule gtk3 = une ligne
  de Cargo.toml + deps CI Linux. Pas préventif.
- **Portabilité Linux (condition de validation)** : `xdg-desktop-portal` est un standard
  freedesktop.org, infrastructure requise de Flatpak et Snap — il est donc installé **par
  défaut sur toutes les grandes distributions** (Ubuntu, Fedora, Debian, Mint, openSUSE,
  Arch avec un bureau, Manjaro…) et pour **tous les principaux bureaux** : GNOME, KDE/Plasma,
  XFCE, Cinnamon, MATE, LXQt, Budgie (le sélecteur est rendu par le bureau lui-même via son
  backend de portail : `xdg-desktop-portal-gnome`, `-kde`, `-gtk`, `-lxqt`…). Seul cas
  dégradé : installation minimale sans bureau (tiling WM seul) sans portail installé — le
  dialogue ne s'ouvre pas (rfd retourne `None`), hors cible produit et couvert par le
  plan B gtk3. Documentation de référence : pages Arch/Gentoo/Void/Mint.

**2. Bibliothèque de projets : dossier dédié + écran d'accueil.**

- Dossier : `$(home_dir())/OmniFab/Projets`, via `std::env::home_dir()` — réhabilité et
  corrigé depuis Rust 1.87, donc **pas de crate `dirs`**. Créé au premier lancement ;
  visible et manipulable à la main, pas de configuration au jalon 2.
- Extension : `.omnifab` (le conteneur ZIP des jalons 0/1). Un projet de la bibliothèque
  = un fichier `.omnifab` du dossier.
- Écran d'accueil : état `Ecran::{Accueil, Atelier}` du démonstrateur. Au lancement →
  Accueil : « Nouveau projet », « Ouvrir un projet… » (sélecteur, n'importe où sur le
  disque), « Reprendre la session en cours » (si une session est en mémoire), et la liste
  du dossier bibliothèque (nom, date de modification, taille ; tri par date
  décroissante ; clic = ouvrir). Bouton « Bibliothèque » dans la barre du haut → retour
  à l'accueil ; la session en cours reste en mémoire tant qu'aucune autre n'est démarrée.
- Enregistrement : « Enregistrer » utilise le chemin courant s'il existe, sinon (et via
  « Enregistrer sous… ») dialogue natif, dossier proposé = bibliothèque. Le chemin courant
  est **un état de l'app UI, pas un événement** (le journal ne porte pas de chemin,
  cohérent avec l'ADR-0011) ; il n'est pas persisté (pas de « récents » au jalon 2 : le
  scan du dossier suffit).
- Ouverture : `Document::open_from_path` + `Session::from_document` comme aujourd'hui,
  plus mémorisation du chemin courant.

**3. Rendu wireframe du maillage STL dans le viewport existant.**

- Projection orthographique de dessus (X, Y). Maillage et tracés partagent la même mise
  à l'échelle : la boîte englobante de fit devient l'**union** tracés + maillage
  (routine de fit factorisée).
- Re-parse du blob d'origine en cache mémoire non autoritaire, clé = nom du blob
  maillage, invalidé quand la référence change — même mécanisme que les tracés
  (ADR-0011).
- Plafond d'affichage : wireframe si `triangle_count() <= 20 000` (60 000 segments
  peints par frame, cadence tenue par le painter) ; au-delà : boîte englobante 2D seule
  + message i18n. Le maillage reste importé et compté — le plafond est visuel, jamais
  une perte de données. **Réglage du démonstrateur, relevable sans ADR** (constante
  documentée) ; **supprimé par la phase C** (wgpu : le rendu GPU ramène la limite au
  budget d'import, 4 millions de triangles).
- Couleur distincte (cyan), trait fin. Pas d'ombrage, pas de caméra (phase C, wgpu : le
  wireframe migre avec les tracés, périmètre borné déjà acté par l'ADR-0012).

## Alternatives rejetées

- **Natif par OS** (zenity/kdialog sous Linux, Win32, NSPanel) : trois implémentations à
  maintenir pour un quick win ; les sous-processus Linux sont fragiles et intestables.
- **rfd backend gtk3** : éprouvé, mais `libgtk-3-dev` exigé à la build sur toute la
  matrice Linux (2 jobs + exclusion aarch64 du desktop) — coût CI permanent pour une
  fonction de confort.
- **Sélecteur egui maison** : non natif, parcours disque à réinventer — l'opposé d'un
  quick win.
- **Crate `dirs`** : inutile depuis la réhabilitation de `std::env::home_dir`
  (Rust >= 1.87 ; le projet est sur 1.99).
- **Bibliothèque complète** (métadonnées, vignettes, recherche, récents) : suspendue par
  l'ADR-0012 (« bibliothèque de projets complète (produit, jalon 2+) »).
- **Rendu rempli/ombré, caméra zoom/pan** : phase C (wgpu).

## Conséquences

- **Positives** : le démonstrateur traite de vrais fichiers choisis par l'utilisateur ;
  le maillage STL devient visible ; **zéro modification de la matrice CI** ; le module
  `fichiers` trace la voie des jalons 7 (Android) et 9 (web).
- **Négatives / coûts acceptés** : une dépendance de plus au desktop (rfd, MIT/Apache-2.0,
  compatible MPL-2.0) ; dialogue modal bloquant pendant la sélection ; le backend portal
  de rfd 0.17 a été réécrit (0.17.2 corrige des défauts ; plan B gtk3 documenté) ; pas de
  suivi de modifications non enregistrées au retour accueil (« Reprendre » = état mémoire).
- **Risques** : portails desktop exotiques (rapports en amont sur NixOS/Hyprland — hors
  cible, Kubuntu/KDE) ; perf painter au plafond des 20 000 triangles à vérifier à la
  première exécution réelle.
- **Dépendance à ajouter** (première étape, par Raphaël, comme pour usvg/dxf) :
  `cargo add rfd@0.17 --no-default-features --features xdg-portal -p omnifab-desktop`,
  commit + push du Cargo.toml/Cargo.lock (CI verte : dépendance non encore utilisée).
- **Tests** : bibliothèque (création du dossier, filtre par extension, tri par date —
  fixtures sous `std::env::temp_dir`), wireframe (fonction pure de projection : boîte
  englobante, respect du plafond), enregistrer/ouvrir par chemin (fixtures tmp). Le
  sélecteur natif lui-même n'est pas testable en CI : couvert à la main par Raphaël —
  c'est le critère de sortie de la phase.
- **i18n / help_id** : clés Fluent et pages d'aide pour chaque nouveau widget interactif
  (règle jalon 1, appliquée par les tests existants) ; **aucun nouveau code d'erreur**
  (les pannes d'IO s'affichent au canal d'erreur du viewport ; le registre `OF-*`
  concerne le noyau).
