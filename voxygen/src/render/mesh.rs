// Local
use super::Pipeline;

/// A `Vec`-based mesh structure used to store mesh data on the CPU.
pub struct Mesh<P: Pipeline> {
    verts: Vec<P::Vertex>,
}

impl<P: Pipeline> Mesh<P> {
    /// Create a new `Mesh`
    pub fn new() -> Self {
        Self { verts: vec![] }
    }

    /// Get a slice referencing the vertices of this mesh.
    pub fn vertices(&self) -> &[P::Vertex] {
        &self.verts
    }

    /// Push a new vertex onto the end of this mesh.
    pub fn push(&mut self, vert: P::Vertex) {
        self.verts.push(vert);
    }

    /// Push a new polygon onto the end of this mesh.
    pub fn push_tri(&mut self, tri: Tri<P>) {
        self.verts.push(tri.a);
        self.verts.push(tri.b);
        self.verts.push(tri.c);
    }

    /// Push a new quad onto the end of this mesh.
    pub fn push_quad(&mut self, quad: Quad<P>) {
        // A quad is composed of two triangles. The code below converts the former to the latter.

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

/// Represents a triangle stored on the CPU.
pub struct Tri<P: Pipeline> {
    a: P::Vertex,
    b: P::Vertex,
    c: P::Vertex,
}

impl<P: Pipeline> Tri<P> {
    pub fn new(
        a: P::Vertex,
        b: P::Vertex,
        c: P::Vertex,
    ) -> Self {
        Self { a, b, c }
    }
}

/// Represents a quad stored on the CPU.
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
