//! Dialogues modal du démonstrateur (ADR-0019 §6) : états des
//! fenêtres « + Nouvel alias » et de suppression d'un alias —
//! extraits de `main.rs` (découpage étape 3, fiche
//! `preparation/decoupage-main-rs.md`).

use forge_session::Unite;

/// Dialogue « + Nouvel alias » (ADR-0019 §6) : nom, entrée ou
/// expression, unité — état de l'app UI, jamais un événement tant
/// que « Poser » n'est pas cliqué.
pub(crate) struct DialogueNouvelAlias {
    /// Nom de l'alias à créer.
    pub(crate) nom: String,
    /// `true` : expression calculée ; `false` : entrée numérique.
    pub(crate) expression: bool,
    /// Valeur de l'entrée.
    pub(crate) valeur: f64,
    /// Texte de l'expression.
    pub(crate) texte: String,
    /// Unité déclarée (`None` = sans unité).
    pub(crate) unite: Option<Unite>,
}

/// Dialogue de suppression d'un alias référencé (ADR-0019 §6,
/// amendement 2026-10-20) : les formules concernées sont listées, la
/// suppression propose cascade, remplacement ou annulation.
pub(crate) struct DialogueSuppression {
    /// Alias demandé en suppression.
    pub(crate) alias: String,
    /// Formules et cotes qui lisent encore l'alias.
    pub(crate) references: Vec<String>,
    /// Alias de remplacement choisi (mode Remplacer).
    pub(crate) remplacement: String,
}
