//! Import/export STL (ADR-0009) : parseur maison, budgets intégrés.
//!
//! Le format STL est **sans unité** ; OmniFab le lit et l'écrit en
//! millimètres (convention usinage). Les deux variantes sont
//! acceptées :
//!
//! - **binaire** : 80 octets d'en-tête, un `u32` de nombre de
//!   triangles, puis 50 octets par triangle (normale et attribut lus
//!   et ignorés ; les normales sont recalculées à l'export) ;
//! - **ASCII** : sous-ensemble strict `solid` / `facet normal` /
//!   `outer loop` / `vertex` / `endloop` / `endfacet` /
//!   `endsolid`, mots-clés insensibles à la casse, fins de ligne LF
//!   ou CRLF tolérées.
//!
//! Sécurité (ADR-0009) : la détection du format se fait par
//! cohérence de la taille (`84 + 50 × n`), pas par l'en-tête — un
//! binaire dont l'en-tête commence par « solid » est bien lu en
//! binaire ; un binaire portant des octets excédentaires est lu
//! en tolérant (bourrage de fin de fichier ignoré). Tout
//! dépassement de [`StlBudgets`] est une erreur typée avant toute
//! allocation proportionnelle.

use crate::IoError;
use forge_geom::{Mesh, Point3};
use forge_units::Length;

/// En-tête du fichier binaire exporté (complété de zéros jusqu'à 80
/// octets).
const EN_TETE: &[u8] = b"OmniFab forge-io STL binaire";

/// Budgets de ressources du parseur STL (ADR-0009).
///
/// La sécurité ne doit pas brider les fonctions : chaque budget est
/// public et réglable par l'appelant ; les défauts couvrent les
/// fichiers réels des logiciels de référence (Blender, PrusaSlicer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StlBudgets {
    /// Taille maximale acceptée en octets (défaut : 256 Mio).
    pub max_bytes: usize,
    /// Nombre maximal de triangles accepté (défaut : 4 000 000).
    pub max_triangles: u32,
}

impl Default for StlBudgets {
    /// Défauts : 256 Mio, 4 000 000 triangles.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_triangles: 4_000_000,
        }
    }
}

/// Importe un STL (binaire ou ASCII, détection par cohérence des
/// tailles) en maillage. Les coordonnées sont lues en millimètres.
/// La détection tolérante précède le test ASCII : un binaire
/// « solid » portant du bourrage reste lu en binaire (audit n°3,
/// A3-14).
///
/// Les budgets protègent contre les fichiers démesurés (ADR-0009) ;
/// [`StlBudgets::default`] couvre les fichiers réels des logiciels
/// de référence.
pub fn import(bytes: &[u8], budgets: &StlBudgets) -> Result<Mesh, IoError> {
    if bytes.len() > budgets.max_bytes {
        return Err(IoError::TooLarge {
            found: bytes.len(),
            max: budgets.max_bytes,
        });
    }
    if est_binaire(bytes) {
        return import_binaire(bytes, budgets);
    }
    if est_binaire_tolerant(bytes) {
        return import_binaire(bytes, budgets);
    }
    if est_ascii(bytes) {
        return import_ascii(bytes, budgets);
    }
    Err(IoError::UnknownFormat)
}

/// Exporte le maillage en STL binaire (unités : millimètres).
///
/// Les normales de facette sont recalculées ; un triangle dégénéré
/// exporte une normale nulle, acceptée par tous les lecteurs. Un
/// maillage de plus de `u32::MAX` triangles est refusé (OF-IO-0003).
pub fn export(mesh: &Mesh) -> Result<Vec<u8>, IoError> {
    let total = compte_u32(mesh.triangle_count())?;
    let mut sortie = Vec::with_capacity(84 + 50 * total as usize);
    sortie.extend_from_slice(EN_TETE);
    sortie.extend_from_slice(&[0u8; 80 - EN_TETE.len()]);
    sortie.extend_from_slice(&total.to_le_bytes());
    for &[ia, ib, ic] in mesh.triangles() {
        let a = mesh
            .vertices()
            .get(ia as usize)
            .expect("invariant : indices validés par Mesh::from_parts");
        let b = mesh
            .vertices()
            .get(ib as usize)
            .expect("invariant : indices validés par Mesh::from_parts");
        let c = mesh
            .vertices()
            .get(ic as usize)
            .expect("invariant : indices validés par Mesh::from_parts");
        for composante in normale_facette(a, b, c) {
            sortie.extend_from_slice(&composante.to_le_bytes());
        }
        for point in [a, b, c] {
            sortie.extend_from_slice(&(point.x().millimeters() as f32).to_le_bytes());
            sortie.extend_from_slice(&(point.y().millimeters() as f32).to_le_bytes());
            sortie.extend_from_slice(&(point.z().millimeters() as f32).to_le_bytes());
        }
        sortie.extend_from_slice(&0u16.to_le_bytes());
    }
    Ok(sortie)
}

