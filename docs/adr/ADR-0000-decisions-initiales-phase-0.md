# ADR-0000 : Décisions initiales de la phase 0

- **Statut : à rédiger** (squelette)
- **Date : (à compléter en phase 0)**
- **Décideurs : Raphaël Olivier**

## Contexte

OmniFab démarre : logiciel libre Rust de fabrication A→Z (CAO, CAM, G-code, pilotage machine), UI egui, rendu wgpu, géométrie OCCT, contraintes planegcs, concurrence tokio, licence GPL-3.0+.

Les décisions structurantes déjà tranchées lors de la conception préalable (hors dépôt) :

1. Conteneur ZIP versionné pour les documents projet.
2. Event sourcing pour l'historique des modifications.
3. planegcs comme solveur de contraintes 2D (via FFI).
4. tokio + canaux pour la concurrence et le pilotage machine.
5. Unités strictes : newtypes pour longueur/angle, tolérance 1e-6 mm.
6. Moteur unique de liaisons (alias + nodal).
7. Assistants de conception = templates déclaratifs (des données).
8. Simulation d'abord pour le pilotage machine (aucun matériel physique).
9. Premier assistant cible : meuble/caisson.

## Décision

*(À rédiger en phase 0 : acter formellement ces décisions, les détailler et les motiver.)*

## Alternatives rejetées

*(À rédiger : pour chaque verrou, les alternatives envisagées et pourquoi elles ont été rejetées.)*

## Conséquences

*(À rédiger : impacts positifs, négatifs, et décisions futures suspendues à celle-ci.)*
