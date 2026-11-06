# 2026-11-05 — [J3] Ordre 1 ADR-0036 : gardes au rejeu (A5-04) + tolérances relatives de valider_contour (A4-16)

**Version de la fiche :** v1 (2026-11-05 — capitalisation de fin d'ordre, discussion Ordre 1.)

## Pourquoi cet ordre

Le lot 3 branchera le noyau OCCT ; le spike phase A avait prouvé qu'OCCT n'apporte AUCUNE garde (abort/segfault sur contour dégénéré, hauteur nulle acceptée). L'audit n°5 a donc exigé, avant le branchement : (A5-04) une garde commune au rejeu — la validation ne vivait qu'à l'écriture (command.rs), un journal hostile contournait toutes les bornes ; (A4-16) des tolérances relatives dans `valider_contour` — les epsilons absolus (EPS_AIRE) acceptaient un triangle (0,0),(1000,0),(500,5e-7) quasi dégénéré quelle que soit son échelle (l'exemple « lame 1 mm × 1e-9 mm » de l'audit 4 était faux : refusé d'emblée par le contrôle d'arêtes). ADR-0036 : critères de sortie explicites, dont « rejeu ×2 → mêmes volumes (déterminisme préservé) ».

## Déroulement (protocole : un commit par point, rouge prouvé avant / vert après)

- **D1 gardes profondeur** `83816be` : `profondeur_valide()` au rejeu (fixe/expression/à travers) — issue #623 fermée au vert avec motif.
- **D2 gardes couleur + « à travers »** `453747b` + reprise rustfmt `f78fabe` : bornes RGBA 0..=1 et enveloppe « à travers » validées comme à l'écriture ; journaux forgés hostiles → feature `EnÉchec` OF-CAD-0005, le corps valide survit — issue #624 fermée.
- **D3 tolérances relatives + prédicats exacts** : rouge `16f67f1` prouvé par CI (run 41998316204) AVANT correctif ; vert `6910406` : recentrage centroïde + hauteur moyenne |lacet|/(2×périmètre) ≥ TOLERANCE_MM, `orientation_exacte`/`est_aligne` via `robust::orient2d` (crate robust 1.2.0, prédicats exacts, licence MIT OR Apache-2.0 vérifiée pour cargo-deny), suppression de `fn orientation` et EPS_AIRE ; rouge `--locked` attendu prouvé (run 42024359370 — le lock est un checkpoint PC), lock poussé par Raphaël `9c59a62` ; reprise clippy `a606521` (double_comparisons : `!(o > 0.0 || o < 0.0)` → `o != 0.0`, suggestion verbatim run 42037511268) ; CI verte intégrale `a542478` — issue #625 fermée au vert avec motif complet (format #624, mention IA).
- **Garde-fou de fin d'ordre** `d562924` : test `rejeu_deux_fois_reconstruit_les_memes_volumes` (forge-session/tests/cad.rs) — journal forgé (corps valide + poche hostile couleur rgba 2.0 hors bornes), conteneur sérialisé UNE fois, deux rejeux indépendants des mêmes octets (`from_bytes_with` + `registre_migrations_courant`) : corps `Ok`, poche `EnÉchec` OF-CAD-0005, corps survivant, volumes identiques bit à bit (`to_bits()`), 4 000 mm³. Vert d'arrivée assumé : le déterminisme est certain (rejeu = fonction pure du journal), le test le verrouille (patron du carré de 1 µm accepté en D3). CI batch PR verte (run 42112006250).

## Débats et décisions

- **macos-latest « cancelled » ×4** : la vague de cancellation (cancel-in-progress) attrape le job au démarrage ; le re-run UI s'annule lui aussi. Jamais un échec de compilation (vert sur `9c59a62`, code identique multi-plateformes) — traité comme orthogonal, Raphaël a validé ce traitement à deux reprises.
- **clippy double_comparisons vs prudence float_cmp** : `!(o > 0.0 || o < 0.0)` semblait plus prudent face à NaN ; clippy exige `o != 0.0` (double_comparisons prime) — NaN rendant les deux fausses, la suggestion verbatim a été suivie.
- **Vert d'arrivée du garde-fou** : pas de rouge test-first pour un garde-fou de non-régression dont le déterminisme est assuré par construction.

## Leçons

1. Les garde-fous in-sandbox protègent aussi contre leurs auteurs : première tentative de push avortée sur une attente erronée (compteur du nom de test), rien n'avait été poussé — relance propre immédiate.
2. Seul un NOUVEAU push ré-arbitre la CI (le re-run « failed jobs » s'annule dans la vague) ; macos est la victime récurrente de la concurrency.
3. L'API ne montre que la dernière tentative par job : vérifier la tête de branche AVANT d'exécuter un plan (verrou de fraîcheur).

## État en fin de discussion

Ordre 1 soldé sur la branche `fix/a5-04-a4-16-gardes-rejeu-tolerances` : les 5 critères de sortie ADR-0036 sont satisfaits (rejeu hostile, écriture, valider_contour, rejeu ×2, CI verte + cargo-deny incluant robust). PR draft #622 (tête `d562924`) : fusion squash + arbitre ci.yml main en discussion dédiée. Suite : ordre 2 A5-01+A4-17 (chaînage en graphe d'extrémités), ordre 3 A5-02+A4-18 + A5-03.

## Liens

ADR-0036 (`docs/adr/ADR-0036-gardes-rejeu-cao-tolerances-relatives.md`) ; PR draft #622 ; issues #623/#624/#625 (fermées completed, motifs avec mention IA) ; fiche état-actuel v76 ; journal des fonctions (2 lignes J3 : `F:cad:gardes-rejeu`, `F:cad:valider-contour-relatif`) ; backlog `preparation/backlog-audit5.md` (miroir privé).