/// Détecte un STL binaire : la taille doit être exactement
/// `84 + 50 × n` (en-tête 80 octets + compteur `u32` + `n` facettes
/// de 50 octets).
fn est_binaire(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let compteur = u32::from_le_bytes(
        bytes[80..84]
            .try_into()
            .expect("invariant : tranche [80..84] garantie par la garde len ≥ 84"),
    );
    bytes.len() as u64 == 84 + 50 * u64::from(compteur)
}

/// Détecte un STL binaire tolérant : le fichier porte au moins les
/// `84 + 50 × n` octets annoncés (arithmétique en `u64`) — les octets
/// excédentaires (bourrage de fin de fichier produit par certains
/// logiciels) sont ignorés à la lecture.
fn est_binaire_tolerant(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let compteur = u32::from_le_bytes(
        bytes[80..84]
            .try_into()
            .expect("invariant : tranche [80..84] garantie par la garde len ≥ 84"),
    );
    bytes.len() as u64 >= 84 + 50 * u64::from(compteur)
}

/// Détecte un STL ASCII : premier mot « solid » (BOM UTF-8 toléré).
fn est_ascii(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if bytes.len() < 5 || !bytes[..5].eq_ignore_ascii_case(b"solid") {
        return false;
    }
    matches!(
        bytes.get(5),
        None | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
    )
}

/// Importe un STL binaire (appelée après détection binaire, exacte
/// ou tolérante — les octets excédentaires sont ignorés).
fn import_binaire(bytes: &[u8], budgets: &StlBudgets) -> Result<Mesh, IoError> {
    let compteur = u32::from_le_bytes(
        bytes[80..84]
            .try_into()
            .expect("invariant : tranche [80..84] garantie par la garde len ≥ 84"),
    );
    if u64::from(compteur) > u64::from(budgets.max_triangles) {
        return Err(IoError::TooManyTriangles {
            found: u64::from(compteur),
            max: u64::from(budgets.max_triangles),
        });
    }
    let mut vertices = Vec::with_capacity(3 * compteur as usize);
    let mut triangles = Vec::with_capacity(compteur as usize);
    for facette in 0..compteur as usize {
        let debut = 84 + facette * 50;
        let bloc = &bytes[debut..debut + 50];
        for sommet in 0..3 {
            let base = 12 + sommet * 12;
            let a = Length::from_millimeters(lire_f32(bloc, base).into());
            let b = Length::from_millimeters(lire_f32(bloc, base + 4).into());
            let c = Length::from_millimeters(lire_f32(bloc, base + 8).into());
            vertices.push(Point3::new(a, b, c).map_err(IoError::InvalidGeometry)?);
        }
        let premier = (facette * 3) as u32;
        triangles.push([premier, premier + 1, premier + 2]);
    }
    Mesh::from_parts(vertices, triangles).map_err(IoError::InvalidGeometry)
}

