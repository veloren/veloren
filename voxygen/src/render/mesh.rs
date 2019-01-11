// Local
use super::Pipeline;

/// Used to store vertex data on the CPU
pub struct Mesh<P: Pipeline> {
    verts: Vec<P::Vertex>,
}

impl<P: Pipeline> Mesh<P> {
    pub fn new() -> Self {
        Self { verts: vec![] }
    }

    pub fn vertices(&self) -> &[P::Vertex] {
        &self.verts
    }

    pub fn push(&mut self, vert: P::Vertex) {
        self.verts.push(vert);
    }

    pub fn push_quad(&mut self, quad: Quad<P>) {
        // Tri 1
        self.verts.push(quad.a.clone());
        self.verts.push(quad.b);
        self.verts.push(quad.c.clone());

        // Tri 2
        self.verts.push(quad.c);
        self.verts.push(quad.d);
        self.verts.push(quad.a);
    }
}

/// Represents a quad
pub struct Quad<P: Pipeline> {
    a: P::Vertex,
    b: P::Vertex,
    c: P::Vertex,
    d: P::Vertex,
}

impl<P: Pipeline> Quad<P> {
    pub fn new(
        a: P::Vertex,
        b: P::Vertex,
        c: P::Vertex,
        d: P::Vertex,
    ) -> Self {
        Self { a, b, c, d }
    }
}
