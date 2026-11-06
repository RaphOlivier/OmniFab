//! Segments typés 2D et contours (ADR-0024, phase B lot 1) :
//! lignes, arcs à balayage signé et Béziers — de vraies courbes,
//! jamais des polylignes ; discrétisation pilotée par tolérance
//! (flèche bornée), budgets publics, chaînage en graphe
//! d'extrémités (ADR-0037 D2) : entrée non ordonnée, élagage des
//! queues, refus typé au-delà du degré 2.
//!
//! Un arc porte son balayage en creux : de `debut` à `fin` dans le
//! sens donné — un balayage nul (`debut` == `fin`) dénote le cercle
//! complet. La discrétisation d'un arc calcule son nombre de points
//! en forme fermée depuis la flèche ; celle d'une Bézier subdivise à
//! la demande (de Casteljau) jusqu'à la flèche satisfaite. Au-delà
//! des budgets publics, [`Segment2::discretiser`] refuse
//! (OF-GEO-0005) et la version bornée rend l'approximation au
//! budget — le rendu survit toujours.

use std::collections::HashMap;

use forge_units::{Angle, Length, TOLERANCE_MM};
use serde::{Deserialize, Serialize};

use crate::{GeomError, Point2};

/// Tolérance par défaut de discrétisation, en millimètres : la
/// flèche de chaque corde reste sous 0,01 mm (ADR-0024 D1).
pub const TOLERANCE_DISCRETISATION_MM: f64 = 0.01;

/// Profondeur maximale de subdivision d'une Bézier (ADR-0024 D1) :
/// au-delà, la discrétisation refuse (OF-GEO-0005).
pub const MAX_PROFONDEUR_SUBDIVISION: u32 = 16;

/// Nombre maximal de points de la discrétisation d'un arc
/// (ADR-0024 D1) : au-delà, la discrétisation refuse
/// (OF-GEO-0005).
pub const MAX_POINTS_DISCRETISATION: usize = 65_536;

/// Sens de balayage d'un arc (ADR-0024 D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensHoraire {
    /// Sens trigonométrique (anti-horaire), angles croissants.
    Trigonometrique,
    /// Sens horaire, angles décroissants.
    Horaire,
}

impl SensHoraire {
    /// Sens de balayage opposé (ADR-0037 D2 — le chaînage retourne
    /// les segments qui vont à contre-sens du cycle extrait).
    pub fn inverse(self) -> Self {
        match self {
            Self::Trigonometrique => Self::Horaire,
            Self::Horaire => Self::Trigonometrique,
        }
    }
}

/// Segment typé 2D (ADR-0024 D1) : ligne, arc à balayage signé ou
/// Bézier — une vraie courbe, jamais une polyligne. Sérialisable :
/// les tracés importés circulent tel quel entre sessions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Segment2 {
    /// Segment de droite entre deux points.
    Ligne {
        /// Départ du segment.
        p1: Point2,
        /// Arrivée du segment.
        p2: Point2,
    },
    /// Arc de cercle : de `debut` à `fin` dans le sens donné, un
    /// balayage nul (debut == fin) dénote le cercle complet.
    Arc {
        /// Centre de l'arc.
        centre: Point2,
        /// Rayon de l'arc, strictement positif.
        rayon: Length,
        /// Angle de départ.
        debut: Angle,
        /// Angle d'arrivée.
        fin: Angle,
        /// Sens de balayage.
        sens: SensHoraire,
    },
    /// Courbe de Bézier quadratique (un point de contrôle).
    BezierQuadratique {
        /// Départ de la courbe.
        p1: Point2,
        /// Point de contrôle.
        controle: Point2,
        /// Arrivée de la courbe.
        p2: Point2,
    },
    /// Courbe de Bézier cubique (deux points de contrôle).
    BezierCubique {
        /// Départ de la courbe.
        p1: Point2,
        /// Premier point de contrôle.
        c1: Point2,
        /// Second point de contrôle.
        c2: Point2,
        /// Arrivée de la courbe.
        p2: Point2,
    },
}

impl Segment2 {
    /// Point de départ du segment.
    pub fn debut(&self) -> Point2 {
        match self {
            Self::Ligne { p1, .. }
            | Self::BezierQuadratique { p1, .. }
            | Self::BezierCubique { p1, .. } => *p1,
            Self::Arc {
                centre,
                rayon,
                debut,
                ..
            } => point_d_angle(*centre, rayon.millimeters(), debut.radians()).unwrap_or(*centre),
        }
    }

    /// Point d'arrivée du segment.
    pub fn fin(&self) -> Point2 {
        match self {
            Self::Ligne { p2, .. }
            | Self::BezierQuadratique { p2, .. }
            | Self::BezierCubique { p2, .. } => *p2,
            Self::Arc {
                centre, rayon, fin, ..
            } => point_d_angle(*centre, rayon.millimeters(), fin.radians()).unwrap_or(*centre),
        }
    }

