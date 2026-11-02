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
        let mut pire = 0.0;
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
        assert_eq!(premier.x().millimeters(), 1.0);
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
        assert_eq!(points.len(), 4, "les fermetures recollées ne se répètent pas");
        assert_eq!(points[0].x().millimeters(), 0.0);
        assert_eq!(points[1].y().millimeters(), 0.0);
        assert_eq!(points[2].x().millimeters(), 10.0);
        assert_eq!(points[3].y().millimeters(), 10.0);
    }

    #[test]
    fn chaine_rompue_refusee() {
        // Un écart réel entre l'arrivée d'un segment et le départ du
        // suivant rompt la chaîne (recollage à 1e-6 mm).
        let rompue = [ligne(0.0, 0.0, 10.0, 0.0), ligne(12.0, 0.0, 12.0, 5.0)];
        let erreur = contours(&rompue).expect_err("chaîne rompue");
        assert_eq!(erreur.code(), "OF-GEO-0006");
    }

    #[test]
    fn chaine_ouverte_et_sens_preserve() {
        // Contour ouvert : l'ordre des sommets suit l'ordre des
        // segments, le sens de rotation est préservé tel quel.
        let chaine = [ligne(0.0, 0.0, 10.0, 0.0), ligne(10.0, 0.0, 10.0, 10.0)];
        let chains = contours(&chaine).expect("chaîne continue");
        assert_eq!(chains.len(), 1);
        let contour = &chains[0];
        assert!(!contour.is_closed());
        assert_eq!(contour.len(), 3, "deux segments, trois sommets");
        let tolerance = Length::from_millimeters(TOLERANCE_DISCRETISATION_MM);
        let points = contour.discretiser(tolerance).expect("discrétisation");
        assert_eq!(points[0].x().millimeters(), 0.0);
        assert_eq!(points[1].x().millimeters(), 10.0);
        assert_eq!(points[2].y().millimeters(), 10.0);
    }

    #[test]
    fn chaine_vide_sans_contour() {
        let chains = contours(&[]).expect("aucun segment");
        assert!(chains.is_empty());
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
