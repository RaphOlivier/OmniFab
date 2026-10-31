[J3] — Dette A3-08 : détection des contours auto-sécants avant le pont OCCT (audit n°3, lot 1) — PR #526, fusion squash `8070f4f`, 2026-10-31

## Ce qui a été poussé (branche `fix/a3-08-contours-auto-secants`, squash)

- Rouge test-first `eeb8d30` : 3 tests ajoutés en premier (croisement — boucle repliée 5 points d'aire non nulle ; contact — sommet posé sur une arête NON adjacente ; garde-fou vert — contour concave sain en L, 6 points) — rouge constaté par l'issue auto #525 AVANT tout correctif : les 2 premiers paniquent sur `unwrap_err` car OCCT extruse une face repliée SANS erreur (le garde-fou concave reste vert, pas d'erreur de compilation ni de diff fmt).
- Correctif `7e3d955` : valider_contour refuse toute paire d'arêtes NON ADJACENTES qui se croise ou se touche — test d'orientation classique (orientation / entre / segments_se_touchent, epsilon d'alignement 1e-12 mm²), O(n²) borné par MAX_POINTS_CONTOUR comme le contrôle de distinctness ; paires adjacentes exclues (fermeture dernier → premier comprise : i == 0 && j == n-1) ; erreur OF-CAD-0002 « contour auto-sécant : arêtes {i} et {j} se croisent » ; docs de valider_contour / prisme / MAX_POINTS_CONTOUR rafraîchies (la mention de dette « pas détectés au lot 1 » est retirée).
- Vert : tête `7e3d955` — lint-test fmt+clippy+tests+rustdoc ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en cours à la fusion, non arbitre — l'arbitre est le job lint-test, conventions v13) ; fusion squash → main `8070f4f` ; issue #525 fermée completed (motif : rouge test-first attendu) ; run main à relever.

## Leçons

- Ouvrir la PR annule les runs push de la branche (groupe de concurrency) : leurs conclusions passent à « cancelled » — l'arbitre se fait sur les check runs de la PR (pull_request_read get_check_runs = voie fiable des conclusions par job), jamais sur une absence d'issue seule.
- Toute garde anti-sécance doit être accompagnée d'un garde-fou vert (contour concave sain accepté) écrit AVANT le correctif — prouver l'absence de sur-refus au même titre que la détection du rouge.
- Worker bash du sandbox revenu : les gros retours d'outils tronqués (fichiers tool-results) redeviennent exploitables via jq — les ancres de capitalisation se vérifient avant le push.

## Dette

- A3-08.e : budget formes_max réel au lot 2 — le binding opencascade 0.3.0 n'itère pas les sous-formes (inchangé ; la garde est_vide sur la forme unique lue couvre le lot 1).

## Suite

Mineurs A3 + hygiène A4-04..13, puis phase B : branchement UI du prisme — les entrées du pont OCCT sont garanties sûres ET non repliées.