    /// Segment parcouru en sens inverse : mêmes courbe et
    /// extrémités, seul le sens de parcours change (ADR-0037 D2 —
    /// le chaînage retourne les segments à contre-sens du cycle
    /// extrait).
    pub fn inverse(&self) -> Self {
        match *self {
            Self::Ligne { p1, p2 } => Self::Ligne { p1: p2, p2: p1 },
            Self::Arc {
                centre,
                rayon,
                debut,
                fin,
                sens,
            } => Self::Arc {
                centre,
                rayon,
                debut: fin,
                fin: debut,
                sens: sens.inverse(),
            },
            Self::BezierQuadratique { p1, controle, p2 } => Self::BezierQuadratique {
                p1: p2,
                controle,
                p2: p1,
            },
            Self::BezierCubique { p1, c1, c2, p2 } => Self::BezierCubique {
                p1: p2,
                c1: c2,
                c2: c1,
                p2: p1,
            },
        }
    }

    /// Discrétisation stricte à la tolérance donnée : les deux
    /// extrémités sont incluses, chaque corde garde la courbe à la
    /// tolérance près (flèche ≤ tolerance). Refus typés : tolérance
    /// ou arc impropre (OF-GEO-0004), budget dépassé
    /// (OF-GEO-0005).
    pub fn discretiser(&self, tolerance: Length) -> Result<Vec<Point2>, GeomError> {
        verifier_tolerance(tolerance)?;
        let (discretisee, depasse) = self.discretisation(tolerance)?;
        if depasse {
            let raison = match self {
                Self::BezierQuadratique { .. } | Self::BezierCubique { .. } => {
                    "profondeur de subdivision au-delà de la borne"
                }
                _ => "nombre de points au-delà de la borne",
            };
            return Err(GeomError::BudgetDiscretisationDepasse { raison });
        }
        Ok(discretisee)
    }

    /// Discrétisation bornée : jamais d'erreur — un segment
    /// impropre ou au-delà du budget rend son approximation au
    /// budget ([`Segment2::debut`] et [`Segment2::fin`] au pire),
    /// le rendu survit.
    fn discretiser_borne(&self, tolerance: Length) -> Vec<Point2> {
        match self.discretisation(tolerance) {
            Ok((discretisee, _)) => discretisee,
            Err(_) => match self {
                Self::Arc { centre, .. } => vec![*centre, *centre],
                _ => vec![self.debut(), self.fin()],
            },
        }
    }

    /// Discrétisation interne : les points, et un drapeau de
    /// dépassement de budget. La tolérance est réputée valide.
    fn discretisation(&self, tolerance: Length) -> Result<(Vec<Point2>, bool), GeomError> {
        match self {
            Self::Ligne { p1, p2 } => Ok((vec![*p1, *p2], false)),
            Self::Arc {
                centre,
                rayon,
                debut,
                fin,
                sens,
            } => {
                let r = rayon.millimeters();
                if !r.is_finite() || r <= 0.0 {
                    return Err(GeomError::DiscretisationImpossible {
                        raison: "rayon d'arc nul, négatif ou non fini",
                    });
                }
                if !debut.radians().is_finite() || !fin.radians().is_finite() {
                    return Err(GeomError::DiscretisationImpossible {
                        raison: "angle d'arc non fini",
                    });
                }
                let balayage = balayage_signe(*debut, *fin, *sens);
                // Flèche d'une corde : r·(1 − cos(balayage/2n)) ≤ tol
                // → n ≥ balayage / (2·arccos(1 − tol/r)).
                let demi = (1.0 - tolerance.millimeters() / r).clamp(-1.0, 1.0).acos();
                let nombre = if demi <= 1e-12 {
                    MAX_POINTS_DISCRETISATION + 1
                } else {
                    ((balayage.abs() / (2.0 * demi)).ceil() as usize).max(1)
                };
                let depasse = nombre > MAX_POINTS_DISCRETISATION;
                let n = nombre.min(MAX_POINTS_DISCRETISATION);
                let mut discretisee = Vec::with_capacity(n + 1);
                for i in 0..=n {
                    let angle = debut.radians() + balayage * i as f64 / n as f64;
                    discretisee.push(point_d_angle(*centre, r, angle)?);
                }
                Ok((discretisee, depasse))
            }
            Self::BezierQuadratique { p1, controle, p2 } => {
                let mut discretisee = vec![*p1];
                let depasse =
                    aplatit_bezier(&[*p1, *controle, *p2], tolerance, &mut discretisee, 0);
                Ok((discretisee, depasse))
            }
            Self::BezierCubique { p1, c1, c2, p2 } => {
                let mut discretisee = vec![*p1];
                let depasse = aplatit_bezier(&[*p1, *c1, *c2, *p2], tolerance, &mut discretisee, 0);
                Ok((discretisee, depasse))
            }
        }
    }
}

