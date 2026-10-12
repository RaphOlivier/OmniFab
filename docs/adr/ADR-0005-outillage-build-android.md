# ADR-0005 : Outillage de build Android pour le PoC egui (jalon 0)

- **Statut : accepté**
- **Date : 2026-10-12**
- **Décideurs : Raphaël Olivier** (plan validé le 2026-10-12 ; le présent ADR tranche le choix de l'outil)

## Contexte

Il reste, pour clore le jalon 0, la preuve de concept egui sur Android (tactile, clavier virtuel). Contraintes : (1) Raphaël n'installe rien en local — l'APK doit être produit par GitHub Actions et téléchargé depuis l'onglet Actions ; (2) ne jamais modifier `.github/workflows/ci.yml` ni son mécanisme d'issues « CI : » ; (3) egui sur Android est connu comme « presque là » : le point faible est le clavier virtuel (egui #3890, #1633, #5590 ; suivi IME dans rust-windowing/winit#3787), d'où un PoC minimal qui doit se construire vite et se diagnostiquer facilement.

## Décision

Construire l'APK avec **cargo-apk** (rust-mobile) : outillage pure Rust, sans Gradle ni Java, métadonnées du manifeste Android dans le `Cargo.toml` de la crate, APK signé debug installable manuellement. La crate `apps/omnifab-android` (cdylib) embarque une démo egui minimale — bouton géant, zone de glisser, deux champs de texte, journal des événements d'entrée bruts — et un point d'entrée `android_main` qui injecte l'`AndroidApp` dans l'`EventLoopBuilder` de winit. Le job `build-apk` du workflow dédié `.github/workflows/android.yml` compile pour `aarch64-linux-android` sur runner Ubuntu (NDK préinstallé), publie l'APK en artefact téléchargeable et réutilise le mécanisme d'issue « CI : … » en cas d'échec. Le PoC utilise le renderer **glow** : eframe 0.29 n'active pas le backend GLES de wgpu (écrans noirs Android connus), or le renderer est indépendant du chemin d'entrée tactile/IME, seul objet du PoC.

## Alternatives rejetées

- **cargo-mobile2** (fork tauri de cargo-mobile) : vivant et soutenu par la communauté Tauri, mais orienté wry/Tauri ; le template egui est cassé/périmé (issues #456, #175), la génération d'un projet Android Studio complet encombre le dépôt d'un boilerplate Gradle, et l'usage headless en CI est maladroit pour un PoC.
- **xbuild** (rust-mobile) : explicitement marqué **non maintenu** ; le binaire publié sur crates.io a un bug de linker connu ; adopter un outil à l'abandon pour une décision d'infrastructure est un risque inutile.
- **Template Gradle manuel + cargo-ndk** : la voie « standard » recommandée (android-activity), la plus pérenne pour une vraie publication (AAB Play Store, signature, Play Console), mais elle impose de maintenir dans le dépôt des fichiers de build Java/Kotlin, le câblage JNI et la config de signature — un poids injustifié pour un PoC dont le seul consommateur est un artefact CI. **À reconsidérer au jalon 7** (vraie application Android) si le verdict egui est positif.
- **cargo-apk2** (fork de cargo-apk) : fonctionnalités supplémentaires (DEX, multi-activités) inutiles ici ; moins établi que cargo-apk, qui a repris une maintenance active (alignement zipalign 16 Kio en 2025) et est la voie documentée par android-activity et Slint pour les démarrages rapides.

## Conséquences

- **Positives** : zéro fichier Gradle/Java dans le dépôt ; APK téléchargeable depuis Actions sans aucune installation locale ; itération rapide (push → APK en quelques minutes) ; échecs CI signalés par issue comme pour le workflow principal.
- **Négatives / coûts acceptés** : APK signé debug (installation manuelle OK, Play Store exclu — hors périmètre du PoC) ; cible unique `aarch64-linux-android` (émulateurs x86 non couverts) ; renderer glow pour le PoC — la validation de wgpu sur Android reste à faire au jalon 7 ; Cargo.lock à regénérer côté clone local (nouvelle crate dans le workspace).
- **Décisions futures suspendues** : verdict egui vs plan B Slint (après test de l'APK sur téléphone) ; passage éventuel à Gradle+cargo-ndk au jalon 7 ; cibles armv7/x86 ; signature release.
