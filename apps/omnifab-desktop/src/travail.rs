//! Fil de travail de omnifab-desktop (ADR-0016 C1) : lectures,
//! parses et exports détachés du fil d'interface — le fil survit
//! à une panique du travail (échec typé OF-WORK-0001, audit n°3
//! N8) ; échecs de parse mémorisés — un fichier invalide n'est
//! plus re-parsé à chaque frame.

use crate::esquisse;
use crate::theme::SEUIL_METRES;
use eframe::egui;
use forge_geom::Mesh;
use forge_geom::Point3;
use forge_io::dxf;
use forge_io::stl;
use forge_io::svg;
use forge_session::CroquisResolu;
use forge_session::EVENT_ALIAS_REMOVED;
use forge_session::EVENT_EXPRESSION_SET;
use forge_session::EVENT_INPUT_SET;
use forge_session::EVENT_SKETCH_CONSTRAINT_UPDATED;
use forge_session::EsquisseProjection;
use forge_units::Length;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

/// Travail confié au fil de travail : lecture et parse d'un
/// import, parse et écriture d'un export — jamais sur le fil
/// d'interface (ADR-0016 C1).
pub(crate) enum Travail {
    /// Lecture puis parse d'un fichier de tracés (SVG ou DXF).
    ImportTraces {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Format attendu : « svg » ou « dxf ».
        format: &'static str,
        /// Fichier à lire.
        chemin: PathBuf,
    },
    /// Lecture puis parse d'un maillage STL.
    ImportMaillage {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Fichier à lire.
        chemin: PathBuf,
    },
    /// Re-parse du blob d'origine puis export des tracés.
    ExportTraces {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Format du blob d'origine : « svg » ou « dxf ».
        format_origine: &'static str,
        /// Format d'export demandé : « svg » ou « dxf ».
        format: &'static str,
        /// Octets du blob d'origine.
        octets: Vec<u8>,
        /// Fichier de destination.
        chemin: PathBuf,
    },
    /// Re-parse du blob d'origine puis export du maillage en STL.
    ExportMaillage {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
        /// Octets du blob d'origine.
        octets: Vec<u8>,
        /// Échelle d'import appliquée avant la sérialisation : l'export
        /// écrit toujours des millimètres (ADR-0017).
        echelle: f64,
        /// Fichier de destination.
        chemin: PathBuf,
    },
    /// Résolution du croquis contraint (ADR-0018 D3) : la projection
    /// du croquis est clonée vers le fil de travail, le solveur natif
    /// y est instancié — jamais sur le fil d’interface (ADR-0016 C1),
    /// jamais dans forge-session (frontière LGPL de l’ADR-0013).
    ResoudreEsquisse {
        /// Génération de l’état au moment de l’envoi.
        generation: u64,
        /// Clé du cache des positions résolues au moment de l’envoi :
        /// numéro d’édition du croquis et génération des liaisons
        /// (ADR-0019 §6).
        cle: (u64, u64),
        /// Clone de la projection du croquis : entités, contraintes
        /// et graines du journal.
        croquis: EsquisseProjection,
    },
    /// Aperçu de glissement du croquis (re-test du 2026-10-23) : la
    /// forme liée suit le point saisi PENDANT le glissement — la
    /// projection clonée porte la position visée comme relâchement
    /// provisoire, jamais écrit au journal ; le résultat sert
    /// l'affichage seul, l'échec n'y remonte jamais.
    ResoudreApercu {
        /// Génération de l’état au moment de l’envoi.
        generation: u64,
        /// Clone de la projection du croquis, graine du point saisi
        /// posée à la position visée.
        croquis: EsquisseProjection,
    },
    /// Travail de test qui panique toujours (audit n°3 N8) : vérifie
    /// que le fil de travail survit à une panique et traite le
    /// travail suivant.
    #[cfg(test)]
    Bogue {
        /// Génération de l'état au moment de l'envoi.
        generation: u64,
    },
}

