//! Constantes du démonstrateur : couleurs de la scène, grille,
//! seuils d'import et repères de session — extraites de `main.rs`
//! (découpage étape 2, fiche `preparation/decoupage-main-rs.md`).

use forge_render::Couleur;

/// Acteur des événements écrits par le démonstrateur (ADR-0011).
pub(crate) const ACTEUR: &str = "desktop.demo";
/// Expression initiale de la diagonale du rectangle.
pub(crate) const DEFAULT_DIAGONAL_EXPR: &str = "sqrt(largeur * largeur + hauteur * hauteur)";
/// Couleur du wireframe 3D du maillage, RGBA normalisé — parité avec
/// le painter egui (40, 220, 220) (ADR-0015, phase C2).
pub(crate) const COULEUR_WIREFRAME: Couleur = [40.0 / 255.0, 220.0 / 255.0, 220.0 / 255.0, 1.0];
/// Couleur du contour du rectangle paramétrique posé sur le plan z = 0
/// de la vue 3D, RGBA normalisé (ADR-0015, phase C2).
pub(crate) const COULEUR_RECTANGLE_3D: Couleur = [1.0, 1.0, 1.0, 1.0];
/// Couleur des tracés importés, RGBA normalisé — parité avec le painter
/// egui (240, 180, 20) (ADR-0015).
pub(crate) const COULEUR_TRACES: Couleur = [240.0 / 255.0, 180.0 / 255.0, 20.0 / 255.0, 1.0];
/// Largeur du trait des tracés importés, en pixels logiques (ADR-0015).
pub(crate) const LARGEUR_TRACES_PX: f32 = 1.5;
/// Couleur de la grille de fond 3D, RGBA normalisé (ADR-0015).
pub(crate) const COULEUR_GRILLE: Couleur = [0.35, 0.40, 0.47, 1.0];
/// Couleur de la grille 2D, plus claire que la feuille bleue du
/// rectangle paramétrique pour éviter la confusion (ADR-0015, phase C2).
pub(crate) const COULEUR_GRILLE_2D: Couleur = [0.55, 0.62, 0.75, 1.0];
/// Espacement visé entre deux lignes de grille, en pixels (ADR-0015).
/// 40 px (re-validation §7) : des cellules plus fines au même niveau
/// de zoom restent lisibles, le pas s'adapte par puissances de dix.
pub(crate) const GRILLE_CIBLE_PX: f64 = 40.0;
/// Seuil de détection d'un export en mètres : un maillage dont la
/// plus grande dimension est sous ce seuil (en millimètres) propose
/// l'échelle × 1000 à l'import (ADR-0017). 10 mm : un objet de 3 m
/// exporté en mètres depuis Blender mesure 3,0 « mm » après lecture
/// (retour checklist jalon 2) — le seuil couvre les objets jusqu'à
/// ~10 m en mètres ; une pièce réellement plus petite déclenche
/// aussi le dialogue, au prix d'un clic « Millimètres ».
pub(crate) const SEUIL_METRES: f64 = 10.0;
/// Échelle proposée pour un maillage exporté en mètres (Blender) :
/// les coordonnées sont multipliées par 1000 pour revenir en
/// millimètres (ADR-0017).
pub(crate) const ECHELLE_METRES: f64 = 1000.0;
/// Nombre d'événements posés par une session vierge : au-delà, la
/// session est considérée « en cours » sur l'écran d'accueil.
pub(crate) const EVENEMENTS_VIERGES: usize = 3;
