//! Catalogue Fluent embarqué : traductions à clés et pages d'aide.

use crate::I18nError;
use fluent::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// Catalogue d'une langue, embarqué à la compilation par l'application
/// qui l'utilise (aucun chargement à l'exécution, ADR-0010).
pub struct Catalogue {
    /// Bundle Fluent sous-jacent (une seule langue au jalon 1).
    bundle: FluentBundle<FluentResource>,
}

/// `Debug` manuel : `FluentBundle` n'implémente pas `Debug`, le
/// catalogue s'affiche sans son contenu.
impl std::fmt::Debug for Catalogue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalogue").finish_non_exhaustive()
    }
}

/// Valeur d'argument passée à une clé (`{ $nom }` dans le FTL).
#[derive(Debug, Clone)]
pub enum Argument {
    /// Nombre entier — mis en forme par Fluent.
    Entier(i64),
    /// Texte — mis en forme par Fluent.
    ///
    /// Par défaut Fluent encadre tout placeable (nombres et textes
    /// compris) de marques d'isolation Unicode FSI/PDI invisibles ;
    /// le catalogue coupe cette isolation au bundle (A4-09) :
    /// OmniFab affiche dans ses propres widgets, sans texte mixte
    /// directionnel.
    Texte(String),
}

impl From<i64> for Argument {
    fn from(valeur: i64) -> Self {
        Self::Entier(valeur)
    }
}

impl From<usize> for Argument {
    /// Conversion depuis `usize` : sature à `i64::MAX` — un `as
    /// i64` silencieux wrapperait en négatif les compteurs
    /// démesurés (audit n°4 A4-13c).
    fn from(valeur: usize) -> Self {
        Self::Entier(valeur.min(i64::MAX as usize) as i64)
    }
}

impl From<String> for Argument {
    fn from(valeur: String) -> Self {
        Self::Texte(valeur)
    }
}

impl From<&str> for Argument {
    fn from(valeur: &str) -> Self {
        Self::Texte(valeur.to_string())
    }
}

/// Page d'aide d'un widget (conventions : description + exemple
/// concret replacé dans le contexte de l'atelier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAide {
    /// Titre court, repris dans « Aide sur X ».
    pub titre: String,
    /// Ce que fait l'élément, en une ou deux phrases.
    pub description: String,
    /// Exemple concret d'utilisation.
    pub exemple: String,
}

impl Catalogue {
    /// Construit le catalogue d'une langue depuis une source FTL.
    ///
    /// L'application embarque la source via `include_str!` : le
    /// fichier FTL vit dans le dépôt, source unique de l'app, du
    /// manuel et du site web (ADR-0010).
    pub fn depuis_ftl(langue: &str, source: &str) -> Result<Self, I18nError> {
        let langue: LanguageIdentifier =
            langue
                .parse()
                .map_err(|erreur| I18nError::RessourceInvalide {
                    raison: format!("identifiant de langue « {langue} » invalide : {erreur}"),
                })?;
        let ressource = FluentResource::try_new(source.to_string()).map_err(|(_, erreurs)| {
            I18nError::RessourceInvalide {
                raison: erreurs
                    .iter()
                    .map(|erreur| erreur.to_string())
                    .collect::<Vec<_>>()
                    .join(" ; "),
            }
        })?;
        let mut bundle = FluentBundle::new(vec![langue]);
        // A4-09 : Fluent isole par défaut tout placeable (nombres et
        // textes compris) derrière des marques FSI/PDI invisibles —
        // utiles au rendu de texte mixte directionnel, sans usage
        // dans les widgets egui d'OmniFab : isolation coupée ici,
        // une seule fois pour tout le catalogue.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(ressource)
            .map_err(|erreurs| I18nError::RessourceInvalide {
                raison: erreurs
                    .iter()
                    .map(|erreur| erreur.to_string())
                    .collect::<Vec<_>>()
                    .join(" ; "),
            })?;
        Ok(Self { bundle })
    }

