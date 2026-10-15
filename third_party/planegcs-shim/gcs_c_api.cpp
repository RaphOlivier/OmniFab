/*
 * gcs_c_api.cpp — Shim ABI C minimal au-dessus de planegcs (projet OmniFab).
 * SPDX-License-Identifier: LGPL-2.1-or-later
 * Copyright (C) 2026 Raphaël Olivier et contributeurs du projet OmniFab.
 *
 * Implémentation de l'ABI déclarée dans gcs_c_api.h (ADR-0013). Le modèle
 * suit l'approche éprouvée du wrapper Python de spookylukey/planegcs
 * (LGPL-2.1-or-later) : une arène de paramètres à adresses croissantes
 * (même principe que son param_arena.h, réécrite ici) et des entités
 * liées par des double* stables.
 *
 * Règles inviolables de ce fichier :
 *  - aucune exception ne fuit à travers l'ABI : chaque point d'entrée
 *    externe rattrape tout (catch (...)) ;
 *  - les identifiants émis ne sont jamais réutilisés ;
 *  - l'ordre d'ajout des paramètres fait foi (reproductibilité, voir
 *    FreeCAD #33399 : planegcs trie certaines listes par adresses).
 */
#include "gcs_c_api.h"

#include "GCS.h"
#include "Geo.h"
#include "Constraints.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <map>
#include <new>
#include <utility>
#include <vector>

#if defined(_WIN32)
// windows.h définit les macros min/max qui cassent std::min/std::max
// (erreur MSVC C2589 sur std::min à la ligne du diagnostic) : le garde
// NOMINMAX est le correctif standard, requis pour compiler le shim.
#define NOMINMAX
#include <windows.h>
#else
#include <sys/mman.h>
#include <unistd.h>
#endif

namespace {

/** Exception interne levée quand un identifiant est hors bornes ; elle
 *  est rattrapée à la frontière et devient GCS_ERR_ID_INCONNU. */
struct IdInconnu {};

/* ------------------------------------------------------------------ */
/* Arène de paramètres                                                  */
/*                                                                      */
/* planegcs ajuste des double* : les adresses doivent rester stables    */
/* pendant toute la vie du système. L'arène réserve une grande plage     */
/* d'adressage puis la valide page par page, ce qui donne des          */
/* adresses croissantes dans l'ordre d'ajout — condition de la          */
/* reproductibilité du solveur (FreeCAD #33399). Au-delà de la plage    */
/* réservée (budget borné), un repli sur le tas est utilisé et la       */
/* reproductibilité est signalée non garantie (params_ordonnes = 0).    */
/* ------------------------------------------------------------------ */
class Arena {
public:
    Arena() = default;
    Arena(const Arena&) = delete;
    Arena& operator=(const Arena&) = delete;

    ~Arena() {
#if defined(_WIN32)
        if (plage_ != nullptr) {
            VirtualFree(plage_, 0, MEM_RELEASE);
        }
#else
        if (plage_ != nullptr) {
            munmap(plage_, GCS_ARENE_TAILLE * sizeof(double));
        }
#endif
        delete[] extension_;
    }

    /** Nombre de paramètres ajoutés. */
    std::size_t taille() const { return taille_; }

    /** Adresse stable du paramètre index ; lève IdInconnu hors bornes. */
    double* ptr(std::size_t index) {
        if (index >= taille_) {
            throw IdInconnu{};
        }
        if (index < GCS_ARENE_TAILLE) {
            return plage_ + index;
        }
        return extension_ + (index - GCS_ARENE_TAILLE);
    }

    /** Ajoute un paramètre ; renvoie son index dans l'arène. */
    std::size_t push_back(double valeur) {
        if (taille_ == 0) {
            reserver();
        }
        if (taille_ < GCS_ARENE_TAILLE) {
            engager_pages(taille_);
            plage_[taille_] = valeur;
        } else {
            if (extension_ == nullptr) {
                // Repli hors plage réservée : adresses non garanties
                // croissantes -> reproductibilité non assurée.
                extension_ = new double[GCS_ARENE_EXTENSION];
                ordonnes_ = false;
            }
            std::size_t index = taille_ - GCS_ARENE_TAILLE;
            if (index >= GCS_ARENE_EXTENSION) {
                throw std::bad_alloc();
            }
            extension_[index] = valeur;
        }
        return taille_++;
    }