impl Travail {
    /// Génération de l'état au moment de l'envoi (audit n°3 N8) :
    /// connue avant l'exécution — un travail qui panique garde sa
    /// génération pour le retour d'erreur typé.
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Travail::ImportTraces { generation, .. }
            | Travail::ImportMaillage { generation, .. }
            | Travail::ExportTraces { generation, .. }
            | Travail::ExportMaillage { generation, .. }
            | Travail::ResoudreEsquisse { generation, .. }
            | Travail::ResoudreApercu { generation, .. } => *generation,
            #[cfg(test)]
            Travail::Bogue { generation } => *generation,
        }
    }

    /// Exécute le travail hors du fil d'interface : tout échec
    /// devient un retour d'erreur à message (ADR-0016 C1).
    fn executer(self) -> Retour {
        match self {
            // Travail de test qui panique (audit n°3 N8) : le fil
            // doit survivre et le traiter comme un échec typé.
            #[cfg(test)]
            Travail::Bogue { generation } => {
                panic!("bogue de test : le travail {generation} doit paniquer")
            }
            Travail::ImportTraces {
                generation,
                format,
                chemin,
            } => {
                let octets = match lire_borne(&chemin, MAX_OCTETS_IMPORT) {
                    Ok(octets) => octets,
                    Err(message) => return erreur_de_travail(generation, message),
                };
                let parse = if format == "svg" {
                    svg::import(&octets, &svg::SvgBudgets::default())
                } else {
                    dxf::import(&octets, &dxf::DxfBudgets::default())
                };
                match parse {
                    Ok(traces) => {
                        let nombre_traces = traces.len();
                        let sommets: usize = traces.iter().map(|trace| trace.len()).sum();
                        Retour::ImportTraces {
                            generation,
                            format,
                            octets,
                            traces: nombre_traces,
                            sommets,
                        }
                    }
                    Err(error) => erreur_de_travail(generation, error),
                }
            }
            Travail::ImportMaillage { generation, chemin } => {
                let octets = match lire_borne(&chemin, MAX_OCTETS_IMPORT) {
                    Ok(octets) => octets,
                    Err(message) => return erreur_de_travail(generation, message),
                };
                match stl::import(&octets, &stl::StlBudgets::default()) {
                    Ok(maillage) => {
                        let triangles = maillage.triangle_count();
                        let dimension = plus_grande_dimension(&maillage);
                        Retour::ImportMaillage {
                            generation,
                            octets,
                            triangles,
                            dimension,
                        }
                    }
                    Err(error) => erreur_de_travail(generation, error),
                }
            }
            Travail::ExportTraces {
                generation,
                format_origine,
                format,
                octets,
                chemin,
            } => {
                let parse = if format_origine == "svg" {
                    svg::import(&octets, &svg::SvgBudgets::default())
                } else {
                    dxf::import(&octets, &dxf::DxfBudgets::default())
                };
                let traces = match parse {
                    Ok(traces) => traces,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                let sortie = if format == "svg" {
                    svg::export(&traces)
                } else {
                    dxf::export(&traces)
                };
                let octets = match sortie {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                ecris_fichier(generation, &chemin, &octets)
            }
            Travail::ExportMaillage {
                generation,
                octets,
                echelle,
                chemin,
            } => {
                let maillage = match stl::import(&octets, &stl::StlBudgets::default()) {
                    Ok(maillage) => maillage,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                let maillage = match maillage_echelle(maillage, echelle) {
                    Ok(maillage) => maillage,
                    Err(erreur) => return erreur_de_travail(generation, erreur),
                };
                let octets = match stl::export(&maillage) {
                    Ok(octets) => octets,
                    Err(error) => return erreur_de_travail(generation, error),
                };
                ecris_fichier(generation, &chemin, &octets)
            }
            Travail::ResoudreEsquisse {
                generation,
                cle,
                croquis,
            } => {
                let resultat = esquisse::resoudre(&croquis);
                Retour::EsquisseResolue {
                    generation,
                    cle,
                    resultat,
                }
            }
            Travail::ResoudreApercu {
                generation,
                croquis,
            } => {
                let resultat = esquisse::resoudre(&croquis);
                Retour::EsquisseApercue {
                    generation,
                    resultat,
                }
            }
        }
    }
}

/// Retour du fil de travail : import réussi (octets d'origine et
/// compteurs), export écrit, ou échec à message (ADR-0016 C1).
pub(crate) enum Retour {
    /// Tracés lus et parsés : octets d'origine pour le blob,
    /// compteurs pour le journal.
    ImportTraces {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Format du fichier importé : « svg » ou « dxf ».
        format: &'static str,
        /// Octets d'origine.
        octets: Vec<u8>,
        /// Nombre de tracés.
        traces: usize,
        /// Nombre de sommets.
        sommets: usize,
    },
    /// Maillage lu et parsé : octets d'origine, compte de triangles
    /// et plus grande dimension (ADR-0017).
    ImportMaillage {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Octets d'origine.
        octets: Vec<u8>,
        /// Nombre de triangles.
        triangles: usize,
        /// Plus grande dimension du maillage, en millimètres
        /// (ADR-0017).
        dimension: f64,
    },
    /// Export écrit au chemin donné.
    Export {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Chemin du fichier écrit.
        chemin: String,
    },
    /// Échec du travail : message affiché au canal d'erreur.
    Erreur {
        /// Génération de l'état au moment de l'envoi du travail.
        generation: u64,
        /// Message d'échec.
        message: String,
    },
    /// Croquis résolu au fil de travail (ADR-0018 D3) : positions et
    /// diagnostic de l’édition demandée, ou message d’échec — le
    /// journal n’est jamais touché (ADR-0018 D7).
    EsquisseResolue {
        /// Génération de l’état au moment de l’envoi du travail.
        generation: u64,
        /// Clé (édition, liaisons) du croquis résolu.
        cle: (u64, u64),
        /// Positions et diagnostic, ou message d’échec.
        resultat: Result<CroquisResolu, String>,
    },
    /// Aperçu de glissement résolu (re-test du 2026-10-23) : la
    /// forme suit le point saisi pendant le glissement —
    /// affichage seul, l'échec ne remonte jamais comme erreur.
    EsquisseApercue {
        /// Génération de l’état au moment de l’envoi du travail.
        generation: u64,
        /// Positions et diagnostic, ou message d'échec muet.
        resultat: Result<CroquisResolu, String>,
    },
}

impl Retour {
    /// Génération de l'état au moment de l'envoi du travail.
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Retour::ImportTraces { generation, .. }
            | Retour::ImportMaillage { generation, .. }
            | Retour::Export { generation, .. }
            | Retour::Erreur { generation, .. } => *generation,
            Retour::EsquisseResolue { generation, .. } => *generation,
            Retour::EsquisseApercue { generation, .. } => *generation,
        }
    }
}

