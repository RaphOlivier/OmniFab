//! Tests de forge-io (ADR-0009) : STL binaire et ASCII, budgets,
//! aller-retour export → import.

use forge_geom::{Mesh, Point3};
use forge_io::stl::{StlBudgets, export, import};
use forge_units::Length;

/// Un STL ASCII minimal : un triangle dans le plan z = 0.
const STL_ASCII: &str = "solid demo
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 0 1 0
    endloop
  endfacet
endsolid demo
";

/// Budgets explicites (la sécurité ne bride pas les fonctions :
/// chaque budget est réglable, ADR-0009).
fn budgets(max_bytes: usize, max_triangles: u32) -> StlBudgets {
    StlBudgets {
        max_bytes,
        max_triangles,
    }
}

/// Un triangle simple : origine, +1 mm en x, +1 mm en y.
fn triangle() -> Mesh {
    let un = Length::from_millimeters(1.0);
    let a = Point3::new(Length::ZERO, Length::ZERO, Length::ZERO).expect("fini");
    let b = Point3::new(un, Length::ZERO, Length::ZERO).expect("fini");
    let c = Point3::new(Length::ZERO, un, Length::ZERO).expect("fini");
    Mesh::from_parts(vec![a, b, c], vec![[0, 1, 2]]).expect("indices valides")
}

#[test]
fn aller_retour_binaire() {
    let budgets = StlBudgets::default();
    let octets = export(&triangle()).expect("export");
    let maillage = import(&octets, &budgets).expect("import");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
    assert_eq!(maillage.vertex_count(), 3, "trois sommets");
    let x = maillage.vertices()[1].x().millimeters();
    assert_eq!(x, 1.0);
}

#[test]
fn import_ascii() {
    let budgets = StlBudgets::default();
    let maillage = import(STL_ASCII.as_bytes(), &budgets).expect("import ASCII");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
    let y = maillage.vertices()[2].y().millimeters();
    assert_eq!(y, 1.0);
}

#[test]
fn import_ascii_crlf() {
    let contenu = STL_ASCII.replace('\n', "\r\n");
    let budgets = StlBudgets::default();
    let maillage = import(contenu.as_bytes(), &budgets).expect("import ASCII CRLF");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
}

#[test]
fn binaire_en_tete_solid() {
    let budgets = StlBudgets::default();
    let mut octets = export(&triangle()).expect("export");
    octets[..5].copy_from_slice(b"solid");
    let maillage = import(&octets, &budgets).expect("lu comme binaire");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
}

#[test]
fn budget_octets_depasse() {
    let budgets = budgets(10, 4_000_000);
    let erreur = import(STL_ASCII.as_bytes(), &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0001");
}

#[test]
fn budget_triangles_depasse() {
    let budgets = budgets(256 * 1024 * 1024, 1);
    let mut octets = vec![0u8; 84 + 50 * 3];
    octets[80..84].copy_from_slice(&3u32.to_le_bytes());
    let erreur = import(&octets, &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0003");
}

#[test]
fn format_inconnu() {
    let budgets = StlBudgets::default();
    let erreur = import(b"ni binaire ni ascii", &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0005");
}

#[test]
fn binaire_tronque_format_inconnu() {
    let budgets = StlBudgets::default();
    let mut octets = export(&triangle()).expect("export");
    octets.truncate(octets.len() - 10);
    let erreur = import(&octets, &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0005");
}

#[test]
fn binaire_avec_bourrage_final_importe() {
    // Des logiciels terminent le STL binaire par des octets
    // excédents : la lecture tolérante les ignore.
    let budgets = StlBudgets::default();
    let mut octets = export(&triangle()).expect("export");
    octets.extend_from_slice(&[0u8; 7]);
    let maillage = import(&octets, &budgets).expect("bourrage final toléré");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
}

#[test]
fn compteur_demensuel_petit_fichier_erreur_propre() {
    // Compteur u32::MAX démenti par la taille : erreur typée,
    // jamais de débordement (arithmétique u64, cible 32 bits
    // comprise).
    let budgets = StlBudgets::default();
    let mut octets = vec![0u8; 84];
    octets[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
    let erreur = import(&octets, &budgets).expect_err("compteur démenti par la taille");
    assert_eq!(erreur.code(), "OF-IO-0005");
}

#[test]
fn ascii_invalide() {
    let budgets = StlBudgets::default();
    let contenu = "solid demo\nbizarre 1 2 3\nendsolid demo\n";
    let erreur = import(contenu.as_bytes(), &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0004");
}

#[test]
fn ascii_coordonnee_non_finie() {
    let budgets = StlBudgets::default();
    let contenu = "solid demo\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex nan 1 0\nendloop\nendfacet\nendsolid\n";
    let erreur = import(contenu.as_bytes(), &budgets).unwrap_err();
    assert_eq!(erreur.code(), "OF-IO-0006");
}

#[test]
fn export_vide_donne_un_stl_valide() {
    let budgets = StlBudgets::default();
    let octets = export(&Mesh::new()).expect("export vide");
    let maillage = import(&octets, &budgets).expect("import");
    assert_eq!(maillage.triangle_count(), 0, "aucun triangle");
}

#[test]
fn binaire_en_tete_solid_avec_bourrage_est_lu_comme_binaire() {
    // Audit n°3, A3-14 : un STL binaire dont l'en-tête commence par
    // « solid » ET portant du bourrage final (cas que la doc du
    // module dit toléré) partait vers le parseur ASCII (« octets non
    // UTF-8 ») — le test tolérant doit passer AVANT la détection
    // ASCII ; sans risque pour un vrai ASCII : ses octets 80..84
    // forment un compteur démenti par la taille du fichier.
    let budgets = StlBudgets::default();
    let mut octets = export(&triangle()).expect("export");
    octets[..6].copy_from_slice(b"solid ");
    octets.extend_from_slice(&[0xFF; 10]);
    let maillage = import(&octets, &budgets).expect("binaire tolérant lu d'abord");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
}

#[test]
fn ascii_fins_de_ligne_cr_seules_lit_les_facettes() {
    // Audit n°3, A3-15 : seuls les premiers mots de chaque ligne
    // étaient lus — des fins de ligne CR seules (tout le fichier sur
    // une « ligne ») ou des facettes entassées sur une ligne
    // produisaient un import « réussi » à 0 triangle, sans erreur.
    let budgets = StlBudgets::default();
    let contenu = "solid demo\rfacet normal 0 0 1\router loop\rvertex 0 0 0\rvertex 1 0 0\rvertex 0 1 0\rendloop\rendfacet\rendsolid demo\r";
    let maillage = import(contenu.as_bytes(), &budgets).expect("jetons lus en flux");
    assert_eq!(maillage.triangle_count(), 1, "un triangle");
}
