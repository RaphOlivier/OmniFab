[J3] — A3-08 : validation des entrées forge-cad avant le pont OCCT (audit n°3, specifs a-f) — PR #519, fusion squash `7a8c53a`, 2026-10-31

## Ce qui a été poussé (branche `fix/a3-08-validation-entrees-cad`, squash)

- Rouge test-first `5c05606` : tests unitaires (permissions 0o600 attendues, suppression du temporaire sur erreur ET sur panique) + tests d'intégration (contours dégénérés, hauteur nulle/négative, booléen vide, import non solide) — rouge constaté par l'issue auto #518 AVANT tout correctif : fs::write crée en 0o644, un remove_file manuel ne survit pas à une panique.
- Correctifs : valider_contour (3 points distincts à TOLERANCE_MM, arêtes cycliques non nulles — fermeture comprise, aire de lacet non nulle, coordonnées finies, plafond MAX_POINTS_CONTOUR 4 096) ; valider_dimension finie > 0 strict (hauteur du prisme comprise) ; garde est_vide côté occt.rs (as_solid sinon sonde de tessellation 0,01 mm) : soustraction au résultat vide refusée, volume rendu en valeur absolue ; FichierTemporaire create_new + 0o600, supprimé par garde RAII même sur panique.
- Reprises fix-forward : #522 (E0433 — les modules step/brep référencent FichierTemporaire sans l'importer : un module ne voit pas les items de son parent ; + 2 éclatements fmt repris du fmt.log) puis #523/#524 (E0609 — le Mesh d'opencascade expose `normals`, pas `normales` : la francisation ne vaut que pour l'API publique Maillage ; l'erreur de résolution du run précédent masquait l'erreur de champ).
- Registre : libellé OF-CAD-0002 élargi aux entrées invalides (commit docs séparé).
- Vert : tête `30a7396` — lint-test fmt+clippy+tests+rustdoc ✅, build ubuntu ✅, build macos ✅, check aarch64 ✅, APK ✅ (windows en cours à la fusion, non arbitre — l'arbitre est le job lint-test, conventions v13) ; fusion squash → main `7a8c53a` ; run main à relever.

## Leçons

- Une erreur de résolution (E0433) arrête rustc AVANT le type-checking : un correctif de portée peut révéler un second rouge (E0609) — ne jamais déclarer « compilé » sur la foi d'un seul run rouge.
- Les noms de champs des types d'une dépendance ne se francisent pas : `Mesh.normals` reste anglais, l'API publique (`Maillage.normales`) est française.
- Sandbox Vibe post-réinitialisation plateforme : plus de sleep possible (setTimeout à délai nul seul, budget de processus borné), worker bash mort (FS local inaccessible) → poll CI par rounds courts, capitalisation via le miroir dépôt uniquement, fiche locale à resynchroniser quand le FS revient.

## Dette

- A3-08.e : budget formes_max réel au lot 2 — le binding opencascade 0.3.0 n'itère pas les sous-formes ; la garde actuelle (est_vide sur la forme unique lue) couvre le lot 1.
- Contours auto-sécants non détectés au lot 1 (documenté aux spécifs de tests).

## Suite

Mineurs A3 + hygiène A4-04..13, puis phase B : branchement UI du prisme — les entrées du pont OCCT sont désormais garanties sûres.
