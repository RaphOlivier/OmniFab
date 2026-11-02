//! Import/export DXF (ADR-0009) : parseur crate `dxf`, budgets
//! intégrés.
//!
//! Convention d'unités : une unité = 1 mm ; le z est ignoré (tracés
//! 2D). Les entités géométriques du jalon 1 sont lues : `LINE`,
//! `LWPOLYLINE`, `POLYLINE` (bulges devenus arcs natifs),
//! `CIRCLE`, `ARC` (cercles et arcs natifs — un balayage nul
//! dénote le cercle complet) et `SPLINE`
//! (B-splines rationnelles échantillonnées par l'algorithme de
//! de Boor — l'export DXF d'Inkscape y code toute courbe, cercle
//! et arc) ; les entités non géométriques (textes, cotes,
//! blocs…) sont ignorées par conception (ADR-0009) ; à l'import,
//! les entités hors dessin sont écartées (audit n°3, A3-05) :
//! espace papier (code 67 — la mise en page, pas le dessin),
//! entités invisibles (code 60), calques éteints (couleur
//! négative dans la table `LAYER`) — [`import_avec_rapport`]
//! en donne le décompte. Limite connue : un calque gelé
//! (code 70, bit 0) est indistinguable d'un calque actif, la
//! crate `dxf` 0.6.2 n'expose pas ce drapeau. À l'export,
//! chaque segment devient une entité `LINE` au z nul.
//!
//! Sécurité (ADR-0009) : tout dépassement de [`DxfBudgets`] est une
//! erreur typée, levée avant tout traitement ultérieur.

use crate::IoError;
use dxf::entities::{Entity, EntityType, Line, Spline};
use forge_geom::{Path2, Point2, Segment2, SensHoraire};
use forge_units::{Angle, Length};
use std::collections::HashSet;

/// Nombre de segments de la discrétisation d'une spline par
/// intervalle de nœuds.
pub const SEGMENTS_SPLINE: usize = 16;

/// Budgets de ressources du parseur DXF (ADR-0009).
///
/// La sécurité ne doit pas brider les fonctions : chaque budget est
/// public et réglable par l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxfBudgets {
    /// Taille maximale acceptée en octets (défaut : 256 Mio).
    pub max_bytes: usize,
    /// Nombre maximal de tracés accepté (défaut : 100 000).
    pub max_paths: usize,
    /// Nombre maximal de sommets accepté (défaut : 100 000).
    pub max_vertices: usize,
}

impl Default for DxfBudgets {
    /// Défauts : 256 Mio, 100 000 tracés, 100 000 sommets.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_paths: 100_000,
            max_vertices: 100_000,
        }
    }
}

/// Importe un DXF en tracés 2D. Convention : une unité = 1 mm ; le
/// z est ignoré. Les cercles et arcs deviennent des segments
/// natifs — un cercle est un arc au balayage nul — ; les bulges
/// des polylignes (arcs codés par sommet, code 42, générés par
/// FreeCAD à l'export) deviennent des arcs natifs le long de la
/// corde ; les splines — B-splines rationnelles, qu'Inkscape
/// exporte pour toute courbe, cercle ou arc — restent des
/// polylignes échantillonnées à [`SEGMENTS_SPLINE`] segments par
/// intervalle de nœuds (algorithme de de Boor). Les budgets protègent contre
/// les documents démesurés (ADR-0009).
/// Délègue à [`import_avec_rapport`], sans exposer le rapport.
pub fn import(bytes: &[u8], budgets: &DxfBudgets) -> Result<Vec<Path2>, IoError> {
    Ok(import_avec_rapport(bytes, budgets)?.traces)
}

/// Comptes des entités écartées de l'import (audit n°3, A3-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RapportImport {
    /// Entités de l'espace papier (code 67) : la mise en page,
    /// pas le dessin.
    pub espace_papier: usize,
    /// Entités invisibles (code 60).
    pub invisibles: usize,
    /// Entités des calques éteints (couleur négative au code 62).
    pub calques_eteints: usize,
}

impl RapportImport {
    /// Total des entités écartées.
    pub fn total_ignorees(&self) -> usize {
        self.espace_papier + self.invisibles + self.calques_eteints
    }
}