/// Balayage signé d'un arc, en radians : positif en sens
/// trigonométrique, négatif en sens horaire ; nul apparent
/// (`debut` == `fin`) dénote le cercle complet.
fn balayage_signe(debut: Angle, fin: Angle, sens: SensHoraire) -> f64 {
    let tour = 2.0 * std::f64::consts::PI;
    match sens {
        SensHoraire::Trigonometrique => {
            let positif = (fin.radians() - debut.radians()).rem_euclid(tour);
            if positif <= 1e-12 { tour } else { positif }
        }
        SensHoraire::Horaire => {
            let negatif = (debut.radians() - fin.radians()).rem_euclid(tour);
            if negatif <= 1e-12 { -tour } else { -negatif }
        }
    }
}

/// Point sur le cercle de centre et rayon donnés, à l'angle donné.
/// Un point hors de la borne de magnitude est refusé
/// (OF-GEO-0003).
fn point_d_angle(centre: Point2, rayon: f64, angle: f64) -> Result<Point2, GeomError> {
    let x = Length::from_millimeters(centre.x().millimeters() + rayon * angle.cos());
    let y = Length::from_millimeters(centre.y().millimeters() + rayon * angle.sin());
    Point2::new(x, y)
}

/// Tolérance de discrétisation valide : strictement positive et
/// finie (OF-GEO-0004 sinon).
fn verifier_tolerance(tolerance: Length) -> Result<(), GeomError> {
    if !tolerance.millimeters().is_finite() || tolerance.millimeters() <= 0.0 {
        return Err(GeomError::DiscretisationImpossible {
            raison: "tolérance nulle, négative ou non finie",
        });
    }
    Ok(())
}

/// Distance d'un point à la droite portant la corde premier–dernier
/// (0 si la corde est dégénérée : distance euclidienne).
fn ecart_a_la_corde(point: Point2, premier: Point2, dernier: Point2) -> f64 {
    let vx = dernier.x().millimeters() - premier.x().millimeters();
    let vy = dernier.y().millimeters() - premier.y().millimeters();
    let wx = point.x().millimeters() - premier.x().millimeters();
    let wy = point.y().millimeters() - premier.y().millimeters();
    let corde = (vx * vx + vy * vy).sqrt();
    if corde <= 1e-12 {
        return (wx * wx + wy * wy).sqrt();
    }
    (vx * wy - vy * wx).abs() / corde
}

/// Milieu de deux points. Invariant : le milieu de deux points
/// dans la borne de magnitude y reste — le repli est inatteignable.
fn milieu(a: Point2, b: Point2) -> Point2 {
    let x = Length::from_millimeters((a.x().millimeters() + b.x().millimeters()) / 2.0);
    let y = Length::from_millimeters((a.y().millimeters() + b.y().millimeters()) / 2.0);
    Point2::new(x, y).unwrap_or(a)
}

/// Aplatit une Bézier (de Casteljau adaptatif) : chaque moitié
/// reprend les bords des couches de milieux, jusqu'à la flèche
/// satisfaite sous la borne de profondeur. Renvoie le dépassement
/// de budget (approximation bornée alors).
fn aplatit_bezier(
    controles: &[Point2],
    tolerance: Length,
    discretisee: &mut Vec<Point2>,
    profondeur: u32,
) -> bool {
    let premier = controles[0];
    let dernier = controles[controles.len() - 1];
    // La flèche d'une Bézier bornée : ¾ de l'écart des contrôles
    // internes à la corde pour une cubique, ½ pour une quadratique
    // — le seuil 4/3·tol garantit flèche ≤ tol dans les deux cas.
    let seuil = tolerance.millimeters() * 4.0 / 3.0;
    let plat = controles[1..controles.len() - 1]
        .iter()
        .all(|controle| ecart_a_la_corde(*controle, premier, dernier) <= seuil);
    if plat {
        discretisee.push(dernier);
        return false;
    }
    if profondeur >= MAX_PROFONDEUR_SUBDIVISION {
        discretisee.push(dernier);
        return true;
    }
    // de Casteljau en t = ½ : les bords des couches de milieux
    // donnent les contrôles des deux moitiés.
    let mut couches: Vec<Vec<Point2>> = vec![controles.to_vec()];
    while let Some(precedente) = couches.last()
        && precedente.len() > 1
    {
        let suivante: Vec<Point2> = precedente
            .windows(2)
            .map(|paire| milieu(paire[0], paire[1]))
            .collect();
        couches.push(suivante);
    }
    let gauche: Vec<Point2> = couches.iter().map(|couche| couche[0]).collect();
    let droite: Vec<Point2> = couches
        .iter()
        .rev()
        .map(|couche| couche[couche.len() - 1])
        .collect();
    let depasse_gauche = aplatit_bezier(&gauche, tolerance, discretisee, profondeur + 1);
    let depasse_droite = aplatit_bezier(&droite, tolerance, discretisee, profondeur + 1);
    depasse_gauche | depasse_droite
}

