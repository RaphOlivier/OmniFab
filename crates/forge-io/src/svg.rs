//! Import/export SVG (ADR-0009) : parseur `usvg`, budgets intégrés.
//!
//! Convention d'axes : le SVG descend (Y vers le bas), le monde
//! monte (Y vers le haut) — l'axe Y est inversé à l'import et à
//! l'export. Convention d'unités : une unité SVG = 1 mm ; les
//! tailles physiques (`width`/`height` en mm, in…) sont
//! résolues à 25,4 dpi — un « pixel » usvg vaut exactement 1 mm.
//! Le parseur `usvg` n'exécute aucun script ; le résolveur
//! d'images est neutralisé (aucun `href` n'est résolu) et une
//! DTD interne est refusée avant l'analyse (audit n°3,
//! A3-16) ; les formes de base (`rect`,
//! `circle`, …) sont normalisées en tracés par `usvg`. Les courbes
//! de Bézier (quadratiques et cubiques) deviennent des segments
//! natifs — usvg a déjà converti les arcs elliptiques et les
//! primitives de courbes en Béziers cubiques, aucun arc natif
//! n'apparaît côté SVG ; l'ancien aplatissement fixe laisse place
//! à la discrétisation à tolérance (ADR-0024, phase B).
//!
//! Sécurité (ADR-0009) : tout dépassement de [`SvgBudgets`] est une
//! erreur typée, levée avant tout traitement ultérieur.

use crate::IoError;
use forge_geom::{Path2, Point2, Segment2, points_recolles};
use forge_units::Length;
use usvg::tiny_skia_path::PathSegment;

/// Taille minimale d'une dimension de la boîte exportée : un tracé
/// plat (ligne horizontale ou verticale) produit sinon un `viewBox`
/// de hauteur nulle, que les navigateurs refusent de rendre — la
/// marge vaut l'épaisseur du trait (audit n°2, N9).
pub const MARGE_BOITE_PLATE: f64 = 0.1;

/// Budgets de ressources du parseur SVG (ADR-0009).
///
/// La sécurité ne doit pas brider les fonctions : chaque budget est
/// public et réglable par l'appelant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgBudgets {
    /// Taille maximale acceptée en octets (défaut : 256 Mio).
    pub max_bytes: usize,
    /// Nombre maximal de tracés accepté (défaut : 100 000).
    pub max_paths: usize,
    /// Nombre maximal de sommets accepté (défaut : 100 000).
    pub max_vertices: usize,
}

impl Default for SvgBudgets {
    /// Défauts : 256 Mio, 100 000 tracés, 100 000 sommets.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_paths: 100_000,
            max_vertices: 100_000,
        }
    }
}

/// Options `usvg` de l'import : résolution des unités physiques à
/// 25,4 dpi — un « pixel » usvg vaut exactement 1 mm (au défaut
/// 96 dpi, `width="50mm"` devenait 189 px et l'import grossissait
/// de ×3,78) — et résolveur d'images neutralisé (audit n°3,
/// A3-16) : le résolveur par défaut lit les fichiers locaux
/// pointés par un `href`, le nôtre ne résout rien.
fn options_import() -> usvg::Options<'static> {
    usvg::Options {
        dpi: 25.4,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            ..usvg::ImageHrefResolver::default()
        },
        ..usvg::Options::default()
    }
}

/// Refuse les DTD internes (audit n°3, A3-16) : `roxmltree`
/// accepte les entités et attributs par défaut du sous-ensemble
/// DTD interne (bombe d'entités, injection d'attributs) et `usvg`
/// n'expose aucune option pour le restreindre — un `<!DOCTYPE`
/// portant un crochet ouvrant avant son `>` de fermeture est
/// refusé ; un DOCTYPE PUBLIC seul (fichiers Inkscape) reste
/// accepté.
fn refuse_dtd_interne(bytes: &[u8]) -> Result<(), IoError> {
    let Some(debut) = bytes.windows(9).position(|fenetre| fenetre == b"<!DOCTYPE") else {
        return Ok(());
    };
    // Un DOCTYPE sans `>` de fermeture est malformé : l'analyse
    // `usvg` qui suit le refusera.
    let Some(fin) = bytes[debut..].iter().position(|&octet| octet == b'>') else {
        return Ok(());
    };
    if bytes[debut..debut + fin].contains(&b'[') {
        return Err(IoError::InvalidSvg {
            raison: "DTD interne refusée".to_string(),
        });
    }
    Ok(())
}

