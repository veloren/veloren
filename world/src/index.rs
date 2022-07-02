use crate::{
    layer::wildlife::{self, DensityFn, SpawnEntry},
    site::{economy::TradeInformation, Site},
    Colors, Features,
};
use common::{
    assets::{AssetExt, AssetHandle},
    store::Store,
    trade::{SiteId, SitePrices},
};
use core::ops::Deref;
use noise::{Fbm, Seedable, SuperSimplex};
use std::sync::Arc;

const WORLD_COLORS_MANIFEST: &str = "world.style.colors";
const WORLD_FEATURES_MANIFEST: &str = "world.features";

pub struct Index {
    pub seed: u32,
    pub time: f32,
    pub noise: Noise,
    pub sites: Store<Site>,
    pub trade: TradeInformation,
    pub wildlife_spawns: Vec<(AssetHandle<SpawnEntry>, DensityFn)>,
    colors: AssetHandle<Arc<Colors>>,
    features: AssetHandle<Arc<Features>>,
}

/// An owned reference to indexed data.
///
/// The data are split out so that we can replace the colors without disturbing
/// the rest of the index, while also keeping all the data within a single
/// indirection.
#[derive(Clone)]
pub struct IndexOwned {
    colors: Arc<Colors>,
    features: Arc<Features>,
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
    pub features: &'a Features,
    pub index: &'a Index,
}

impl<'a> Deref for IndexRef<'a> {
    type Target = Index;

    fn deref(&self) -> &Self::Target { self.index }
}

impl Index {
    /// NOTE: Panics if the color manifest cannot be loaded.
    pub fn new(seed: u32) -> Self {
        let colors = Arc::<Colors>::load_expect(WORLD_COLORS_MANIFEST);
        let features = Arc::<Features>::load_expect(WORLD_FEATURES_MANIFEST);
        let wildlife_spawns = wildlife::spawn_manifest()
            .into_iter()
            .map(|(e, f)| (SpawnEntry::load_expect(e), f))
            .collect();

        Self {
            seed,
            time: 0.0,
            noise: Noise::new(seed),
            sites: Store::default(),
            trade: Default::default(),
            wildlife_spawns,
            colors,
            features,
        }
    }

    pub fn colors(&self) -> impl Deref<Target = Arc<Colors>> + '_ { self.colors.read() }

    pub fn features(&self) -> impl Deref<Target = Arc<Features>> + '_ { self.features.read() }

    pub fn get_site_prices(&self, site_id: SiteId) -> Option<SitePrices> {
        self.sites
            .recreate_id(site_id)
            .map(|i| self.sites.get(i))
            .map(|s| s.economy.get_site_prices())
    }
}

impl IndexOwned {
    pub fn new(index: Index) -> Self {
        let colors = index.colors.cloned();
        let features = index.features.cloned();

        Self {
            index: Arc::new(index),
            colors,
            features,
        }
    }

    /// NOTE: Callback is called only when colors actually have to be reloaded.
    /// The server is responsible for making sure that all affected chunks are
    /// reloaded; a naive approach will just regenerate every chunk on the
    /// server, but it is possible that eventually we can find a better
    /// solution.
    ///
    /// Ideally, this should be called about once per tick.
    pub fn reload_if_changed<R>(&mut self, reload: impl FnOnce(&mut Self) -> R) -> Option<R> {
        let reloaded = self.index.colors.reloaded_global() || self.index.features.reloaded_global();
        reloaded.then(move || {
            // Reload the fields from the asset handle, which is updated automatically
            self.colors = self.index.colors.cloned();
            self.features = self.index.features.cloned();
            // Update wildlife spawns which is based on base_density in features
            reload(self)
        })
    }

    pub fn as_index_ref(&self) -> IndexRef {
        IndexRef {
            colors: &self.colors,
            features: &self.features,
            index: &self.index,
        }
    }
}

pub struct Noise {
    pub cave_nz: SuperSimplex,
    pub scatter_nz: SuperSimplex,
    pub cave_fbm_nz: Fbm,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self {
            cave_nz: SuperSimplex::new().set_seed(seed + 0),
            scatter_nz: SuperSimplex::new().set_seed(seed + 1),
            cave_fbm_nz: Fbm::new().set_seed(seed + 2),
        }
    }
}
