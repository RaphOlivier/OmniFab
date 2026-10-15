//! Erreurs typées de forge-i18n (ADR-0003 : codes stables
//! `OF-I18N-NNNN`, registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreur du catalogue Fluent. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum I18nError {
    /// Clé absente du catalogue : message absent, ou message sans
    /// valeur (OF-I18N-0001).
    #[error("OF-I18N-0001 : clé absente du catalogue : {cle}")]
    CleAbsente {
        /// Clé demandée.
        cle: String,
    },
    /// Ressource FTL invalide : syntaxe refusée par le parseur,
    /// message en doublon, ou identifiant de langue invalide
    /// (OF-I18N-0002).
    #[error("OF-I18N-0002 : ressource FTL invalide : {raison}")]
    RessourceInvalide {
        /// Explication.
        raison: String,
    },
    /// Page d'aide incomplète : attribut manquant parmi `titre`,
    /// `description` et `exemple` (OF-I18N-0003).
    #[error("OF-I18N-0003 : page d'aide incomplète ({help_id}) : attribut {attribut} manquant")]
    AideIncomplete {
        /// Identifiant de la page (help_id).
        help_id: String,
        /// Attribut manquant.
        attribut: String,
    },
    /// Erreur de mise en forme : argument manquant ou inconnu
    /// (OF-I18N-0004).
    #[error("OF-I18N-0004 : erreur de mise en forme ({cle}) : {raison}")]
    MiseEnForme {
        /// Clé mise en forme.
        cle: String,
        /// Explication.
        raison: String,
    },
}

impl I18nError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CleAbsente { .. } => "OF-I18N-0001",
            Self::RessourceInvalide { .. } => "OF-I18N-0002",
            Self::AideIncomplete { .. } => "OF-I18N-0003",
            Self::MiseEnForme { .. } => "OF-I18N-0004",
        }
    }
}
