use super::Vertex;
use core::{iter::FromIterator, ops::Range};

/// A `Vec`-based mesh structure used to store mesh data on the CPU.
pub struct Mesh<V: Vertex> {
    verts: Vec<V>,
}

impl<V: Vertex> Clone for Mesh<V> {
    fn clone(&self) -> Self {
        Self {
            verts: self.verts.clone(),
        }
    }
}

impl<V: Vertex> Mesh<V> {
    #[allow(clippy::new_without_default)]
    /// Create a new `Mesh`.
    pub fn new() -> Self { Self { verts: Vec::new() } }

    /// Clear vertices, allows reusing allocated memory of the underlying Vec.
    pub fn clear(&mut self) { self.verts.clear(); }

    /// Get a slice referencing the vertices of this mesh.
    pub fn vertices(&self) -> &[V] { &self.verts }

    /// Get a mutable slice referencing the vertices of this mesh.
    pub fn vertices_mut(&mut self) -> &mut [V] { &mut self.verts }

    /// Get a mutable vec referencing the vertices of this mesh.
    pub fn vertices_mut_vec(&mut self) -> &mut Vec<V> { &mut self.verts }

    /// Push a new vertex onto the end of this mesh.
    pub fn push(&mut self, vert: V) { self.verts.push(vert); }

    /// Push a new polygon onto the end of this mesh.
    pub fn push_tri(&mut self, tri: Tri<V>) {
        self.verts.push(tri.a);
        self.verts.push(tri.b);
        self.verts.push(tri.c);
    }

    /// Push a new quad onto the end of this mesh.
    pub fn push_quad(&mut self, quad: Quad<V>) {
        // A quad is composed of two triangles. The code below converts the former to
        // the latter.
        if V::QUADS_INDEX.is_some() {
            // 0, 1, 2, 2, 1, 3
            // b, c, a, a, c, d
            self.verts.push(quad.b);
            self.verts.push(quad.c);
            self.verts.push(quad.a);
            self.verts.push(quad.d);
        } else {
            // Tri 1
            self.verts.push(quad.a);
            self.verts.push(quad.b);
            self.verts.push(quad.c);

            // Tri 2
            self.verts.push(quad.c);
            self.verts.push(quad.d);
            self.verts.push(quad.a);
        }
    }

    /// Overwrite a quad
    pub fn replace_quad(&mut self, index: usize, quad: Quad<V>) {
        if V::QUADS_INDEX.is_some() {
            debug_assert!(index % 4 == 0);
            assert!(index + 3 < self.verts.len());
            self.verts[index] = quad.b;
            self.verts[index + 1] = quad.c;
            self.verts[index + 2] = quad.a;
            self.verts[index + 3] = quad.d;
        } else {
            debug_assert!(index % 3 == 0);
            assert!(index + 5 < self.verts.len());
            // Tri 1
            self.verts[index] = quad.a;
            self.verts[index + 1] = quad.b;
            self.verts[index + 2] = quad.c;

            // Tri 2
            self.verts[index + 3] = quad.c;
            self.verts[index + 4] = quad.d;
            self.verts[index + 5] = quad.a;
        }
    }

    /// Push the vertices of another mesh onto the end of this mesh.
    pub fn push_mesh(&mut self, other: &Mesh<V>) { self.verts.extend_from_slice(other.vertices()); }

    /// Map and push the vertices of another mesh onto the end of this mesh.
    pub fn push_mesh_map<F: FnMut(V) -> V>(&mut self, other: &Mesh<V>, mut f: F) {
        // Reserve enough space in our Vec. This isn't necessary, but it tends to reduce
        // the number of required (re)allocations.
        self.verts.reserve(other.vertices().len());

        for vert in other.vertices() {
            self.verts.push(f(*vert));
        }
    }

    pub fn iter(&self) -> std::slice::Iter<V> { self.verts.iter() }

    /// NOTE: Panics if vertex_range is out of bounds of vertices.
    pub fn iter_mut(&mut self, vertex_range: Range<usize>) -> std::slice::IterMut<V> {
        self.verts[vertex_range].iter_mut()
    }

    pub fn len(&self) -> usize { self.verts.len() }

    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

impl<V: Vertex> IntoIterator for Mesh<V> {
    type IntoIter = std::vec::IntoIter<V>;
    type Item = V;

    fn into_iter(self) -> Self::IntoIter { self.verts.into_iter() }
}

impl<V: Vertex> FromIterator<Tri<V>> for Mesh<V> {
    fn from_iter<I: IntoIterator<Item = Tri<V>>>(tris: I) -> Self {
        let mut this = Self::new();
        let tris = tris.into_iter();
        let (lower, upper) = tris.size_hint();
        this.verts.reserve(3 * upper.unwrap_or(lower));
        tris.fold(this, |mut this, tri| {
            this.push_tri(tri);
            this
        })
    }
}

impl<V: Vertex> FromIterator<Quad<V>> for Mesh<V> {
    fn from_iter<I: IntoIterator<Item = Quad<V>>>(quads: I) -> Self {
        let mut this = Self::new();
        let quads = quads.into_iter();
        let (lower, upper) = quads.size_hint();
        this.verts
            .reserve(if V::QUADS_INDEX.is_some() { 4 } else { 6 } * upper.unwrap_or(lower));
        quads.fold(this, |mut this, quad| {
            this.push_quad(quad);
            this
        })
    }
}

/// Represents a triangle stored on the CPU.
pub struct Tri<V: Vertex> {
    a: V,
    b: V,
    c: V,
}

impl<V: Vertex> Tri<V> {
    pub fn new(a: V, b: V, c: V) -> Self { Self { a, b, c } }
}

/// Represents a quad stored on the CPU.
pub struct Quad<V: Vertex> {
    a: V,
    b: V,
    c: V,
    d: V,
}

impl<V: Vertex> Quad<V> {
    pub fn new(a: V, b: V, c: V, d: V) -> Self { Self { a, b, c, d } }

    #[must_use]
    pub fn rotated_by(self, n: usize) -> Self {
        let verts = [self.a, self.b, self.c, self.d];

        Self {
            a: verts[n % 4],
            b: verts[(1 + n) % 4],
            c: verts[(2 + n) % 4],
            d: verts[(3 + n) % 4],
        }
    }
}
