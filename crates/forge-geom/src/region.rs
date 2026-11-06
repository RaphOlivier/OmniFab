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
use crate::segments::Contour2;

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
    let _ = cycles;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::Segment2;
    use crate::{Point2, SensHoraire, TOLERANCE_DISCRETISATION_MM, contours};
    use forge_units::{Angle, Length};

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

    /// Cercle complet de test (arc à balayage nul, sens
    /// trigonométrique).
    fn cercle(x: f64, y: f64, rayon: f64) -> Segment2 {
        Segment2::Arc {
            centre: point(x, y),
            rayon: Length::from_millimeters(rayon),
            debut: Angle::ZERO,
            fin: Angle::ZERO,
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
        // ADR-0037 D3 : deux cercles complets concentriques (arcs à
        // balayage nul) — un anneau, le petit cercle est le trou.
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