    /** 1 tant que toutes les adresses sont croissantes dans l'ordre
     *  d'ajout (reproductibilité garantie). */
    int monotonic() const {
        return ordonnes_ ? 1 : 0;
    }

private:
    // Budget du spike : ~1 Mo de plage réservée (131 072 paramètres),
    // extension de repli bornée. Un croquis réel en est loin.
    static constexpr std::size_t GCS_ARENE_TAILLE = 1u << 20;
    static constexpr std::size_t GCS_ARENE_EXTENSION = 1u << 16;

    void reserver() {
#if defined(_WIN32)
        plage_ = static_cast<double*>(VirtualAlloc(
            nullptr, GCS_ARENE_TAILLE * sizeof(double), MEM_RESERVE,
            PAGE_NOACCESS));
        if (plage_ == nullptr) {
            throw std::bad_alloc();
        }
#else
        plage_ = static_cast<double*>(mmap(
            nullptr, GCS_ARENE_TAILLE * sizeof(double), PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS, -1, 0));
        if (plage_ == MAP_FAILED) {
            plage_ = nullptr;
            throw std::bad_alloc();
        }
#endif
    }

    /** Rend lisibles/écrivables les pages couvrant [0, index]. */
    void engager_pages(std::size_t index) {
        std::size_t octets = (index + 1) * sizeof(double);
#if defined(_WIN32)
        SYSTEM_INFO info;
        GetSystemInfo(&info);
        std::size_t page = info.dwPageSize;
#else
        long page = sysconf(_SC_PAGESIZE);
        if (page <= 0) {
            throw std::bad_alloc();
        }
#endif
        std::size_t pages = (octets + static_cast<std::size_t>(page) - 1)
            / static_cast<std::size_t>(page);
        if (pages <= pages_engagees_) {
            return;
        }
#if defined(_WIN32)
        void* debut = static_cast<char*>(
                          static_cast<void*>(plage_))
            + pages_engagees_ * page;
        std::size_t taille_pages =
            (pages - pages_engagees_) * page;
        if (!VirtualAlloc(debut, taille_pages, MEM_COMMIT,
                          PAGE_READWRITE)) {
            throw std::bad_alloc();
        }
#else
        void* debut = static_cast<char*>(static_cast<void*>(plage_))
            + pages_engagees_ * page;
        std::size_t taille_pages =
            (pages - pages_engagees_) * static_cast<std::size_t>(page);
        if (mprotect(debut, taille_pages, PROT_READ | PROT_WRITE) != 0) {
            throw std::bad_alloc();
        }
#endif
        pages_engagees_ = pages;
    }

    double* plage_ = nullptr;
    double* extension_ = nullptr;
    std::size_t taille_ = 0;
    std::size_t pages_engagees_ = 0;
    bool ordonnes_ = true;
};

} // namespace

/* ------------------------------------------------------------------ */
/* Système de contraintes                                              */
/* ------------------------------------------------------------------ */
/* Défini à l'échelle globale : le header ne déclare qu'un handle      */
/* opaque (`typedef struct GcsSysteme GcsSysteme;`), c'est ici qu'il  */
/* est complété — pattern classique de frontière C.                   */
/* ------------------------------------------------------------------ */
struct GcsSysteme {
    GCS::System systeme;
    Arena params;
    std::map<int, std::size_t> index_param;
    std::map<int, bool> fixe_param;
    std::map<int, GCS::Point> points;
    std::map<int, std::pair<int, int>> params_point;
    std::map<int, GCS::Line> lignes;
    std::map<int, GCS::Circle> cercles;
    std::map<int, GCS::Arc> arcs;
    int prochain_param = 0;
    int prochaine_entite = 0;
    int prochain_tag = 1;

    // --- Accès (lèvent IdInconnu si hors bornes) ---
    double* ptr_param(int id) {
        auto it = index_param.find(id);
        if (it == index_param.end()) {
            throw IdInconnu{};
        }
        return params.ptr(it->second);
    }

    GCS::Point& point(int id) {
        auto it = points.find(id);
        if (it == points.end()) {
            throw IdInconnu{};
        }
        return it->second;
    }