/// Vrai quand deux points sont recollés à la tolérance
/// géométrique près (1e-6 mm) — une fermeture recollée n'ajoute pas
/// de sommet redondant.
pub fn points_recolles(a: Point2, b: Point2) -> bool {
    let dx = a.x().millimeters() - b.x().millimeters();
    let dy = a.y().millimeters() - b.y().millimeters();
    (dx * dx + dy * dy).sqrt() <= TOLERANCE_MM
}

/// Vrai quand la chaîne se referme : l'arrivée du dernier segment
/// rejoint le départ du premier, à la tolérance géométrique près.
pub(crate) fn fermeture(segments: &[Segment2]) -> bool {
    match (segments.first(), segments.last()) {
        (Some(premier), Some(dernier)) => points_recolles(dernier.fin(), premier.debut()),
        _ => false,
    }
}

/// Erreur de chaînage de contour : les refus de segments
/// (OF-GEO-0004/0005), l'embranchement (OF-GEO-0008) et le
/// plafond d'entités (OF-GEO-0009) portent des codes stables au
/// registre `docs/error-codes.md`.
pub type ErreurContour = GeomError;

/// Contour 2D : chaîne de segments dont chacun part de l'arrivée
/// du précédent, à la tolérance géométrique près ; fermé quand le
/// dernier segment rejoint le premier (ADR-0024 D1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Contour2 {
    /// Segments enchaînés, dans l'ordre du contour.
    segments: Vec<Segment2>,
}

impl Contour2 {
    /// Segments du contour, dans l'ordre.
    pub fn segments(&self) -> &[Segment2] {
        &self.segments
    }

    /// Le contour est fermé : le dernier segment rejoint le
    /// premier, à la tolérance géométrique près.
    pub fn is_closed(&self) -> bool {
        fermeture(&self.segments)
    }

    /// Nombre de sommets de la chaîne : autant que de segments sur
    /// un contour fermé, un de plus sur un contour ouvert.
    pub fn len(&self) -> usize {
        if self.segments.is_empty() {
            0
        } else if self.is_closed() {
            self.segments.len()
        } else {
            self.segments.len() + 1
        }
    }

    /// Contour sans segment.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Discrétisation stricte du contour à la tolérance donnée :
    /// chaîne de points recollés, sans répétition du premier sommet
    /// en fermeture ; propage les refus des segments
    /// (OF-GEO-0004/0005).
    pub fn discretiser(&self, tolerance: Length) -> Result<Vec<Point2>, GeomError> {
        chaine_discretisee(&self.segments, tolerance, true)
    }
}

/// Nombre maximal d'entités soumises au chaînage (ADR-0037 D2) :
/// au-delà, le chaînage refuse (OF-GEO-0009). Plafond public —
/// les appelants peuvent l'afficher à l'utilisateur.
pub const MAX_ENTITES_CHAINAGE: usize = 10_000;

/// Soudure d'une extrémité au sommet recollé le plus proche, à la
/// tolérance géométrique près — index spatial en grille au pas de
/// la tolérance (O(n) attendu) ; sans sommet recollé, l'extrémité
/// crée le sien. Renvoie l'indice du sommet soudé.
fn soude(
    extremite: Point2,
    sommets: &mut Vec<Point2>,
    grille: &mut HashMap<(i64, i64), Vec<usize>>,
) -> usize {
    let cellule = |coordonnee: f64| (coordonnee / TOLERANCE_MM).floor() as i64;
    let (cx, cy) = (
        cellule(extremite.x().millimeters()),
        cellule(extremite.y().millimeters()),
    );
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(voisins) = grille.get(&(cx + dx, cy + dy)) {
                for &sommet in voisins {
                    if points_recolles(sommets[sommet], extremite) {
                        return sommet;
                    }
                }
            }
        }
    }
    sommets.push(extremite);
    let sommet = sommets.len() - 1;
    grille.entry((cx, cy)).or_default().push(sommet);
    sommet
}

