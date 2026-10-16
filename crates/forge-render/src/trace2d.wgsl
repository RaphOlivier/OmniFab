// Vertex 2D du viewport forge-render (ADR-0015).
// Les sommets arrivent en pixels physiques RELATIFS AU COIN du
// viewport : egui-wgpu borne la passe au rect du callback, l'uniform
// donne donc la taille du rect — jamais celle de la fenêtre. La
// couleur est portée par sommet, un vec4 RGBA normalisé.

struct Uniforms {
    taille_px: vec2<f32>,
    remplissage: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct Entree {
    @location(0) position_px: vec2<f32>,
    @location(1) couleur: vec4<f32>,
};

struct Sortie {
    @builtin(position) position: vec4<f32>,
    @location(0) couleur: vec4<f32>,
};

@vertex
fn vs_main(entree: Entree) -> Sortie {
    var sortie: Sortie;
    let ndc_x = entree.position_px.x / uniforms.taille_px.x * 2.0 - 1.0;
    let ndc_y = 1.0 - entree.position_px.y / uniforms.taille_px.y * 2.0;
    sortie.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    sortie.couleur = entree.couleur;
    return sortie;
}

@fragment
fn fs_main(sortie: Sortie) -> @location(0) vec4<f32> {
    return sortie.couleur;
}
