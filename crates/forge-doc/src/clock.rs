//! Horloge injectable : les horodatages des événements passent par le
//! trait [`Clock`] (ADR-0000 : aucun accès natif hors trait d'abstraction,
//! et des journaux déterministes en test).

use std::fmt::Debug;

/// Source de temps d'un [`Document`](crate::Document) : fournit
/// l'horodatage des événements ajoutés par
/// [`append`](crate::Document::append).
pub trait Clock: Debug {
    /// Horodatage Unix courant, en secondes.
    fn now_unix(&self) -> i64;
}

/// Horloge système réelle (feature `native`). En wasm, où
/// `std::time::SystemTime` panique à l'appel, elle retourne 0.
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> i64 {
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0)
        }
    }
}

/// Horloge figée pour les tests : retourne toujours le même horodatage.
#[derive(Debug)]
pub struct FixedClock {
    /// Horodatage Unix retourné, en secondes.
    pub unix: i64,
}

impl Clock for FixedClock {
    fn now_unix(&self) -> i64 {
        self.unix
    }
}