/// Boucle du fil de travail : chaque travail reçu est exécuté et son
/// retour émis ; le fil s'éteint quand l'émetteur des travaux est
/// abandonné (ADR-0016 C1). Le fil survit à une panique du travail
/// (audit n°3 N8) : l'échec inopiné devient un retour d'erreur typé
/// OF-WORK-0001 — code du registre central docs/error-codes.md — et
/// le travail suivant est traité normalement.
pub(crate) fn fil_de_travail(recepteur: mpsc::Receiver<Travail>, emetteur: mpsc::Sender<Retour>) {
    use std::panic::AssertUnwindSafe;
    for travail in recepteur {
        let generation = travail.generation();
        let execution = std::panic::catch_unwind(AssertUnwindSafe(|| travail.executer()));
        let retour = match execution {
            Ok(retour) => retour,
            Err(poigne) => {
                let message = format!(
                    "OF-WORK-0001 : le travail a échoué inopinément : {}",
                    resume_de_poigne(&poigne)
                );
                Retour::Erreur {
                    generation,
                    message,
                }
            }
        };
        let _ = emetteur.send(retour);
    }
}

/// Message d'une panique rattrapée (audit n°3 N8) : `&str`, `String`
/// ou résumé générique — le fil ne panique jamais pour afficher une
/// erreur.
fn resume_de_poigne(poigne: &(dyn std::any::Any + Send)) -> String {
    if let Some(texte) = poigne.downcast_ref::<&str>() {
        (*texte).to_string()
    } else if let Some(texte) = poigne.downcast_ref::<String>() {
        texte.clone()
    } else {
        "panique sans message".to_string()
    }
}

