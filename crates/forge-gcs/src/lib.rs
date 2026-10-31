//! # forge-gcs — solveur de contraintes 2D, wrapper sûr sur planegcs
//!
//! Crate de la frontière FFI de l'ADR-0013 (jalon 2, phase A). Elle
//! expose un **trait d'abstraction** [`Solveur2D`] — unique API du
//! solveur consommée par le reste du projet — et son implémentation
//! native [`Systeme2D`] (feature `native`, par défaut), au-dessus du
//! shim ABI C de planegcs (`forge-gcs-sys`).
//!
//! ## Frontières
//!
//! - Les **unités strictes** sont réintroduites ici : l'ABI C parle en
//!   `double` (millimètres, radians), mais l'API publique de cette
//!   crate n'accepte et ne renvoie que les newtypes de `forge-units`
//!   ([`Length`] en millimètres, [`Angle`] en radians). Jamais de `f64`
//!   nu à la frontière (conventions du projet).
//! - Les identifiants ([`IdPoint`], [`IdLigne`], [`IdCercle`],
//!   [`IdArc`], [`TagContrainte`]) sont des newtypes opaques émis par
//!   le solveur dans l'ordre d'ajout : l'ordre d'ajout fait foi pour la
//!   reproductibilité (FreeCAD #33399).
//! - Les valeurs des contraintes (cotes) sont des **paramètres fixes**
//!   du solveur : c'est ce qui permettra au moteur de liaisons de
//!   piloter les cotes par expressions et alias en phase E (ADR-0012).
//! - Aucune dépendance à forge-session : c'est forge-session (phase D)
//!   qui consommera ce trait via ses commandes d'esquisse.
//!
//! ## Licences
//!
//! LGPL-2.1-or-later, héritage direct de planegcs (ADR-0013) ; cf.
//! `LICENSE` de la crate. Les crates MPL-2.0 du dépôt n'en dépendent
//! pas.

#![deny(missing_docs)]

mod error;
#[cfg(feature = "native")]
mod native;

pub use error::GcsError;
#[cfg(feature = "native")]
pub use native::Systeme2D;

use forge_units::{Angle, Length};

/// Identifiant d'un point du croquis (opaque, émis à l'ajout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdPoint(i32);

/// Identifiant d'une ligne du croquis (opaque, émise à l'ajout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdLigne(i32);

/// Identifiant d'un cercle du croquis (opaque, émis à l'ajout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdCercle(i32);

/// Identifiant d'un arc du croquis (opaque, émis à l'ajout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdArc(i32);

/// Tag d'une contrainte (opaque, émis à l'ajout ; ≥ 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TagContrainte(i32);

/// Algorithme de résolution (miroir des algorithmes de planegcs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithme {
    /// BFGS (quasi-Newton) — le plus rapide sur de petits croquis.
    Bfgs,
    /// Levenberg-Marquardt (moindres carrés dampés).
    LevenbergMarquardt,
    /// DogLeg (région de confiance) — le plus robuste par défaut.
    DogLeg,
}

impl Algorithme {
    /// Code de l'algorithme pour l'ABI C (miroir de `GCS::Algorithm`).
    pub fn code_abi(self) -> i32 {
        match self {
            Self::Bfgs => 0,
            Self::LevenbergMarquardt => 1,
            Self::DogLeg => 2,
        }
    }
}

/// Contrainte géométrique typée. Les valeurs de cote (distance, angle,
/// rayon) sont des newtypes : l'ABI les convertit en paramètres fixes.
#[derive(Debug, Clone, Copy)]
pub enum Contrainte {
    /// Coïncidence de deux points.
    Coincidence {
        /// Premier point.
        p1: IdPoint,
        /// Second point.
        p2: IdPoint,
    },
    /// Ligne horizontale.
    Horizontale {
        /// Ligne contrainte.
        ligne: IdLigne,
    },
    /// Ligne verticale.
    Verticale {
        /// Ligne contrainte.
        ligne: IdLigne,
    },
    /// Distance entre deux points (cote pilotable en phase E).
    Distance {
        /// Premier point.
        p1: IdPoint,
        /// Second point.
        p2: IdPoint,
        /// Valeur de la cote.
        valeur: Length,
    },
    /// Angle entre deux lignes (cote pilotable en phase E, en radians
    /// côté solveur).
    Angle {
        /// Première ligne.
        ligne1: IdLigne,
        /// Seconde ligne.
        ligne2: IdLigne,
        /// Valeur de l'angle.
        valeur: Angle,
    },
    /// Deux lignes de même longueur.
    MemeLongueur {
        /// Première ligne.
        ligne1: IdLigne,
        /// Seconde ligne.
        ligne2: IdLigne,
    },
    /// Point sur une ligne.
    PointSurLigne {
        /// Point contraint.
        point: IdPoint,
        /// Ligne support.
        ligne: IdLigne,
    },
    /// Deux lignes parallèles.
    Paralleles {
        /// Première ligne.
        ligne1: IdLigne,
        /// Seconde ligne.
        ligne2: IdLigne,
    },
    /// Deux lignes perpendiculaires.
    Perpendiculaires {
        /// Première ligne.
        ligne1: IdLigne,
        /// Seconde ligne.
        ligne2: IdLigne,
    },
    /// Rayon d'un cercle (cote pilotable en phase E).
    RayonCercle {
        /// Cercle contraint.
        cercle: IdCercle,
        /// Valeur du rayon.
        valeur: Length,
    },
    /// Point sur un cercle.
    PointSurCercle {
        /// Point contraint.
        point: IdPoint,
        /// Cercle support.
        cercle: IdCercle,
    },
}