/// Enchaîne des segments en contours par graphe d'extrémités
/// (ADR-0037 D2) : l'entrée est **non ordonnée**. Les extrémités
/// sont soudées à la tolérance géométrique, les queues (arêtes à
/// extrémité libre) sont élaguées itérativement — le carré à queue
/// rend son carré ; tout sommet réunissant plus de deux extrémités
/// est un embranchement refusé (OF-GEO-0008), jamais tranché
/// arbitrairement. Les composantes restantes sont des cycles
/// fermés : un `Contour2` chacune, les segments à contre-sens
/// retournés pour suivre le sens de parcours. Au-delà du plafond
/// [`MAX_ENTITES_CHAINAGE`], refus typé (OF-GEO-0009) ; l'entrée
/// vide rend aucun contour.
pub fn contours(segments: &[Segment2]) -> Result<Vec<Contour2>, ErreurContour> {
    if segments.is_empty() {
        return Ok(Vec::new());
    }
    if segments.len() > MAX_ENTITES_CHAINAGE {
        return Err(GeomError::PlafondChainageDepasse {
            entites: segments.len(),
            plafond: MAX_ENTITES_CHAINAGE,
        });
    }
    // Soudure des extrémités : un sommet par grappe recollée, une
    // arête orientée par segment.
    let mut sommets: Vec<Point2> = Vec::with_capacity(segments.len() * 2);
    let mut grille: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let aretes: Vec<(usize, usize)> = segments
        .iter()
        .map(|segment| {
            let depart = soude(segment.debut(), &mut sommets, &mut grille);
            let arrivee = soude(segment.fin(), &mut sommets, &mut grille);
            (depart, arrivee)
        })
        .collect();
    // Arêtes incidentes par sommet ; une boucle (extrémités soudées
    // au même sommet) compte deux fois.
    let mut incident: Vec<Vec<usize>> = vec![Vec::new(); sommets.len()];
    for (arete, &(depart, arrivee)) in aretes.iter().enumerate() {
        incident[depart].push(arete);
        incident[arrivee].push(arete);
    }
    let mut degres: Vec<usize> = incident.iter().map(Vec::len).collect();
    // Élagage itératif des queues : l'arête d'un sommet à
    // extrémité libre disparaît, ses voisines peuvent à leur tour
    // se retrouver libres — jusqu'à stabilisation.
    let mut vivante = vec![true; aretes.len()];
    let mut pile: Vec<usize> = (0..sommets.len()).filter(|&s| degres[s] == 1).collect();
    while let Some(s) = pile.pop() {
        if degres[s] != 1 {
            continue;
        }
        let Some(&arete) = incident[s].iter().find(|a| vivante[*a]) else {
            continue;
        };
        vivante[arete] = false;
        let (depart, arrivee) = aretes[arete];
        for sommet in [depart, arrivee] {
            degres[sommet] -= 1;
            if degres[sommet] == 1 {
                pile.push(sommet);
            }
        }
    }
    // Après l'élagage, tout sommet de degré supérieur à deux est
    // un vrai embranchement (OF-GEO-0008).
    for (s, &degre) in degres.iter().enumerate() {
        if degre > 2 {
            return Err(GeomError::EmbranchementContour {
                x: sommets[s].x(),
                y: sommets[s].y(),
                degre,
            });
        }
    }
    // Les composantes restantes sont des cycles : un parcours
    // orienté par cycle, retourner les segments à contre-sens.
    let mut cycles = Vec::new();
    for premiere in 0..aretes.len() {
        if !vivante[premiere] {
            continue;
        }
        let depart_initial = aretes[premiere].0;
        let mut cycle = Vec::new();
        let mut sommet = depart_initial;
        let mut arete = premiere;
        for _ in 0..aretes.len() {
            vivante[arete] = false;
            let segment = segments[arete];
            let (depart, arrivee) = aretes[arete];
            if depart == sommet {
                cycle.push(segment);
                sommet = arrivee;
            } else {
                cycle.push(segment.inverse());
                sommet = depart;
            }
            if sommet == depart_initial {
                break;
            }
            let Some(&suivante) = incident[sommet].iter().find(|a| vivante[*a]) else {
                break;
            };
            arete = suivante;
        }
        cycles.push(Contour2 { segments: cycle });
    }
    Ok(cycles)
}

/// Chaîne discrétisée d'une suite de segments : chaque segment est
/// discrétisé à la tolérance, les extrémités recollées ne se
/// répètent pas, la fermeture ne répète pas le premier sommet. En
/// mode strict les refus se propagent ; en mode borné, chaque
/// segment rend son approximation au budget.
pub(crate) fn chaine_discretisee(
    segments: &[Segment2],
    tolerance: Length,
    stricte: bool,
) -> Result<Vec<Point2>, GeomError> {
    let Some(premier) = segments.first() else {
        return Ok(Vec::new());
    };
    let mut discretisee = vec![premier.debut()];
    for segment in segments {
        let points = if stricte {
            segment.discretiser(tolerance)?
        } else {
            segment.discretiser_borne(tolerance)
        };
        for point in points.iter().skip(1) {
            if let Some(dernier) = discretisee.last()
                && !points_recolles(*dernier, *point)
            {
                discretisee.push(*point);
            }
        }
    }
    // Fermeture recollée : le premier sommet ne se répète pas.
    if fermeture(segments)
        && let Some(dernier) = discretisee.last()
        && points_recolles(discretisee[0], *dernier)
    {
        discretisee.pop();
    }
    Ok(discretisee)
}

/// Chaîne discrétisée bornée : jamais d'erreur — en mode borné
/// chaque segment est infaillible, le repli vide est inatteignable.
pub(crate) fn chaine_bornee(segments: &[Segment2], tolerance: Length) -> Vec<Point2> {
    chaine_discretisee(segments, tolerance, false).unwrap_or_default()
}
#[cfg(test)]
mod tests {
    use super::*;
    use forge_units::{Angle, Length};