/// Plus grande dimension du maillage, en millimètres : le plus grand
/// côté de la boîte englobante (ADR-0017). Un maillage vide mesure 0.
pub(crate) fn plus_grande_dimension(maillage: &Mesh) -> f64 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in maillage.vertices() {
        let coordonnees = [
            point.x().millimeters(),
            point.y().millimeters(),
            point.z().millimeters(),
        ];
        for axe in 0..3 {
            min[axe] = min[axe].min(coordonnees[axe]);
            max[axe] = max[axe].max(coordonnees[axe]);
        }
    }
    let mut dimension: f64 = 0.0;
    for axe in 0..3 {
        dimension = dimension.max(max[axe] - min[axe]);
    }
    dimension
}

/// Un maillage dont la plus grande dimension est sous le seuil
/// ressemble à un export en mètres (Blender exporte en mètres) :
/// l'import propose l'échelle × 1000 (ADR-0017). Un maillage vide ou
/// déjà grand ne déclenche rien.
pub(crate) fn propose_echelle_metres(dimension: f64) -> bool {
    (0.0..SEUIL_METRES).contains(&dimension) && dimension > 0.0
}

/// Applique l'échelle d'import aux coordonnées du maillage (ADR-0017) :
/// l'événement porte le facteur, le blob garde les octets bruts — le
/// rendu et l'export travaillent sur le maillage mis à l'échelle.
pub(crate) fn maillage_echelle(maillage: Mesh, echelle: f64) -> Result<Mesh, String> {
    if echelle == 1.0 {
        return Ok(maillage);
    }
    let triangles = maillage.triangles().to_vec();
    let mut sommets = Vec::with_capacity(maillage.vertex_count());
    for point in maillage.vertices() {
        let x = Length::from_millimeters(point.x().millimeters() * echelle);
        let y = Length::from_millimeters(point.y().millimeters() * echelle);
        let z = Length::from_millimeters(point.z().millimeters() * echelle);
        sommets.push(Point3::new(x, y, z).map_err(|erreur| erreur.to_string())?);
    }
    Mesh::from_parts(sommets, triangles).map_err(|erreur| erreur.to_string())
}

/// Ne garde que les retours de la génération courante : les travaux
/// lancés avant une réinitialisation ou une ouverture sont
/// obsolètes, leurs résultats ne touchent plus l'état (ADR-0016 C1).
pub(crate) fn garde_actuels(retours: Vec<Retour>, generation: u64) -> Vec<Retour> {
    retours
        .into_iter()
        .filter(|retour| retour.generation() == generation)
        .collect()
}

/// Mémorise le résultat d'un parse dans le cache, clé à l'appui :
/// la même clé n'est plus re-parsée, même en échec (ADR-0016 C1) —
/// un fichier invalide ne revient pas à chaque frame. Retourne
/// « true » si un parse a eu lieu.
pub(crate) fn met_en_cache<T>(
    cache: &mut Option<(String, Result<T, String>)>,
    cle: String,
    parse: impl FnOnce() -> Result<T, String>,
) -> bool {
    if cache.as_ref().is_some_and(|(nom, _)| *nom == cle) {
        return false;
    }
    *cache = Some((cle, parse()));
    true
}

/// Raccourcis globaux d'annulation et de rétablissement
/// (ADR-0016 C2) : `command` (Ctrl sous Windows/Linux, Cmd sous
/// macOS) avec Z annule, avec Y ou Maj+Z rétablit. Aucun raccourci
/// quand un champ de texte a le focus (`clavier_pris`) : la saisie
/// en cours n'est pas écrasée par un Ctrl+Z global.
///
/// Retourne `(annuler, retablir)`.
pub(crate) fn raccourcis_annulation(
    modifiers: &egui::Modifiers,
    z: bool,
    y: bool,
    clavier_pris: bool,
) -> (bool, bool) {
    if clavier_pris {
        return (false, false);
    }
    let cmd = modifiers.command;
    let maj = modifiers.shift;
    (cmd && !maj && z, cmd && (y || maj && z))
}