/// Résultat d'un import DXF avec rapport (audit n°3, A3-05) :
/// les tracés importés et le décompte des entités écartées.
#[derive(Debug, Clone)]
pub struct ImportDxf {
    /// Tracés importés.
    pub traces: Vec<Path2>,
    /// Décompte des entités écartées.
    pub rapport: RapportImport,
}

/// Importe un DXF en tracés 2D avec rapport des entités
/// écartées (audit n°3, A3-05) : espace papier (code 67),
/// invisibles (code 60), calques éteints (couleur négative).
/// Le reste de la convention est celui de [`import`] ; les
/// budgets protègent contre les documents démesurés (ADR-0009).
pub fn import_avec_rapport(bytes: &[u8], budgets: &DxfBudgets) -> Result<ImportDxf, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    let mut curseur = std::io::Cursor::new(bytes);
    let dessin = dxf::Drawing::load(&mut curseur).map_err(|erreur| IoError::InvalidDxf {
        raison: erreur.to_string(),
    })?;
    // Calques éteints (couleur négative au code 62) : leurs
    // entités ne sont pas dessinées par les éditeurs — elles ne
    // deviennent pas des tracés.
    let calques_eteints: HashSet<&str> = dessin
        .layers()
        .filter(|calque| !calque.is_layer_on)
        .map(|calque| calque.name.as_str())
        .collect();
    let mut rapport = RapportImport::default();
    let mut traces: Vec<Path2> = Vec::new();
    let mut total = 0usize;
    for entite in dessin.entities() {
        if entite.common.is_in_paper_space {
            rapport.espace_papier += 1;
            continue;
        }
        if !entite.common.is_visible {
            rapport.invisibles += 1;
            continue;
        }
        if calques_eteints.contains(entite.common.layer.as_str()) {
            rapport.calques_eteints += 1;
            continue;
        }
        let mut segments: Vec<Segment2> = Vec::new();
        let geometrique = match &entite.specific {
            EntityType::Line(ligne) => {
                let p1 = sommet(ligne.p1.x, ligne.p1.y, &mut total, budgets)?;
                let p2 = sommet(ligne.p2.x, ligne.p2.y, &mut total, budgets)?;
                segments.push(Segment2::Ligne { p1, p2 });
                true
            }
            EntityType::LwPolyline(polyligne) => {
                let sommets: Vec<(f64, f64, f64)> = polyligne
                    .vertices
                    .iter()
                    .map(|v| (v.x, v.y, v.bulge))
                    .collect();
                segments_polyligne(
                    &sommets,
                    polyligne.is_closed(),
                    &mut segments,
                    &mut total,
                    budgets,
                )?;
                true
            }
            EntityType::Polyline(polyligne) => {
                let sommets: Vec<(f64, f64, f64)> = polyligne
                    .vertices()
                    .map(|v| (v.location.x, v.location.y, v.bulge))
                    .collect();
                segments_polyligne(
                    &sommets,
                    polyligne.is_closed(),
                    &mut segments,
                    &mut total,
                    budgets,
                )?;
                true
            }
            EntityType::Circle(cercle) => {
                segments.push(arc_cercle_natif(
                    cercle.center.x,
                    cercle.center.y,
                    cercle.radius,
                    0.0,
                    0.0,
                    &mut total,
                    budgets,
                )?);
                true
            }
            EntityType::Arc(arc) => {
                let mut span = arc.end_angle - arc.start_angle;
                if span <= 0.0 {
                    span += 360.0;
                }
                segments.push(arc_cercle_natif(
                    arc.center.x,
                    arc.center.y,
                    arc.radius,
                    arc.start_angle,
                    arc.start_angle + span,
                    &mut total,
                    budgets,
                )?);
                true
            }
            EntityType::Spline(spline) => {
                let points = spline_points(spline, &mut total, budgets)?;
                for paire in points.windows(2) {
                    segments.push(Segment2::Ligne {
                        p1: paire[0],
                        p2: paire[1],
                    });
                }
                if spline.is_closed() && points.len() >= 2 {
                    let premier = points[0];
                    let dernier = points[points.len() - 1];
                    segments.push(Segment2::Ligne {
                        p1: dernier,
                        p2: premier,
                    });
                }
                true
            }
            _ => false,
        };
        if !geometrique || segments.is_empty() {
            continue;
        }
        if traces.len() >= budgets.max_paths {
            return Err(IoError::TooManyPaths {
                found: traces.len() as u64 + 1,
                max: budgets.max_paths as u64,
                unite: "tracés",
            });
        }
        traces.push(Path2::from_segments(std::mem::take(&mut segments)));
    }
    Ok(ImportDxf { traces, rapport })
}