    GCS::Line& ligne(int id) {
        auto it = lignes.find(id);
        if (it == lignes.end()) {
            throw IdInconnu{};
        }
        return it->second;
    }

    GCS::Circle& cercle(int id) {
        auto it = cercles.find(id);
        if (it == cercles.end()) {
            throw IdInconnu{};
        }
        return it->second;
    }

    // --- Paramètres ---
    int ajouter_parametre(double valeur, bool fixe) {
        int id = prochain_param++;
        index_param[id] = params.push_back(valeur);
        fixe_param[id] = fixe;
        return id;
    }

    void declarer_inconnues() {
        GCS::VEC_pD inconnus;
        for (const auto& entree : index_param) {
            if (!fixe_param[entree.first]) {
                inconnus.push_back(params.ptr(entree.second));
            }
        }
        GCS::VEC_pD pilotes; // vide au spike : phase E (cotes pilotées)
        systeme.declareUnknowns(inconnus);
        systeme.declareDrivenParams(pilotes);
    }

    // --- Entités ---
    int ajouter_point(double x, double y) {
        int px = ajouter_parametre(x, false);
        int py = ajouter_parametre(y, false);
        int id = prochaine_entite++;
        GCS::Point p;
        p.x = ptr_param(px);
        p.y = ptr_param(py);
        points[id] = p;
        params_point[id] = {px, py};
        return id;
    }

    int ajouter_ligne(int p1, int p2) {
        GCS::Line l;
        l.p1 = point(p1);
        l.p2 = point(p2);
        int id = prochaine_entite++;
        lignes[id] = l;
        return id;
    }

    int ajouter_cercle(int centre, double rayon) {
        int pr = ajouter_parametre(rayon, false);
        GCS::Circle c;
        c.center = point(centre);
        c.rad = ptr_param(pr);
        int id = prochaine_entite++;
        cercles[id] = c;
        return id;
    }

    int ajouter_arc(int centre, int debut, int fin, double rayon,
                    double angle_debut, double angle_fin) {
        int pr = ajouter_parametre(rayon, false);
        int pa1 = ajouter_parametre(angle_debut, false);
        int pa2 = ajouter_parametre(angle_fin, false);
        GCS::Arc a;
        a.center = point(centre);
        a.start = point(debut);
        a.end = point(fin);
        a.rad = ptr_param(pr);
        a.startAngle = ptr_param(pa1);
        a.endAngle = ptr_param(pa2);
        int id = prochaine_entite++;
        arcs[id] = a;
        // Règles d'arc (extrémités sur le cercle aux angles donnés) :
        // mêmes contraintes internes que le wrapper de référence.
        systeme.addConstraintArcRules(arcs[id], prochain_tag++, true);
        return id;
    }

    void fixer_point(int id, bool fixe) {
        auto ent = params_point.find(id);
        if (ent == params_point.end()) {
            throw IdInconnu{};
        }
        fixe_param[ent->second.first] = fixe;
        fixe_param[ent->second.second] = fixe;
    }

    // --- Contraintes : chaque ajout renvoie un tag frais ---
    int contrainte_coincidence(int p1, int p2) {
        int tag = prochain_tag++;
        systeme.addConstraintP2PCoincident(point(p1), point(p2), tag, true);
        return tag;
    }

    int contrainte_horizontale(int l) {
        int tag = prochain_tag++;
        systeme.addConstraintHorizontal(ligne(l), tag, true);
        return tag;
    }

    int contrainte_verticale(int l) {
        int tag = prochain_tag++;
        systeme.addConstraintVertical(ligne(l), tag, true);
        return tag;
    }

    int contrainte_distance(int p1, int p2, int valeur) {
        int tag = prochain_tag++;
        systeme.addConstraintP2PDistance(point(p1), point(p2),
                                         ptr_param(valeur), tag, true);
        return tag;
    }

    int contrainte_angle_l2l(int l1, int l2, int valeur) {
        int tag = prochain_tag++;
        systeme.addConstraintL2LAngle(ligne(l1), ligne(l2),
                                      ptr_param(valeur), tag, true);
        return tag;
    }

