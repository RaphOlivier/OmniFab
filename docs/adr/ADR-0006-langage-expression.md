# ADR-0006 : extensions du langage d'expression (point 10 de la revue)

## Statut

Proposé le 2026-10-13 — validé par Raphaël Olivier le 2026-10-13.
Implémentation différée au jalon 2 (moteur de liaisons complet) ; ce document
fixe les choix pour éviter tout réchauffement de grammaire plus tard.

## Contexte

La revue de code externe (point 10) a constaté que le parseur de forge-values
(crates/forge-values/src/expr.rs) :

- n'accepte que des identifiants ASCII (`is_ascii_alphabetic`) : « epaisseur »
  passe, « épaisseur » est refusé ;
- n'a ni notation scientifique, ni opérateur puissance, ni comparaisons, ni
  conditionnel ;
- n'a **aucune limite de profondeur d'imbrication** : une expression de
  100 000 parenthèses fait déborder la pile du parseur récursif (bug latent,
  confirmé dans le code).

## Décisions

1. **Identifiants Unicode** : le lexer accepte tout caractère alphabétique
   Unicode (`char::is_alphabetic`) + `_` + chiffres. Les identifiants suivent
   la convention UAX #31 (algorithme XID) dès que le besoin apparaît ; pour
   l'instant `is_alphabetic` suffit (pas de normalisation NFC à gérer dans le
   lexer : le nom saisi est le nom stocké).
2. **Notation scientifique** : `1.5e3`, `2E-6` acceptés au lex (le suffixe
   exposant est obligatoirement entier, signé au plus une fois).
3. **Puissance : `^` seul, associatif à droite**, priorité supérieure à
   l'unaire : `2^3^2 = 2^(3^2) = 512`, `2^-2 = 0.25`, `-2^2 = -(2^2) = -4`
   (convention mathématique/tableur). Le résultat non fini reste rejeté par
   `check_finite` (OF-VAL-0004), `0^0 = 1` (convention tableur).
4. **Comparaisons** : `==`, `!=`, `<`, `<=`, `>`, `>=` retournent `1.0`/`0.0`
   (pas de type booléen dans le moteur : les valeurs restent des nombres,
   cf. ADR-0007 phase 1).
5. **Conditionnel : `si(cond; a; b)` seul**, comme fonction arity-3 du
   mécanisme existant — pas de ternaire `?:`. Évaluation **paresseuse** :
   `si` n'évalue que la branche retenue (sinon `1/0` dans la branche morte
   échouerait alors qu'elle n'est pas prise). Exige un traitement spécial du
   collecteur de dépendances : les références des DEUX branches comptent
   comme dépendances du nœud (la branche non évaluée à la définition peut
   l'être plus tard).
6. **Espaces de noms** : `sketch1.largeur` — le point fait partie de
   l'identifiant ; le résolveur reste à plat (le graphe stocke le nom
   complet). L'espace de nommage est une convention des couches métier,
   pas du moteur.
7. **Limite de profondeur : 64 niveaux** d'imbrication (parenthèses et
   appels de fonction). Au-delà : erreur `InvalidExpression` (OF-VAL-0003,
   raison « imbrication trop profonde »). Protège la pile du parseur
   récursif descendant.

## Alternatives rejetées

- **`**` (Python/Rust)** : synonyme de `^` → un seul opérateur, un seul
  comportement ; ajouter un alias plus tard est non-cassant, le retirer
  casserait des journaux.
- **Ternaire `cond ? a : b`** : exige un opérateur à trois branches dans un
  parseur qui n'a que des binaires/ulaires ; le `?` entre en conflit avec de
  futurs caractères (interpolation i18n). La fonction `si` réutilise
  l'arithmétique d'arité existante.
- **Type booléen natif** : rejeté en phase 1 (ADR-0007) ; les comparaisons
  retournent des nombres.
- **Adoption d'un langage existant (CEL, etc.)** : hors sous-ensemble
  déterministe vérifiable, dépendance externe.

## Conséquences

- Le format des expressions dans le journal d'événements est étendu : le
  **schema_version** des événements portant des expressions passera à 2
  (migration des journaux v1 : aucune, la grammaire v1 est incluse dans v2).
- Tests : identifiants Unicode, notation scientifique, associativité `^`,
  paresse de `si`, limite 64 (expression à 65 niveaux rejetée, 64 acceptée).
- Le doc de référence des expressions (rustdoc forge-values) sera mis à
  jour à l'implémentation.

## Amendement (2026-10-29) — bornes de robustesse (audit n°3, A3-02)

La garde de l'audit n°2 (`PROFONDEUR_MAX = 256` niveaux d'imbrication) ne
couvrait pas la chaîne d'opérateurs hostile : « 1+1+1+… » sur 200 000 termes
passe le parseur — sa boucle est itérative — et construit un peigne gauche de
200 000 niveaux que `collect_references`, `evaluate` et le `Drop` de
l'arbre débordent tous. Décisions :

- **Longueur du texte d'expression bornée à 4 096 caractères**
  (`LONGUEUR_MAX_EXPRESSION`, constante publique de forge-values), refusée
  **dans le parseur de la crate** — le journal peut être forgé, la borne ne
  vit pas seulement au plan de la commande — et répliquée au plan de
  `SetExpression` côté session : rien au journal (OF-VAL-0003 / OF-SES-0001).
- **Compteur de nœuds d'AST plafonné à 4 096** dans le parseur : chaque nœud
  consomme au moins un caractère du texte, aucune expression sous la borne de
  longueur ne peut l'atteindre — il garantit l'invariant si la borne de texte
  venait à bouger (OF-VAL-0003).
- La décision 7 (imbrication à 64 niveaux) reste en attente
  d'implémentation : l'implémentation courante garde 256 niveaux.

## Amendement (2026-10-29, second) — profondeur de chaîne bornée (audit n°3, A3-03)

Les bornes du premier amendement couvrent le texte d'une expression, pas
sa chaîne de dépendances : 5 000 alias en ligne (`a1 = a0 + 1`,
`a2 = a1 + 1`, …) restent honnêtes un par un, et l'évaluation récursive
de `a4999` déborde la pile du fil de 2 Mio — abort. Décisions :

- **Profondeur de chaîne bornée à 1 024 niveaux de pile**
  (`PROFONDEUR_MAX_CHAINE`, constante publique de forge-values) : le
  refus vient à la définition, dans `prepare_expression`, par la
  nouvelle erreur typée OF-VAL-0011 — la valeur précédente tient.
- **Métrique : le coût d'évaluation** — la profondeur de l'AST de
  l'expression plus, pour chaque saut de dépendance, le coût de la
  dépendance la plus chère (un nœud d'entrée coûte 1). Calcul itératif
  (pile explicite et mémo) : mesurer une chaîne profonde ne déborde
  jamais la pile. Une chaîne `a_i = a_{i-1} + 1` coûte `3i + 1` : le
  refus tombe au 342e maillon, le 341e s'évalue.
- Le code OF-VAL-0010 reste réservé (table dimensionnelle, design de
  phase D — ADR-0025/0026) : le registre attribue 0011 directement.
