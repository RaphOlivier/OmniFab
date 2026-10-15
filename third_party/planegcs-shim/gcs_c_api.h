/*
 * gcs_c_api.h — ABI C minimale au-dessus de planegcs (projet OmniFab).
 * SPDX-License-Identifier: LGPL-2.1-or-later
 * Copyright (C) 2026 Raphaël Olivier et contributeurs du projet OmniFab.
 *
 * Frontière FFI de l'ADR-0013 : toute la richesse C++ de planegcs
 * (GCS::System, templates, Eigen) reste derrière cette ABI. Le header est
 * volontairement minuscule : il est consommé par bindgen (forge-gcs-sys)
 * et ne doit exposer que des types C stables.
 *
 * Conventions :
 *  - les longueurs sont en millimètres, les angles en radians ;
 *  - les identifiants (paramètres, points, lignes, cercles, arcs)
 *    commencent à 0 et sont émis dans l'ordre d'ajout ;
 *  - les fonctions d'ajout et de contrainte renvoient un identifiant
 *    (paramètre/entité/tag >= 1 pour les contraintes) ou un code
 *    d'erreur négatif ;
 *  - chaque point d'entrée rattrape toute exception C++ : aucune ne
 *    fuit jamais à travers cette frontière.
 */
#ifndef OMNIFAB_GCS_C_API_H
#define OMNIFAB_GCS_C_API_H

