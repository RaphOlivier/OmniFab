# ADR-0010 : forge-i18n — internationalisation Fluent et aide intégrée

- **Statut : accepté** (validé par Raphaël Olivier le 2026-10-14)
- **Date :** 2026-10-14
- **Décideurs :** Raphaël Olivier

## Contexte

Conventions posées pour le jalon 1 : toute chaîne d'interface vit derrière une clé dès la première application (un rattrapage de chaînes en dur serait coûteux), et chaque widget porte un `help_id` — un clic droit ouvre « Aide sur X », dont le contenu (description + exemple concret replacé dans le contexte de l'atelier) a pour source unique le même fichier que l'application, le manuel et le site web ; une page sans exemple fait échouer la CI. Le démonstrateur omnifab-desktop (jalon 0) affiche aujourd'hui toutes ses chaînes en dur dans `main.rs`.

Contraintes : ADR-0003 (erreurs typées `OF-<DOMAINE>-<NNNN>`, registre `docs/error-codes.md`, test bidirectionnel par crate) ; crates du noyau pures (aucun I/O disque) ; MPL-2.0 pour les crates réutilisables, GPL-3.0-or-later pour les apps ; ajouter une langue ne doit jamais demander de toucher au code des widgets.

## Décision

1. **Crate `forge-i18n`** (MPL-2.0, pure — ni egui, ni I/O disque, `#![forbid(unsafe_code)]`) : encapsule Fluent (crates `fluent` 0.17 et `unic-langid`). API : `Catalogue` (construit depuis une source FTL, traduit à clés, avec arguments typés), `PageAide` (titre, description, exemple), `Argument` (entier ou texte). Erreurs typées OF-I18N-0001..0004 au registre central, test bidirectionnel ADR-0003.

2. **FTL embarqué à la compilation** (`include_str!`) : le fichier FTL vit dans le dépôt de chaque application (ex. `apps/omnifab-desktop/locales/fr/main.ftl`), jamais chargé à l'exécution — fichier manquant ou corrompu = erreur de compilation, pas une panne à l'usage. **Français = langue de référence** ; ajouter une langue = un fichier FTL supplémentaire + le choix du catalogue, aucun widget ne bouge.

3. **Pages d'aide = messages Fluent** : la page d'un widget est le message du même nom que son `help_id`, avec les attributs `titre`, `description` et `exemple`. Une page incomplète est une erreur typée (OF-I18N-0003) ; les tests de chaque application vérifient (a) que chaque clé utilisée existe dans le FTL, (b) que chaque page est complète avec un exemple non vide — la convention « une page sans exemple = CI en échec » devient mécanique.

4. **Wrappers par application** : `avec_aide(&response, help_id, &mut ouverte)` (un clic droit — bouton secondaire — ouvre la page) et `fenetre(ctx, &catalogue, &mut ouverte)` (dessine la fenêtre d'aide). Les widgets n'affichent aucune chaîne en dur : tout passe par le catalogue. Le jalon 1 duplique volontairement ces ~80 lignes dans omnifab-desktop ; la mutualisation (crate `omnifab-ui`) est reportée au jalon 7, quand le PoC Android et le choix de l'UI 3D auront stabilisé l'API des wrappers.

5. **Sécurité** : le FTL est un document embarqué à la compilation, jamais une entrée utilisateur ; les arguments des patterns viennent de l'application ; Fluent encadre les arguments textuels de marques d'isolation Unicode invisibles à l'affichage. Aucune surface d'attaque nouvelle par rapport aux données du projet.

## Alternatives rejetées

- **Fluent utilisé directement dans chaque application** : duplication du code de catalogue et d'erreurs, pas de frontière unique, chaque app réinvente ses tests de complétude.
- **Chargement des FTL à l'exécution** : I/O disque dans les apps, cas « fichier absent ou corrompu » rencontré seulement à l'usage et contournable ; l'embarquement rend le binaire autonome et la complétude vérifiable par la CI.
- **Macros de génération à la compilation** (build.rs, fluent-templates et similaires) : complexité de build, messages d'erreur moins lisibles, dépendances supplémentaires — injustifié pour une langue de référence et quelques dizaines de clés.
- **Crate `omnifab-ui` (wrappers egui + aide) dès maintenant** : l'API des wrappers bougera avec le PoC Android et l'UI 3D (jalon 7) ; mutualiser maintenant figerait un design non éprouvé.
- **PoC Android intégré au périmètre** : jetable par définition ; l'UI Android réelle arrive au jalon 7 — forge-i18n suffit à préparer sa localisation.

## Conséquences

- **Positives** : aucune chaîne d'interface en dur dès le jalon 1 ; l'ajout d'une langue ne touche ni le code ni le registre des erreurs ; l'aide intégrée a la même source que le manuel et le site web (le FTL) ; une page d'aide incomplète casse la CI mécaniquement.
- **Négatives / coûts acceptés** : duplication volontaire de ~80 lignes de wrappers dans omnifab-desktop (dette assumée jusqu'au jalon 7) ; les arguments textuels portent des marques d'isolation Unicode invisibles (sans effet à l'affichage) ; le français est la seule langue livrée au jalon 1.
- **Décisions futures suspendues** : sélection de la langue par préférence utilisateur (persistance par événement, jalon 2) ; crate omnifab-ui (jalon 7) ; extraction du FTL vers le manuel et le site web (génération de documentation).
