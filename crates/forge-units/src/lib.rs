#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # forge-units — unités strictes du noyau OmniFab
//!
//! Crate de base du workspace (ADR-0000, verrou nº5) : les grandeurs
//! physiques circulent dans les API métier sous forme de newtypes, jamais
//! de `f64` nu. Une confusion d'unité est ainsi une erreur de compilation,
//! pas une pièce usinée trop courte.
//!
//! Invariants :
//!
//! - cette crate ne dépend d'**aucune** autre crate du noyau ;
//! - une longueur est une [`Length`], un angle une [`Angle`] ;
//! - la tolérance géométrique par défaut est [`TOLERANCE_MM`] (1e-6 mm).

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Tolérance géométrique par défaut, en millimètres (ADR-0000, verrou nº5).
pub const TOLERANCE_MM: f64 = 1e-6;

/// Tolérance angulaire par défaut, en radians.
pub const TOLERANCE_RAD: f64 = 1e-9;

/// Longueur physique. Représentation interne : millimètres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Length {
    /// Valeur en millimètres.
    mm: f64,
}

impl Length {
    /// Longueur nulle.
    pub const ZERO: Self = Self { mm: 0.0 };

    /// Construit une longueur à partir de millimètres.
    pub fn from_millimeters(mm: f64) -> Self {
        Self { mm }
    }

    /// Construit une longueur à partir de centimètres.
    pub fn from_centimeters(cm: f64) -> Self {
        Self { mm: cm * 10.0 }
    }

    /// Construit une longueur à partir de mètres.
    pub fn from_meters(meters: f64) -> Self {
        Self { mm: meters * 1000.0 }
    }

    /// Construit une longueur à partir de pouces (1 pouce = 25,4 mm).
    pub fn from_inches(inches: f64) -> Self {
        Self { mm: inches * 25.4 }
    }

    /// Retourne la longueur en millimètres.
    pub fn millimeters(self) -> f64 {
        self.mm
    }

    /// Retourne la longueur en centimètres.
    pub fn centimeters(self) -> f64 {
        self.mm / 10.0
    }

    /// Retourne la longueur en mètres.
    pub fn meters(self) -> f64 {
        self.mm / 1000.0
    }

    /// Retourne la longueur en pouces.
    pub fn inches(self) -> f64 {
        self.mm / 25.4
    }

    /// Valeur absolue de la longueur.
    pub fn abs(self) -> Self {
        Self { mm: self.mm.abs() }
    }

    /// La plus petite des deux longueurs.
    pub fn min(self, other: Self) -> Self {
        Self { mm: self.mm.min(other.mm) }
    }

    /// La plus grande des deux longueurs.
    pub fn max(self, other: Self) -> Self {
        Self { mm: self.mm.max(other.mm) }
    }

    /// Égalité à la tolérance géométrique près ([`TOLERANCE_MM`]).
    pub fn approx_eq(self, other: Self) -> bool {
        (self.mm - other.mm).abs() <= TOLERANCE_MM
    }
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Self) -> Self::Output {
        Self { mm: self.mm + rhs.mm }
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Self) -> Self::Output {
        Self { mm: self.mm - rhs.mm }
    }
}

impl Neg for Length {
    type Output = Length;
    fn neg(self) -> Self::Output {
        Self { mm: -self.mm }
    }
}

impl Mul<f64> for Length {
    type Output = Length;
    fn mul(self, rhs: f64) -> Self::Output {
        Self { mm: self.mm * rhs }
    }
}

impl Mul<Length> for f64 {
    type Output = Length;
    fn mul(self, rhs: Length) -> Self::Output {
        Length { mm: self * rhs.mm }
    }
}

impl Div<f64> for Length {
    type Output = Length;
    fn div(self, rhs: f64) -> Self::Output {
        Self { mm: self.mm / rhs }
    }
}

impl Div for Length {
    type Output = f64;
    /// Rapport de deux longueurs : un nombre sans dimension.
    fn div(self, rhs: Self) -> Self::Output {
        self.mm / rhs.mm
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} mm", self.mm)
    }
}

/// Angle. Représentation interne : radians.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Angle {
    /// Valeur en radians.
    rad: f64,
}

impl Angle {
    /// Angle nul.
    pub const ZERO: Self = Self { rad: 0.0 };

    /// Construit un angle à partir de degrés.
    pub fn from_degrees(degrees: f64) -> Self {
        Self {
            rad: degrees.to_radians(),
        }
    }

    /// Construit un angle à partir de radians.
    pub fn from_radians(radians: f64) -> Self {
        Self { rad: radians }
    }

    /// Retourne l'angle en degrés.
    pub fn degrees(self) -> f64 {
        self.rad.to_degrees()
    }

    /// Retourne l'angle en radians.
    pub fn radians(self) -> f64 {
        self.rad
    }

    /// Égalité à la tolérance angulaire près ([`TOLERANCE_RAD`]).
    pub fn approx_eq(self, other: Self) -> bool {
        (self.rad - other.rad).abs() <= TOLERANCE_RAD
    }
}

impl Add for Angle {
    type Output = Angle;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            rad: self.rad + rhs.rad,
        }
    }
}

impl Sub for Angle {
    type Output = Angle;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            rad: self.rad - rhs.rad,
        }
    }
}

impl Neg for Angle {
    type Output = Angle;
    fn neg(self) -> Self::Output {
        Self { rad: -self.rad }
    }
}

impl Mul<f64> for Angle {
    type Output = Angle;
    fn mul(self, rhs: f64) -> Self::Output {
        Self { rad: self.rad * rhs }
    }
}

impl Div<f64> for Angle {
    type Output = Angle;
    fn div(self, rhs: f64) -> Self::Output {
        Self { rad: self.rad / rhs }
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}°", self.degrees())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_sont_coherentes() {
        assert_eq!(Length::from_millimeters(25.0).millimeters(), 25.0);
        assert_eq!(Length::from_centimeters(2.5).millimeters(), 25.0);
        assert_eq!(Length::from_meters(0.025).millimeters(), 25.0);
        assert_eq!(Length::from_inches(1.0).millimeters(), 25.4);
        assert_eq!(Length::from_millimeters(25.4).inches(), 1.0);
    }

    #[test]
    fn tolerance_geometrique() {
        let a = Length::from_millimeters(10.0);
        assert!(a.approx_eq(Length::from_millimeters(10.0 + 5e-7)));
        assert!(!a.approx_eq(Length::from_millimeters(10.0 + 2e-6)));
    }

    #[test]
    fn arithmetique() {
        let total = Length::from_millimeters(80.0) + Length::from_millimeters(20.0);
        assert_eq!(total.millimeters(), 100.0);
        assert_eq!((total / 2.0).millimeters(), 50.0);
        let rapport = Length::from_millimeters(600.0) / Length::from_millimeters(300.0);
        assert_eq!(rapport, 2.0);
    }

    #[test]
    fn angles() {
        let somme = Angle::from_degrees(90.0) + Angle::from_degrees(45.0);
        assert!((somme.degrees() - 135.0).abs() < 1e-9);
        assert!(Angle::from_degrees(180.0).approx_eq(Angle::from_radians(std::f64::consts::PI)));
    }
}