/// Statut d'une résolution réussie (miroir de `GCS::SolveStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatutResolution {
    /// Solution exacte.
    Succes,
    /// Solution convergée sous tolérance.
    Converge,
}

/// Diagnostic d'un croquis après analyse.
///
/// Attention (ADR-0013, risques) : le diagnostic de redondance de
/// planegcs est **instable** sur certains croquis (proposition de
/// subvention FPA #106) ; l'UI d'esquisse (phase D) ne devra pas s'y
/// fier sans hystérésis.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Degrés de liberté restants (0 : croquis entièrement contraint).
    dof: usize,
    /// Vrai si la reproductibilité des positions est garantie
    /// (adresses de paramètres croissantes, FreeCAD #33399).
    reproductible: bool,
    /// Tags des contraintes conflictuelles (surentraînement).
    conflictuelles: Vec<TagContrainte>,
    /// Tags des contraintes redondantes.
    redondantes: Vec<TagContrainte>,
    /// Tags des contraintes partiellement redondantes.
    partiellement_redondantes: Vec<TagContrainte>,
    /// Vrai si au moins une liste de tags a été tronquée au plafond
    /// ABI (32) par le shim : le diagnostic est alors incomplet.
    tronque: bool,
}

impl Diagnostic {
    /// Degrés de liberté restants (0 : croquis entièrement contraint).
    pub fn dof(&self) -> usize {
        self.dof
    }

    /// Vrai si la reproductibilité des positions est garantie.
    pub fn reproductible(&self) -> bool {
        self.reproductible
    }

    /// Tags des contraintes conflictuelles (surentraînement).
    pub fn conflictuelles(&self) -> &[TagContrainte] {
        &self.conflictuelles
    }

    /// Tags des contraintes redondantes.
    pub fn redondantes(&self) -> &[TagContrainte] {
        &self.redondantes
    }

    /// Tags des contraintes partiellement redondantes.
    pub fn partiellement_redondantes(&self) -> &[TagContrainte] {
        &self.partiellement_redondantes
    }

    /// Vrai si au moins une liste de tags a été tronquée au plafond
    /// ABI : le diagnostic est alors incomplet (A4-04).
    pub fn tronque(&self) -> bool {
        self.tronque
    }
}

/// Abstraction du solveur de contraintes 2D (frontière unique du projet
/// vers planegcs — ADR-0013).
///
/// L'implémentation native ([`Systeme2D`], feature `native`) s'appuie
/// sur le shim ABI C ; une implémentation `wasm` du même trait est
/// prévue au jalon 9. Le reste du projet ne voit jamais que ce trait :
/// c'est la garantie « wasm-friendly » du projet, dès le spike.
pub trait Solveur2D {
    /// Ajoute un point au croquis (coordonnées en millimètres).
    fn ajouter_point(&mut self, x: Length, y: Length) -> Result<IdPoint, GcsError>;

    /// Ajoute une ligne entre deux points existants : les extrémités
    /// sont **partagées** (déplacer un point déplace l'extrémité).
    fn ajouter_ligne(&mut self, p1: IdPoint, p2: IdPoint) -> Result<IdLigne, GcsError>;

    /// Ajoute un cercle par centre et rayon.
    fn ajouter_cercle(&mut self, centre: IdPoint, rayon: Length) -> Result<IdCercle, GcsError>;

    /// Ajoute un arc par centre, extrémités, rayon et angles aux
    /// extrémités (radians).
    fn ajouter_arc(
        &mut self,
        centre: IdPoint,
        debut: IdPoint,
        fin: IdPoint,
        rayon: Length,
        angle_debut: Angle,
        angle_fin: Angle,
    ) -> Result<IdArc, GcsError>;

    /// Fixe un point : il n'est plus une inconnue du solveur (ancrage
    /// du croquis).
    fn fixer_point(&mut self, point: IdPoint) -> Result<(), GcsError>;

    /// Ajoute une contrainte typée ; renvoie son tag.
    fn ajouter_contrainte(&mut self, contrainte: Contrainte) -> Result<TagContrainte, GcsError>;

    /// Résout le croquis et applique la solution aux positions.
    /// Échec : [`GcsError::ResolutionEchouee`] (croquis conflictuel),
    /// [`GcsError::NonReproductible`] si l'arène a débordé.
    fn resoudre(&mut self, algorithme: Algorithme) -> Result<StatutResolution, GcsError>;

    /// Position courante d'un point (en millimètres).
    fn position_point(&self, point: IdPoint) -> Result<(Length, Length), GcsError>;

    /// Analyse le croquis : degrés de liberté, conflits, redondances,
    /// reproductibilité.
    fn diagnostic(&mut self, algorithme: Algorithme) -> Result<Diagnostic, GcsError>;

    /// Vide le croquis : supprime toutes les entités, contraintes,
    /// paramètres et points fixes, comme un système fraîchement créé.
    /// Sert aux résolutions en plusieurs passes (forge-session) qui
    /// reconstruisent le système sans recréer le handle.
    fn vider(&mut self) -> Result<(), GcsError>;
}