    int contrainte_meme_longueur(int l1, int l2) {
        int tag = prochain_tag++;
        systeme.addConstraintEqualLength(ligne(l1), ligne(l2), tag, true);
        return tag;
    }

    int contrainte_point_sur_ligne(int p, int l) {
        int tag = prochain_tag++;
        systeme.addConstraintPointOnLine(point(p), ligne(l), tag, true);
        return tag;
    }

    int contrainte_paralleles(int l1, int l2) {
        int tag = prochain_tag++;
        systeme.addConstraintParallel(ligne(l1), ligne(l2), tag, true);
        return tag;
    }

    int contrainte_perpendiculaires(int l1, int l2) {
        int tag = prochain_tag++;
        systeme.addConstraintPerpendicular(ligne(l1), ligne(l2), tag,
                                            true);
        return tag;
    }

    int contrainte_rayon_cercle(int c, int valeur) {
        int tag = prochain_tag++;
        systeme.addConstraintCircleRadius(cercle(c), ptr_param(valeur),
                                          tag, true);
        return tag;
    }

    int contrainte_point_sur_cercle(int p, int c) {
        int tag = prochain_tag++;
        systeme.addConstraintPointOnCircle(point(p), cercle(c), tag, true);
        return tag;
    }

    // --- Résolution ---
    static bool algorithme_valide(int algorithme) {
        return algorithme == GCS_ALGO_BFGS
            || algorithme == GCS_ALGO_LEVENBERG_MARQUARDT
            || algorithme == GCS_ALGO_DOGLEG;
    }

    int resoudre(int algorithme) {
        GCS::Algorithm alg = static_cast<GCS::Algorithm>(algorithme);
        declarer_inconnues();
        systeme.initSolution(alg);
        int statut = systeme.solve(true, alg);
        if (statut == GCS::Success || statut == GCS::Converged) {
            systeme.applySolution();
        }
        return statut;
    }

    int diagnostiquer(int algorithme, GcsDiagnostic* out) {
        GCS::Algorithm alg = static_cast<GCS::Algorithm>(algorithme);
        declarer_inconnues();
        systeme.initSolution(alg);
        systeme.diagnose(alg);
        out->dof = systeme.dofsNumber();
        out->params_ordonnes = params.monotonic();

        GCS::VEC_I conflits;
        GCS::VEC_I redondants;
        GCS::VEC_I partiels;
        systeme.getConflicting(conflits);
        systeme.getRedundant(redondants);
        systeme.getPartiallyRedundant(partiels);

        auto copier = [](const GCS::VEC_I& source, int* dest,
                         int* nombre) {
            std::size_t n = std::min(
                source.size(), static_cast<std::size_t>(GCS_MAX_TAGS));
            for (std::size_t i = 0; i < n; ++i) {
                dest[i] = static_cast<int>(source[i]);
            }
            *nombre = static_cast<int>(n);
        };
        copier(conflits, out->conflictuelles, &out->conflictuelles_nombre);
        copier(redondants, out->redondantes, &out->redondantes_nombre);
        copier(partiels, out->partiellement_redondantes,
               &out->partiellement_redondantes_nombre);
        return GCS_OK;
    }
};

// Fonction utilitaire à liaison interne (répétée après le namespace
// anonyme : le struct GcsSysteme, handle opaque, doit être défini à
// l'échelle globale, pas dans le namespace anonyme).
static bool valeur_finie(double v) { return std::isfinite(v); }

/* ------------------------------------------------------------------ */
/* Frontière extern "C" : rattrapage de toutes les exceptions            */
/* ------------------------------------------------------------------ */

