use super::Pipeline;

/// A `Vec`-based mesh structure used to store mesh data on the CPU.
pub struct Mesh<P: Pipeline> {
    verts: Vec<P::Vertex>,
}

impl<P: Pipeline> Clone for Mesh<P>
where
    P::Vertex: Clone,
{
    fn clone(&self) -> Self {
        Self {
            verts: self.verts.clone(),
        }
    }
}

impl<P: Pipeline> Mesh<P> {
    /// Create a new `Mesh`.
    pub fn new() -> Self { Self { verts: vec![] } }

    /// Clear vertices, allows reusing allocated memory of the underlying Vec.
    pub fn clear(&mut self) { self.verts.clear(); }

    /// Get a slice referencing the vertices of this mesh.
    pub fn vertices(&self) -> &[P::Vertex] { &self.verts }

    /// Push a new vertex onto the end of this mesh.
    pub fn push(&mut self, vert: P::Vertex) { self.verts.push(vert); }

    /// Push a new polygon onto the end of this mesh.
    pub fn push_tri(&mut self, tri: Tri<P>) {
        self.verts.push(tri.a);
        self.verts.push(tri.b);
        self.verts.push(tri.c);
    }

    /// Push a new quad onto the end of this mesh.
    pub fn push_quad(&mut self, quad: Quad<P>) {
        // A quad is composed of two triangles. The code below converts the former to
        // the latter.

        // Tri 1
        self.verts.push(quad.a.clone());
        self.verts.push(quad.b);
        self.verts.push(quad.c.clone());

        // Tri 2
        self.verts.push(quad.c);
        self.verts.push(quad.d);
        self.verts.push(quad.a);
    }

    /// Push the vertices of another mesh onto the end of this mesh.
    pub fn push_mesh(&mut self, other: &Mesh<P>) { self.verts.extend_from_slice(other.vertices()); }

    /// Map and push the vertices of another mesh onto the end of this mesh.
    pub fn push_mesh_map<F: FnMut(P::Vertex) -> P::Vertex>(&mut self, other: &Mesh<P>, mut f: F) {
        // Reserve enough space in our Vec. This isn't necessary, but it tends to reduce
        // the number of required (re)allocations.
        self.verts.reserve(other.vertices().len());

        for vert in other.vertices() {
            self.verts.push(f(vert.clone()));
        }
    }

    pub fn iter(&self) -> std::slice::Iter<P::Vertex> { self.verts.iter() }
}

impl<P: Pipeline> IntoIterator for Mesh<P> {
    type IntoIter = std::vec::IntoIter<P::Vertex>;
    type Item = P::Vertex;

    fn into_iter(self) -> Self::IntoIter { self.verts.into_iter() }
}

/// Represents a triangle stored on the CPU.
pub struct Tri<P: Pipeline> {
    a: P::Vertex,
    b: P::Vertex,
    c: P::Vertex,
}

impl<P: Pipeline> Tri<P> {
    pub fn new(a: P::Vertex, b: P::Vertex, c: P::Vertex) -> Self { Self { a, b, c } }
}

/// Represents a quad stored on the CPU.
pub struct Quad<P: Pipeline> {
    a: P::Vertex,
    b: P::Vertex,
    c: P::Vertex,
    d: P::Vertex,
}

impl<P: Pipeline> Quad<P> {
    pub fn new(a: P::Vertex, b: P::Vertex, c: P::Vertex, d: P::Vertex) -> Self {
        Self { a, b, c, d }
    }
}
