//! Régions 2D par orientation et parité (ADR-0037 D3, ordre 2) :
//! les cycles fermés extraits par [`crate::contours`] sont classés en
//! régions — un contour extérieur trigonométrique et ses trous
//! horaires. La classification discrétise chaque cycle à la
//! tolérance de discrétisation (refus OF-GEO-0004/0005 propagés),
//! mesure l'aire signée recentrée (patron ADR-0036 D3 : le lacet se
//! calcule autour du centroïde, stable aux grandes coordonnées) et
//! décide de la parité par point-dans-polygone — le point témoin
//! (premier sommet) d'un cycle, testé par ray-cast contre la
//! polyligne discrétisée de chaque autre cycle. Les cycles ne
//! partagent aucun sommet (un embranchement est refusé,
//! OF-GEO-0008) : le témoin est strictement dedans ou dehors. Un
//! cycle de profondeur paire ouvre une région, un cycle de profondeur
//! impaire est le trou du cycle qui le contient directement ;
//! l'orientation est ensuite normalisée (ADR-0037 D3) : l'extérieur
//! trigonométrique, les trous horaires.
//!
//! Limites connues et assumées : deux cycles quasi tangents (sommet
//! de l'un à moins de la tolérance du bord de l'autre, sans sommet
//! partagé) laissent le témoin en zone d'ambiguïté, tranchée par le
//! ray-cast ; deux cycles dont les bords se croisent (possible sans
//! sommet partagé) classent de façon déterministe mais sans
//! signification topologique — aucun refus typé ne leur est dédié,
//! hors du périmètre D3.

use crate::error::GeomError;
use crate::segments::{Contour2, Segment2};
use crate::{Point2, TOLERANCE_DISCRETISATION_MM, contours};
use forge_units::Length;

/// Région 2D (ADR-0037 D3) : un contour extérieur et ses trous.
/// L'extérieur est orienté trigonométriquement (aire signée
/// positive), les trous horairement (aire signée négative) ; le
/// moteur d'extrusion consomme la région et calcule l'aire nette
/// (extérieur moins trous). Une feature extrude exactement une
/// région — un croquis multi-régions est refusé au plan
/// (OF-SES-0001, multi-corps interdit au jalon 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// Contour extérieur, trigonométrique.
    exterieur: Contour2,
    /// Trous, horaires, strictement intérieurs à l'extérieur.
    trous: Vec<Contour2>,
}

impl Region {
    /// Contour extérieur de la région, orienté
    /// trigonométriquement (ADR-0037 D3).
    pub fn exterieur(&self) -> &Contour2 {
        &self.exterieur
    }

    /// Trous de la région, orientés horairement (ADR-0037 D3).
    pub fn trous(&self) -> &[Contour2] {
        &self.trous
    }
}

/// Classe les cycles fermés en régions (ADR-0037 D3) : parité par
/// point-dans-polygone — le point témoin d'un cycle, testé contre
/// chaque autre cycle — puis orientation normalisée, l'extérieur
/// trigonométrique et les trous horaires. L'entrée vide rend aucune
/// région ; les refus de discrétisation (OF-GEO-0004/0005) sont
/// propagés.
pub fn regions(cycles: &[Contour2]) -> Result<Vec<Region>, GeomError> {
    // Discrétisation commune : le lacet et le ray-cast jugent sur
    // les mêmes polylignes, à la tolérance du crate — les refus
    // de discrétisation (OF-GEO-0004/0005) sont propagés.
    let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
    let polylignes: Vec<Vec<Point2>> = cycles
        .iter()
        .map(|cycle| cycle.discretiser(tolerance))
        .collect::<Result<_, _>>()?;
    let mut aires = vec![0.0_f64; cycles.len()];
    for (i, points) in polylignes.iter().enumerate() {
        aires[i] = aire_signee(points);
    }
    // Profondeur de chaque cycle : le point témoin — premier
    // sommet de la polyligne — contre chaque autre cycle.
    let mut profondeurs = vec![0_usize; cycles.len()];
    for (i, points) in polylignes.iter().enumerate() {
        let Some(temoin) = points.first() else {
            continue;
        };
        for (j, autre) in polylignes.iter().enumerate() {
            if j != i && temoin_dedans(*temoin, autre) {
                profondeurs[i] += 1;
            }
        }
    }
    // Assemblage : profondeur paire = extérieur d'une région,
    // profondeur impaire = trou de son conteneur direct —
    // l'englobant le plus profond du point témoin.
    let mut ouvertes: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut region_du_cycle: Vec<Option<usize>> = vec![None; cycles.len()];
    for i in 0..cycles.len() {
        if profondeurs[i].is_multiple_of(2) {
            region_du_cycle[i] = Some(ouvertes.len());
            ouvertes.push((i, Vec::new()));
        }
    }
    for (i, points) in polylignes.iter().enumerate() {
        if profondeurs[i].is_multiple_of(2) {
            continue;
        }
        let Some(temoin) = points.first() else {
            continue;
        };
        let englobants: Vec<usize> = (0..cycles.len())
            .filter(|&j| j != i && temoin_dedans(*temoin, &polylignes[j]))
            .collect();
        let conteneur = englobants.iter().copied().max_by_key(|&j| profondeurs[j]);
        if let Some(conteneur) = conteneur
            && let Some(region) = region_du_cycle[conteneur]
        {
            ouvertes[region].1.push(i);
        }
    }
    // Orientation normalisée, ordre déterministe : régions par
    // cycle extérieur croissant, trous par cycle croissant.
    let mut classees = Vec::with_capacity(ouvertes.len());
    for (exterieur, trous) in ouvertes {
        let exterieur_normalise = normalise(&cycles[exterieur], aires[exterieur], true)?;
        let mut trous_normes = Vec::with_capacity(trous.len());
        for trou in trous {
            trous_normes.push(normalise(&cycles[trou], aires[trou], false)?);
        }
        classees.push(Region {
            exterieur: exterieur_normalise,
            trous: trous_normes,
        });
    }
    Ok(classees)
}

