# CMakeLists.txt — shim ABI C au-dessus de planegcs (projet OmniFab).
# SPDX-License-Identifier: LGPL-2.1-or-later
#
# Piloté par forge-gcs-sys (crate `cmake`), ADR-0013 : le sous-module
# third_party/planegcs est construit ici, puis lié statiquement avec le
# shim gcs_c_api. Eigen3 et fmt doivent être découvrables par find_package
# (dépendances publiques de planegcs) ; Boost n'est requis que pour
# construire planegcs lui-même (headers, Boost.Graph).
#
# CONTRAT d'install : la crate `cmake` de forge-gcs-sys lance
# `cmake --build … --target install` — ce fichier DOIT installer les deux
# bibliothèques statiques dans ${CMAKE_INSTALL_PREFIX}/lib, le chemin que
# build.rs transmet au linker Rust (gcs_shim puis planegcs y sont reliés
# explicitement).

cmake_minimum_required(VERSION 3.26)

project(omnifab_gcs_shim LANGUAGES CXX)

# planegcs (extraction standalone, sous-module git figé par le MANIFEST
# de third_party/). PLANEGCS_INSTALL reste OFF (pas top level) : aucune
# règle install amont, celle-ci est fournie en bas de fichier. La cible
# est tirée par gcs_shim, qui en dépend (liaison PRIVATE).
add_subdirectory(
    ${CMAKE_CURRENT_SOURCE_DIR}/../planegcs
    ${CMAKE_CURRENT_BINARY_DIR}/planegcs
    EXCLUDE_FROM_ALL
)

# Bibliothèque statique du shim : le seul objet exposé à Rust.
add_library(gcs_shim STATIC gcs_c_api.cpp)
target_compile_features(gcs_shim PRIVATE cxx_std_20)
target_include_directories(gcs_shim PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(gcs_shim PRIVATE PlaneGCS::planegcs)

# Informations de debug utiles en cas d'échec de configuration CI.
message(STATUS "omnifab_gcs_shim : CXX=${CMAKE_CXX_COMPILER}, "
               "CMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}")

# Règles d'install attendues par forge-gcs-sys (crate `cmake`) : les
# deux .a finissent dans ${CMAKE_INSTALL_PREFIX}/lib. install(FILES) +
# expression $<TARGET_FILE:...> car install(TARGETS) refuse une cible
# définie dans un autre répertoire (planegcs vient du sous-module).
install(TARGETS gcs_shim DESTINATION lib)
install(FILES $<TARGET_FILE:planegcs> DESTINATION lib)
