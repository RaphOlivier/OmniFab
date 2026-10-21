use forge_values::{ValueGraph, ValuesError};

fn assert_code(error: &ValuesError, code: &str) {
    assert!(
        error.to_string().starts_with(code),
        "attendu {code}, obtenu : {error}"
    );
}

#[test]
fn chaine_de_liaisons_simple() {
    let mut graph = ValueGraph::new();
    graph.set_input("width_mm", 800.0).unwrap();
    graph.set_input("height_mm", 600.0).unwrap();
    graph
        .set_expression(
            "diagonal",
            "sqrt(width_mm * width_mm + height_mm * height_mm)",
        )
        .unwrap();
    let diagonal = graph.value("diagonal").unwrap();
    assert!((diagonal - 1000.0).abs() < 1e-9);

    // Invalidation paresseuse : la dépendance est recalculée à la lecture.
    graph.set_input("width_mm", 600.0).unwrap();
    let diagonal = graph.value("diagonal").unwrap();
    assert!((diagonal - 848.5281374238571).abs() < 1e-9);
}

#[test]
fn marquage_dirty_explicite() {
    let mut graph = ValueGraph::new();
    graph.set_input("a", 1.0).unwrap();
    assert_eq!(graph.is_dirty("a"), Some(true));
    graph.value("a").unwrap();
    assert_eq!(graph.is_dirty("a"), Some(false));
    graph.set_input("a", 2.0).unwrap();
    assert_eq!(graph.is_dirty("a"), Some(true));
    assert_eq!(graph.is_dirty("inconnue"), None);
}

#[test]
fn erreur_differee_jusqua_la_lecture() {
    // ADR-0002 : une erreur n'apparaît qu'au moment de la lecture.
    let mut graph = ValueGraph::new();
    graph.set_input("a", 1.0).unwrap();
    graph.set_expression("boom", "1 / (a - a)").unwrap();
    assert!(graph.value("boom").is_err());
    // Le reste du graphe reste sain.
    assert_eq!(graph.value("a").unwrap(), 1.0);
}

#[test]
fn cycle_traverse_refuse_a_la_definition() {
    // ADR-0020 : un cycle traversé est refusé au moment de poser
    // l'expression — la valeur précédente tient, rien n'est gelé.
    let mut graph = ValueGraph::new();
    graph.set_input("b", 1.0).unwrap();
    graph.set_expression("a", "b").unwrap();
    let error = graph.set_expression("b", "a").unwrap_err();
    assert_code(&error, "OF-VAL-0001");
    match &error {
        ValuesError::Cycle { path } => {
            assert_eq!(path, "b → a → b", "chemin du cycle");
        }
        other => panic!("cycle attendu, obtenu {other:?}"),
    }
    // La valeur précédente tient : a suit b, b reste son entrée.
    assert_eq!(graph.value("a").unwrap(), 1.0, "a suit encore b");
    assert_eq!(graph.value("b").unwrap(), 1.0, "b garde son entrée");
}

#[test]
fn cycle_profond_refuse_aussi() {
    // Cycle à trois maillons : la traversée transitive est couverte.
    let mut graph = ValueGraph::new();
    graph.set_input("x", 1.0).unwrap();
    graph.set_expression("a", "x").unwrap();
    graph.set_expression("b", "a").unwrap();
    let error = graph.set_expression("x", "b").unwrap_err();
    assert_code(&error, "OF-VAL-0001");
    assert_eq!(graph.value("x").unwrap(), 1.0, "x garde son entrée");
    assert_eq!(graph.value("b").unwrap(), 1.0, "b suit a qui suit x");
}

#[test]
fn rebranchement_sans_cycle_accepte() {
    // Garde-fou : redéfinir une expression sur un graphe sain sans
    // créer de cycle reste accepté (ADR-0020).
    let mut graph = ValueGraph::new();
    graph.set_input("a", 1.0).unwrap();
    graph.set_expression("b", "a").unwrap();
    graph.set_expression("c", "b + 1").unwrap();
    graph.set_expression("b", "c - 1").unwrap_err();
    graph.set_expression("b", "a + 1").unwrap();
    assert_eq!(graph.value("c").unwrap(), 3.0, "c suit le nouveau b");
}

#[test]
fn reference_inconnue() {
    let mut graph = ValueGraph::new();
    let error = graph.set_expression("x", "introuvable + 1").unwrap_err();
    assert_code(&error, "OF-VAL-0002");
}

#[test]
fn expression_invalide() {
    let mut graph = ValueGraph::new();
    let error = graph.set_expression("x", "1 +").unwrap_err();
    assert_code(&error, "OF-VAL-0003");
    let error = graph
        .set_expression("y", "fonction_inconnue(2)")
        .unwrap_err();
    assert_code(&error, "OF-VAL-0003");
}

#[test]
fn division_par_zero() {
    let mut graph = ValueGraph::new();
    graph.set_input("zero", 0.0).unwrap();
    graph.set_expression("x", "1 / zero").unwrap();
    let error = graph.value("x").unwrap_err();
    assert_code(&error, "OF-VAL-0004");
}

#[test]
fn valeur_inconnue() {
    let mut graph = ValueGraph::new();
    let error = graph.value("fantome").unwrap_err();
    assert_code(&error, "OF-VAL-0005");
}

#[test]
fn recalculer_tout_expose_les_erreurs() {
    let mut graph = ValueGraph::new();
    graph.set_input("a", 1.0).unwrap();
    graph.set_expression("ok", "a * 2").unwrap();
    graph.set_expression("ko", "1 / (a - a)").unwrap();
    assert!(graph.evaluate_all().is_err());

    graph.set_expression("ko", "1 / a").unwrap();
    graph.evaluate_all().unwrap();
    assert!((graph.value("ok").unwrap() - 2.0).abs() < 1e-12);
}

#[test]
fn fonctions_et_precedence() {
    let mut graph = ValueGraph::new();
    graph.set_input("w", 800.0).unwrap();
    graph.set_input("h", 600.0).unwrap();
    graph
        .set_expression("ratio", "min(w, h) / max(w, h)")
        .unwrap();
    let ratio = graph.value("ratio").unwrap();
    assert!((ratio - 0.75).abs() < 1e-12);

    graph.set_expression("neg", "-w + 100").unwrap();
    assert!((graph.value("neg").unwrap() + 700.0).abs() < 1e-12);
}

#[test]
fn rejeu_deterministe() {
    let build = || {
        let mut graph = ValueGraph::new();
        graph.set_input("w", 12.0).unwrap();
        graph
            .set_expression("x", "sqrt(w * w + 1) + cos(0)")
            .unwrap();
        graph.value("x").unwrap()
    };
    assert!((build() - build()).abs() < 1e-12);
}
