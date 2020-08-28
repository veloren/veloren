use crate::{site::Site, Colors};
use common::{
    assets::{watch::ReloadIndicator, Asset, Ron},
    store::Store,
};
use core::ops::Deref;
use noise::{Seedable, SuperSimplex};
use std::sync::Arc;

const WORLD_COLORS_MANIFEST: &str = "world.style.colors";

pub struct Index {
    pub seed: u32,
    pub time: f32,
    pub noise: Noise,
    pub sites: Store<Site>,
    indicator: ReloadIndicator,
}

/// An owned reference to indexed data.
///
/// The data are split out so that we can replace the colors without disturbing
/// the rest of the index, while also keeping all the data within a single
/// indirection.
#[derive(Clone)]
pub struct IndexOwned {
    colors: Arc<Colors>,
    index: Arc<Index>,
}

impl Deref for IndexOwned {
    type Target = Index;

    fn deref(&self) -> &Self::Target { &self.index }
}

/// A shared reference to indexed data.
///
/// This is copyable and can be used from either style of index.
#[derive(Clone, Copy)]
pub struct IndexRef<'a> {
    pub colors: &'a Colors,
    pub index: &'a Index,
}

impl<'a> Deref for IndexRef<'a> {
    type Target = Index;

    fn deref(&self) -> &Self::Target { &self.index }
}

impl Index {
    /// NOTE: Panics if the color manifest cannot be loaded.
    pub fn new(seed: u32) -> (Self, Arc<Colors>) {
        let mut indicator = ReloadIndicator::new();
        let colors = Ron::<Colors>::load_watched(WORLD_COLORS_MANIFEST, &mut indicator)
            .expect("Could not load world colors!");

        (
            Self {
                seed,
                time: 0.0,
                noise: Noise::new(seed),
                sites: Store::default(),
                indicator,
            },
            colors,
        )
    }
}

impl IndexOwned {
    pub fn new(index: Index, colors: Arc<Colors>) -> Self {
        Self {
            index: Arc::new(index),
            colors,
        }
    }

    /// NOTE: Callback is called only when colors actually have to be reloaded.
    /// The server is responsible for making sure that all affected chunks are
    /// reloaded; a naive approach will just regenerate every chunk on the
    /// server, but it is possible that eventually we can find a better
    /// solution.
    ///
    /// Ideally, this should be called about once per tick.
    pub fn reload_colors_if_changed<R>(
        &mut self,
        reload: impl FnOnce(&mut Self) -> R,
    ) -> Option<R> {
        self.indicator.reloaded().then(move || {
            // We know the asset was loaded before, so load_expect should be fine.
            self.colors = Ron::<Colors>::load_expect(WORLD_COLORS_MANIFEST);
            reload(self)
        })
    }

    pub fn as_index_ref(&self) -> IndexRef {
        IndexRef {
            colors: &self.colors,
            index: &self.index,
        }
    }
}

pub struct Noise {
    pub cave_nz: SuperSimplex,
    pub scatter_nz: SuperSimplex,
}

impl Noise {
    #[allow(clippy::identity_op)]
    fn new(seed: u32) -> Self {
        Self {
            cave_nz: SuperSimplex::new().set_seed(seed + 0),
            scatter_nz: SuperSimplex::new().set_seed(seed + 1),
        }
    }
}