/// Retour d'erreur du fil de travail : message à afficher au canal
/// d'erreur (ADR-0016 C1).
fn erreur_de_travail(generation: u64, erreur: impl std::fmt::Display) -> Retour {
    Retour::Erreur {
        generation,
        message: erreur.to_string(),
    }
}

/// Budget d'octets d'un fichier importé (audit n°3 A3-07,
/// ADR-0009) : un fichier d'import démesuré est refusé avant
/// lecture — 1 Gio dépasse déjà de loin les budgets des parseurs
/// SVG, DXF et STL.
const MAX_OCTETS_IMPORT: u64 = 1024 * 1024 * 1024;

/// Lit un fichier d'import sous un budget d'octets, refusé AVANT
/// lecture (audit n°3 A3-07, ADR-0009) : la taille du fichier est
/// mesurée sur ses métadonnées, un fichier démesuré est refusé sans
/// être lu — ni la mémoire ni le fil de travail ne paient pour un
/// fichier que les parseurs refuseraient. Le refus porte le code
/// OF-IO-0001 du registre central docs/error-codes.md ; la lecture
/// elle-même reste bornée : la taille peut changer entre la mesure
/// et la lecture.
pub(crate) fn lire_borne(chemin: &Path, budget: u64) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let annonce = std::fs::metadata(chemin)
        .map_err(|erreur| erreur.to_string())?
        .len();
    if annonce > budget {
        return Err(forge_io::IoError::TooLarge {
            found: annonce as usize,
            max: budget as usize,
        }
        .to_string());
    }
    let fichier = std::fs::File::open(chemin).map_err(|erreur| erreur.to_string())?;
    let mut octets = Vec::new();
    fichier
        .take(budget + 1)
        .read_to_end(&mut octets)
        .map_err(|erreur| erreur.to_string())?;
    if octets.len() as u64 > budget {
        return Err(forge_io::IoError::TooLarge {
            found: octets.len(),
            max: budget as usize,
        }
        .to_string());
    }
    Ok(octets)
}

/// Écrit les octets exportés au chemin choisi (ADR-0016 C1) —
/// atomiquement (audit n°2, N7) : un temporaire voisin est écrit,
/// synchronisé, puis renommé ; un échec en cours d'écriture ne
/// tronque jamais le fichier de l'utilisateur, et le temporaire est
/// nettoyé.
pub(crate) fn ecris_fichier(generation: u64, chemin: &Path, octets: &[u8]) -> Retour {
    let mut tmp = chemin.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let ecriture = (|| -> std::io::Result<()> {
        use std::io::Write as _;
        let mut fichier = std::fs::File::create(&tmp)?;
        fichier.write_all(octets)?;
        fichier.sync_all()?;
        drop(fichier);
        std::fs::rename(&tmp, chemin)
    })();
    if let Err(error) = ecriture {
        let _ = std::fs::remove_file(&tmp);
        return erreur_de_travail(generation, error);
    }
    Retour::Export {
        generation,
        chemin: chemin.display().to_string(),
    }
}

/// Un événement touche-t-il le graphe des liaisons ? Entrées,
/// expressions et suppressions d'alias redéfinissent des nœuds, la
/// mise à jour d'une contrainte réécrit une cote pilotée
/// (ADR-0019 §1) : la génération des liaisons avance et la
/// résolution du croquis re-fige ses cotes (ADR-0019 §6).
pub(crate) fn touche_les_liaisons(evenement: &str) -> bool {
    evenement == EVENT_INPUT_SET
        || evenement == EVENT_EXPRESSION_SET
        || evenement == EVENT_ALIAS_REMOVED
        || evenement == EVENT_SKETCH_CONSTRAINT_UPDATED
}