/// Importe un SVG en tracés 2D. Convention : une unité SVG = 1 mm ;
/// les tailles physiques (`width`/`height` en mm, in…) sont
/// résolues à 25,4 dpi — un « pixel » usvg vaut exactement 1 mm ;
/// chaque tracé est collecté avec sa transformation absolue
/// (usvg compose les transformations des ancêtres et du tracé), les
/// courbes deviennent des segments natifs (ADR-0024, phase B),
/// les nœuds non géométriques
/// (images, textes) sont ignorés ; l'axe Y du SVG (vers le bas)
/// est inversé pour entrer dans le monde (Y vers le haut) ; une
/// `viewBox` décalée est compensée — les coordonnées du contenu
/// sont restituées à l'origine de leurs unités, à l'échelle
/// physique déclarée (ADR-0016 B3). Les budgets protègent contre
/// les documents démesurés (ADR-0009).
pub fn import(bytes: &[u8], budgets: &SvgBudgets) -> Result<Vec<Path2>, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    refuse_dtd_interne(bytes)?;
    let options = options_import();
    let arbre = usvg::Tree::from_data(bytes, &options).map_err(|erreur| IoError::InvalidSvg {
        raison: erreur.to_string(),
    })?;
    // Compensation de la translation de la viewBox (ADR-0016
    // B3) : `usvg` normalise le contenu dans la viewBox, les
    // coordonnées importées sont décalées — l'origine des
    // unités du document est restituée, à l'échelle physique
    // déclarée.
    let decalage = decalage_viewbox(bytes, arbre.size());
    let mut traces: Vec<Path2> = Vec::new();
    let mut etat = Etat {
        chemins: 0,
        sommets: 0,
        budgets,
        decalage,
    };
    parcourir(arbre.root(), &mut traces, &mut etat)?;
    Ok(traces)
}

/// Translation de la `viewBox` compensée à l'import : `usvg`
/// normalise le contenu dans la boîte déclarée, l'origine des
/// unités du document est restituée (ADR-0016 B3).
fn decalage_viewbox(bytes: &[u8], taille: usvg::Size) -> (f64, f64) {
    let Some((x, y, largeur, hauteur)) = lis_viewbox(bytes) else {
        return (0.0, 0.0);
    };
    if largeur <= 0.0 || hauteur <= 0.0 {
        return (0.0, 0.0);
    }
    // Le viewport (taille résolue du document) met la viewBox
    // à l'échelle : la translation compensée suit la même
    // échelle, l'origine est restituée dans les unités du monde.
    let echelle_x = f64::from(taille.width()) / largeur;
    let echelle_y = f64::from(taille.height()) / hauteur;
    (x * echelle_x, y * echelle_y)
}

/// Extrait la `viewBox` de l'élément racine — `min_x min_y
/// largeur hauteur` — ou `None` si absente ou mal formée : le
/// contrôle échoue silencieusement, l'import continue sans
/// compensation. La lecture se fait dans l'arbre XML (audit
/// n°3, A3-04), jamais dans le texte brut : une `viewBox` citée
/// dans un commentaire ou portée par un `<symbol>` ne
/// s'applique pas ; virgules et espaces sont des séparateurs
/// valides au format SVG.
fn lis_viewbox(bytes: &[u8]) -> Option<(f64, f64, f64, f64)> {
    let texte = std::str::from_utf8(bytes).ok()?;
    let document = roxmltree::Document::parse(texte).ok()?;
    let racine = document.root_element();
    let attribut = racine.attribute("viewBox")?;
    let nombres: Vec<f64> = attribut
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|nombre| !nombre.is_empty())
        .map(|nombre| nombre.parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;
    if nombres.len() == 4 {
        Some((nombres[0], nombres[1], nombres[2], nombres[3]))
    } else {
        None
    }
}

