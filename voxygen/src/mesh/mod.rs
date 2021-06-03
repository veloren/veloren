pub mod greedy;
pub mod segment;
pub mod terrain;

use crate::render::Mesh;

pub type MeshGen<V, T, S, R> = (Mesh<V>, Mesh<T>, Mesh<S>, R);
