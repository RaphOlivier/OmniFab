# ADR-0038 : plafonds et bornes — types ignorés, imports par format, features CAO, budget BREP (audit n°5)

- **Statut : accepté** (validé par Raphaël Olivier le 2026-11-03 — choix actés : `MAX_FEATURES` = 10 000 ; `octets_max_brep` = 64 Mio ; refus typé au rejeu au-delà du plafond de features)
- **Date :** 2026-11-03
- **Décideurs :** Raphaël Olivier
- **Phase concernée :** jalon 3 phase B, transverse (forge-session, forge-cad, omnifab-desktop)
- **Origine :** audit extérieur n°5 (Claude, 2026-11-03, code `c36cb15`) — A4-20, A4-19, A5-05, A5-06, ordre 3 du backlog audit n°5 (`preparation/backlog-audit5.md`, miroir privé) ; chaque constat recoupé dans le dépôt avant inclusion (registre de couverture `coverage-ledger.json`)

## Contexte

Quatre zones où une limite existe sur le papier mais pas dans le code :

1. **A4-20** — `TypesIgnores::noter` (forge-session/src/projection.rs) teste le plafond **avant** l'appartenance : au-delà de 256 types vus, un type **déjà connu** retombe dans la branche plafond et est recompté dans `supplementaires`. Les quatre projections (values, geom, esquisse, cad) partagent la structure — le défaut est central.
2. **A4-19** — le fil de travail pré-lit tout fichier d'import sous `MAX_OCTETS_IMPORT = 1 Gio` uniforme (apps/omnifab-desktop/src/travail.rs), alors qu'aucun parseur n'accepte au-delà de 256 Mio par défaut (`StlBudgets`, `SvgBudgets`, `DxfBudgets`, `CadBudgets`) ; `DemandeEchelle` (apps/omnifab-desktop/src/app.rs) conserve une copie intégrale des octets en attendant la réponse d'échelle (ADR-0017) : pic mémoire ≈ 2 Gio.
3. **A5-05** — la projection CAO (forge-session/src/cad.rs) accumule les features sans plafond (un journal borné en octets peut en contenir des dizaines de milliers), `prochaine_id_feature` tronque silencieusement par `len() as u32`, `feature()` est linéaire, et le numéro d'édition croît à **chaque** événement appliqué de tout domaine : un événement d'alias ou d'import de maillage invalide le corps et relance `reconstruire()` (O(n) appels moteur) pour rien.
4. **A5-06** — l'import BREP (forge-cad, `lire_brep`) n'applique aucun budget de formes : `formes_max` n'est vérifié (par `compte_instances`, sur-count conservatoire) que pour STEP ; le binding opencascade 0.3.0 n'itère pas les sous-formes (constat A3-08.e), le comptage de `TShape` est hors d'atteinte, et l'expansion mémoire d'OCCT à partir d'un BREP textuel compact n'est bornée par rien.

**Méthode imposée** (backlog audit n°5) : branche courte + PR, test d'abord (rouge constaté avant le correctif, vert après), un commit par point, CI verte entre chacun.

## Décision

### D1 — A4-20 : appartenance avant plafond dans `TypesIgnores::noter`

`noter()` réordonné : le dédoublonnage précède le plafond — un type déjà connu n'est **jamais** recompté, où qu'il arrive :

```rust
pub(crate) fn noter(&mut self, nom: &str) {
    if self.vus.contains(nom) {
        return; // déjà connu : ignoré, jamais compté (audit n°5 A4-20)
    }
    if self.vus.len() >= MAX_TYPES_INCONNUS {
        self.supplementaires += 1;
        return;
    }
    self.vus.insert(nom.to_string());
    self.retenus.push(nom.to_string());
}
```

Invariants conservés : `vus` et `retenus` ne croissent que ensemble (le plafond garde son sens), dédoublonnage O(1) par `HashSet`, ordre de rencontre des retenus inchangé. Une seule correction suffit : les quatre projections (values, geom, esquisse, cad) partagent `TypesIgnores` et sont couvertes d'office.

