---
name: securite
description: "Modèle de menace et règles de sécurité OmniFab, par jalon : fichiers importés non fiables, réseau forge-link, usinage physique, chaîne d'approvisionnement, plugins WASM. Principe directeur : sécurité par défaut SANS limiter fonctions ni performances — tout arbitrage fonction↔sécurité passe par un ADR explicite."
---
**Version :** v3 (2026-10-29 — **correction A4-12 (audit n°4)** : le « job cargo-deny en commentaire dans ci.yml » affirmé en v2 **n'a jamais existé** (grep zéro occurrence dans les workflows — la proposition vivait dans `docs/ci-cargo-deny.md`) ; **workflow dédié `.github/workflows/cargo-deny.yml` activé le 2026-10-29** (décision Raphaël) : push main/PR sur `Cargo.lock`/`deny.toml` + `workflow_dispatch`, action EmbarkStudios v2, **aucun mécanisme d'issues** (le spam d'issues reste celui de ci.yml, signalé A4-12) ; APK de CI noté debug (keystore jetable, mot de passe public) ; épinglage chaîne APK = hygiène supply chain backlog) ; v2 (2026-10-26) — statuts datés ajoutés (revue du miroir : cargo-deny « en place » → « job rédigé, à activer » ; plafond cumulé de blobs précisé) ; v1 (2026-10-25) : versionnage mis en place

# Sécurité — modèle de menace et règles par jalon

## Principe directeur (condition de Raphaël Olivier, 2026-10-14)

**La sécurité ne doit pas limiter les fonctions ni les performances.** Conséquences de gouvernance :

- Chaque règle ci-dessous déclare son coût (souvent nul ou négligeable).
- Une règle de sécurité qui contraindrait matériellement une fonction ou la performance **ne s'impose pas en silence** : elle remonte en arbitrage explicite sous forme d'ADR, avec les deux options sur la table (Raphaël décide).
- Modèle : **sécurité par défaut, fonctions par consentement explicite** — les fonctions sensibles (réseau distant, ressources externes, plugins natifs éventuels) sont débloquées par une action/config explicite de l'utilisateur, jamais interdites par principe.
- Tout ADR de jalon comporte une **section « sécurité » obligatoire** (surface ajoutée, attaques envisagées, coûts).

## Modèle de menace — les cinq portes d'entrée

1. **Fichier malveillant ouvert par la victime** (jalon 1+, porte n°1) : SVG (scripts, href externes), DXF (historique d'exécution de code dans AutoCAD), PDF, STL/maillages malformés, projet `.omnifab` piégé. Vecteur : fichier partagé (forum, e-mail, site) + ouverture.
2. **Réseau** (jalons 5/9) : pont WebSocket machine, mode distant `linuxcncrsh` (mots de passe en clair — réseau de confiance ou tunnel uniquement), télécommande Android.
3. **Machine physique** (jalons 4/5) : un G-code téléchargé ou une session compromise cassent la machine ou blessent quelqu'un — la sécurité y est aussi **physique**.
4. **Chaîne d'approvisionnement** (continu) : dépendances crates.io ; frontières FFI C++ (OCCT, planegcs, NML) = code non mémoire-sûr à isoler strictement.
5. **Plugins** (jalon 8) : extension malveillante = porte d'entrée royale si plugins natifs ; sandbox WASM sinon.

## En place ou prévu — chaque règle porte son statut et sa date (protocole 2026-10-26)

**En place (faits vérifiés au code, ne pas régresser) :**

- Rust + `#![forbid(unsafe_code)]` dans forge-doc — les données non fiables passent d'abord par du Rust mémoire-sûr. *(statut : en place, vérifié 2026-10-17)*
- Garde-fous anti-bombe ZIP (revue point 7) : 10 000 entrées, manifeste 1 MiB, journal/snapshot 64 MiB, **blob 1 GiB par blob + plafond cumulé de 2 GiB pour l'ensemble des blobs (ADR-0016 A4)** ; noms de blobs validés (OF-DOC-0013, anti zip-slip). *(statut : en place, vérifié audit 2, 2026-10-25)*
- `Cargo.lock` versionné + `--locked` en CI. *(statut : en place)*
- Erreurs typées ADR-0003 : jamais de panique silencieuse sur entrée malformée. *(statut : en place)*
- Machines en simulation d'abord ; local-first sans compte cloud ni télémétrie (aucune surface serveur). *(statut : en place par conception)*

**En place (cargo-deny — activation 2026-10-29) :**

- cargo-deny (advisories RustSec, sources crates.io uniquement, licences) : `deny.toml` en place (`3474f1d`, B5 — conformité : sources recompilables + GPL-3.0+ racine, A4-11 validé par Raphaël) ; **workflow dédié `.github/workflows/cargo-deny.yml` activé le 2026-10-29** (décision Raphaël, audit n°4 A4-12 ; `ci.yml` inchangé ; aucun mécanisme d'issues sur ce workflow — un échec est visible dans le run). *(statut : en place — premier verdict à relever à la prochaine veille)*

## Règles par jalon (avec coûts)

| Règle | Jalon | Coût / impact fonction-perf |
|---|---|---|
| Interdire `unsafe` hors frontières FFI (module dédié + revue obligatoire) | convention, figée | Nul — forge-doc l'applique déjà |
| ADR forge-io (actée : ADR-0009) : budget de ressources par parseur (nœuds/sommets/allocation), XML sans DTD ni entités externes (XXE), jamais d'extraction par chemin d'archive, parsing hors fil UI | 1 | Négligeable ; un fichier corrompu ne gèle pas l'UI |
| SVG assaini : pas de scripts, href externes chargés **uniquement sur action explicite de l'utilisateur** | 1 | Fonction préservée (import à ressources liées possible), juste opt-in |
| Fuzzing des parseurs en CI (cargo-fuzz + corpus de fichiers réels du verrou « imports testés ») | 1 | Temps CI uniquement |
| Job CI cargo-deny (workflow dédié — **activé le 2026-10-29**) | 1 | Nul |
| Usinage : courses/limites logicielles vérifiées contre le profil machine avant envoi + recalcul obligatoire (ADR-0002, déjà acté) ; G-code téléchargé = non fiable par défaut | 4 | Fonction de sécurité, pas une limite ; simuler avant usiner reste possible |
| ADR forge-link : WebSocket sur localhost + token d'appairage ; commandes entrantes refusées sans auth ; `linuxcncrsh` en réseau de confiance/tunnel ; chemin e-stop jamais bloqué par l'UI | 5 | Contrôle distant reste possible (config explicite + TLS le moment venu) |
| Plugins : sandbox WASM + API à capacités (pas d'accès direct fichiers/réseau) — **invariant à figer avant le design des API du noyau** | 8 | Plugins numériques : coût perf négligeable ; un plugin nécessitant du natif = décision explicite (ADR) |
| Distribution : APK signé clé release, checksums, SECURITY.md + divulgation responsable (l'APK de CI actuel est un keystore **debug** généré à la volée, mot de passe public — signalé audit n°4 A4-12, à nommer « debug » ; épinglage cargo-apk/actions = hygiène backlog) | dès 1ʳᵉ release | Nul |
| Invariant local-first : aucune connexion sortante ni télémétrie non initiée explicitement par l'utilisateur | permanent | Nul (fonctions en ligne = opt-in) |

## Arbitrages à prévoir (à trancher par ADR, pas en silence)

- Plugins natifs « lourds » (accès OCCT direct, temps réel) : concéder via capability dédiée + avertissement, ou rester WASM pur.
- Contrôle machine sur réseau non local (atelier multi-bâtiments) : TLS + auth forte le moment venu.
- Ressources liées dans les imports (images référencées par un SVG) : opt-in à l'import, jamais de téléchargement automatique.

## Rappels process

- Les règles ci-dessus entrent dans leurs ADR respectifs (forge-io, forge-link, plugins) avec la section « sécurité » obligatoire et le coût déclaré.
- Régression interdite sans ADR : toute suppression d'un garde-fou (budgets, sanitize, auth) = ADR explicite.

## Historique

- v1 (2026-10-25) : mise en place du versionnage — n° de version en tête + cette section (protocole : fiche historique).
- v2 (2026-10-26) : statuts + dates sur chaque règle (revue du miroir) — cargo-deny reclassé « rédigé, inactif » (job CI en commentaire, activation jalon 1) ; plafond cumulé 2 GiB des blobs précisé (ADR-0016 A4) ; garde-fous anti-bombe datés de leur vérification.
- v3 (2026-10-29) : correction A4-12 (audit n°4) — le « job en commentaire dans ci.yml » de la v2 n'a jamais existé ; workflow cargo-deny dédié **activé** (décision Raphaël) ; conformité licences deny.toml réécrite (sources recompilables + GPL-3.0+, A4-11) ; APK CI noté debug.
