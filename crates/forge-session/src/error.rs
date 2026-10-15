//! Erreurs typées de forge-session (ADR-0003 : codes stables `OF-SES-NNNN`,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreur du domaine session. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Commande refusée à la planification (OF-SES-0001).
    #[error("OF-SES-0001 : commande refusée : {reason}")]
    CommandRejected {
        /// Explication du refus.
        reason: String,
    },
    /// Rejet par une projection lors de l'application de l'événement (OF-SES-0002).
    #[error("OF-SES-0002 : la projection « {projection} » rejette l'événement : {message}")]
    ProjectionFailed {
        /// Nom du domaine de la projection qui refuse.
        projection: String,
        /// Message de l'erreur du domaine (avec son code OF-*).
        message: String,
    },
    /// Undo demandé alors que tout est déjà annulé (OF-SES-0003).
    #[error("OF-SES-0003 : rien à annuler")]
    UndoEmpty,
    /// Redo demandé alors qu'aucun événement défait n'est en attente (OF-SES-0004).
    #[error("OF-SES-0004 : rien à rétablir")]
    RedoEmpty,
    /// Données d'événement non conformes au schéma connu du type (OF-SES-0005).
    #[error("OF-SES-0005 : données invalides pour « {event_type} » : {reason}")]
    InvalidEventData {
        /// Type de l'événement fautive.
        event_type: String,
        /// Champ manquant ou de type inattendu.
        reason: String,
    },
    /// Blob refusé par le document lors de l'exécution d'une commande
    /// (OF-SES-0006) : nom invalide (OF-DOC-0013) ou taille hors bornes
    /// (OF-DOC-0011).
    #[error("OF-SES-0006 : blob refusé par le document : {reason}")]
    BlobRejected {
        /// Message de l'erreur du document (avec son code OF-DOC-*).
        reason: String,
    },
}

impl SessionError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CommandRejected { .. } => "OF-SES-0001",
            Self::ProjectionFailed { .. } => "OF-SES-0002",
            Self::UndoEmpty => "OF-SES-0003",
            Self::RedoEmpty => "OF-SES-0004",
            Self::InvalidEventData { .. } => "OF-SES-0005",
            Self::BlobRejected { .. } => "OF-SES-0006",
        }
    }
}