    /// Point de test aux coordonnées données (mm).
    fn point(x: f64, y: f64) -> Point2 {
        let abscisse = Length::from_millimeters(x);
        let ordonnee = Length::from_millimeters(y);
        Point2::new(abscisse, ordonnee).expect("coordonnées finies")
    }

    /// Ligne de test entre deux points (mm).
    fn ligne(x1: f64, y1: f64, x2: f64, y2: f64) -> Segment2 {
        Segment2::Ligne {
            p1: point(x1, y1),
            p2: point(x2, y2),
        }
    }

    /// Distance d'un point au segment [a, b].
    fn distance_au_segment(p: Point2, a: Point2, b: Point2) -> f64 {
        let vx = b.x().millimeters() - a.x().millimeters();
        let vy = b.y().millimeters() - a.y().millimeters();
        let wx = p.x().millimeters() - a.x().millimeters();
        let wy = p.y().millimeters() - a.y().millimeters();
        let corde2 = vx * vx + vy * vy;
        if corde2 <= 1e-24 {
            return (wx * wx + wy * wy).sqrt();
        }
        let t = ((wx * vx + wy * vy) / corde2).clamp(0.0, 1.0);
        let dx = wx - t * vx;
        let dy = wy - t * vy;
        (dx * dx + dy * dy).sqrt()
    }

    /// Flèche maximale de la discrétisation : distance de la courbe
    /// exacte (échantillonnée à 1 001 points) à la polyligne.
    fn fleche_maximale(echantillons: &[Point2], discretise: &[Point2]) -> f64 {
        let mut pire: f64 = 0.0;
        for point in echantillons {
            let mut distance = f64::INFINITY;
            for paire in discretise.windows(2) {
                distance = distance.min(distance_au_segment(*point, paire[0], paire[1]));
            }
            pire = pire.max(distance);
        }
        pire
    }