*Tests :* 256 types distincts puis re-notage d'un type connu → `supplementaires` inchangé ; type nouveau au-delà du plafond → +1 ; le test existant `types_inconnus_plafonnes` (300 types → 256 retenus, 44 comptés) reste vert sans modification.

### D2 — A4-19 : budget d'import par format, `DemandeEchelle` sans copie d'octets

- **Suppression de la constante uniforme** : `lire_borne` reçoit le budget **du format importé**, lu depuis les défauts des parseurs — une seule source de vérité, zéro chiffre dupliqué au desktop : STL → `StlBudgets::default().max_bytes`, SVG → `SvgBudgets::default().max_bytes`, DXF → `DxfBudgets::default().max_bytes` (256 Mio chacun aujourd'hui), STEP/BREP → `CadBudgets::default().octets_max`. Fonction pure `budget_du_format(format)` ; à l'implémentation, recenser **tous** les appels de `lire_borne` dans `travail.rs` (tracés, maillage, modèles CAO) pour leur passer le budget de leur format — plus aucun 1 Gio.
- **`DemandeEchelle` ne garde que le chemin et la dimension mesurée** — plus de copie d'octets. À la confirmation d'échelle, le fichier est **relu** sous `lire_borne(chemin, budget_du_format)` : la relecture revérifie la taille (le fichier a pu changer entre la mesure et la réponse), et un échec de relecture devient un message d'import — rien au journal dans tous les cas. Pic mémoire : une tranche ≤ 256 Mio au lieu de ~2 Gio (octets d'origine + copie).

*Tests :* `budget_du_format` couvre chaque format et délègue aux défauts des parseurs (la pré-lecture suit toute montée de budget des parseurs, sans duplication à dériver) ; fichier modifié ou effacé entre la mesure et la confirmation → échec propre, aucun crash, aucun journal altéré.

### D3 — A5-05 : `MAX_FEATURES`, accès direct, invalidation ciblée de l'édition CAO

