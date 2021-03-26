use crate::{
    site::{economy::TradeInformation, Site},
    Colors,
};
use common::{
    assets::{AssetExt, AssetHandle},
    comp::Agent,
    store::Store,
    trade::SitePrices,
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
    pub trade: TradeInformation,
    colors: AssetHandle<Arc<Colors>>,
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
    pub fn new(seed: u32) -> Self {
        let colors = Arc::<Colors>::load_expect(WORLD_COLORS_MANIFEST);

        Self {
            seed,
            time: 0.0,
            noise: Noise::new(seed),
            sites: Store::default(),
            trade: Default::default(),
            colors,
        }
    }

    pub fn colors(&self) -> AssetHandle<Arc<Colors>> { self.colors }

    pub fn get_site_prices(&self, agent: &Agent) -> Option<SitePrices> {
        agent
            .trade_for_site
            .map(|i| self.sites.recreate_id(i))
            .flatten()
            .map(|i| self.sites.get(i))
            .map(|s| s.economy.get_site_prices())
    }
}

impl IndexOwned {
    pub fn new(index: Index) -> Self {
        let colors = index.colors.cloned();

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
        self.index.colors.reloaded_global().then(move || {
            // Reload the color from the asset handle, which is updated automatically
            self.colors = self.index.colors.cloned();
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