/// Exporte les tracés en SVG : chaque tracé devient un élément
/// `path` (`M`/`L` par sommet, `Z` si fermé), coordonnées en
/// millimètres, l'axe Y du monde (vers le haut) inversé — le SVG
/// descend. L'en-tête déclare la boîte englobante (`width`/
/// `height` en mm, `viewBox` exact, aucune translation) : un
/// lecteur extérieur interprète le document à la bonne taille
/// physique ; chaque tracé est émis sans remplissage, avec un
/// trait fin.
pub fn export(traces: &[Path2]) -> Result<Vec<u8>, IoError> {
    use std::fmt::Write as _;
    // Boîte englobante en millimètres, coordonnées SVG (le monde
    // monte, le SVG descend) : l'en-tête déclare la taille physique
    // du document ; la viewBox couvre les Y négatifs, aucune
    // translation — les coordonnées restent exactes.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for trace in traces {
        for point in trace.points() {
            let x = point.x().millimeters();
            let y = -point.y().millimeters();
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    let mut sortie = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    if min_x <= max_x {
        // Boîte plate (ligne horizontale ou verticale) : une dimension
        // nulle rend le document invisible chez les lecteurs — chaque
        // dimension est plancher à la marge du trait (audit n°2, N9).
        let largeur = (max_x - min_x).max(MARGE_BOITE_PLATE);
        let hauteur = (max_y - min_y).max(MARGE_BOITE_PLATE);
        #[expect(clippy::expect_used)]
        writeln!(
            sortie,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}mm\" height=\"{hauteur}mm\" viewBox=\"{min_x} {min_y} {largeur} {hauteur}\">"
        )
        .expect("invariant : write! dans String infaillible hors OOM");
    } else {
        sortie.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\">\n");
    }
    for trace in traces {
        let points = trace.points();
        if points.is_empty() {
            continue;
        }
        sortie.push_str("<path d=\"");
        for (index, point) in points.iter().enumerate() {
            if index > 0 {
                sortie.push(' ');
            }
            let marque = if index == 0 { 'M' } else { 'L' };
            #[expect(clippy::expect_used)]
            write!(
                sortie,
                "{marque} {} {}",
                point.x().millimeters(),
                -point.y().millimeters()
            )
            .expect("invariant : write! dans String infaillible hors OOM");
        }
        if trace.is_closed() {
            sortie.push_str(" Z");
        }
        sortie.push_str("\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.1\"/>\n");
    }
    sortie.push_str("</svg>\n");
    Ok(sortie.into_bytes())
}

/// Compteurs et budgets de l'import en cours.
struct Etat<'a> {
    /// Tracés déjà collectés.
    chemins: usize,
    /// Sommets déjà collectés.
    sommets: usize,
    /// Budgets de l'import.
    budgets: &'a SvgBudgets,
    /// Translation de la viewBox compensée, mise à l'échelle du
    /// viewport : restitue l'origine des coordonnées du contenu
    /// (ADR-0016 B3).
    decalage: (f64, f64),
}