/// Exporte les tracés en DXF : chaque segment devient une entité
/// `LINE` (z nul, tracés 2D) ; un tracé fermé ajoute la ligne de
/// fermeture dernier → premier sommet.
pub fn export(traces: &[Path2]) -> Result<Vec<u8>, IoError> {
    let mut dessin = dxf::Drawing::new();
    for trace in traces {
        let points = trace.points();
        if points.len() < 2 {
            continue;
        }
        for fenetre in points.windows(2) {
            ajoute_ligne(&mut dessin, &fenetre[0], &fenetre[1]);
        }
        if trace.is_closed() && points.len() >= 3 {
            #[expect(clippy::expect_used)]
            let premier = points
                .first()
                .expect("invariant : garde len < 2 → continue ci-dessus");
            #[expect(clippy::expect_used)]
            let dernier = points
                .last()
                .expect("invariant : garde len < 2 → continue ci-dessus");
            ajoute_ligne(&mut dessin, premier, dernier);
        }
    }
    let mut sortie = Vec::new();
    if let Err(erreur) = dessin.save(&mut sortie) {
        return Err(IoError::InvalidDxf {
            raison: format!("écriture impossible : {erreur}"),
        });
    }
    Ok(sortie)
}

/// Construit un sommet 2D (z ignoré), sous garde de budget.
fn sommet(x: f64, y: f64, total: &mut usize, budgets: &DxfBudgets) -> Result<Point2, IoError> {
    if *total >= budgets.max_vertices {
        return Err(IoError::TooManyPaths {
            found: *total as u64 + 1,
            max: budgets.max_vertices as u64,
            unite: "sommets",
        });
    }
    let x_mm = Length::from_millimeters(x);
    let y_mm = Length::from_millimeters(y);
    let point = Point2::new(x_mm, y_mm).map_err(IoError::InvalidGeometry)?;
    *total += 1;
    Ok(point)
}