- **Plafond `MAX_FEATURES = 10 000`** (aligné sur `formes_max` de forge-cad et `MAX_ENTITES_CHAINAGE` d'ADR-0037). Refusé à deux frontières :
  - **au plan** (`command.rs::feature_valide`) : `features.len() >= MAX_FEATURES` → refus OF-SES-0001, rien au journal (pattern ADR-0036/ADR-0011 — l'erreur est visible, le projet intact) ;
  - **au rejeu** (`cad.rs::apply`) : un journal qui annonce une MAX+1-ième feature est refusé par une **nouvelle variante typée OF-SES-0007** — le document refuse de s'ouvrir ; un journal hostile ne construit pas dix mille et une features en mémoire.
- **Ids denses ⇒ accès direct** : `apply` garantit déjà `id == features.len()` (garde `InvalidEventData` à l'ajout). `feature(id)` passe de la recherche linéaire à `features.get(id as usize)` — O(1) avec garde de bornes ; pas de carte d'index à maintenir.
- **Cast exact** : sous plafond, `features.len() ≤ 10 000`, le `len() as u32` de `prochaine_id_feature` est exact ; un `debug_assert!` porte l'invariant.
- **Invalidation ciblée** : le numéro d'édition CAO ne croît plus « à chaque événement appliqué » mais **seulement quand la projection CAO change d'état** — événements CAO traités (croquis, features, y compris features marquées en échec), pas les types inconnus (ignorés : l'état ne change pas), pas les autres domaines (alias, expressions, imports). Un `value.input_set` ou un `geom.mesh.imported` n'invalide plus le corps : `reconstruire()` (O(n) appels moteur) n'est plus relancé pour rien (lignage conseil n°8 — budget par frame, cache à clé d'invalidation explicite).

*Tests :* journal forgé de MAX+1 features → OF-SES-0007 à l'ouverture ; commande de feature au plan au plafond → OF-SES-0001, rien au journal, piles undo/redo intactes ; événement d'un autre domaine → le compteur d'appels du moteur de test n'augmente pas (le corps n'est pas reconstruit) ; `feature(id)` retourne la feature en accès direct.

### D4 — A5-06 : budget d'octets BREP distinct

- `CadBudgets` gagne **`octets_max_brep` (défaut 64 Mio)**, vérifié à l'import BREP **avant** tout appel OCCT : le BREP textuel est compact et son expansion en formes OCCT n'est bornée autrement par rien ; 64 Mio de BREP représentent déjà un nombre de formes très au-delà des documents réels. Refus → **OF-CAD-0004** (budget de ressources dépassé — code existant, aucun nouveau code) ; `octets_max` (256 Mio) reste la borne des STEP.
- **Comptage de `TShape` rejeté** : le binding opencascade 0.3.0 n'itère pas les sous-formes (constat A3-08.e déjà acté) — à re-évaluer à la montée du binding (critères fiche veille), pas avant.
- **`formes_max` = STEP only, explicite** : rustdoc de `CadBudgets` et wording du registre le disent mot pour mot ; la fiche état-actuel v74 avait rectifié l'imprécision v73, l'ADR grave la restriction dans le code et sa documentation.

*Tests :* octets BREP au-delà du plafond → OF-CAD-0004 avant tout appel OCCT (miroir du test STEP `budget_octets_depasse_refuse_avant_occt`) ; rustdoc public vérifié par la CI.

### Registre des codes

- **OF-SES-0007** (nouveau) : « Plafond de features dépassé — un document ne porte pas plus de `MAX_FEATURES` (10 000) features CAO » ; cause probable : journal anormal ou forgé ; action corrective : restaurer une copie saine du projet. Inscription **réservée** au registre `docs/error-codes.md` dès cet ADR (pattern OF-VAL-0010 : ligne « code réservé, non encore attribué »), levée à l'implémentation de D3 — la CI bidirectionnelle fait foi.
- **OF-CAD-0004 réutilisé** pour le refus BREP : « Budget de ressources dépassé » couvre déjà « octets, formes ou instances » ; aucune frontière de domaine ne bouge.
- **OF-SES-0001 inchangé** pour le refus au plan (motif : plafond atteint).

## Alternatives rejetées

- **Tolérance + marquage `EnEchec` au rejeu au-delà du plafond** (pattern garde ADR-0036) : préserverait l'ouverture du journal mais laisserait un journal hostile construire des dizaines de milliers de features en mémoire — l'exact contraire du but anti-bombe. Le refus typé est cohérent avec OF-GEO-0009 (plafond de chaînage, ADR-0037) et OF-DOC-0012 (plafond d'entrées du conteneur, ADR-0016).
- **`BTreeMap`/`HashMap` d'index des features** : les ids sont denses par construction (garde `id == len()` à l'apply) — la carte ne servirait qu'à masquer une propriété déjà garantie ; l'accès direct suffit.
- **Comptage de `TShape` pour le BREP** : inaccessible avec opencascade 0.3.0 (A3-08.e) ; le plafond d'octets borne le risque aujourd'hui, le comptage suivra la montée du binding si elle l'expose.
- **Documentation seule « `formes_max` = STEP only »** : ne borne rien — le risque mémoire du BREP resterait entier.
- **Chiffres de budget dupliqués au desktop** (1 Gio ou 256 Mio en dur) : la pré-lecture doit lire les défauts des parseurs ; toute constante dupliquée re-dérive à la prochaine montée de budget.
- **`MAX_FEATURES` = 1 000** : marge serrée sans bénéfice anti-bombe supplémentaire (le refus est déjà typé aux deux frontières) ; **65 536** : trop large pour mériter le nom de plafond. 10 000 aligne les trois constantes du projet (`formes_max`, `MAX_ENTITES_CHAINAGE`).

## Implémentation (checkpoint PC)

Branche courte + PR (le code ne se pousse jamais en direct), test d'abord : rouge constaté, vert après, **un commit par point** (D1 puis D2 puis D3 puis D4 — les plus petits d'abord), CI verte entre chacun. Aucune dépendance nouvelle (pas d'édition de lock). L'implémentation de D3 inscrit OF-SES-0007 au registre (levée du « réservé ») et ajoute sa variante `SessionError`. Le recomptage des `expect` (arbitrage audit n°5) se fait au même checkpoint. L'ordre 4 (tolérances unifiées + proptest, conseils 1 et 2) suit.