/// Aire signée recentrée d'une polyligne discrétisée (mm², patron
/// ADR-0036 D3 : le lacet se calcule autour du centroïde, stable
/// aux grandes coordonnées) — positive au sens trigonométrique ;
/// les sommets doublés par la discrétisation s'annulent d'eux-mêmes
/// et la polyligne se referme par wraparound.
fn aire_signee(polyligne: &[Point2]) -> f64 {
    let n = polyligne.len();
    if n == 0 {
        return 0.0;
    }
    let (cx, cy) = (
        polyligne.iter().map(|p| p.x().millimeters()).sum::<f64>() / n as f64,
        polyligne.iter().map(|p| p.y().millimeters()).sum::<f64>() / n as f64,
    );
    (0..n)
        .map(|i| {
            let a = polyligne[i];
            let b = polyligne[(i + 1) % n];
            (a.x().millimeters() - cx) * (b.y().millimeters() - cy)
                - (b.x().millimeters() - cx) * (a.y().millimeters() - cy)
        })
        .sum::<f64>()
        / 2.0
}

/// Point témoin dans la polyligne discrétisée d'un autre cycle,
/// par ray-cast pair-impair (ADR-0037 D3) : les cycles ne
/// partagent aucun sommet (embranchement refusé, OF-GEO-0008), le
/// témoin est dedans ou dehors ; les arêtes nulles de la
/// discrétisation ne traversent jamais la hauteur du témoin.
fn temoin_dedans(temoin: Point2, polyligne: &[Point2]) -> bool {
    let (px, py) = (temoin.x().millimeters(), temoin.y().millimeters());
    let n = polyligne.len();
    let mut dedans = false;
    for i in 0..n {
        let a = polyligne[i];
        let b = polyligne[(i + 1) % n];
        let (ax, ay) = (a.x().millimeters(), a.y().millimeters());
        let (bx, by) = (b.x().millimeters(), b.y().millimeters());
        if (ay > py) != (by > py) && px < (bx - ax) * (py - ay) / (by - ay) + ax {
            dedans = !dedans;
        }
    }
    dedans
}

/// Cycle normalisé dans le sens voulu (ADR-0037 D3) :
/// trigonométrique pour un extérieur, horaire pour un trou ; un
/// cycle d'aire nulle est conservé tel quel — le retourner ne
/// changerait pas le signe de son aire.
fn normalise(cycle: &Contour2, aire: f64, trigo: bool) -> Result<Contour2, GeomError> {
    let bon_sens = if trigo { aire > 0.0 } else { aire < 0.0 };
    if aire == 0.0 || bon_sens {
        Ok(cycle.clone())
    } else {
        retourne(cycle)
    }
}