/// Pousse les segments d'une polyligne (x, y, bulge) : chaque paire
/// de sommets consécutifs devient un segment — corde droite sans
/// bulge, arc natif avec (mathématiques du bulge, cf.
/// [`arc_de_bulge`]). Sur une polyligne fermée, le bulge du dernier
/// sommet s'applique au segment de fermeture vers le premier — la
/// fermeture est un segment explicite.
fn segments_polyligne(
    sommets: &[(f64, f64, f64)],
    ferme: bool,
    segments: &mut Vec<Segment2>,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<(), IoError> {
    if sommets.len() < 2 {
        return Ok(());
    }
    let points: Vec<Point2> = sommets
        .iter()
        .map(|(x, y, _)| sommet(*x, *y, total, budgets))
        .collect::<Result<_, _>>()?;
    let nombre = points.len() - 1 + usize::from(ferme);
    for i in 0..nombre {
        let depart = points[i];
        let arrivee = if i + 1 < points.len() {
            points[i + 1]
        } else {
            points[0]
        };
        let bulge = sommets[i].2;
        if bulge.abs() <= 1e-12 {
            segments.push(Segment2::Ligne {
                p1: depart,
                p2: arrivee,
            });
        } else {
            segments.push(arc_de_bulge(depart, arrivee, bulge)?);
        }
    }
    Ok(())
}

/// Arc natif d'un segment à bulge DXF (code 42, généré par FreeCAD
/// à l'export) : mathématiques du bulge — angle inclus
/// θ = 4·atan(b), positif = sens trigonométrique ; rayon signé
/// r = demi-corde / sin(θ/2) ; centre = milieu de la corde décalé
/// de r·cos(θ/2) le long de la normale à gauche de la corde.
fn arc_de_bulge(depart: Point2, arrivee: Point2, bulge: f64) -> Result<Segment2, IoError> {
    let theta = 4.0 * bulge.atan();
    let dx = arrivee.x().millimeters() - depart.x().millimeters();
    let dy = arrivee.y().millimeters() - depart.y().millimeters();
    let corde = (dx * dx + dy * dy).sqrt();
    if !theta.is_finite() || corde <= 1e-12 {
        return Err(IoError::InvalidDxf {
            raison: format!("bulge {bulge} sur une corde vide"),
        });
    }
    let demi_theta = theta / 2.0;
    let r_signe = (corde / 2.0) / demi_theta.sin();
    let h = r_signe * demi_theta.cos();
    let mx = (depart.x().millimeters() + arrivee.x().millimeters()) / 2.0;
    let my = (depart.y().millimeters() + arrivee.y().millimeters()) / 2.0;
    // Normale à gauche de la corde depart → arrivee.
    let nx = -dy / corde;
    let ny = dx / corde;
    let centre_x = mx + h * nx;
    let centre_y = my + h * ny;
    let abscisse = Length::from_millimeters(centre_x);
    let ordonnee = Length::from_millimeters(centre_y);
    let centre = Point2::new(abscisse, ordonnee).map_err(IoError::InvalidGeometry)?;
    let rayon = r_signe.abs();
    if !rayon.is_finite() || rayon <= 0.0 {
        return Err(IoError::InvalidDxf {
            raison: format!("bulge {bulge} : rayon d'arc invalide"),
        });
    }
    let alpha = (depart.y().millimeters() - centre_y).atan2(depart.x().millimeters() - centre_x);
    Ok(Segment2::Arc {
        centre,
        rayon: Length::from_millimeters(rayon),
        debut: Angle::from_radians(alpha),
        fin: Angle::from_radians(alpha + theta),
        sens: if theta >= 0.0 {
            SensHoraire::Trigonometrique
        } else {
            SensHoraire::Horaire
        },
    })
}

/// Arc natif d'un cercle ou d'un arc DXF (z ignoré) : un balayage
/// nul dénote le cercle complet ; le centre et les points extrêmes
/// sont validés comme sommets (garde de budget et de bornes), le
/// rayon doit être strictement positif et fini.
fn arc_cercle_natif(
    centre_x: f64,
    centre_y: f64,
    rayon: f64,
    debut_degres: f64,
    fin_degres: f64,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<Segment2, IoError> {
    if !rayon.is_finite() || rayon <= 0.0 {
        return Err(IoError::InvalidDxf {
            raison: format!("rayon de cercle ou d'arc invalide : {rayon}"),
        });
    }
    let centre = sommet(centre_x, centre_y, total, budgets)?;
    // Les points extrêmes bornent les coordonnées réellement
    // importées : le corps du cercle ou de l'arc ne porte que ses
    // extrémités, le reste est discrétisé à la lecture.
    sommet(
        centre_x + rayon * debut_degres.to_radians().cos(),
        centre_y + rayon * debut_degres.to_radians().sin(),
        total,
        budgets,
    )?;
    sommet(
        centre_x + rayon * fin_degres.to_radians().cos(),
        centre_y + rayon * fin_degres.to_radians().sin(),
        total,
        budgets,
    )?;
    Ok(Segment2::Arc {
        centre,
        rayon: Length::from_millimeters(rayon),
        debut: Angle::from_degrees(debut_degres),
        fin: Angle::from_degrees(fin_degres),
        sens: SensHoraire::Trigonometrique,
    })
}

/// Discrétise une spline en polyligne : échantillonnage à
/// [`SEGMENTS_SPLINE`] segments par intervalle de nœuds sur le
/// domaine valide, évalué par l'algorithme de de Boor en
/// coordonnées homogènes (poids portés en w, divisés en sortie —
/// splines rationnelles comprises).
///
/// Le contrat d'une spline DXF est vérifié avant tout calcul :
/// degré ≥ 1, au moins deux points de contrôle et exactement
/// points de contrôle + degré + 1 nœuds ; les splines à points
/// d'ajustement seuls sont ignorées.
fn spline_points(
    spline: &Spline,
    total: &mut usize,
    budgets: &DxfBudgets,
) -> Result<Vec<Point2>, IoError> {
    let degree = usize::try_from(spline.degree_of_curve).unwrap_or(0);
    let noeuds = &spline.knot_values;
    let controles = &spline.control_points;
    if degree < 1 || controles.len() < 2 || noeuds.len() != controles.len() + degree + 1 {
        return Ok(Vec::new());
    }
    let premier = noeuds[degree];
    let dernier = noeuds[controles.len()];
    if dernier - premier <= 1e-12 {
        return Ok(Vec::new());
    }
    // Intervalles de nœuds non vides dans le domaine.
    let intervalles = noeuds[degree..=controles.len()]
        .windows(2)
        .filter(|paire| paire[1] - paire[0] > 1e-12)
        .count()
        .max(1);
    let segments = (intervalles * SEGMENTS_SPLINE).clamp(2, 8192);
    let poids: Vec<f64> = if spline.weight_values.len() == controles.len() {
        spline.weight_values.clone()
    } else {
        vec![1.0; controles.len()]
    };
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let u = premier + (dernier - premier) * i as f64 / segments as f64;
        let (x, y) = de_boor(u, degree, noeuds, controles, &poids);
        points.push(sommet(x, y, total, budgets)?);
    }
    Ok(points)
}

/// Évalue la spline en `u` : itération de de Boor en coordonnées
/// homogènes. L'intervalle porteur `k` est le dernier dont le nœud
/// de gauche précède `u` (borné au dernier intervalle du domaine,
/// pour `u` tombant exactement sur le nœud final) ; un écart de
/// nœuds nul donne un alpha nul (nœuds répétés, extrémités).
fn de_boor(
    u: f64,
    degree: usize,
    noeuds: &[f64],
    controles: &[dxf::Point],
    poids: &[f64],
) -> (f64, f64) {
    let n = controles.len();
    let mut k = degree;
    while k + 1 < n && noeuds[k + 1] <= u {
        k += 1;
    }
    // Points homogènes de l'itération courante (x·w, y·w, w).
    let mut d: Vec<(f64, f64, f64)> = (0..=degree)
        .map(|i| {
            let j = k - degree + i;
            (
                controles[j].x * poids[j],
                controles[j].y * poids[j],
                poids[j],
            )
        })
        .collect();
    for r in 1..=degree {
        for i in (r..=degree).rev() {
            let j = k - degree + i;
            let ecart = noeuds[j + degree + 1 - r] - noeuds[j];
            let alpha = if ecart.abs() < 1e-12 {
                0.0
            } else {
                (u - noeuds[j]) / ecart
            };
            let (px, py, pw) = d[i - 1];
            let (qx, qy, qw) = d[i];
            d[i] = (
                px + (qx - px) * alpha,
                py + (qy - py) * alpha,
                pw + (qw - pw) * alpha,
            );
        }
    }
    let (x, y, w) = d[degree];
    if w.abs() < 1e-12 {
        (x, y)
    } else {
        (x / w, y / w)
    }
}

/// Ajoute une entité `LINE` au dessin, z nul (tracés 2D).
fn ajoute_ligne(dessin: &mut dxf::Drawing, debut: &Point2, fin: &Point2) {
    let p1 = dxf::Point {
        x: debut.x().millimeters(),
        y: debut.y().millimeters(),
        z: 0.0,
    };
    let p2 = dxf::Point {
        x: fin.x().millimeters(),
        y: fin.y().millimeters(),
        z: 0.0,
    };
    dessin.add_entity(Entity::new(EntityType::Line(Line::new(p1, p2))));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DXF ASCII minimal : une LWPOLYLINE ouverte à deux sommets
    /// (0,0) et (10,0), le premier portant le bulge donné (code 42).
    fn dxf_bulge(bulge: &str) -> Vec<u8> {
        let texte = format!(
            "0
SECTION
2
ENTITIES
0
LWPOLYLINE
100
AcDbEntity
8
0
             100
AcDbPolyline
90
2
70
0
10
0.0
20
0.0
42
{bulge}
             10
10.0
20
0.0
0
ENDSEC
0
EOF
"
        );
        texte.into_bytes()
    }

    #[test]
    fn lwpolyline_sans_bulge_reste_un_segment() {
        let traces = import(&dxf_bulge("0.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        let points = traces[0].points();
        assert_eq!(points.len(), 2);
        assert!(points[0].x().millimeters().abs() < 1e-9);
        assert!(points[0].y().millimeters().abs() < 1e-9);
        assert!((points[1].x().millimeters() - 10.0).abs() < 1e-9);
        assert!(points[1].y().millimeters().abs() < 1e-9);
    }

    /// Distance d'un point au segment [a, b] (mm).
    fn distance_au_segment(x: f64, y: f64, a: Point2, b: Point2) -> f64 {
        let vx = b.x().millimeters() - a.x().millimeters();
        let vy = b.y().millimeters() - a.y().millimeters();
        let wx = x - a.x().millimeters();
        let wy = y - a.y().millimeters();
        let corde2 = vx * vx + vy * vy;
        if corde2 <= 1e-24 {
            return (wx * wx + wy * wy).sqrt();
        }
        let t = ((wx * vx + wy * vy) / corde2).clamp(0.0, 1.0);
        let dx = wx - t * vx;
        let dy = wy - t * vy;
        (dx * dx + dy * dy).sqrt()
    }

    #[test]
    fn lwpolyline_bulge_positif_devient_un_arc_sous_la_corde() {
        // bulge 1 = demi-cercle trigonométrique de (0,0) vers
        // (10,0) : centre (5, 0), rayon 5 — l'arc natif passe par
        // (5, -5), sa discrétisation y reste à la tolérance près.
        let traces = import(&dxf_bulge("1.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].segments().len(), 1, "un bulge = un arc natif");
        match traces[0].segments()[0] {
            Segment2::Arc {
                centre,
                rayon,
                sens,
                ..
            } => {
                assert_eq!(centre.x().millimeters(), 5.0);
                assert!(centre.y().millimeters().abs() < 1e-9);
                assert!((rayon.millimeters() - 5.0).abs() < 1e-9);
                assert_eq!(sens, SensHoraire::Trigonometrique);
            }
            _ => panic!("le bulge doit devenir un arc natif"),
        }
        let points = traces[0].points();
        let fleche = points
            .windows(2)
            .map(|paire| distance_au_segment(5.0, -5.0, paire[0], paire[1]))
            .fold(f64::INFINITY, f64::min);
        assert!(
            fleche <= 0.011,
            "l'arc doit passer par (5, -5) : flèche {fleche}"
        );
    }

    #[test]
    fn lwpolyline_bulge_negatif_devient_un_arc_au_dessus() {
        // bulge -1 = demi-cercle horaire de (0,0) vers (10,0) :
        // l'arc natif passe par (5, 5).
        let traces = import(&dxf_bulge("-1.0"), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].segments().len(), 1, "un bulge = un arc natif");
        match traces[0].segments()[0] {
            Segment2::Arc {
                centre,
                rayon,
                sens,
                ..
            } => {
                assert_eq!(centre.x().millimeters(), 5.0);
                assert!(centre.y().millimeters().abs() < 1e-9);
                assert!((rayon.millimeters() - 5.0).abs() < 1e-9);
                assert_eq!(sens, SensHoraire::Horaire);
            }
            _ => panic!("le bulge doit devenir un arc natif"),
        }
        let points = traces[0].points();
        let fleche = points
            .windows(2)
            .map(|paire| distance_au_segment(5.0, 5.0, paire[0], paire[1]))
            .fold(f64::INFINITY, f64::min);
        assert!(
            fleche <= 0.011,
            "l'arc doit passer par (5, 5) : flèche {fleche}"
        );
    }

    /// DXF ASCII minimal : une entité `SPLINE` (70 = 8, planaire) ;
    /// `corps` reçoit les paires code/valeur des nœuds (40), des
    /// poids (41) et des points de contrôle (10/20/30).
    fn dxf_spline(degre: &str, corps: &str) -> Vec<u8> {
        let texte = format!(
            "0
SECTION
2
ENTITIES
0
SPLINE
100
AcDbEntity
8
0
100
AcDbSpline
70
8
71
{degre}
74
0
{corps}0
ENDSEC
0
EOF
"
        );
        texte.into_bytes()
    }

    #[test]
    fn spline_cubique_d_inkscape_se_discretise() {
        // Le format d'Inkscape : quatre points de contrôle, nœuds
        // 0,0,0,0,1,1,1,1 — un Bézier cubique par entité.
        let corps = "40\n0.0\n40\n0.0\n40\n0.0\n40\n0.0\n40\n1.0\n40\n1.0\n40\n1.0\n40\n1.0\n10\n0.0\n20\n0.0\n30\n0.0\n10\n0.0\n20\n10.0\n30\n0.0\n10\n10.0\n20\n10.0\n30\n0.0\n10\n10.0\n20\n0.0\n30\n0.0\n";
        let traces = import(&dxf_spline("3", corps), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1, "la spline est importée");
        let points = traces[0].points();
        assert_eq!(points.len(), 17, "seize segments par intervalle");
        // Extrémités = premier et dernier points de contrôle.
        assert!(points[0].x().millimeters().abs() < 1e-9);
        assert!((points[16].x().millimeters() - 10.0).abs() < 1e-9);
        // Milieu du Bézier : (P0 + 3·P1 + 3·P2 + P3) / 8.
        let milieu = points[8];
        assert!((milieu.x().millimeters() - 5.0).abs() < 1e-9);
        assert!((milieu.y().millimeters() - 7.5).abs() < 1e-9);
    }

    #[test]
    fn spline_de_degre_un_relie_ses_points_de_controle() {
        // Degré 1, nœuds 0,0,1,2,2 : la polyligne (0,0), (5,5),
        // (10,0) en deux intervalles de nœuds.
        let corps = "40\n0.0\n40\n0.0\n40\n1.0\n40\n2.0\n40\n2.0\n10\n0.0\n20\n0.0\n30\n0.0\n10\n5.0\n20\n5.0\n30\n0.0\n10\n10.0\n20\n0.0\n30\n0.0\n";
        let traces = import(&dxf_spline("1", corps), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1, "la spline est importée");
        let points = traces[0].points();
        assert_eq!(points.len(), 33, "deux intervalles de seize");
        // Au nœud u = 1, la spline passe par (5, 5).
        let milieu = points[16];
        assert!((milieu.x().millimeters() - 5.0).abs() < 1e-9);
        assert!((milieu.y().millimeters() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn spline_rationnelle_traverse_le_quart_de_cercle() {
        // NURBS du quart de cercle unité (degré 2, poids 1, √2/2,
        // 1) : tous les points échantillonnés restent sur le cercle.
        let racine = 2.0_f64.sqrt() / 2.0;
        let poids = format!("41\n1.0\n41\n{racine}\n41\n1.0\n");
        let corps = format!(
            "40\n0.0\n40\n0.0\n40\n0.0\n40\n1.0\n40\n1.0\n40\n1.0\n{poids}10\n1.0\n20\n0.0\n30\n0.0\n10\n1.0\n20\n1.0\n30\n0.0\n10\n0.0\n20\n1.0\n30\n0.0\n"
        );
        let traces = import(&dxf_spline("2", &corps), &DxfBudgets::default()).expect("dxf valide");
        assert_eq!(traces.len(), 1, "la spline est importée");
        let points = traces[0].points();
        assert_eq!(points.len(), 17);
        // Milieu de la courbe : (cos 45°, sin 45°).
        let milieu = points[8];
        assert!((milieu.x().millimeters() - racine).abs() < 1e-9);
        assert!((milieu.y().millimeters() - racine).abs() < 1e-9);
        for point in points {
            let x = point.x().millimeters();
            let y = point.y().millimeters();
            assert!(((x * x + y * y).sqrt() - 1.0).abs() < 1e-9);
        }
    }
}