#ifdef __cplusplus
extern "C" {
#endif

/* --- Codes d'erreur (valeurs négatives, jamais des identifiants) --- */

/** Succès des fonctions qui ne renvoient ni identifiant ni statut. */
#define GCS_OK 0

/** Exception C++ rattrapée dans le shim (erreur interne). */
#define GCS_ERR_EXCEPTION (-1)
/** Identifiant (paramètre, entité, contrainte) hors des identifiants émis. */
#define GCS_ERR_ID_INCONNU (-2)
/** Valeur non finie (NaN ou infini) refusée à l'entrée. */
#define GCS_ERR_VALEUR (-3)

/* --- Statuts de résolution (miroir de GCS::SolveStatus) --- */

/** Résolution exacte. */
#define GCS_STATUT_SUCCES 0
/** Résolution convergée sous tolérance. */
#define GCS_STATUT_CONVERGE 1
/** Résolution échouée (croquis conflictuel, pas de solution). */
#define GCS_STATUT_ECHEC 2

/* --- Algorithmes (miroir de GCS::Algorithm) --- */

#define GCS_ALGO_BFGS 0
#define GCS_ALGO_LEVENBERG_MARQUARDT 1
#define GCS_ALGO_DOGLEG 2

/** Capacité maximale des listes de tags dans GcsDiagnostic. */
#define GCS_MAX_TAGS 32

/** Diagnostic d'un croquis après analyse (interprété par forge-gcs). */
typedef struct GcsDiagnostic {
    /** Degrés de liberté restants (0 : croquis entièrement contraint). */
    int dof;
    /** 1 si les adresses de paramètres restent croissantes (résultat
     *  reproductible, voir FreeCAD #33399), 0 sinon. */
    int params_ordonnes;
    /** Contraintes conflictuelles (surentraînement). */
    int conflictuelles[GCS_MAX_TAGS];
    int conflictuelles_nombre;
    /** Contraintes redondantes. */
    int redondantes[GCS_MAX_TAGS];
    int redondantes_nombre;
    /** Contraintes partiellement redondantes. */
    int partiellement_redondantes[GCS_MAX_TAGS];
    int partiellement_redondantes_nombre;
} GcsDiagnostic;

/** Handle opaque sur un système de contraintes planegcs. */
typedef struct GcsSysteme GcsSysteme;

/* --- Cycle de vie --- */

/** Crée un système de contraintes vide. Renvoie NULL si l'allocation échoue. */
GcsSysteme* gcs_creer(void);

/** Détruit un système créé par gcs_creer. NULL est accepté (no-op). */
void gcs_detruire(GcsSysteme* systeme);

/* --- Paramètres (arène à adresses croissantes) ---
 *
 * Un paramètre « fixe » n'est pas une inconnue du solveur : c'est la
 * valeur pilote d'une contrainte (cote). Les paramètres libres sont les
 * inconnues ajustées par la résolution. Les valeurs de contraintes sont
 * des paramètres fixes pour que forge-values puisse les piloter plus
 * tard (phase E) en réécrivant le paramètre avant résolution.
 */

/** Ajoute un paramètre ; renvoie son identifiant ou un code d'erreur. */
int gcs_ajouter_parametre(GcsSysteme* systeme, double valeur, int fixe);

/** Lit la valeur courante d'un paramètre. */
int gcs_lire_parametre(GcsSysteme* systeme, int parametre, double* valeur);

/** Écrit la valeur d'un paramètre (utile aux cotes pilotées, phase E). */
int gcs_ecrire_parametre(GcsSysteme* systeme, int parametre, double valeur);

/** 1 si l'arène maintient des adresses croissantes, 0 sinon. */
int gcs_params_ordonnes(GcsSysteme* systeme);

/* --- Entités --- */

/** Ajoute un point (coordonnées en mm) ; renvoie son identifiant. */
int gcs_ajouter_point(GcsSysteme* systeme, double x, double y);

/** Ajoute une ligne entre deux points existants (extrémités partagées). */
int gcs_ajouter_ligne(GcsSysteme* systeme, int p1, int p2);

/** Ajoute un cercle par centre et rayon (mm). */
int gcs_ajouter_cercle(GcsSysteme* systeme, int centre, double rayon);

/** Ajoute un arc par centre, extrémités, rayon (mm) et angles (radians). */
int gcs_ajouter_arc(GcsSysteme* systeme, int centre, int debut, int fin,
                    double rayon, double angle_debut, double angle_fin);

/** Fixe (fixe=1) ou libère (fixe=0) les deux paramètres d'un point :
 * un point fixe ancre le croquis et n'est pas déplacé par le solveur. */
int gcs_fixer_point(GcsSysteme* systeme, int point, int fixe);

/** Lit la position courante d'un point (mm). */
int gcs_lire_point(GcsSysteme* systeme, int point, double* x, double* y);

/** Lit centre et rayon d'un cercle (mm). */
int gcs_lire_cercle(GcsSysteme* systeme, int cercle, double* x, double* y,
                   double* rayon);

/* --- Contraintes --- */
/* Chaque fonction renvoie un tag >= 1 (identifiant de contrainte) ou un
 * code d'erreur négatif. Les valeurs de contraintes (distance, angle,
 * rayon) sont passées par paramètre fixe (voir gcs_ajouter_parametre) :
 * c'est ce qui permettra aux expressions de forge-values de piloter les
 * cotes en phase E. */

/** Coïncidence de deux points. */
int gcs_contrainte_coincidence(GcsSysteme* systeme, int p1, int p2);

/** Ligne horizontale. */
int gcs_contrainte_horizontale(GcsSysteme* systeme, int ligne);

/** Ligne verticale. */
int gcs_contrainte_verticale(GcsSysteme* systeme, int ligne);

/** Distance entre deux points (valeur : paramètre fixe en mm). */
int gcs_contrainte_distance(GcsSysteme* systeme, int p1, int p2,
                            int valeur);

/** Angle entre deux lignes (valeur : paramètre fixe en radians). */
int gcs_contrainte_angle_l2l(GcsSysteme* systeme, int ligne1, int ligne2,
                             int valeur);

/** Deux lignes de même longueur. */
int gcs_contrainte_meme_longueur(GcsSysteme* systeme, int ligne1,
                                 int ligne2);

/** Point sur une ligne. */
int gcs_contrainte_point_sur_ligne(GcsSysteme* systeme, int point,
                                   int ligne);

/** Deux lignes parallèles. */
int gcs_contrainte_paralleles(GcsSysteme* systeme, int ligne1, int ligne2);

/** Deux lignes perpendiculaires. */
int gcs_contrainte_perpendiculaires(GcsSysteme* systeme, int ligne1,
                                    int ligne2);

/** Rayon d'un cercle (valeur : paramètre fixe en mm). */
int gcs_contrainte_rayon_cercle(GcsSysteme* systeme, int cercle,
                                int valeur);

/** Point sur un cercle. */
int gcs_contrainte_point_sur_cercle(GcsSysteme* systeme, int point,
                                    int cercle);

/* --- Résolution et diagnostic --- */

/* Résout le croquis avec l'algorithme choisi (GCS_ALGO_*). Renvoie un
 * statut GCS_STATUT_* ou GCS_ERR_EXCEPTION ; en cas de succès ou de
 * convergence, la solution est appliquée aux paramètres. */
int gcs_resoudre(GcsSysteme* systeme, int algorithme);

/* Analyse le croquis (degrés de liberté, contraintes conflictuelles,
 * redondantes, partiellement redondantes, reproductibilité). Renvoie
 * GCS_OK (0) ou GCS_ERR_EXCEPTION. */
int gcs_diagnostiquer(GcsSysteme* systeme, int algorithme,
                      GcsDiagnostic* diagnostic);

#ifdef __cplusplus
}
#endif
#endif /* OMNIFAB_GCS_C_API_H */