    #[test]
    fn ligne_discretisee_reste_ses_extremites() {
        let segment = ligne(0.0, 0.0, 10.0, 5.0);
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = segment.discretiser(tolerance).expect("ligne valide");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].x().millimeters(), 0.0);
        assert_eq!(points[1].y().millimeters(), 5.0);
    }

    #[test]
    fn fleche_d_un_arc_sous_tolerance() {
        // Arc trigonométrique de rayon 10, de 0 à 180 degrés.
        let segment = Segment2::Arc {
            centre: point(0.0, 0.0),
            rayon: Length::from_millimeters(10.0),
            debut: Angle::ZERO,
            fin: Angle::from_degrees(180.0),
            sens: SensHoraire::Trigonometrique,
        };
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let discretise = segment.discretiser(tolerance).expect("arc valide");
        // Extrémités exactes.
        assert_eq!(discretise[0].x().millimeters(), 10.0);
        assert_eq!(discretise[0].y().millimeters(), 0.0);
        let dernier = discretise.last().expect("au moins deux points");
        assert_eq!(dernier.x().millimeters(), -10.0);
        assert!(dernier.y().millimeters().abs() < 1e-9);
        // Flèche : la courbe exacte reste à la tolérance de la polyligne.
        let echantillons: Vec<Point2> = (0..=1000)
            .map(|i| {
                let angle = std::f64::consts::PI * i as f64 / 1000.0;
                point(10.0 * angle.cos(), 10.0 * angle.sin())
            })
            .collect();
        let fleche = fleche_maximale(&echantillons, &discretise);
        assert!(
            fleche <= TOLERANCE_DISCRETISATION_MM + 1e-9,
            "flèche {fleche} au-delà de la tolérance"
        );
    }

    #[test]
    fn cercle_complet_debut_egal_fin() {
        // Balayage nul : l'arc dénote le cercle complet, discrétisé
        // du point de départ au point de départ.
        let segment = Segment2::Arc {
            centre: point(1.0, 2.0),
            rayon: Length::from_millimeters(5.0),
            debut: Angle::from_degrees(90.0),
            fin: Angle::from_degrees(90.0),
            sens: SensHoraire::Trigonometrique,
        };
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = segment.discretiser(tolerance).expect("cercle valide");
        let premier = points[0];
        let dernier = points[points.len() - 1];
        assert!(points.len() > 8, "cercle discrétisé : {}", points.len());
        // cos(π/2) n'est pas nul en f64 : le sommet du cercle se
        // vérifie à la tolérance près.
        assert!((premier.x().millimeters() - 1.0).abs() < 1e-9);
        assert_eq!(premier.y().millimeters(), 7.0);
        assert!(points_recolles(premier, dernier), "le cercle se referme");
    }

    #[test]
    fn fleche_d_une_bezier_sous_tolerance() {
        let segment = Segment2::BezierCubique {
            p1: point(0.0, 0.0),
            c1: point(0.0, 10.0),
            c2: point(20.0, 10.0),
            p2: point(20.0, 0.0),
        };
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let discretise = segment.discretiser(tolerance).expect("bézier valide");
        let echantillons: Vec<Point2> = (0..=1000)
            .map(|i| {
                let t = i as f64 / 1000.0;
                let u = 1.0 - t;
                let x = 20.0 * (3.0 * u * t * t + t * t * t);
                let y = 10.0 * (3.0 * u * u * t + 3.0 * u * t * t);
                point(x, y)
            })
            .collect();
        let fleche = fleche_maximale(&echantillons, &discretise);
        assert!(
            fleche <= TOLERANCE_DISCRETISATION_MM + 1e-9,
            "flèche {fleche} au-delà de la tolérance"
        );
    }

    #[test]
    fn tolerance_invalide_refusee() {
        let segment = ligne(0.0, 0.0, 10.0, 0.0);
        let erreur = segment
            .discretiser(Length::ZERO)
            .expect_err("tolérance nulle refusée");
        assert_eq!(erreur.code(), "OF-GEO-0004");
    }

    #[test]
    fn arc_de_rayon_invalide_refuse() {
        let segment = Segment2::Arc {
            centre: point(0.0, 0.0),
            rayon: Length::ZERO,
            debut: Angle::ZERO,
            fin: Angle::from_degrees(90.0),
            sens: SensHoraire::Trigonometrique,
        };
        let erreur = segment
            .discretiser(Length::from_millimeters(TOLERANCE_DISCRETISATION_MM))
            .expect_err("rayon nul refusé");
        assert_eq!(erreur.code(), "OF-GEO-0004");
    }

    #[test]
    fn budget_de_subdivision_depasse() {
        // Tolérance démesurée face à une vraie courbe : la
        // subdivision atteint sa borne de profondeur.
        let segment = Segment2::BezierQuadratique {
            p1: point(0.0, 0.0),
            controle: point(10.0, 30.0),
            p2: point(20.0, 0.0),
        };
        let erreur = segment
            .discretiser(Length::from_millimeters(1e-12))
            .expect_err("budget de subdivision dépassé");
        assert_eq!(erreur.code(), "OF-GEO-0005");
    }

    #[test]
    fn budget_de_points_d_un_arc_depasse() {
        // Rayon démesuré face à la tolérance : satisfaire la flèche
        // exigerait plus de points que le budget public.
        let segment = Segment2::Arc {
            centre: point(0.0, 0.0),
            rayon: Length::from_millimeters(1e8),
            debut: Angle::ZERO,
            fin: Angle::from_degrees(180.0),
            sens: SensHoraire::Trigonometrique,
        };
        let erreur = segment
            .discretiser(Length::from_millimeters(1e-9))
            .expect_err("budget de points dépassé");
        assert_eq!(erreur.code(), "OF-GEO-0005");
    }

    #[test]
    fn chaine_rectangle_fermee() {
        let rectangle = [
            ligne(0.0, 0.0, 10.0, 0.0),
            ligne(10.0, 0.0, 10.0, 10.0),
            ligne(10.0, 10.0, 0.0, 10.0),
            ligne(0.0, 10.0, 0.0, 0.0),
        ];
        let chains = contours(&rectangle).expect("chaîne continue");
        assert_eq!(chains.len(), 1);
        let contour = &chains[0];
        assert!(contour.is_closed(), "le rectangle se referme");
        assert_eq!(contour.len(), 4, "quatre sommets");
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = contour.discretiser(tolerance).expect("discrétisation");
        assert_eq!(
            points.len(),
            4,
            "les fermetures recollées ne se répètent pas"
        );
        assert_eq!(points[0].x().millimeters(), 0.0);
        assert_eq!(points[1].y().millimeters(), 0.0);
        assert_eq!(points[2].x().millimeters(), 10.0);
        assert_eq!(points[3].y().millimeters(), 10.0);
    }

    #[test]
    fn ecart_reel_composantes_distinctes() {
        // ADR-0037 D2 : OF-GEO-0006 est retiré — un écart n'est
        // plus une erreur mais des composantes distinctes ; sans
        // boucle fermée, aucun contour ne sort du chaînage.
        let disjointes = [ligne(0.0, 0.0, 10.0, 0.0), ligne(12.0, 0.0, 12.0, 5.0)];
        let chains = contours(&disjointes).expect("composantes distinctes");
        assert!(chains.is_empty(), "des queues ne font pas un contour");
    }

    #[test]
    fn chaine_ouverte_elaguee_sans_contour() {
        // ADR-0037 D2 : une chaîne ouverte est une queue —
        // élaguée itérativement, elle ne laisse aucun contour.
        let chaine = [ligne(0.0, 0.0, 10.0, 0.0), ligne(10.0, 0.0, 10.0, 10.0)];
        let chains = contours(&chaine).expect("queue élaguée");
        assert!(chains.is_empty(), "une queue ne referme pas un contour");
    }

    #[test]
    fn chaine_vide_sans_contour() {
        let chains = contours(&[]).expect("aucun segment");
        assert!(chains.is_empty());
    }

    #[test]
    fn carres_secs_en_un_sommet_refuses() {
        // ADR-0037 D2 : deux boucles partageant un sommet — le
        // sommet réunit quatre extrémités, embranchement refusé
        // (OF-GEO-0008), jamais un contour fusionné « en huit ».
        let carres = [
            ligne(0.0, 0.0, 1.0, 0.0),
            ligne(1.0, 0.0, 1.0, 1.0),
            ligne(1.0, 1.0, 0.0, 1.0),
            ligne(0.0, 1.0, 0.0, 0.0),
            ligne(1.0, 1.0, 2.0, 1.0),
            ligne(2.0, 1.0, 2.0, 2.0),
            ligne(2.0, 2.0, 1.0, 2.0),
            ligne(1.0, 2.0, 1.0, 1.0),
        ];
        let erreur = contours(&carres).expect_err("embranchement refusé");
        assert_eq!(erreur.code(), "OF-GEO-0008");
    }

    #[test]
    fn carre_avec_queue_rend_son_carre() {
        // ADR-0037 D2 : la queue (extrémité libre) est élaguée
        // avant le contrôle d'embranchement — le carré demeure.
        let carre_queue = [
            ligne(0.0, 0.0, 1.0, 0.0),
            ligne(1.0, 0.0, 1.0, 1.0),
            ligne(1.0, 1.0, 0.0, 1.0),
            ligne(0.0, 1.0, 0.0, 0.0),
            ligne(1.0, 1.0, 2.0, 1.0),
        ];
        let chains = contours(&carre_queue).expect("queue élaguée");
        assert_eq!(chains.len(), 1, "un contour, jamais deux ni zéro");
        let contour = &chains[0];
        assert!(contour.is_closed(), "le carré se referme");
        assert_eq!(contour.len(), 4, "quatre sommets, la queue n'en est pas un");
    }

    #[test]
    fn plafond_d_entites_depasse_refuse() {
        // ADR-0037 D2 : au-delà du plafond public de 10 000
        // entités, le chaînage refuse (OF-GEO-0009).
        let mut lame = Vec::with_capacity(10_001);
        for i in 0..=10_000u32 {
            let debut = f64::from(i) * 0.001;
            lame.push(ligne(debut, 0.0, debut + 0.001, 0.0));
        }
        let erreur = contours(&lame).expect_err("plafond dépassé");
        assert_eq!(erreur.code(), "OF-GEO-0009");
    }

    #[test]
    fn entites_desordonnees_et_inversees_meme_contour() {
        // ADR-0037 D2 : l'entrée est non ordonnée — désordre et
        // inversions rendent le même contour, les segments à
        // contre-sens retournés pour suivre le parcours.
        let desordre = [
            ligne(1.0, 1.0, 1.0, 0.0),
            ligne(0.0, 1.0, 0.0, 0.0),
            ligne(0.0, 0.0, 1.0, 0.0),
            ligne(1.0, 1.0, 0.0, 1.0),
        ];
        let chains = contours(&desordre).expect("le graphe ignore l'ordre d'entrée");
        assert_eq!(chains.len(), 1);
        let contour = &chains[0];
        assert!(contour.is_closed(), "le carré se referme");
        assert_eq!(contour.len(), 4, "quatre sommets");
        for paire in contour.segments().windows(2) {
            assert!(
                points_recolles(paire[0].fin(), paire[1].debut()),
                "l'arrivée de chacun rejoint le départ du suivant"
            );
        }
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = contour.discretiser(tolerance).expect("discrétisation");
        let coins: Vec<(f64, f64)> = points
            .iter()
            .map(|p| (p.x().millimeters(), p.y().millimeters()))
            .collect();
        for coin in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            assert!(
                coins
                    .iter()
                    .any(|c| (c.0 - coin.0).abs() < 1e-9 && (c.1 - coin.1).abs() < 1e-9),
                "coin {coin:?} absent du contour"
            );
        }
    }
    #[test]
    fn segments_serialisables() {
        let segments = [
            ligne(0.0, 0.0, 10.0, 5.0),
            Segment2::Arc {
                centre: point(1.0, 2.0),
                rayon: Length::from_millimeters(5.0),
                debut: Angle::from_degrees(90.0),
                fin: Angle::from_degrees(180.0),
                sens: SensHoraire::Horaire,
            },
            Segment2::BezierQuadratique {
                p1: point(0.0, 0.0),
                controle: point(5.0, 5.0),
                p2: point(10.0, 0.0),
            },
            Segment2::BezierCubique {
                p1: point(0.0, 0.0),
                c1: point(1.0, 5.0),
                c2: point(9.0, 5.0),
                p2: point(10.0, 0.0),
            },
        ];
        for segment in &segments {
            let texte = serde_json::to_string(segment).expect("sérialisation");
            let relu: Segment2 = serde_json::from_str(&texte).expect("désérialisation");
            assert_eq!(relu, *segment, "aller-retour sérialisation : {texte}");
        }
    }
}