/// Importe un STL ASCII (sous-ensemble strict, cf. documentation du
/// module). Tous les mots d'une ligne sont lus : des fins de ligne
/// CR seules ou des facettes entassées ne produisent plus un import
/// silencieusement vide (audit n°3, A3-15).
fn import_ascii(bytes: &[u8], budgets: &StlBudgets) -> Result<Mesh, IoError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| ascii(1, "octets non UTF-8"))?;
    let mut vertices: Vec<Point3> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut facet_vertices: [Option<Point3>; 3] = [None, None, None];
    let mut en_facet = false;
    let mut en_boucle = false;
    for (index, ligne) in text.split('\n').enumerate() {
        let numero = index + 1;
        let mut tokens = ligne.split_whitespace();
        // Tous les mots de la ligne sont lus (audit n°3, A3-15) :
        // des fins de ligne CR seules (tout le fichier sur une
        // « ligne ») ou des facettes entassées ne produisent plus
        // un import silencieusement vide.
        let mut nom_de_solide = false;
        while let Some(mot) = tokens.next() {
            let mot = mot.to_ascii_lowercase();
            match mot.as_str() {
                "solid" | "endsolid" => {
                    // Le nom du solide peut suivre sur la même
                    // ligne : les jetons non reconnus sont ignorés
                    // jusqu'au prochain mot-clé.
                    nom_de_solide = true;
                }
                "facet" => {
                    nom_de_solide = false;
                    if en_facet {
                        return Err(ascii(numero, "facet imbriqué"));
                    }
                    let Some(nom) = tokens.next() else {
                        return Err(ascii(numero, "attendu : facet normal"));
                    };
                    if !nom.eq_ignore_ascii_case("normal") {
                        return Err(ascii(numero, "attendu : facet normal"));
                    }
                    for _ in 0..3 {
                        nombre(tokens.next(), numero)?;
                    }
                    en_facet = true;
                    en_boucle = false;
                    facet_vertices = [None, None, None];
                }
                "outer" => {
                    nom_de_solide = false;
                    if !en_facet {
                        return Err(ascii(numero, "outer loop hors facet"));
                    }
                    let Some(nom) = tokens.next() else {
                        return Err(ascii(numero, "attendu : outer loop"));
                    };
                    if !nom.eq_ignore_ascii_case("loop") {
                        return Err(ascii(numero, "attendu : outer loop"));
                    }
                    en_boucle = true;
                }
                "vertex" => {
                    nom_de_solide = false;
                    if !en_boucle {
                        return Err(ascii(numero, "sommet hors boucle"));
                    }
                    let x = nombre(tokens.next(), numero)?;
                    let y = nombre(tokens.next(), numero)?;
                    let z = nombre(tokens.next(), numero)?;
                    let a = Length::from_millimeters(x);
                    let b = Length::from_millimeters(y);
                    let c = Length::from_millimeters(z);
                    let point = Point3::new(a, b, c).map_err(IoError::InvalidGeometry)?;
                    let Some(slot) = facet_vertices.iter_mut().find(|slot| slot.is_none()) else {
                        return Err(ascii(numero, "plus de trois sommets dans la boucle"));
                    };
                    *slot = Some(point);
                }
                "endloop" => {
                    nom_de_solide = false;
                    en_boucle = false;
                }
                "endfacet" => {
                    nom_de_solide = false;
                    if !en_facet {
                        return Err(ascii(numero, "endfacet sans facet"));
                    }
                    let points = match facet_vertices {
                        [Some(a), Some(b), Some(c)] => [a, b, c],
                        _ => return Err(ascii(numero, "trois sommets attendus par facet")),
                    };
                    if triangles.len() >= budgets.max_triangles as usize {
                        return Err(IoError::TooManyTriangles {
                            found: triangles.len() as u64 + 1,
                            max: budgets.max_triangles as u64,
                        });
                    }
                    let base = compte_u32(vertices.len())?;
                    vertices.extend(points);
                    triangles.push([base, base + 1, base + 2]);
                    en_facet = false;
                    en_boucle = false;
                }
                _ => {
                    if !nom_de_solide {
                        return Err(ascii(numero, &format!("mot-clé inattendu « {mot} »")));
                    }
                }
            }
        }
    }
    if en_facet {
        return Err(ascii(0, "fin de fichier au milieu d'un facet"));
    }
    Mesh::from_parts(vertices, triangles).map_err(IoError::InvalidGeometry)
}

/// Lit un `f32` petit-boutiste à l'offset donné.
fn lire_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("invariant : offset borné par est_binaire_tolerant (len ≥ 84 + 50n)"),
    )
}

/// Convertit un compte `usize` en `u32` (garde OF-IO-0003 au-delà de
/// `u32::MAX`).
fn compte_u32(compte: usize) -> Result<u32, IoError> {
    u32::try_from(compte).map_err(|_| IoError::TooManyTriangles {
        found: compte as u64,
        max: u64::from(u32::MAX),
    })
}

/// Normale unitaire d'une facette ; vecteur nul si dégénérée.
fn normale_facette(a: &Point3, b: &Point3, c: &Point3) -> [f32; 3] {
    let ux = b.x().millimeters() - a.x().millimeters();
    let uy = b.y().millimeters() - a.y().millimeters();
    let uz = b.z().millimeters() - a.z().millimeters();
    let vx = c.x().millimeters() - a.x().millimeters();
    let vy = c.y().millimeters() - a.y().millimeters();
    let vz = c.z().millimeters() - a.z().millimeters();
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let norme = (nx * nx + ny * ny + nz * nz).sqrt();
    if !norme.is_finite() || norme == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    [
        (nx / norme) as f32,
        (ny / norme) as f32,
        (nz / norme) as f32,
    ]
}

/// Lit le prochain nombre de la ligne, ou erreur ASCII typée.
fn nombre(token: Option<&str>, line: usize) -> Result<f64, IoError> {
    let token = token.ok_or_else(|| ascii(line, "nombre manquant"))?;
    token
        .parse::<f64>()
        .map_err(|_| ascii(line, &format!("nombre invalide « {token} »")))
}

/// Erreur ASCII typée : ligne fautive et raison.
fn ascii(line: usize, reason: &str) -> IoError {
    IoError::InvalidAscii {
        line,
        reason: reason.to_string(),
    }
}