    /// La clé existe-t-elle dans le catalogue ?
    ///
    /// Sert aux tests des applications : chaque clé utilisée doit
    /// exister dans le FTL embarqué (conventions : jamais de chaîne
    /// d'interface hors Fluent).
    pub fn contient(&self, cle: &str) -> bool {
        self.bundle.has_message(cle)
    }

    /// Traduit une clé sans arguments.
    pub fn traduction(&self, cle: &str) -> Result<String, I18nError> {
        let message = self
            .bundle
            .get_message(cle)
            .ok_or_else(|| I18nError::CleAbsente {
                cle: cle.to_string(),
            })?;
        let Some(modele) = message.value() else {
            return Err(I18nError::CleAbsente {
                cle: cle.to_string(),
            });
        };
        let mut erreurs = Vec::new();
        let texte = self.bundle.format_pattern(modele, None, &mut erreurs);
        self.verifie_mise_en_forme(cle, &erreurs)?;
        Ok(texte.into_owned())
    }

    /// Traduit une clé avec arguments (`{ $nom }` dans le FTL).
    pub fn traduction_avec(
        &self,
        cle: &str,
        arguments: &[(&str, Argument)],
    ) -> Result<String, I18nError> {
        let mut args = FluentArgs::new();
        for (nom, valeur) in arguments {
            match valeur {
                Argument::Entier(entier) => args.set(*nom, FluentValue::from(*entier)),
                Argument::Texte(texte) => args.set(*nom, FluentValue::from(texte.clone())),
            };
        }
        let message = self
            .bundle
            .get_message(cle)
            .ok_or_else(|| I18nError::CleAbsente {
                cle: cle.to_string(),
            })?;
        let Some(modele) = message.value() else {
            return Err(I18nError::CleAbsente {
                cle: cle.to_string(),
            });
        };
        let mut erreurs = Vec::new();
        let texte = self
            .bundle
            .format_pattern(modele, Some(&args), &mut erreurs);
        self.verifie_mise_en_forme(cle, &erreurs)?;
        Ok(texte.into_owned())
    }

    /// Page d'aide d'un widget : le message du même nom que le
    /// `help_id`, avec les attributs `titre`, `description` et
    /// `exemple`.
    ///
    /// Une page incomplète est une erreur typée (OF-I18N-0003) : la
    /// convention « une page sans exemple = build en échec » se
    /// vérifie par les tests des applications.
    pub fn page_aide(&self, help_id: &str) -> Result<PageAide, I18nError> {
        let message = self
            .bundle
            .get_message(help_id)
            .ok_or_else(|| I18nError::CleAbsente {
                cle: help_id.to_string(),
            })?;
        Ok(PageAide {
            titre: self.attribut(help_id, &message, "titre")?,
            description: self.attribut(help_id, &message, "description")?,
            exemple: self.attribut(help_id, &message, "exemple")?,
        })
    }

    /// Met en forme un attribut de page d'aide, sans arguments.
    fn attribut(
        &self,
        help_id: &str,
        message: &fluent::FluentMessage,
        nom: &str,
    ) -> Result<String, I18nError> {
        let Some(attribut) = message.get_attribute(nom) else {
            return Err(I18nError::AideIncomplete {
                help_id: help_id.to_string(),
                attribut: nom.to_string(),
            });
        };
        let mut erreurs = Vec::new();
        let texte = self
            .bundle
            .format_pattern(attribut.value(), None, &mut erreurs);
        self.verifie_mise_en_forme(help_id, &erreurs)?;
        Ok(texte.into_owned())
    }

    /// Signale OF-I18N-0004 si Fluent a rencontré des erreurs de mise
    /// en forme (argument manquant, fonction inconnue).
    fn verifie_mise_en_forme(
        &self,
        cle: &str,
        erreurs: &[fluent::FluentError],
    ) -> Result<(), I18nError> {
        if erreurs.is_empty() {
            return Ok(());
        }
        Err(I18nError::MiseEnForme {
            cle: cle.to_string(),
            raison: erreurs
                .iter()
                .map(|erreur| erreur.to_string())
                .collect::<Vec<_>>()
                .join(" ; "),
        })
    }
}
