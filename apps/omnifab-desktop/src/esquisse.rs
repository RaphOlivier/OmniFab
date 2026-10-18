//! Résolution du croquis contraint au fil de travail (ADR-0018, jalon 2
//! phase D).
//!
//! Le solveur natif ([`Systeme2D`], planegcs par le shim ABI C) est
//! instancié **dans le fil de travail** : forge-session ne l’instancie
//! jamais (ADR-0018 D3, frontière LGPL de l’ADR-0013) et le fil
//! d’interface n’est jamais bloqué par une résolution (ADR-0016 C1).
//! Le handle planegcs n’est ni `Send` ni `Sync` : il vit et meurt
//! dans [`resoudre`], jamais renvoyé au fil d’interface.
//!
//! Les positions résolues sont un état dérivé en mémoire de
//! l’application — jamais écrites au journal, jamais sérialisées
//! (ADR-0018 D3).

use forge_gcs::Systeme2D;
use forge_session::{CroquisResolu, EsquisseProjection};

/// Résout le croquis avec un solveur natif fraîchement instancié :
/// l’échec est un message, l’esquisse du journal reste intacte et
/// ses graines demeurent la vérité affichable (ADR-0018 D7).
pub(crate) fn resoudre(croquis: &EsquisseProjection) -> Result<CroquisResolu, String> {
    let mut systeme = Systeme2D::new().map_err(|erreur| erreur.to_string())?;
    croquis.resoudre(&mut systeme).map_err(|erreur| erreur.to_string())
}