extern "C" {

GcsSysteme* gcs_creer(void) {
    try {
        return new GcsSysteme();
    } catch (...) {
        return nullptr;
    }
}

void gcs_detruire(GcsSysteme* systeme) {
    delete systeme; // delete de nullptr est sans effet
}

int gcs_ajouter_parametre(GcsSysteme* systeme, double valeur, int fixe) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!valeur_finie(valeur)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->ajouter_parametre(valeur, fixe != 0);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_lire_parametre(GcsSysteme* systeme, int parametre, double* valeur) {
    if (systeme == nullptr || valeur == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        *valeur = *systeme->ptr_param(parametre);
        return GCS_OK;
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_ecrire_parametre(GcsSysteme* systeme, int parametre,
                         double valeur) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!valeur_finie(valeur)) {
        return GCS_ERR_VALEUR;
    }
    try {
        *systeme->ptr_param(parametre) = valeur;
        return GCS_OK;
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_params_ordonnes(GcsSysteme* systeme) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    return systeme->params.monotonic();
}

int gcs_ajouter_point(GcsSysteme* systeme, double x, double y) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!valeur_finie(x) || !valeur_finie(y)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->ajouter_point(x, y);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_ajouter_ligne(GcsSysteme* systeme, int p1, int p2) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->ajouter_ligne(p1, p2);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_ajouter_cercle(GcsSysteme* systeme, int centre, double rayon) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!valeur_finie(rayon)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->ajouter_cercle(centre, rayon);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_ajouter_arc(GcsSysteme* systeme, int centre, int debut, int fin,
                    double rayon, double angle_debut, double angle_fin) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!valeur_finie(rayon) || !valeur_finie(angle_debut)
        || !valeur_finie(angle_fin)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->ajouter_arc(centre, debut, fin, rayon, angle_debut,
                                    angle_fin);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_fixer_point(GcsSysteme* systeme, int point, int fixe) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        systeme->fixer_point(point, fixe != 0);
        return GCS_OK;
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_lire_point(GcsSysteme* systeme, int point, double* x, double* y) {
    if (systeme == nullptr || x == nullptr || y == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        GCS::Point& p = systeme->point(point);
        *x = *p.x;
        *y = *p.y;
        return GCS_OK;
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_lire_cercle(GcsSysteme* systeme, int cercle, double* x, double* y,
                    double* rayon) {
    if (systeme == nullptr || x == nullptr || y == nullptr
        || rayon == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        GCS::Circle& c = systeme->cercle(cercle);
        *x = *c.center.x;
        *y = *c.center.y;
        *rayon = *c.rad;
        return GCS_OK;
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_coincidence(GcsSysteme* systeme, int p1, int p2) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_coincidence(p1, p2);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_horizontale(GcsSysteme* systeme, int ligne) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_horizontale(ligne);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_verticale(GcsSysteme* systeme, int ligne) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_verticale(ligne);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_distance(GcsSysteme* systeme, int p1, int p2,
                            int valeur) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_distance(p1, p2, valeur);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_angle_l2l(GcsSysteme* systeme, int ligne1, int ligne2,
                             int valeur) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_angle_l2l(ligne1, ligne2, valeur);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_meme_longueur(GcsSysteme* systeme, int ligne1,
                                 int ligne2) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_meme_longueur(ligne1, ligne2);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_point_sur_ligne(GcsSysteme* systeme, int point,
                                   int ligne) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_point_sur_ligne(point, ligne);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_paralleles(GcsSysteme* systeme, int ligne1,
                              int ligne2) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_paralleles(ligne1, ligne2);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_perpendiculaires(GcsSysteme* systeme, int ligne1,
                                    int ligne2) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_perpendiculaires(ligne1, ligne2);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_rayon_cercle(GcsSysteme* systeme, int cercle,
                                int valeur) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_rayon_cercle(cercle, valeur);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_contrainte_point_sur_cercle(GcsSysteme* systeme, int point,
                                    int cercle) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    try {
        return systeme->contrainte_point_sur_cercle(point, cercle);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_resoudre(GcsSysteme* systeme, int algorithme) {
    if (systeme == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!GcsSysteme::algorithme_valide(algorithme)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->resoudre(algorithme);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

int gcs_diagnostiquer(GcsSysteme* systeme, int algorithme,
                      GcsDiagnostic* diagnostic) {
    if (systeme == nullptr || diagnostic == nullptr) {
        return GCS_ERR_EXCEPTION;
    }
    if (!GcsSysteme::algorithme_valide(algorithme)) {
        return GCS_ERR_VALEUR;
    }
    try {
        return systeme->diagnostiquer(algorithme, diagnostic);
    } catch (const IdInconnu&) {
        return GCS_ERR_ID_INCONNU;
    } catch (...) {
        return GCS_ERR_EXCEPTION;
    }
}

} // extern "C"