/// Cycle retourné (ADR-0037 D3) : chaque segment est inversé
/// ([`Segment2::inverse`]) puis la chaîne est re-soudée par
/// [`crate::contours`] — soudure déterministe, parcours en sens
/// opposé.
fn retourne(cycle: &Contour2) -> Result<Contour2, GeomError> {
    let inverses: Vec<Segment2> = cycle
        .segments()
        .iter()
        .map(|segment| segment.inverse())
        .collect();
    // Invariant du graphe (ADR-0037 D2) : retourner chaque arête
    // d'un cycle laisse exactement un cycle ; un état inattendu,
    // jamais tranché arbitrairement, conserve le cycle tel quel.
    let retournes = contours(&inverses)?;
    Ok(match retournes.as_slice() {
        [inverse] => inverse.clone(),
        _ => cycle.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::Segment2;
    use crate::{Point2, SensHoraire, TOLERANCE_DISCRETISATION_MM, contours};
    use forge_units::Length;

    /// Point de test (mm).
    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(Length::from_millimeters(x), Length::from_millimeters(y)).expect("point fini")
    }

    /// Ligne de test entre deux points (mm).
    fn ligne(x1: f64, y1: f64, x2: f64, y2: f64) -> Segment2 {
        Segment2::Ligne {
            p1: point(x1, y1),
            p2: point(x2, y2),
        }
    }

    /// Cercle complet de test (variant explicite, sens
    /// trigonométrique).
    fn cercle(x: f64, y: f64, rayon: f64) -> Segment2 {
        Segment2::Cercle {
            centre: point(x, y),
            rayon: Length::from_millimeters(rayon),
            sens: SensHoraire::Trigonometrique,
        }
    }

    /// Carré de test (mm) : quatre lignes trigonométriques.
    fn carre(x: f64, y: f64, cote: f64) -> [Segment2; 4] {
        [
            ligne(x, y, x + cote, y),
            ligne(x + cote, y, x + cote, y + cote),
            ligne(x + cote, y + cote, x, y + cote),
            ligne(x, y + cote, x, y),
        ]
    }

    /// Cycles fermés d'un croquis de test (graphe ADR-0037 D2).
    fn cycles(segments: &[Segment2]) -> Vec<Contour2> {
        contours(segments).expect("cycles du croquis de test")
    }

    /// Aire signée recentrée d'un contour (mm², patron ADR-0036 D3 :
    /// lacet autour du centroïde) — positive au sens
    /// trigonométrique.
    fn aire_signee(contour: &Contour2) -> f64 {
        let points = contour
            .discretiser(Length::from_millimeters(TOLERANCE_DISCRETISATION_MM))
            .expect("discrétisation du contour de test");
        let n = points.len();
        let (cx, cy) = (
            points.iter().map(|p| p.x().millimeters()).sum::<f64>() / n as f64,
            points.iter().map(|p| p.y().millimeters()).sum::<f64>() / n as f64,
        );
        (0..n)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % n];
                (a.x().millimeters() - cx) * (b.y().millimeters() - cy)
                    - (b.x().millimeters() - cx) * (a.y().millimeters() - cy)
            })
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn carre_avec_carre_interieur_une_region_trou() {
        // ADR-0037 D3 : deux carrés imbriqués dessinés dans le même
        // sens (horaire ici) — une seule région, l'extérieur
        // normalisé trigonométrique, le trou horaire.
        let exterieur = [
            ligne(0.0, 0.0, 0.0, 10.0),
            ligne(0.0, 10.0, 10.0, 10.0),
            ligne(10.0, 10.0, 10.0, 0.0),
            ligne(10.0, 0.0, 0.0, 0.0),
        ];
        let interieur = [
            ligne(2.0, 2.0, 2.0, 8.0),
            ligne(2.0, 8.0, 8.0, 8.0),
            ligne(8.0, 8.0, 8.0, 2.0),
            ligne(8.0, 2.0, 2.0, 2.0),
        ];
        let mut segments = exterieur.to_vec();
        segments.extend_from_slice(&interieur);
        let regions = regions(&cycles(&segments)).expect("régions classées");
        assert_eq!(regions.len(), 1, "un carré à carré intérieur = une région");
        let region = &regions[0];
        assert_eq!(region.trous().len(), 1, "le carré intérieur est un trou");
        assert!(
            aire_signee(region.exterieur()) > 0.0,
            "extérieur normalisé trigonométrique"
        );
        assert!(
            aire_signee(&region.trous()[0]) < 0.0,
            "trou normalisé horaire"
        );
    }

    #[test]
    fn deux_cercles_concentriques_anneau() {
        // ADR-0037 D3 : deux cercles concentriques (Segment2::Cercle
        // explicites) — un anneau, le petit cercle est le trou.
        let segments = [cercle(0.0, 0.0, 10.0), cercle(0.0, 0.0, 3.0)];
        let regions = regions(&cycles(&segments)).expect("régions classées");
        assert_eq!(regions.len(), 1, "deux cercles concentriques = un anneau");
        assert_eq!(regions[0].trous().len(), 1, "le petit cercle est le trou");
        assert!(
            aire_signee(regions[0].exterieur()) > 0.0,
            "extérieur normalisé trigonométrique"
        );
        assert!(
            aire_signee(&regions[0].trous()[0]) < 0.0,
            "trou normalisé horaire"
        );
        // l'anneau garde l'ordre de grandeur des aires (cordes de la
        // discrétisation : flèche sous la tolérance)
        let grande = aire_signee(regions[0].exterieur());
        let petite = -aire_signee(&regions[0].trous()[0]);
        assert!((grande - std::f64::consts::PI * 100.0).abs() < 1.0);
        assert!((petite - std::f64::consts::PI * 9.0).abs() < 0.5);
    }

    #[test]
    fn carres_disjoints_deux_regions() {
        // ADR-0037 D3 : deux carrés disjoints — deux régions sans
        // trous ; le refus multi-corps appartient au plan
        // (OF-SES-0001), pas à la classification.
        let mut segments = carre(0.0, 0.0, 5.0).to_vec();
        segments.extend_from_slice(&carre(20.0, 0.0, 5.0));
        let regions = regions(&cycles(&segments)).expect("régions classées");
        assert_eq!(regions.len(), 2, "deux carrés disjoints = deux régions");
        assert!(
            regions.iter().all(|region| region.trous().is_empty()),
            "aucun trou"
        );
    }

    #[test]
    fn desordre_et_inversions_memes_regions() {
        // ADR-0037 D2/D3 : le graphe accepte l'entrée non ordonnée
        // et les segments à contre-sens — mêmes régions, mêmes
        // aires (lacet exact des lignes).
        let brouillon = [
            ligne(0.0, 10.0, 10.0, 10.0),
            ligne(8.0, 2.0, 2.0, 2.0),
            ligne(0.0, 10.0, 0.0, 0.0),
            ligne(2.0, 8.0, 8.0, 8.0),
            ligne(10.0, 10.0, 10.0, 0.0),
            ligne(8.0, 2.0, 8.0, 8.0),
            ligne(2.0, 2.0, 2.0, 8.0),
            ligne(10.0, 0.0, 0.0, 0.0),
        ];
        let regions = regions(&cycles(&brouillon)).expect("régions classées");
        assert_eq!(regions.len(), 1, "une région malgré le désordre");
        let region = &regions[0];
        assert_eq!(region.trous().len(), 1, "le carré intérieur est un trou");
        let nette =
            aire_signee(region.exterieur()) + region.trous().iter().map(aire_signee).sum::<f64>();
        assert!(
            (nette - 64.0).abs() < 1e-9,
            "aire nette 100 − 36 = 64 mm², obtenue {nette}"
        );
    }

    #[test]
    fn aire_nette_recentree_invariante_par_translation() {
        // ADR-0037 D3 / ADR-0036 D3 : aire nette 10×10 − 2×2 =
        // 96 mm², à l'origine et translatée de 1e8 mm — le lacet
        // recentré sur le centroïde reste stable aux grandes
        // coordonnées (1e8² dépasse la précision exacte du f64).
        for (dx, dy) in [(0.0, 0.0), (1.0e8, 1.0e8)] {
            let mut segments = carre(dx, dy, 10.0).to_vec();
            segments.extend_from_slice(&carre(dx + 4.0, dy + 4.0, 2.0));
            let regions = regions(&cycles(&segments)).expect("régions classées");
            assert_eq!(regions.len(), 1, "une région où que soit le croquis");
            let region = &regions[0];
            assert_eq!(region.trous().len(), 1, "le carré intérieur est un trou");
            let nette = aire_signee(region.exterieur())
                + region.trous().iter().map(aire_signee).sum::<f64>();
            assert!(
                (nette - 96.0).abs() < 1e-6,
                "aire nette 96 mm² à ({dx}, {dy}), obtenue {nette}"
            );
        }
    }
}
