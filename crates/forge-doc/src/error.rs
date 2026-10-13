//! Erreurs typées de forge-doc (ADR-0003 : codes stables `OF-DOC-NNNN`,
//! registre central `docs/error-codes.md`).

use thiserror::Error;

/// Erreur du domaine document. Chaque variante porte un code stable
/// documenté dans `docs/error-codes.md`.
#[derive(Debug, Error)]
pub enum DocError {
    /// Conteneur écrit par une version plus récente du logiciel (OF-DOC-0001).
    #[error("OF-DOC-0001 : version du conteneur non supportée : {found} (supportée au plus : {supported})")]
    UnsupportedContainerVersion {
        /// Version rencontrée dans le manifeste.
        found: u32,
        /// Version maximale supportée par ce logiciel.
        supported: u32,
    },
    /// Le champ `format` du manifeste est inconnu (OF-DOC-0002).
    #[error("OF-DOC-0002 : format de conteneur inconnu « {format} »")]
    UnknownContainerFormat {
        /// Valeur du champ `format`.
        format: String,
    },
    /// Ligne du journal non conforme (OF-DOC-0003).
    #[error("OF-DOC-0003 : journal corrompu, ligne {line} : {reason}")]
    CorruptJournal {
        /// Numéro de la ligne fautive (1-based).
        line: usize,
        /// Explication de la non-conformité.
        reason: String,
    },
    /// Séquence d'événements non strictement croissante (OF-DOC-0004).
    #[error(
        "OF-DOC-0004 : séquence d'événements incohérente : attendu {expected}, trouvé {found}"
    )]
    BadSequence {
        /// Numéro de séquence attendu.
        expected: u64,
        /// Numéro de séquence rencontré.
        found: u64,
    },
    /// Événement d'une version de schéma trop récente (OF-DOC-0005).
    #[error("OF-DOC-0005 : version de schéma {version} de « {event_type} » supérieure à la plus récente connue")]
    SchemaTooNew {
        /// Type de l'événement.
        event_type: String,
        /// Version de schéma rencontrée.
        version: u32,
    },
    /// Étape de migration manquante (OF-DOC-0006).
    #[error("OF-DOC-0006 : migration manquante pour « {event_type} » v{from} → v{to}")]
    MissingMigration {
        /// Type de l'événement.
        event_type: String,
        /// Version source absente du registre de migrations.
        from: u32,
        /// Version cible attendue.
        to: u32,
    },
    /// Entrée absente du conteneur ZIP (OF-DOC-0007).
    #[error("OF-DOC-0007 : entrée absente du conteneur : {path}")]
    MissingEntry {
        /// Chemin de l'entrée manquante.
        path: String,
    },
    /// Erreur d'archive ZIP (OF-DOC-0008).
    #[error("OF-DOC-0008 : erreur d'archive : {0}")]
    Archive(#[from] zip::result::ZipError),
    /// Erreur de sérialisation JSON (OF-DOC-0009).
    #[error("OF-DOC-0009 : erreur de sérialisation : {0}")]
    Serialization(#[from] serde_json::Error),
    /// Erreur d'entrée/sortie fichier, feature `native` (OF-DOC-0010).
    #[error("OF-DOC-0010 : erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    /// Entrée dépassant la taille maximale acceptée (OF-DOC-0011).
    #[error("OF-DOC-0011 : entrée trop volumineuse « {entry} » : {found} octets (maximum {max})")]
    EntryTooLarge {
        /// Nom de l'entrée fautive.
        entry: String,
        /// Taille rencontrée, en octets.
        found: u64,
        /// Taille maximale acceptée, en octets.
        max: u64,
    },
    /// Trop d'entrées dans l'archive (OF-DOC-0012).
    #[error("OF-DOC-0012 : trop d'entrées dans le conteneur : {found} (maximum {max})")]
    TooManyEntries {
        /// Nombre d'entrées rencontré.
        found: usize,
        /// Nombre maximal accepté.
        max: usize,
    },
    /// Nom de blob refusé : vide, traversée de répertoire, chemin
    /// absolu, séparateur Windows ou octet nul (OF-DOC-0013).
    #[error("OF-DOC-0013 : nom de blob invalide « {name} »")]
    InvalidBlobName {
        /// Nom rejeté.
        name: String,
    },
}

impl DocError {
    /// Code stable de l'erreur (ADR-0003), présent dans le registre
    /// central `docs/error-codes.md`. Le match est exhaustif : ajouter
    /// une variante sans code fait échouer la compilation.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedContainerVersion { .. } => "OF-DOC-0001",
            Self::UnknownContainerFormat { .. } => "OF-DOC-0002",
            Self::CorruptJournal { .. } => "OF-DOC-0003",
            Self::BadSequence { .. } => "OF-DOC-0004",
            Self::SchemaTooNew { .. } => "OF-DOC-0005",
            Self::MissingMigration { .. } => "OF-DOC-0006",
            Self::MissingEntry { .. } => "OF-DOC-0007",
            Self::Archive(_) => "OF-DOC-0008",
            Self::Serialization(_) => "OF-DOC-0009",
            Self::Io(_) => "OF-DOC-0010",
            Self::EntryTooLarge { .. } => "OF-DOC-0011",
            Self::TooManyEntries { .. } => "OF-DOC-0012",
            Self::InvalidBlobName { .. } => "OF-DOC-0013",
        }
    }
}