/// Parcourt un groupe : descend dans les sous-groupes et collecte
/// les tracés, chacun avec sa transformation absolue (ancêtres
/// compris, fournie par usvg). Les nœuds non géométriques (images,
/// textes) sont ignorés.
fn parcourir(
    groupe: &usvg::Group,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    for noeud in groupe.children() {
        match noeud {
            usvg::Node::Group(sous_groupe) => parcourir(sous_groupe, traces, etat)?,
            usvg::Node::Path(chemin) => {
                let globale = chemin.abs_transform();
                collecte(chemin.data(), &globale, traces, etat)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Collecte les segments d'un tracé : un sous-tracé par `MoveTo`,
/// fermé par `Close` — la fermeture est un segment explicite — et
/// courbes gardées natives (Béziers quadratiques et cubiques ;
/// usvg a déjà converti arcs elliptiques et primitives en Béziers
/// cubiques, cf. ADR-0024). Après un `Close`, le point courant
/// revient au départ du sous-tracé (sémantique SVG) : les
/// commandes suivantes reprennent depuis ce sommet, aucun segment
/// n'est perdu.
fn collecte(
    donnees: &usvg::tiny_skia_path::Path,
    transformation: &usvg::Transform,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    let mut segments: Vec<Segment2> = Vec::new();
    let mut courant: Option<Point2> = None;
    let mut depart: Option<Point2> = None;
    for segment in donnees.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                termine(&mut segments, traces, etat)?;
                let cible = construit(point.x, point.y, transformation, etat)?;
                courant = Some(cible);
                depart = Some(cible);
            }
            PathSegment::LineTo(point) => {
                let cible = construit(point.x, point.y, transformation, etat)?;
                if let Some(origine) = courant {
                    segments.push(Segment2::Ligne {
                        p1: origine,
                        p2: cible,
                    });
                }
                courant = Some(cible);
            }
            PathSegment::QuadTo(controle, fin) => {
                let c_controle = construit(controle.x, controle.y, transformation, etat)?;
                let c_fin = construit(fin.x, fin.y, transformation, etat)?;
                if let Some(origine) = courant {
                    segments.push(Segment2::BezierQuadratique {
                        p1: origine,
                        controle: c_controle,
                        p2: c_fin,
                    });
                }
                courant = Some(c_fin);
            }
            PathSegment::CubicTo(controle1, controle2, fin) => {
                let c1 = construit(controle1.x, controle1.y, transformation, etat)?;
                let c2 = construit(controle2.x, controle2.y, transformation, etat)?;
                let c_fin = construit(fin.x, fin.y, transformation, etat)?;
                if let Some(origine) = courant {
                    segments.push(Segment2::BezierCubique {
                        p1: origine,
                        c1,
                        c2,
                        p2: c_fin,
                    });
                }
                courant = Some(c_fin);
            }
            PathSegment::Close => {
                if let (Some(origine), Some(revient)) = (courant, depart)
                    && !points_recolles(origine, revient)
                {
                    segments.push(Segment2::Ligne {
                        p1: origine,
                        p2: revient,
                    });
                }
                termine(&mut segments, traces, etat)?;
                // Sémantique SVG : après `Z`, le point courant revient au
                // départ du sous-tracé — le sous-tracé suivant repart
                // de ce sommet.
                courant = depart;
            }
        }
    }
    termine(&mut segments, traces, etat)
}

/// Termine le sous-tracé courant : un tracé d'au moins un segment
/// est conservé (sous garde de budget), sinon les segments sont
/// abandonnés.
fn termine(
    segments: &mut Vec<Segment2>,
    traces: &mut Vec<Path2>,
    etat: &mut Etat,
) -> Result<(), IoError> {
    if segments.is_empty() {
        return Ok(());
    }
    if etat.chemins >= etat.budgets.max_paths {
        return Err(IoError::TooManyPaths {
            found: etat.chemins as u64 + 1,
            max: etat.budgets.max_paths as u64,
            unite: "tracés",
        });
    }
    traces.push(Path2::from_segments(std::mem::take(segments)));
    etat.chemins += 1;
    Ok(())
}

/// Construit un sommet ou un point de contrôle (espace transformé),
/// sous garde de budget. L'axe Y du SVG descend : il est inversé
/// ici — le monde monte.
fn construit(
    x: f32,
    y: f32,
    transformation: &usvg::Transform,
    etat: &mut Etat,
) -> Result<Point2, IoError> {
    if etat.sommets >= etat.budgets.max_vertices {
        return Err(IoError::TooManyPaths {
            found: etat.sommets as u64 + 1,
            max: etat.budgets.max_vertices as u64,
            unite: "sommets",
        });
    }
    let (gx, gy) = applique(transformation, x, y);
    let gx_mm = Length::from_millimeters(gx + etat.decalage.0);
    let gy_mm = Length::from_millimeters(-(gy + etat.decalage.1));
    let point = Point2::new(gx_mm, gy_mm).map_err(IoError::InvalidGeometry)?;
    etat.sommets += 1;
    Ok(point)
}

/// Applique une transformation à un point (espace SVG vers espace
/// global, en `f64`).
fn applique(t: &usvg::Transform, x: f32, y: f32) -> (f64, f64) {
    (
        (t.sx * x + t.kx * y + t.tx) as f64,
        (t.ky * x + t.sy * y + t.ty) as f64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_resolveur_d_images_ne_charge_rien() {
        // Le résolveur par défaut de `usvg` lit les fichiers locaux
        // pointés par un `href` (audit n°3, A3-16) : le nôtre ne
        // résout rien — même un PNG valide existant sur disque.
        const PNG_1X1: &[u8] =
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\rIDATx\x9cb\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82";
        let fichier = std::env::temp_dir().join("omnifab-resolveur-ne-charge-rien.png");
        std::fs::write(&fichier, PNG_1X1).expect("écriture du png de test");
        let options = options_import();
        let resolution = &options.image_href_resolver.resolve_string;
        let chemin = fichier.to_string_lossy();
        assert!(
            resolution(&chemin, &options).is_none(),
            "même un png valide sur disque ne doit pas être résolu"
        );
        let _ = std::fs::remove_file(&fichier);
    }
}
