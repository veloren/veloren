mod gen;
pub mod genstat;
pub mod plot;
mod tile;
pub mod util;

use self::tile::{HazardKind, KeepKind, RoofKind, TILE_SIZE, Tile, TileGrid};
pub use self::{
    gen::{Fill, Painter, Primitive, PrimitiveRef, Structure, aabr_with_z},
    genstat::{GenStatPlotKind, GenStatSiteKind, SitesGenMeta},
    plot::{Plot, PlotKind, foreach_plot},
    tile::TileKind,
    util::Dir,
};
use crate::{
    Canvas, IndexRef, Land,
    config::CONFIG,
    sim::Path,
    site::{SpawnRules, namegen::NameGen},
    util::{CARDINALS, DHashSet, Grid, SQUARE_4, SQUARE_9, attempt},
};
use common::{
    astar::Astar,
    calendar::Calendar,
    comp::Alignment,
    generation::EntityInfo,
    lottery::Lottery,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::{Block, BlockKind, SpriteKind, TerrainChunkSize},
    vol::RectVolSize,
};
use hashbrown::DefaultHashBuilder;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::ops::Range;
use vek::*;

/// Seed a new RNG from an old RNG, thereby making the old RNG indepedent of
/// changing use of the new RNG. The practical effect of this is to reduce the
/// extent to which changes to child generation algorithm produce a 'butterfly
/// effect' on their parent generators, meaning that generators will be less
/// likely to produce entirely different outcomes if some detail of a generation
/// algorithm changes slightly. This is generally good and makes worldgen code
/// easier to maintain and less liable to breaking changes.
fn reseed(rng: &mut impl Rng) -> impl Rng { ChaChaRng::from_seed(rng.gen::<[u8; 32]>()) }

#[derive(Default)]
pub struct Site {
    pub origin: Vec2<i32>,
    name: String,
    // NOTE: Do we want these to be public?
    pub tiles: TileGrid,
    pub plots: Store<Plot>,
    pub plazas: Vec<Id<Plot>>,
    pub roads: Vec<Id<Plot>>,
}

impl Site {
    pub fn radius(&self) -> f32 {
        ((self
            .tiles
            .bounds
            .min
            .map(|e| e.abs())
            .reduce_max()
            .max(self.tiles.bounds.max.map(|e| e.abs()).reduce_max())
            // Temporary solution for giving giant_tree's leaves enough space to be painted correctly
            // TODO: This will have to be replaced by a system as described on discord :
            // https://discord.com/channels/449602562165833758/450064928720814081/937044837461536808
            + if self
                .plots
                .values()
                .any(|p| matches!(&p.kind, PlotKind::GiantTree(_)))
            {
                // 25 Seems to be big enough for the current scale of 4.0
                25
            } else {
                5
            })
            * TILE_SIZE as i32) as f32
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        let tile_pos = self.wpos_tile_pos(wpos);
        let max_warp = SQUARE_9
            .iter()
            .filter_map(|rpos| {
                let tile_pos = tile_pos + rpos;
                if self.tiles.get(tile_pos).is_natural() {
                    None
                } else {
                    let clamped =
                        wpos.clamped(self.tile_wpos(tile_pos), self.tile_wpos(tile_pos + 1) - 1);
                    Some(clamped.distance_squared(wpos) as f32)
                }
            })
            .min_by_key(|d2| *d2 as i32)
            .map(|d2| d2.sqrt() / TILE_SIZE as f32)
            .unwrap_or(1.0);
        let base_spawn_rules = SpawnRules {
            trees: max_warp == 1.0,
            max_warp,
            paths: max_warp > f32::EPSILON,
            waypoints: true,
        };
        self.plots
            .values()
            .filter_map(|plot| match &plot.kind {
                PlotKind::Gnarling(g) => Some(g.spawn_rules(wpos)),
                PlotKind::Adlet(ad) => Some(ad.spawn_rules(wpos)),
                PlotKind::SeaChapel(p) => Some(p.spawn_rules(wpos)),
                PlotKind::Haniwa(ha) => Some(ha.spawn_rules(wpos)),
                PlotKind::TerracottaPalace(tp) => Some(tp.spawn_rules(wpos)),
                PlotKind::TerracottaHouse(th) => Some(th.spawn_rules(wpos)),
                PlotKind::TerracottaYard(ty) => Some(ty.spawn_rules(wpos)),
                PlotKind::Cultist(cl) => Some(cl.spawn_rules(wpos)),
                PlotKind::Sahagin(sg) => Some(sg.spawn_rules(wpos)),
                PlotKind::DwarvenMine(dm) => Some(dm.spawn_rules(wpos)),
                PlotKind::VampireCastle(vc) => Some(vc.spawn_rules(wpos)),
                PlotKind::MyrmidonArena(ma) => Some(ma.spawn_rules(wpos)),
                PlotKind::MyrmidonHouse(mh) => Some(mh.spawn_rules(wpos)),
                _ => None,
            })
            .fold(base_spawn_rules, |a, b| a.combine(b))
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let border = 1;
        Aabr {
            min: self.tile_wpos(self.tiles.bounds.min - border),
            max: self.tile_wpos(self.tiles.bounds.max + 1 + border),
        }
    }

    pub fn plot(&self, id: Id<Plot>) -> &Plot { &self.plots[id] }

    pub fn plots(&self) -> impl ExactSizeIterator<Item = &Plot> + '_ { self.plots.values() }

    pub fn plazas(&self) -> impl ExactSizeIterator<Item = Id<Plot>> + '_ {
        self.plazas.iter().copied()
    }

    pub fn create_plot(&mut self, plot: Plot) -> Id<Plot> { self.plots.insert(plot) }

    pub fn blit_aabr(&mut self, aabr: Aabr<i32>, tile: Tile) {
        for y in 0..aabr.size().h {
            for x in 0..aabr.size().w {
                self.tiles.set(aabr.min + Vec2::new(x, y), tile.clone());
            }
        }
    }

    pub fn create_road(
        &mut self,
        land: &Land,
        rng: &mut impl Rng,
        a: Vec2<i32>,
        b: Vec2<i32>,
        w: u16,
    ) -> Option<Id<Plot>> {
        const MAX_ITERS: usize = 4096;
        let range = &(-(w as i32) / 2..w as i32 - (w as i32 + 1) / 2);
        // Manhattan distance.
        let heuristic =
            |(tile, _): &(Vec2<i32>, Vec2<i32>)| (tile - b).map(|e| e.abs()).sum() as f32;
        let (path, _cost) = Astar::new(MAX_ITERS, (a, Vec2::zero()), DefaultHashBuilder::default())
            .poll(
                MAX_ITERS,
                &heuristic,
                |(tile, prev_dir)| {
                    let tile = *tile;
                    let prev_dir = *prev_dir;
                    let this = &self;
                    CARDINALS.iter().map(move |dir| {
                        let neighbor = (tile + *dir, *dir);

                        // Transition cost
                        let alt_a = land.get_alt_approx(this.tile_center_wpos(tile));
                        let alt_b = land.get_alt_approx(this.tile_center_wpos(neighbor.0));
                        let mut cost = 1.0
                            + (alt_a - alt_b).abs() / TILE_SIZE as f32
                            + (prev_dir != *dir) as i32 as f32;

                        for i in range.clone() {
                            let orth = dir.yx() * i;
                            let tile = this.tiles.get(neighbor.0 + orth);
                            if tile.is_obstacle() {
                                cost += 1000.0;
                            } else if !tile.is_empty() && !tile.is_road() {
                                cost += 25.0;
                            }
                        }

                        (neighbor, cost)
                    })
                },
                |(tile, _)| *tile == b,
            )
            .into_path()?;

        let plot = self.create_plot(Plot {
            kind: PlotKind::Road(path.iter().map(|(tile, _)| *tile).collect()),
            root_tile: a,
            tiles: path.iter().map(|(tile, _)| *tile).collect(),
            seed: rng.gen(),
        });

        self.roads.push(plot);

        for (i, (tile, _)) in path.iter().enumerate() {
            for y in range.clone() {
                for x in range.clone() {
                    let tile = tile + Vec2::new(x, y);
                    if self.tiles.get(tile).is_empty() {
                        self.tiles.set(tile, Tile {
                            kind: TileKind::Road {
                                a: i.saturating_sub(1) as u16,
                                b: (i + 1).min(path.len() - 1) as u16,
                                w,
                            },
                            plot: Some(plot),
                            hard_alt: Some(land.get_alt_approx(self.tile_center_wpos(tile)) as i32),
                        });
                    }
                }
            }
        }

        Some(plot)
    }

    pub fn find_aabr(
        &mut self,
        search_pos: Vec2<i32>,
        area_range: Range<u32>,
        min_dims: Extent2<u32>,
    ) -> Option<(Aabr<i32>, Vec2<i32>, Vec2<i32>)> {
        let ((aabr, door_dir), door_pos) = self.tiles.find_near(search_pos, |center, _| {
            let dir = CARDINALS
                .iter()
                .find(|dir| self.tiles.get(center + *dir).is_road())?;
            self.tiles
                .grow_aabr(center, area_range.clone(), min_dims)
                .ok()
                .zip(Some(*dir))
        })?;
        Some((aabr, door_pos, door_dir))
    }

    pub fn find_roadside_aabr(
        &mut self,
        rng: &mut impl Rng,
        area_range: Range<u32>,
        min_dims: Extent2<u32>,
    ) -> Option<(Aabr<i32>, Vec2<i32>, Vec2<i32>)> {
        let dir = Vec2::<f32>::zero()
            .map(|_| rng.gen_range(-1.0..1.0))
            .normalized();
        let search_pos = if rng.gen() {
            let plaza = self.plot(*self.plazas.choose(rng)?);
            let sz = plaza.find_bounds().size();
            plaza.root_tile + dir.map(|e: f32| e.round() as i32) * (sz + 1)
        } else if let PlotKind::Road(path) = &self.plot(*self.roads.choose(rng)?).kind {
            *path.nodes().choose(rng)? + (dir * 1.0).map(|e: f32| e.round() as i32)
        } else {
            unreachable!()
        };

        self.find_aabr(search_pos, area_range, min_dims)
    }

    pub fn find_rural_aabr(
        &mut self,
        rng: &mut impl Rng,
        area_range: Range<u32>,
        min_dims: Extent2<u32>,
    ) -> Option<(Aabr<i32>, Vec2<i32>, Vec2<i32>)> {
        let dir = Vec2::<f32>::zero()
            .map(|_| rng.gen_range(-1.0..1.0))
            .normalized();

        // go from the site origin (0,0) at a random angle, as far as possible (up to
        // the site radius / 6 because sites have ridiculously big radii like 160-600)
        let search_pos = dir.map(|e: f32| e.round() as i32) * ((self.radius() / 6.0) as i32);

        self.find_aabr(search_pos, area_range, min_dims)
    }

    pub fn make_plaza_at(
        &mut self,
        land: &Land,
        pos: &Vec2<i32>,
        radius: i32,
        rng: &mut impl Rng,
    ) -> Option<Id<Plot>> {
        let plaza_alt = land.get_alt_approx(self.tile_center_wpos(*pos)) as i32;

        let aabr = Aabr {
            min: pos + Vec2::broadcast(-radius),
            max: pos + Vec2::broadcast(radius + 1),
        };
        let plaza = self.create_plot(Plot {
            kind: PlotKind::Plaza,
            root_tile: *pos,
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });
        self.plazas.push(plaza);
        self.blit_aabr(aabr, Tile {
            kind: TileKind::Plaza,
            plot: Some(plaza),
            hard_alt: Some(plaza_alt),
        });

        let mut already_pathed = vec![];
        // One major, one minor road
        for _ in (0..rng.gen_range(1.25..2.25) as u16).rev() {
            if let Some(&p) = self
                .plazas
                .iter()
                .filter(|&&p| {
                    !already_pathed.contains(&p)
                        && p != plaza
                        && already_pathed.iter().all(|&ap| {
                            (self.plot(ap).root_tile - pos)
                                .map(|e| e as f32)
                                .normalized()
                                .dot(
                                    (self.plot(p).root_tile - pos)
                                        .map(|e| e as f32)
                                        .normalized(),
                                )
                                < 0.0
                        })
                })
                .min_by_key(|&&p| self.plot(p).root_tile.distance_squared(*pos))
            {
                self.create_road(land, rng, self.plot(p).root_tile, *pos, 2 /* + i */);
                already_pathed.push(p);
            } else {
                break;
            }
        }

        Some(plaza)
    }

    pub fn make_plaza(
        &mut self,
        land: &Land,
        rng: &mut impl Rng,
        generator_stats: &mut SitesGenMeta,
        site_name: &String,
    ) -> Option<Id<Plot>> {
        generator_stats.attempt(site_name, GenStatPlotKind::Plaza);
        let plaza_radius = rng.gen_range(1..4);
        let plaza_dist = 6.5 + plaza_radius as f32 * 4.0;
        let pos = attempt(32, || {
            self.plazas
                .choose(rng)
                .map(|&p| {
                    self.plot(p).root_tile
                        + (Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0))
                            .normalized()
                            * plaza_dist)
                            .map(|e| e as i32)
                })
                .or_else(|| Some(Vec2::zero()))
                .filter(|tile| !self.tiles.get(*tile).is_obstacle())
                .filter(|&tile| {
                    self.plazas.iter().all(|&p| {
                        self.plot(p).root_tile.distance_squared(tile) as f32
                            > (plaza_dist * 0.85).powi(2)
                    }) && rng.gen_range(0..48) > tile.map(|e| e.abs()).reduce_max()
                })
        })?;
        generator_stats.success(site_name, GenStatPlotKind::Plaza);
        self.make_plaza_at(land, &pos, plaza_radius, rng)
    }

    pub fn demarcate_obstacles(&mut self, land: &Land) {
        const SEARCH_RADIUS: u32 = 96;

        Spiral2d::new()
            .take((SEARCH_RADIUS * 2 + 1).pow(2) as usize)
            .for_each(|tile| {
                let wpos = self.tile_center_wpos(tile);
                if let Some(kind) = Spiral2d::new()
                    .take(9)
                    .find_map(|rpos| wpos_is_hazard(land, wpos + rpos))
                {
                    for &rpos in &SQUARE_4 {
                        // `get_mut` doesn't increase generation bounds
                        self.tiles
                            .get_mut(tile - rpos - 1)
                            .map(|tile| tile.kind = TileKind::Hazard(kind));
                    }
                }
                if let Some((_, path_wpos, Path { width }, _)) = land.get_nearest_path(wpos) {
                    let tile_aabb = Aabr {
                        min: self.tile_wpos(tile),
                        max: self.tile_wpos(tile + 1) - 1,
                    };

                    if (tile_aabb
                        .projected_point(path_wpos.as_())
                        .distance_squared(path_wpos.as_()) as f32)
                        < width.powi(2)
                    {
                        self.tiles
                            .get_mut(tile)
                            .map(|tile| tile.kind = TileKind::Path);
                    }
                }
            });
    }

    /// The find_roadside_aabr function wants to have an existing plaza or road.
    /// This function is used to find a suitable location for the first plaza in
    /// a town, which has the side-effect of creating at least one road.
    /// This function is more expensive than the make_plaza function but
    /// fails to find a plaza location only if there are no suitable
    /// locations within the entire search radius.
    ///
    /// It works by exhaustively finding all tiles within a ring pattern around
    /// the town center where the tile and all surrounding tiles to the
    /// plaza radius are not hazards or roads. It then chooses the tile with
    /// the minimum distance from the town center as the plaza location. See
    /// the comments in common/src/spiral.rs for more information on the spiral
    /// ring pattern.
    ///
    /// demarcate_obstacles() should be called before this function to mark the
    /// obstacles and roads. (Cliff Towns are an exception).
    pub fn make_initial_plaza(
        &mut self,
        land: &Land,
        rng: &mut impl Rng,
        plaza_radius: u32,
        search_inner_radius: u32,
        search_width: u32,
        generator_stats: &mut SitesGenMeta,
        site_name: &String,
    ) -> Option<Id<Plot>> {
        generator_stats.attempt(site_name, GenStatPlotKind::InitialPlaza);
        // Find all the suitable locations for a plaza.
        let mut plaza_locations = vec![];
        // Search over a spiral ring pattern
        Spiral2d::with_ring(search_inner_radius, search_width).for_each(|tile| {
            // if the tile is not a hazard or road
            if self.tiles.get_known(tile).is_none() {
                // if all the tiles in the proposed plaza location are also not hazards or roads
                // then add the tile as a candidate for a plaza location
                if Spiral2d::new()
                    .take((plaza_radius * 2 + 1).pow(2) as usize)
                    .all(|rpos| self.tiles.get_known(rpos + tile).is_none())
                {
                    plaza_locations.push(tile);
                }
            }
        });
        if plaza_locations.is_empty() {
            // No suitable plaza locations were found, it's unlikely that the town will be
            // able to be generated, but we can try to make a plaza anyway with
            // the original make_plaza function.
            self.make_plaza(land, rng, generator_stats, site_name)
        } else {
            // Choose the minimum distance from the town center.
            plaza_locations.sort_by_key(|&pos| pos.distance_squared(Vec2::zero()));
            // use the first plaza location as the plaza position
            let pos = plaza_locations.first()?;
            generator_stats.success(site_name, GenStatPlotKind::InitialPlaza);
            self.make_plaza_at(land, pos, plaza_radius as i32, rng)
        }
    }

    /// This is make_initial_plaza with default options/parameters. This calls
    /// make_initial_plaza with the default parameters for the plaza_radius
    /// and search_inner_radius. The plaza_radius will be in the range 1-3,
    /// and the search_inner_radius will be 7 + plaza_radius. The search_width
    /// will be PLAZA_MAX_SEARCH_RADIUS - search_inner_radius. The
    /// search_inner_radius is approximately the same distance
    /// from the center of town as for the original make_plaza function, so this
    /// function will place the initial plaza and roads near where the
    /// original make_plaza function would place them in the case where the site
    /// is clear of hazards.
    ///
    /// This default plaza generation function is used for generating cities,
    /// cliff towns, savannah towns, and coastal towns. The other town types
    /// (terracotta, myrmidon, desert city) have a central feature so they
    /// use specific plaza generation parameters and call the make_initial_plaza
    /// function directly.
    ///
    /// demarcate_obstacles() should be called before this function to mark the
    /// obstacles and roads.
    pub fn make_initial_plaza_default(
        &mut self,
        land: &Land,
        rng: &mut impl Rng,
        generator_stats: &mut SitesGenMeta,
        site_name: &String,
    ) -> Option<Id<Plot>> {
        // The plaza radius can be 1, 2, or 3.
        let plaza_radius = rng.gen_range(1..4);
        // look for plaza locations within a ring with an outer dimension
        // of 24 tiles and an inner dimension that will offset the plaza from the town
        // center.
        let search_inner_radius = 7 + plaza_radius;
        const PLAZA_MAX_SEARCH_RADIUS: u32 = 24;
        self.make_initial_plaza(
            land,
            rng,
            plaza_radius,
            search_inner_radius,
            PLAZA_MAX_SEARCH_RADIUS - search_inner_radius,
            generator_stats,
            site_name,
        )
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn generate_mine(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };

        let size = 60.0;

        let aabr = Aabr {
            min: Vec2::broadcast(-size as i32),
            max: Vec2::broadcast(size as i32),
        };

        let wpos: Vec2<i32> = [1, 2].into();

        let dwarven_mine =
            plot::DwarvenMine::generate(land, &mut reseed(&mut rng), &site, wpos, aabr);
        site.name = dwarven_mine.name().to_string();
        let plot = site.create_plot(Plot {
            kind: PlotKind::DwarvenMine(dwarven_mine),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });

        site.blit_aabr(aabr, Tile {
            kind: TileKind::Empty,
            plot: Some(plot),
            hard_alt: Some(1_i32),
        });

        site
    }

    pub fn generate_citadel(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };
        site.demarcate_obstacles(land);
        let citadel = plot::Citadel::generate(origin, land, &mut rng);
        site.name = citadel.name().to_string();
        let size = citadel.radius() / tile::TILE_SIZE as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        let plot = site.create_plot(Plot {
            kind: PlotKind::Citadel(citadel),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });
        site.blit_aabr(aabr, Tile {
            kind: TileKind::Building,
            plot: Some(plot),
            hard_alt: None,
        });
        site
    }

    pub fn generate_gnarling(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };
        site.demarcate_obstacles(land);
        let gnarling_fortification = plot::GnarlingFortification::generate(origin, land, &mut rng);
        site.name = gnarling_fortification.name().to_string();
        let size = gnarling_fortification.radius() / TILE_SIZE as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        let plot = site.create_plot(Plot {
            kind: PlotKind::Gnarling(gnarling_fortification),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });
        site.blit_aabr(aabr, Tile {
            kind: TileKind::GnarlingFortification,
            plot: Some(plot),
            hard_alt: None,
        });
        site
    }

    pub fn generate_adlet(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        index: IndexRef,
    ) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };
        site.demarcate_obstacles(land);
        let adlet_stronghold = plot::AdletStronghold::generate(origin, land, &mut rng, index);
        site.name = adlet_stronghold.name().to_string();
        let (cavern_aabr, wall_aabr) = adlet_stronghold.plot_tiles(origin);
        let plot = site.create_plot(Plot {
            kind: PlotKind::Adlet(adlet_stronghold),
            root_tile: cavern_aabr.center(),
            tiles: aabr_tiles(cavern_aabr)
                .chain(aabr_tiles(wall_aabr))
                .collect(),
            seed: rng.gen(),
        });
        site.blit_aabr(cavern_aabr, Tile {
            kind: TileKind::AdletStronghold,
            plot: Some(plot),
            hard_alt: None,
        });
        site.blit_aabr(wall_aabr, Tile {
            kind: TileKind::AdletStronghold,
            plot: Some(plot),
            hard_alt: None,
        });
        site
    }

    pub fn generate_terracotta(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let gen_name = NameGen::location(&mut rng).generate_terracotta();
        let suffix = [
            "Tombs",
            "Necropolis",
            "Ruins",
            "Mausoleum",
            "Cemetery",
            "Burial Grounds",
            "Remains",
            "Temples",
            "Gardens",
        ]
        .choose(&mut rng)
        .unwrap();
        let name = match rng.gen_range(0..2) {
            0 => format!("{} {}", gen_name, suffix),
            _ => format!("{} of {}", suffix, gen_name),
        };
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        // The terracotta_palace is 15 tiles in radius, so the plaza should be outside
        // the palace.
        const TERRACOTTA_PLAZA_RADIUS: u32 = 3;
        const TERRACOTTA_PLAZA_SEARCH_INNER: u32 = 17;
        const TERRACOTTA_PLAZA_SEARCH_WIDTH: u32 = 12;
        generator_stats.add(&site.name, GenStatSiteKind::Terracotta);
        site.make_initial_plaza(
            land,
            &mut rng,
            TERRACOTTA_PLAZA_RADIUS,
            TERRACOTTA_PLAZA_SEARCH_INNER,
            TERRACOTTA_PLAZA_SEARCH_WIDTH,
            generator_stats,
            &name,
        );

        let size = 15.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let terracotta_palace =
                plot::TerracottaPalace::generate(land, &mut reseed(&mut rng), &site, aabr);
            let terracotta_palace_alt = terracotta_palace.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::TerracottaPalace(terracotta_palace),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(terracotta_palace_alt),
            });
        }
        let build_chance = Lottery::from(vec![(12.0, 1), (4.0, 2)]);
        for _ in 0..16 {
            match *build_chance.choose_seeded(rng.gen()) {
                1 => {
                    // TerracottaHouse
                    generator_stats.attempt(&site.name, GenStatPlotKind::House);
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    if let Some((aabr, _, _)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            9..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let terracotta_house = plot::TerracottaHouse::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            aabr,
                        );
                        let terracotta_house_alt = terracotta_house.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::TerracottaHouse(terracotta_house),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(terracotta_house_alt),
                        });

                        generator_stats.success(&site.name, GenStatPlotKind::House);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },

                2 => {
                    // TerracottaYard
                    generator_stats.attempt(&site.name, GenStatPlotKind::Yard);
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    if let Some((aabr, _, _)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            9..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let terracotta_yard = plot::TerracottaYard::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            aabr,
                        );
                        let terracotta_yard_alt = terracotta_yard.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::TerracottaYard(terracotta_yard),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(terracotta_yard_alt),
                        });

                        generator_stats.success(&site.name, GenStatPlotKind::Yard);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                _ => {},
            }
        }
        site
    }

    pub fn generate_myrmidon(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let gen_name = NameGen::location(&mut rng).generate_danari();
        let suffix = ["City", "Metropolis"].choose(&mut rng).unwrap();
        let name = match rng.gen_range(0..2) {
            0 => format!("{} {}", gen_name, suffix),
            _ => format!("{} of {}", suffix, gen_name),
        };
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        // The myrmidon_arena is 16 tiles in radius, so the plaza should be outside the
        // palace.
        const MYRMIDON_PLAZA_RADIUS: u32 = 3;
        const MYRMIDON_PLAZA_SEARCH_INNER: u32 = 18;
        const MYRMIDON_PLAZA_SEARCH_WIDTH: u32 = 12;
        generator_stats.add(&site.name, GenStatSiteKind::Myrmidon);
        generator_stats.attempt(&site.name, GenStatPlotKind::InitialPlaza);
        site.make_initial_plaza(
            land,
            &mut rng,
            MYRMIDON_PLAZA_RADIUS,
            MYRMIDON_PLAZA_SEARCH_INNER,
            MYRMIDON_PLAZA_SEARCH_WIDTH,
            generator_stats,
            &name,
        );

        let size = 16.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let myrmidon_arena =
                plot::MyrmidonArena::generate(land, &mut reseed(&mut rng), &site, aabr);
            let myrmidon_arena_alt = myrmidon_arena.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::MyrmidonArena(myrmidon_arena),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(myrmidon_arena_alt),
            });
        }
        for _ in 0..30 {
            // MyrmidonHouse
            generator_stats.attempt(&site.name, GenStatPlotKind::House);
            let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
            if let Some((aabr, _, _)) = attempt(32, || {
                site.find_roadside_aabr(&mut rng, 9..(size + 1).pow(2), Extent2::broadcast(size))
            }) {
                let myrmidon_house =
                    plot::MyrmidonHouse::generate(land, &mut reseed(&mut rng), &site, aabr);
                let myrmidon_house_alt = myrmidon_house.alt;
                let plot = site.create_plot(Plot {
                    kind: PlotKind::MyrmidonHouse(myrmidon_house),
                    root_tile: aabr.center(),
                    tiles: aabr_tiles(aabr).collect(),
                    seed: rng.gen(),
                });

                site.blit_aabr(aabr, Tile {
                    kind: TileKind::Building,
                    plot: Some(plot),
                    hard_alt: Some(myrmidon_house_alt),
                });

                generator_stats.success(&site.name, GenStatPlotKind::House);
            } else {
                site.make_plaza(land, &mut rng, generator_stats, &name);
            }
        }

        site
    }

    pub fn generate_giant_tree(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };
        site.demarcate_obstacles(land);
        let giant_tree = plot::GiantTree::generate(&site, Vec2::zero(), land, &mut rng);
        site.name = giant_tree.name().to_string();
        let size = (giant_tree.radius() / TILE_SIZE as f32).ceil() as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size) + 1,
        };
        let plot = site.create_plot(Plot {
            kind: PlotKind::GiantTree(giant_tree),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });
        site.blit_aabr(aabr, Tile {
            kind: TileKind::Building,
            plot: Some(plot),
            hard_alt: None,
        });
        site
    }

    // Size is 0..1
    pub fn generate_city(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        size: f32,
        calendar: Option<&Calendar>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let name = NameGen::location(&mut rng).generate_town();
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        generator_stats.add(&site.name, GenStatSiteKind::City);
        site.make_initial_plaza_default(land, &mut rng, generator_stats, &name);

        let build_chance = Lottery::from(vec![
            (64.0, 1), // house
            (5.0, 2),  // guard tower
            (15.0, 3), // field
            (5.0, 4),  // castle
            (5.0, 5),  // workshop
            (15.0, 6), // airship dock
            (15.0, 7), // tavern
            (5.0, 8),  // barn
        ]);

        // These plots have minimums or limits.
        let mut workshops = 0;
        let mut castles = 0;
        let mut taverns = 0;
        let mut airship_docks = 0;

        for _ in 0..(size * 200.0) as i32 {
            match *build_chance.choose_seeded(rng.gen()) {
                // Workshop
                n if (n == 5 && workshops < (size * 5.0) as i32) || workshops == 0 => {
                    generator_stats.attempt(&site.name, GenStatPlotKind::Workshop);
                    let size = (3.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            4..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let workshop = plot::Workshop::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let workshop_alt = workshop.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::Workshop(workshop),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(workshop_alt),
                        });
                        workshops += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Workshop);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                // House
                1 => {
                    let size = (1.5 + rng.gen::<f32>().powf(5.0) * 1.0).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::House);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            4..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let house = plot::House::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                            calendar,
                        );
                        let house_alt = house.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::House(house),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(house_alt),
                        });
                        generator_stats.success(&site.name, GenStatPlotKind::House);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                // Guard tower
                2 => {
                    generator_stats.attempt(&site.name, GenStatPlotKind::GuardTower);
                    if let Some((_aabr, _, _door_dir)) = attempt(10, || {
                        site.find_roadside_aabr(&mut rng, 4..4, Extent2::new(2, 2))
                    }) {
                        // let plot = site.create_plot(Plot {
                        //     kind: PlotKind::Castle(plot::Castle::generate(
                        //         land,
                        //         &mut rng,
                        //         &site,
                        //         aabr,
                        //     )),
                        //     root_tile: aabr.center(),
                        //     tiles: aabr_tiles(aabr).collect(),
                        //     seed: rng.gen(),
                        // });

                        // site.blit_aabr(aabr, Tile {
                        //     kind: TileKind::Castle,
                        //     plot: Some(plot),
                        //     hard_alt: None,
                        // });
                    }
                },
                // Field
                3 => {
                    Self::generate_farm(false, &mut rng, &mut site, land);
                },
                // Castle
                4 if castles < 1 => {
                    generator_stats.attempt(&site.name, GenStatPlotKind::Castle);
                    if let Some((aabr, _entrance_tile, _door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(&mut rng, 16 * 16..18 * 18, Extent2::new(16, 16))
                    }) {
                        let offset = rng.gen_range(5..(aabr.size().w.min(aabr.size().h) - 4));
                        let gate_aabr = Aabr {
                            min: Vec2::new(aabr.min.x + offset - 1, aabr.min.y),
                            max: Vec2::new(aabr.min.x + offset + 2, aabr.min.y + 1),
                        };
                        let castle = plot::Castle::generate(land, &mut rng, &site, aabr, gate_aabr);
                        let castle_alt = castle.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::Castle(castle),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        let wall_north = Tile {
                            kind: TileKind::Wall(Dir::Y),
                            plot: Some(plot),
                            hard_alt: Some(castle_alt),
                        };

                        let wall_east = Tile {
                            kind: TileKind::Wall(Dir::X),
                            plot: Some(plot),
                            hard_alt: Some(castle_alt),
                        };
                        for x in 0..aabr.size().w {
                            site.tiles
                                .set(aabr.min + Vec2::new(x, 0), wall_east.clone());
                            site.tiles.set(
                                aabr.min + Vec2::new(x, aabr.size().h - 1),
                                wall_east.clone(),
                            );
                        }
                        for y in 0..aabr.size().h {
                            site.tiles
                                .set(aabr.min + Vec2::new(0, y), wall_north.clone());
                            site.tiles.set(
                                aabr.min + Vec2::new(aabr.size().w - 1, y),
                                wall_north.clone(),
                            );
                        }

                        let gate = Tile {
                            kind: TileKind::Gate,
                            plot: Some(plot),
                            hard_alt: Some(castle_alt),
                        };
                        let tower_parapet = Tile {
                            kind: TileKind::Tower(RoofKind::Parapet),
                            plot: Some(plot),
                            hard_alt: Some(castle_alt),
                        };
                        let tower_pyramid = Tile {
                            kind: TileKind::Tower(RoofKind::Pyramid),
                            plot: Some(plot),
                            hard_alt: Some(castle_alt),
                        };

                        site.tiles.set(
                            Vec2::new(aabr.min.x + offset - 2, aabr.min.y),
                            tower_parapet.clone(),
                        );
                        site.tiles
                            .set(Vec2::new(aabr.min.x + offset - 1, aabr.min.y), gate.clone());
                        site.tiles
                            .set(Vec2::new(aabr.min.x + offset, aabr.min.y), gate.clone());
                        site.tiles
                            .set(Vec2::new(aabr.min.x + offset + 1, aabr.min.y), gate.clone());
                        site.tiles.set(
                            Vec2::new(aabr.min.x + offset + 2, aabr.min.y),
                            tower_parapet.clone(),
                        );

                        site.tiles
                            .set(Vec2::new(aabr.min.x, aabr.min.y), tower_parapet.clone());
                        site.tiles
                            .set(Vec2::new(aabr.max.x - 1, aabr.min.y), tower_parapet.clone());
                        site.tiles
                            .set(Vec2::new(aabr.min.x, aabr.max.y - 1), tower_parapet.clone());
                        site.tiles.set(
                            Vec2::new(aabr.max.x - 1, aabr.max.y - 1),
                            tower_parapet.clone(),
                        );

                        // Courtyard
                        site.blit_aabr(
                            Aabr {
                                min: aabr.min + 1,
                                max: aabr.max - 1,
                            },
                            Tile {
                                kind: TileKind::Road { a: 0, b: 0, w: 0 },
                                plot: Some(plot),
                                hard_alt: Some(castle_alt),
                            },
                        );

                        // Keep
                        site.blit_aabr(
                            Aabr {
                                min: aabr.center() - 3,
                                max: aabr.center() + 3,
                            },
                            Tile {
                                kind: TileKind::Wall(Dir::Y),
                                plot: Some(plot),
                                hard_alt: Some(castle_alt),
                            },
                        );
                        site.tiles.set(
                            Vec2::new(aabr.center().x + 2, aabr.center().y + 2),
                            tower_pyramid.clone(),
                        );
                        site.tiles.set(
                            Vec2::new(aabr.center().x + 2, aabr.center().y - 3),
                            tower_pyramid.clone(),
                        );
                        site.tiles.set(
                            Vec2::new(aabr.center().x - 3, aabr.center().y + 2),
                            tower_pyramid.clone(),
                        );
                        site.tiles.set(
                            Vec2::new(aabr.center().x - 3, aabr.center().y - 3),
                            tower_pyramid.clone(),
                        );

                        site.blit_aabr(
                            Aabr {
                                min: aabr.center() - 2,
                                max: aabr.center() + 2,
                            },
                            Tile {
                                kind: TileKind::Keep(KeepKind::Middle),
                                plot: Some(plot),
                                hard_alt: Some(castle_alt),
                            },
                        );

                        castles += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Castle);
                    }
                },
                //airship dock
                6 if (size > 0.125 && airship_docks == 0) => {
                    generator_stats.attempt(&site.name, GenStatPlotKind::AirshipDock);
                    if let Some((_aabr, _, _door_dir)) = attempt(10, || {
                        site.find_roadside_aabr(&mut rng, 4..4, Extent2::new(2, 2))
                    }) {
                        let size = 3.0 as u32;
                        if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                            site.find_roadside_aabr(
                                &mut rng,
                                4..(size + 1).pow(2),
                                Extent2::broadcast(size),
                            )
                        }) {
                            let airship_dock = plot::AirshipDock::generate(
                                land,
                                &mut reseed(&mut rng),
                                &site,
                                door_tile,
                                door_dir,
                                aabr,
                            );
                            let airship_dock_alt = airship_dock.alt;
                            let plot = site.create_plot(Plot {
                                kind: PlotKind::AirshipDock(airship_dock),
                                root_tile: aabr.center(),
                                tiles: aabr_tiles(aabr).collect(),
                                seed: rng.gen(),
                            });

                            site.blit_aabr(aabr, Tile {
                                kind: TileKind::Building,
                                plot: Some(plot),
                                hard_alt: Some(airship_dock_alt),
                            });
                            airship_docks += 1;
                            generator_stats.success(&site.name, GenStatPlotKind::AirshipDock);
                        } else {
                            site.make_plaza(land, &mut rng, generator_stats, &name);
                        }
                    }
                },
                7 if (size > 0.125 && taverns < 2) => {
                    generator_stats.attempt(&site.name, GenStatPlotKind::Tavern);
                    let size = (3.5 + rng.gen::<f32>().powf(5.0) * 2.0).round() as u32;
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            7..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let tavern = plot::Tavern::generate(
                            land,
                            index,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            Dir::from_vec2(door_dir),
                            aabr,
                        );
                        let tavern_alt = tavern.door_wpos.z;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::Tavern(tavern),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(tavern_alt),
                        });

                        taverns += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Tavern);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                8 => {
                    Self::generate_barn(false, &mut rng, &mut site, land);
                },
                _ => {},
            }
        }

        site
    }

    pub fn generate_glider_course(
        land: &Land,
        _index: IndexRef,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
    ) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            ..Site::default()
        };

        // TODO use the nearest peak name. Unfortunately this requires `Civs` but we
        // only have access to `WorldSim`
        site.name = NameGen::location(&mut rng).generate_town() + " Glider Course";

        // Pick the starting downhill direction based on the average drop over
        // two chunks in the four cardinal directions
        let origin_alt = land.get_alt_approx(origin);
        let alt_drops: Vec<f32> = CARDINALS
            .iter()
            .map(|c| {
                origin_alt
                    - 0.5
                        * (land.get_alt_approx(origin + *c * TerrainChunkSize::RECT_SIZE.x as i32)
                            + land.get_alt_approx(
                                origin + 2 * *c * TerrainChunkSize::RECT_SIZE.x as i32,
                            ))
            })
            .collect();
        let mut cardinal = 0;
        let mut max_drop = 0.0;
        for (i, drop) in alt_drops.iter().enumerate() {
            if *drop > max_drop {
                max_drop = *drop;
                cardinal = i;
            }
        }
        let dir = match cardinal {
            0 => Dir::X,
            1 => Dir::Y,
            2 => Dir::NegX,
            3 => Dir::NegY,
            _ => Dir::X,
        };
        let size = 2.0;

        let mut valid_course = true;
        let mut positions = Vec::new();

        // Platform
        let pos = origin;
        let tile_pos: Vec2<i32> = Vec2::zero();
        positions.push((pos, tile_pos));

        // This defines the distance between rings
        // An offset of 5 results in courses that are about 1 minute long
        // An offset of 6+ results in not all plots being in range of the site
        const CHUNK_OFFSET: usize = 5;
        // WARNING: This assumes x and y lengths of a chunk are the same!!!
        let offset = CHUNK_OFFSET as i32 * TerrainChunkSize::RECT_SIZE.x as i32;
        // Always convert to tiles then back to wpos to remove any integer division
        // artifacts
        let tile_offset = offset / TILE_SIZE as i32;
        let pos_offset = tile_offset * TILE_SIZE as i32;

        // Loop 1 is always straight forward from the launch platform
        let pos = origin + pos_offset * dir.to_vec2();
        let tile_pos = tile_offset * dir.to_vec2();
        positions.push((pos, tile_pos));

        // Loops 2-9 follow the downhill path of terrain chunks
        // In the future it may be desirable to follow ridges and the like but that
        // would be a future MR
        let mut last_pos = pos;
        let mut last_tile_pos = tile_pos;
        for j in 1..(CHUNK_OFFSET * 9 + 1) {
            let c_downhill = land.get_chunk_wpos(last_pos).and_then(|c| c.downhill);
            if let Some(downhill) = c_downhill {
                let downhill_chunk =
                    downhill.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / (sz as i32));
                let downhill_chunk_pos = TerrainChunkSize::center_wpos(downhill_chunk);
                let downhill_vec = downhill_chunk_pos - last_pos;
                // Convert to tiles first, then back to wpos to ensure coordinates align, as
                // chunks are not tile aligned
                let tile_offset = downhill_vec / (TILE_SIZE as i32);
                let pos_offset = tile_offset * TILE_SIZE as i32;
                let pos = last_pos + pos_offset;
                let tile_pos = last_tile_pos + tile_offset;
                last_pos = pos;
                last_tile_pos = tile_pos;
                // Only want to save positions with large enough chunk offsets, not every chunk
                // position
                if j % CHUNK_OFFSET == 0 {
                    positions.push((pos, tile_pos));
                }
            } else {
                valid_course = false;
            }
        }
        // Currently there is no check to ensure the delta z between rings is
        // sufficient to successfully fly through the course. This should cause
        // no glider course site to be created. Right now it just doesn't spawn
        // one in the world (similar to towns when placed near/on bodies of water).
        // In the future maybe the generate functions should return an `Option`
        // instead of a `Site`
        if valid_course && positions.len() > 1 {
            for (i, window) in positions.windows(2).enumerate() {
                if !window.is_empty() {
                    let [(pos, tile_pos), (next_pos, next_tile_pos)] = window else {
                        panic!(
                            "previous condition required positions Vec to have at least two \
                             elements"
                        );
                    };
                    if i == 0 {
                        // Launch platform
                        let aabr = Aabr {
                            min: Vec2::broadcast(-size as i32),
                            max: Vec2::broadcast(size as i32),
                        };
                        let glider_platform = plot::GliderPlatform::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            *pos,
                            dir,
                        );
                        let alt = glider_platform.alt - 5;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::GliderPlatform(glider_platform),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(alt),
                        });
                    } else if i < 9 {
                        // Point each ring after 1 towards the next ring
                        // This provides a subtle guide through the course
                        let dir = if i > 1 {
                            Dir::from_vec2(next_pos - pos)
                        } else {
                            dir
                        };
                        let aabr = Aabr {
                            min: Vec2::broadcast(-size as i32) + tile_pos,
                            max: Vec2::broadcast(size as i32) + tile_pos,
                        };
                        let glider_ring = plot::GliderRing::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            pos,
                            i,
                            dir,
                        );
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::GliderRing(glider_ring),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: None,
                        });
                    } else if i == 9 {
                        // last ring (ring 9) and finish platform
                        // Separate condition due to window iterator to ensure
                        // the finish platform is generated
                        let dir = Dir::from_vec2(next_pos - pos);
                        let aabr = Aabr {
                            min: Vec2::broadcast(-size as i32) + tile_pos,
                            max: Vec2::broadcast(size as i32) + tile_pos,
                        };
                        let glider_ring = plot::GliderRing::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            pos,
                            i,
                            dir,
                        );
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::GliderRing(glider_ring),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: None,
                        });
                        // Finish
                        let size = 10.0;
                        let aabr = Aabr {
                            min: Vec2::broadcast(-size as i32) + next_tile_pos,
                            max: Vec2::broadcast(size as i32) + next_tile_pos,
                        };
                        let glider_finish = plot::GliderFinish::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            *next_pos,
                        );
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::GliderFinish(glider_finish),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: None,
                        });
                    }
                }
            }
        }

        site
    }

    pub fn generate_cliff_town(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let name = NameGen::location(&mut rng).generate_arabic();
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };
        let mut campfires = 0;

        // place the initial plaza
        generator_stats.add(&site.name, GenStatSiteKind::CliffTown);
        site.make_initial_plaza_default(land, &mut rng, generator_stats, &name);

        let build_chance = Lottery::from(vec![(30.0, 1), (50.0, 2)]);
        let mut airship_docks = 0;
        for _ in 0..80 {
            match *build_chance.choose_seeded(rng.gen()) {
                1 => {
                    // CliffTower
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.0).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::House);
                    let campfire = campfires < 4;
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            8..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let cliff_tower = plot::CliffTower::generate(
                            land,
                            index,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                            campfire,
                        );
                        let cliff_tower_alt = cliff_tower.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::CliffTower(cliff_tower),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(cliff_tower_alt),
                        });
                        campfires += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::House);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                2 if airship_docks < 1 => {
                    // CliffTownAirshipDock
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.0).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::AirshipDock);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            8..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let cliff_town_airship_dock = plot::CliffTownAirshipDock::generate(
                            land,
                            index,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let cliff_town_airship_dock_alt = cliff_town_airship_dock.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::CliffTownAirshipDock(cliff_town_airship_dock),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(cliff_town_airship_dock_alt),
                        });
                        airship_docks += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::AirshipDock);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                _ => {},
            }
        }

        site
    }

    pub fn generate_savannah_town(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let name = NameGen::location(&mut rng).generate_savannah_custom();
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        generator_stats.add(&site.name, GenStatSiteKind::SavannahTown);
        site.make_initial_plaza_default(land, &mut rng, generator_stats, &name);

        let mut workshops = 0;
        let mut airship_dock = 0;
        let build_chance = Lottery::from(vec![(25.0, 1), (5.0, 2), (5.0, 3), (15.0, 4), (5.0, 5)]);

        for _ in 0..50 {
            match *build_chance.choose_seeded(rng.gen()) {
                n if (n == 2 && workshops < 3) || workshops == 0 => {
                    // SavannahWorkshop
                    let size = (4.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::Workshop);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            4..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let savannah_workshop = plot::SavannahWorkshop::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let savannah_workshop_alt = savannah_workshop.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::SavannahWorkshop(savannah_workshop),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(savannah_workshop_alt),
                        });
                        workshops += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Workshop);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                1 => {
                    // SavannahHut

                    let size = (4.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::House);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            4..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let savannah_hut = plot::SavannahHut::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let savannah_hut_alt = savannah_hut.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::SavannahHut(savannah_hut),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(savannah_hut_alt),
                        });
                        generator_stats.success(&site.name, GenStatPlotKind::House);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                3 if airship_dock < 1 => {
                    // SavannahAirshipDock

                    let size = (6.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::AirshipDock);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            4..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let savannah_airship_dock = plot::SavannahAirshipDock::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let savannah_airship_dock_alt = savannah_airship_dock.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::SavannahAirshipDock(savannah_airship_dock),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(savannah_airship_dock_alt),
                        });
                        airship_dock += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::AirshipDock);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                // Field
                4 => {
                    Self::generate_farm(false, &mut rng, &mut site, land);
                },
                5 => {
                    Self::generate_barn(false, &mut rng, &mut site, land);
                },
                _ => {},
            }
        }
        site
    }

    pub fn generate_coastal_town(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);
        let name = NameGen::location(&mut rng).generate_danari();
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        generator_stats.add(&site.name, GenStatSiteKind::CoastalTown);
        site.make_initial_plaza_default(land, &mut rng, generator_stats, &name);

        let mut workshops = 0;
        let build_chance = Lottery::from(vec![(38.0, 1), (5.0, 2), (15.0, 3), (15.0, 4), (5.0, 5)]);
        let mut airship_docks = 0;
        for _ in 0..55 {
            match *build_chance.choose_seeded(rng.gen()) {
                n if (n == 2 && workshops < 3) || workshops == 0 => {
                    // CoastalWorkshop
                    let size = (7.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::Workshop);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            7..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let coastal_workshop = plot::CoastalWorkshop::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let coastal_workshop_alt = coastal_workshop.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::CoastalWorkshop(coastal_workshop),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(coastal_workshop_alt),
                        });
                        workshops += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Workshop);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                1 => {
                    // CoastalHouse

                    let size = (7.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::House);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            7..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let coastal_house = plot::CoastalHouse::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let coastal_house_alt = coastal_house.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::CoastalHouse(coastal_house),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(coastal_house_alt),
                        });

                        generator_stats.success(&site.name, GenStatPlotKind::House);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                3 if airship_docks < 1 => {
                    // CoastalAirshipDock
                    let size = (7.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::AirshipDock);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            7..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let coastal_airship_dock = plot::CoastalAirshipDock::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let coastal_airship_dock_alt = coastal_airship_dock.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::CoastalAirshipDock(coastal_airship_dock),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(coastal_airship_dock_alt),
                        });
                        airship_docks += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::AirshipDock);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                // Field
                4 => {
                    Self::generate_farm(false, &mut rng, &mut site, land);
                },
                5 => {
                    Self::generate_barn(false, &mut rng, &mut site, land);
                },
                _ => {},
            }
        }
        site
    }

    pub fn generate_desert_city(
        land: &Land,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
        generator_stats: &mut SitesGenMeta,
    ) -> Self {
        let mut rng = reseed(rng);

        let name = NameGen::location(&mut rng).generate_arabic();
        let mut site = Site {
            origin,
            name: name.clone(),
            ..Site::default()
        };

        // place the initial plaza
        site.demarcate_obstacles(land);
        // The desert_city_arena is 17 tiles in radius, so the plaza should be outside
        // the palace.
        const DESERT_CITY_PLAZA_RADIUS: u32 = 3;
        const DESERT_CITY_PLAZA_SEARCH_INNER: u32 = 19;
        const DESERT_CITY_PLAZA_SEARCH_WIDTH: u32 = 12;
        generator_stats.add(&site.name, GenStatSiteKind::DesertCity);
        site.make_initial_plaza(
            land,
            &mut rng,
            DESERT_CITY_PLAZA_RADIUS,
            DESERT_CITY_PLAZA_SEARCH_INNER,
            DESERT_CITY_PLAZA_SEARCH_WIDTH,
            generator_stats,
            &name,
        );

        let size = 17.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };

        let desert_city_arena =
            plot::DesertCityArena::generate(land, &mut reseed(&mut rng), &site, aabr);

        let desert_city_arena_alt = desert_city_arena.alt;
        let plot = site.create_plot(Plot {
            kind: PlotKind::DesertCityArena(desert_city_arena),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });

        site.blit_aabr(aabr, Tile {
            kind: TileKind::Building,
            plot: Some(plot),
            hard_alt: Some(desert_city_arena_alt),
        });

        let build_chance = Lottery::from(vec![(20.0, 1), (10.0, 2), (5.0, 3), (10.0, 4), (0.0, 5)]);

        let mut temples = 0;
        let mut airship_docks = 0;
        let mut campfires = 0;

        for _ in 0..35 {
            match *build_chance.choose_seeded(rng.gen()) {
                // DesertCityMultiplot
                1 => {
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::MultiPlot);
                    let campfire = campfires < 4;
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            8..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let desert_city_multi_plot = plot::DesertCityMultiPlot::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                            campfire,
                        );
                        let desert_city_multi_plot_alt = desert_city_multi_plot.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::DesertCityMultiPlot(desert_city_multi_plot),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(desert_city_multi_plot_alt),
                        });
                        campfires += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::MultiPlot);
                    } else {
                        site.make_plaza(land, &mut rng, generator_stats, &name);
                    }
                },
                // DesertCityTemple
                2 if temples < 1 => {
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::Temple);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            8..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let desert_city_temple = plot::DesertCityTemple::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let desert_city_temple_alt = desert_city_temple.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::DesertCityTemple(desert_city_temple),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(desert_city_temple_alt),
                        });
                        temples += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::Temple);
                    }
                },
                // DesertCityAirshipDock
                3 if airship_docks < 1 => {
                    let size = (6.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
                    generator_stats.attempt(&site.name, GenStatPlotKind::AirshipDock);
                    if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                        site.find_roadside_aabr(
                            &mut rng,
                            8..(size + 1).pow(2),
                            Extent2::broadcast(size),
                        )
                    }) {
                        let desert_city_airship_dock = plot::DesertCityAirshipDock::generate(
                            land,
                            &mut reseed(&mut rng),
                            &site,
                            door_tile,
                            door_dir,
                            aabr,
                        );
                        let desert_city_airship_dock_alt = desert_city_airship_dock.alt;
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::DesertCityAirshipDock(desert_city_airship_dock),
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building,
                            plot: Some(plot),
                            hard_alt: Some(desert_city_airship_dock_alt),
                        });
                        airship_docks += 1;
                        generator_stats.success(&site.name, GenStatPlotKind::AirshipDock);
                    }
                },
                // cactus farm
                4 => {
                    Self::generate_farm(true, &mut rng, &mut site, land);
                },
                // desert barn - disabled for now (0.0 spawn chance)
                // need desert-variant sprite
                5 => {
                    Self::generate_barn(true, &mut rng, &mut site, land);
                },
                _ => {},
            }
        }
        site
    }

    pub fn generate_farm(
        is_desert: bool,
        mut rng: &mut impl Rng,
        site: &mut Site,
        land: &Land,
    ) -> bool {
        let size = (3.0 + rng.gen::<f32>().powf(5.0) * 6.0).round() as u32;
        if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
            site.find_rural_aabr(&mut rng, 6..(size + 1).pow(2), Extent2::broadcast(size))
        }) {
            let field = plot::FarmField::generate(
                land,
                &mut reseed(&mut rng),
                site,
                door_tile,
                door_dir,
                aabr,
                is_desert,
            );

            let field_alt = field.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::FarmField(field),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Field,
                plot: Some(plot),
                hard_alt: Some(field_alt),
            });
            true
        } else {
            false
        }
    }

    pub fn generate_barn(
        is_desert: bool,
        mut rng: &mut impl Rng,
        site: &mut Site,
        land: &Land,
    ) -> bool {
        let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
        if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
            site.find_rural_aabr(&mut rng, 9..(size + 1).pow(2), Extent2::broadcast(size))
        }) {
            let barn = plot::Barn::generate(
                land,
                &mut reseed(&mut rng),
                site,
                door_tile,
                door_dir,
                aabr,
                is_desert,
            );
            let barn_alt = barn.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Barn(barn),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(barn_alt),
            });

            true
        } else {
            false
        }
    }

    pub fn generate_haniwa(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: format!(
                "{} {}",
                NameGen::location(&mut rng).generate_haniwa(),
                [
                    "Catacombs",
                    "Crypt",
                    "Tomb",
                    "Gravemound",
                    "Tunnels",
                    "Vault",
                    "Chambers",
                    "Halls",
                    "Tumulus",
                    "Barrow",
                ]
                .choose(&mut rng)
                .unwrap()
            ),
            ..Site::default()
        };
        let size = 24.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let haniwa = plot::Haniwa::generate(land, &mut reseed(&mut rng), &site, aabr);
            let haniwa_alt = haniwa.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Haniwa(haniwa),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(haniwa_alt),
            });
        }
        site
    }

    pub fn generate_chapel_site(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: NameGen::location(&mut rng).generate_danari(),
            ..Site::default()
        };

        // SeaChapel
        let size = 10.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let sea_chapel = plot::SeaChapel::generate(land, &mut reseed(&mut rng), &site, aabr);
            let sea_chapel_alt = sea_chapel.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::SeaChapel(sea_chapel),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(sea_chapel_alt),
            });
        }
        site
    }

    pub fn generate_pirate_hideout(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: "".to_string(),
            ..Site::default()
        };

        let size = 8.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let pirate_hideout =
                plot::PirateHideout::generate(land, &mut reseed(&mut rng), &site, aabr);
            let pirate_hideout_alt = pirate_hideout.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::PirateHideout(pirate_hideout),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(pirate_hideout_alt),
            });
        }
        site
    }

    pub fn generate_jungle_ruin(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: "".to_string(),
            ..Site::default()
        };
        let size = 8.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let jungle_ruin = plot::JungleRuin::generate(land, &mut reseed(&mut rng), &site, aabr);
            let jungle_ruin_alt = jungle_ruin.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::JungleRuin(jungle_ruin),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(jungle_ruin_alt),
            });
        }
        site
    }

    pub fn generate_rock_circle(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: "".to_string(),
            ..Site::default()
        };
        let size = 8.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let rock_circle = plot::RockCircle::generate(land, &mut reseed(&mut rng), &site, aabr);
            let rock_circle_alt = rock_circle.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::RockCircle(rock_circle),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(rock_circle_alt),
            });
        }
        site
    }

    pub fn generate_troll_cave(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: "".to_string(),
            ..Site::default()
        };
        let size = 2.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        let site_temp = temp_at_wpos(land, origin);
        {
            let troll_cave =
                plot::TrollCave::generate(land, &mut reseed(&mut rng), &site, aabr, site_temp);
            let troll_cave_alt = troll_cave.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::TrollCave(troll_cave),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(troll_cave_alt),
            });
        }
        site
    }

    pub fn generate_camp(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: "".to_string(),
            ..Site::default()
        };
        let size = 2.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        let site_temp = temp_at_wpos(land, origin);
        {
            let camp = plot::Camp::generate(land, &mut reseed(&mut rng), &site, aabr, site_temp);
            let camp_alt = camp.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Camp(camp),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(camp_alt),
            });
        }
        site
    }

    pub fn generate_cultist(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: {
                let name = NameGen::location(&mut rng).generate();
                match rng.gen_range(0..5) {
                    0 => format!("{} Dungeon", name),
                    1 => format!("{} Lair", name),
                    2 => format!("{} Crib", name),
                    3 => format!("{} Catacombs", name),
                    _ => format!("{} Pit", name),
                }
            },
            ..Site::default()
        };
        let size = 22.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let cultist = plot::Cultist::generate(land, &mut reseed(&mut rng), &site, aabr);
            let cultist_alt = cultist.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Cultist(cultist),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(cultist_alt),
            });
        }
        site
    }

    pub fn generate_sahagin(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        origin: Vec2<i32>,
    ) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: {
                let name = NameGen::location(&mut rng).generate();
                match rng.gen_range(0..5) {
                    0 => format!("{} Isle", name),
                    1 => format!("{} Islet", name),
                    2 => format!("{} Key", name),
                    3 => format!("{} Cay", name),
                    _ => format!("{} Rock", name),
                }
            },
            ..Site::default()
        };
        let size = 16.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let sahagin = plot::Sahagin::generate(land, index, &mut reseed(&mut rng), &site, aabr);
            let sahagin_alt = sahagin.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Sahagin(sahagin),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(sahagin_alt),
            });
        }
        site
    }

    pub fn generate_vampire_castle(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: {
                let name = NameGen::location(&mut rng).generate_vampire();
                match rng.gen_range(0..4) {
                    0 => format!("{} Keep", name),
                    1 => format!("{} Chateau", name),
                    2 => format!("{} Manor", name),
                    _ => format!("{} Palace", name),
                }
            },
            ..Site::default()
        };
        let size = 22.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let vampire_castle =
                plot::VampireCastle::generate(land, &mut reseed(&mut rng), &site, aabr);
            let vampire_castle_alt = vampire_castle.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::VampireCastle(vampire_castle),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(vampire_castle_alt),
            });
        }
        site
    }

    pub fn generate_bridge(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        start: Vec2<i32>,
        end: Vec2<i32>,
    ) -> Self {
        let mut rng = reseed(rng);
        let start = TerrainChunkSize::center_wpos(start);
        let end = TerrainChunkSize::center_wpos(end);
        let origin = (start + end) / 2;

        let mut site = Site {
            origin,
            name: format!("Bridge of {}", NameGen::location(&mut rng).generate_town()),
            ..Site::default()
        };

        let start_tile = site.wpos_tile_pos(start);
        let end_tile = site.wpos_tile_pos(end);

        let width = 1;

        let orth = (start_tile - end_tile).yx().map(|dir| dir.signum().abs());

        let start_aabr = Aabr {
            min: start_tile.map2(end_tile, |a, b| a.min(b)) - orth * width,
            max: start_tile.map2(end_tile, |a, b| a.max(b)) + 1 + orth * width,
        };

        let bridge = plot::Bridge::generate(land, index, &mut rng, &site, start_tile, end_tile);

        let start_tile = site.wpos_tile_pos(bridge.start.xy());
        let end_tile = site.wpos_tile_pos(bridge.end.xy());

        let width = (bridge.width() + TILE_SIZE as i32 / 2) / TILE_SIZE as i32;
        let aabr = Aabr {
            min: start_tile.map2(end_tile, |a, b| a.min(b)) - orth * width,
            max: start_tile.map2(end_tile, |a, b| a.max(b)) + 1 + orth * width,
        };

        site.create_road(
            land,
            &mut rng,
            bridge.dir.select_aabr_with(aabr, aabr.center()) + bridge.dir.to_vec2(),
            bridge.dir.select_aabr_with(start_aabr, aabr.center()),
            2,
        );
        site.create_road(
            land,
            &mut rng,
            (-bridge.dir).select_aabr_with(aabr, aabr.center()) - bridge.dir.to_vec2(),
            (-bridge.dir).select_aabr_with(start_aabr, aabr.center()),
            2,
        );

        let plot = site.create_plot(Plot {
            kind: PlotKind::Bridge(bridge),
            root_tile: start_tile,
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });

        site.blit_aabr(aabr, Tile {
            kind: TileKind::Bridge,
            plot: Some(plot),
            hard_alt: None,
        });

        site
    }

    pub fn wpos_tile_pos(&self, wpos2d: Vec2<i32>) -> Vec2<i32> {
        (wpos2d - self.origin).map(|e| e.div_euclid(TILE_SIZE as i32))
    }

    pub fn wpos_tile(&self, wpos2d: Vec2<i32>) -> &Tile {
        self.tiles.get(self.wpos_tile_pos(wpos2d))
    }

    pub fn tile_wpos(&self, tile: Vec2<i32>) -> Vec2<i32> { self.origin + tile * TILE_SIZE as i32 }

    pub fn tile_center_wpos(&self, tile: Vec2<i32>) -> Vec2<i32> {
        self.origin + tile * TILE_SIZE as i32 + TILE_SIZE as i32 / 2
    }

    pub fn render_tile(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng, tpos: Vec2<i32>) {
        let tile = self.tiles.get(tpos);
        let twpos = self.tile_wpos(tpos);
        let twpos_center = self.tile_center_wpos(tpos);
        let border = TILE_SIZE as i32;
        let cols = (-border..TILE_SIZE as i32 + border).flat_map(|y| {
            (-border..TILE_SIZE as i32 + border)
                .map(move |x| (twpos + Vec2::new(x, y), Vec2::new(x, y)))
        });
        let calendar = None;

        match &tile.kind {
            TileKind::Plaza | TileKind::Path => {
                let near_roads = CARDINALS.iter().filter_map(|rpos| {
                    if self.tiles.get(tpos + rpos) == tile {
                        Some(Aabr {
                            min: self.tile_wpos(tpos).map(|e| e as f32),
                            max: self.tile_wpos(tpos + 1).map(|e| e as f32),
                        })
                    } else {
                        None
                    }
                });

                cols.for_each(|(wpos2d, _offs)| {
                    let wpos2df = wpos2d.map(|e| e as f32);
                    let dist = near_roads
                        .clone()
                        .map(|aabr| aabr.distance_to_point(wpos2df))
                        .min_by_key(|d| (*d * 100.0) as i32);

                    if dist.is_some_and(|d| d <= 1.5) {
                        let alt = canvas.col(wpos2d).map_or(0, |col| col.alt as i32);
                        let sub_surface_color = canvas
                            .col(wpos2d)
                            .map_or(Rgb::zero(), |col| col.sub_surface_color * 0.5);
                        for z in -8..6 {
                            canvas.map(Vec3::new(wpos2d.x, wpos2d.y, alt + z), |b| {
                                if b.kind() == BlockKind::Snow {
                                    b.into_vacant()
                                } else if b.is_filled() {
                                    if b.is_terrain() {
                                        Block::new(
                                            BlockKind::Earth,
                                            (sub_surface_color * 255.0).as_(),
                                        )
                                    } else {
                                        b
                                    }
                                } else {
                                    b.into_vacant()
                                }
                            })
                        }
                        if wpos2d == twpos_center && dynamic_rng.gen_bool(0.01) {
                            let spec = [
                                "common.entity.wild.peaceful.cat",
                                "common.entity.wild.peaceful.dog",
                            ]
                            .choose(dynamic_rng)
                            .unwrap();
                            canvas.spawn(
                                EntityInfo::at(Vec3::new(wpos2d.x, wpos2d.y, alt).as_())
                                    .with_asset_expect(spec, dynamic_rng, calendar)
                                    .with_alignment(Alignment::Tame),
                            );
                        }
                    }
                });
            },
            _ => {},
        }
    }

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        canvas.foreach_col(|canvas, wpos2d, col| {

            let tpos = self.wpos_tile_pos(wpos2d);
            let near_roads = CARDINALS
                .iter()
                .filter_map(|rpos| {
                    let tile = self.tiles.get(tpos + rpos);
                    if let TileKind::Road { a, b, w } = &tile.kind {
                        if let Some(PlotKind::Road(path)) = tile.plot.map(|p| &self.plot(p).kind) {
                            Some((LineSegment2 {
                                start: self.tile_wpos(path.nodes()[*a as usize]).map(|e| e as f32),
                                end: self.tile_wpos(path.nodes()[*b as usize]).map(|e| e as f32),
                            }, *w, tile.hard_alt))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

            let wpos2df = wpos2d.map(|e| e as f32);
            let mut min_dist = None;
            let mut avg_hard_alt = None;
            for (line, w, hard_alt) in near_roads {
                let dist = line.distance_to_point(wpos2df);
                let path_width = w as f32 * 2.0;
                if dist < path_width {
                    min_dist = Some(min_dist.map(|d: f32| d.min(dist)).unwrap_or(dist));

                    if let Some(ha) = hard_alt {
                        let w = path_width - dist;
                        let (sum, weight) = avg_hard_alt.unwrap_or((0.0, 0.0));
                        avg_hard_alt = Some((sum + ha as f32 * w, weight + w));
                    }
                }
            }

            // let dist  = near_roads
            //     .map(|(line, w)| (line.distance_to_point(wpos2df) - w as f32 * 2.0).max(0.0))
            //     .min_by_key(|d| (*d * 100.0) as i32);

            if min_dist.is_some() {
                let alt = /*avg_hard_alt.map(|(sum, weight)| sum / weight).unwrap_or_else(||*/ canvas.col(wpos2d).map_or(0.0, |col| col.alt)/*)*/ as i32;
                let mut underground = true;
                let sub_surface_color = canvas
                    .col(wpos2d)
                    .map_or(Rgb::zero(), |col| col.sub_surface_color * 0.5);
                for z in -6..4 {
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, alt + z),
                        |b| {
                            let sprite = if underground && self.tile_wpos(tpos) == wpos2d && (tpos + tpos.yx() / 2) % 2 == Vec2::zero() {
                                SpriteKind::StreetLamp
                            } else {
                                SpriteKind::Empty
                            };
                            if b.kind() == BlockKind::Snow {
                                underground = false;
                                b.into_vacant().with_sprite(sprite)
                            } else if b.is_filled() {
                                if b.is_terrain() {
                                    Block::new(
                                        BlockKind::Earth,
                                        (sub_surface_color * 255.0).as_(),
                                    )
                                } else {
                                    b
                                }
                            } else {
                                underground = false;
                                b.into_vacant().with_sprite(sprite)
                            }
                        },
                    );
                }
            }

            let tile = self.wpos_tile(wpos2d);
            let seed = tile.plot.map_or(0, |p| self.plot(p).seed);
            match tile.kind {
                TileKind::Field /*| TileKind::Road*/ => (-4..5).for_each(|z| canvas.map(
                    Vec3::new(wpos2d.x, wpos2d.y, col.alt as i32 + z),
                    |b| if [
                        BlockKind::Grass,
                        BlockKind::Earth,
                        BlockKind::Sand,
                        BlockKind::Snow,
                        BlockKind::Rock,
                    ]
                    .contains(&b.kind()) {
                        match tile.kind {
                            TileKind::Field => Block::new(BlockKind::Earth, Rgb::new(40, 5 + (seed % 32) as u8, 0)),
                            TileKind::Road { .. } => Block::new(BlockKind::Rock, Rgb::new(55, 45, 65)),
                            _ => unreachable!(),
                        }
                    } else {
                        b.with_sprite(SpriteKind::Empty)
                    },
                )),
                // TileKind::Building => {
                //     let base_alt = tile.plot.map(|p| self.plot(p)).map_or(col.alt as i32, |p| p.base_alt);
                //     for z in base_alt - 12..base_alt + 16 {
                //         canvas.set(
                //             Vec3::new(wpos2d.x, wpos2d.y, z),
                //             Block::new(BlockKind::Wood, Rgb::new(180, 90 + (seed % 64) as u8, 120))
                //         );
                //     }
                // },
                // TileKind::Castle | TileKind::Wall => {
                //     let base_alt = tile.plot.map(|p| self.plot(p)).map_or(col.alt as i32, |p| p.base_alt);
                //     for z in base_alt - 12..base_alt + if tile.kind == TileKind::Wall { 24 } else { 40 } {
                //         canvas.set(
                //             Vec3::new(wpos2d.x, wpos2d.y, z),
                //             Block::new(BlockKind::Wood, Rgb::new(40, 40, 55))
                //         );
                //     }
                // },
                _ => {},
            }

            for z_off in (-2..4).rev() {
                if let Some(plot) = tile.plot.map(|p| &self.plots[p]) {
                    canvas.map_resource(
                        Vec3::new(wpos2d.x, wpos2d.y, col.alt as i32 + z_off),
                        |block| foreach_plot!(&plot.kind, plot => plot.terrain_surface_at(wpos2d, block, dynamic_rng, col, z_off).unwrap_or(block), block),
                    )
                }
            }
        });

        let tile_aabr = Aabr {
            min: self.wpos_tile_pos(canvas.wpos()) - 1,
            max: self
                .wpos_tile_pos(canvas.wpos() + TerrainChunkSize::RECT_SIZE.map(|e| e as i32) + 2)
                + 3, // Round up, uninclusive, border
        };

        // Don't double-generate the same plot per chunk!
        let mut plots = DHashSet::default();

        for y in tile_aabr.min.y..tile_aabr.max.y {
            for x in tile_aabr.min.x..tile_aabr.max.x {
                self.render_tile(canvas, dynamic_rng, Vec2::new(x, y));

                if let Some(plot) = self.tiles.get(Vec2::new(x, y)).plot {
                    plots.insert(plot);
                }
            }
        }

        // TODO: Solve the 'trees are too big' problem and remove this
        for (id, plot) in self.plots.iter() {
            if matches!(&plot.kind, PlotKind::GiantTree(_)) {
                plots.insert(id);
            }
        }

        let mut plots_to_render = plots.into_iter().collect::<Vec<_>>();
        plots_to_render.sort_unstable();

        let wpos2d = canvas.info().wpos();
        let chunk_aabr = Aabr {
            min: wpos2d,
            max: wpos2d + TerrainChunkSize::RECT_SIZE.as_::<i32>(),
        };

        let info = canvas.info();

        for plot in plots_to_render {
            let (prim_tree, fills, mut entities) = match &self.plots[plot].kind {
                PlotKind::House(house) => house.render_collect(self, canvas),
                PlotKind::AirshipDock(airship_dock) => airship_dock.render_collect(self, canvas),
                PlotKind::GliderRing(glider_ring) => glider_ring.render_collect(self, canvas),
                PlotKind::GliderPlatform(glider_platform) => {
                    glider_platform.render_collect(self, canvas)
                },
                PlotKind::GliderFinish(glider_finish) => glider_finish.render_collect(self, canvas),
                PlotKind::Tavern(tavern) => tavern.render_collect(self, canvas),
                PlotKind::CoastalAirshipDock(coastal_airship_dock) => {
                    coastal_airship_dock.render_collect(self, canvas)
                },
                PlotKind::CoastalHouse(coastal_house) => coastal_house.render_collect(self, canvas),
                PlotKind::CoastalWorkshop(coastal_workshop) => {
                    coastal_workshop.render_collect(self, canvas)
                },
                PlotKind::JungleRuin(jungle_ruin) => jungle_ruin.render_collect(self, canvas),
                PlotKind::Workshop(workshop) => workshop.render_collect(self, canvas),
                PlotKind::Castle(castle) => castle.render_collect(self, canvas),
                PlotKind::SeaChapel(sea_chapel) => sea_chapel.render_collect(self, canvas),
                PlotKind::Gnarling(gnarling) => gnarling.render_collect(self, canvas),
                PlotKind::Adlet(adlet) => adlet.render_collect(self, canvas),
                PlotKind::Haniwa(haniwa) => haniwa.render_collect(self, canvas),
                PlotKind::GiantTree(giant_tree) => giant_tree.render_collect(self, canvas),
                PlotKind::CliffTower(cliff_tower) => cliff_tower.render_collect(self, canvas),
                PlotKind::CliffTownAirshipDock(cliff_town_airship_dock) => {
                    cliff_town_airship_dock.render_collect(self, canvas)
                },
                PlotKind::Sahagin(sahagin) => sahagin.render_collect(self, canvas),
                PlotKind::SavannahAirshipDock(savannah_airship_dock) => {
                    savannah_airship_dock.render_collect(self, canvas)
                },
                PlotKind::SavannahHut(savannah_hut) => savannah_hut.render_collect(self, canvas),
                PlotKind::SavannahWorkshop(savannah_workshop) => {
                    savannah_workshop.render_collect(self, canvas)
                },
                PlotKind::DwarvenMine(dwarven_mine) => dwarven_mine.render_collect(self, canvas),
                PlotKind::TerracottaPalace(terracotta_palace) => {
                    terracotta_palace.render_collect(self, canvas)
                },
                PlotKind::TerracottaHouse(terracotta_house) => {
                    terracotta_house.render_collect(self, canvas)
                },
                PlotKind::TerracottaYard(terracotta_yard) => {
                    terracotta_yard.render_collect(self, canvas)
                },
                PlotKind::Cultist(cultist) => cultist.render_collect(self, canvas),
                PlotKind::VampireCastle(vampire_castle) => {
                    vampire_castle.render_collect(self, canvas)
                },
                PlotKind::MyrmidonArena(myrmidon_arena) => {
                    myrmidon_arena.render_collect(self, canvas)
                },
                PlotKind::MyrmidonHouse(myrmidon_house) => {
                    myrmidon_house.render_collect(self, canvas)
                },
                PlotKind::DesertCityMultiPlot(desert_city_multi_plot) => {
                    desert_city_multi_plot.render_collect(self, canvas)
                },
                PlotKind::DesertCityTemple(desert_city_temple) => {
                    desert_city_temple.render_collect(self, canvas)
                },
                PlotKind::DesertCityArena(desert_city_arena) => {
                    desert_city_arena.render_collect(self, canvas)
                },
                PlotKind::DesertCityAirshipDock(desert_city_airship_dock) => {
                    desert_city_airship_dock.render_collect(self, canvas)
                },
                PlotKind::Citadel(citadel) => citadel.render_collect(self, canvas),
                PlotKind::Barn(barn) => barn.render_collect(self, canvas),
                PlotKind::Bridge(bridge) => bridge.render_collect(self, canvas),
                PlotKind::PirateHideout(pirate_hideout) => {
                    pirate_hideout.render_collect(self, canvas)
                },
                PlotKind::RockCircle(rock_circle) => rock_circle.render_collect(self, canvas),
                PlotKind::TrollCave(troll_cave) => troll_cave.render_collect(self, canvas),
                PlotKind::Camp(camp) => camp.render_collect(self, canvas),
                PlotKind::FarmField(farm_field) => farm_field.render_collect(self, canvas),
                PlotKind::Plaza | PlotKind::Road(_) => continue,
                // _ => continue, Avoid using a wildcard here!!
            };

            let mut spawn = |pos, last_block| {
                if let Some(entity) = match &self.plots[plot].kind {
                    PlotKind::GiantTree(tree) => tree.entity_at(pos, &last_block, dynamic_rng),
                    _ => None,
                } {
                    entities.push(entity);
                }
            };

            for (prim, fill) in fills {
                for mut aabb in Fill::get_bounds_disjoint(&prim_tree, prim) {
                    aabb.min = Vec2::max(aabb.min.xy(), chunk_aabr.min).with_z(aabb.min.z);
                    aabb.max = Vec2::min(aabb.max.xy(), chunk_aabr.max).with_z(aabb.max.z);

                    for x in aabb.min.x..aabb.max.x {
                        for y in aabb.min.y..aabb.max.y {
                            let wpos = Vec2::new(x, y);
                            let col_tile = self.wpos_tile(wpos);
                            if
                            /* col_tile.is_building() && */
                            col_tile
                                .plot
                                .and_then(|p| self.plots[p].z_range())
                                .zip(self.plots[plot].z_range())
                                .is_some_and(|(a, b)| a.end > b.end)
                            {
                                continue;
                            }
                            let mut last_block = None;

                            let col = canvas
                                .col(wpos)
                                .map(|col| col.get_info())
                                .unwrap_or_default();

                            for z in aabb.min.z..aabb.max.z {
                                let pos = Vec3::new(x, y, z);

                                let mut sprite_cfg = None;

                                let map = |block| {
                                    let current_block = fill.sample_at(
                                        &prim_tree,
                                        prim,
                                        pos,
                                        &info,
                                        block,
                                        &mut sprite_cfg,
                                        &col,
                                    );
                                    if let (Some(last_block), None) = (last_block, current_block) {
                                        spawn(pos, last_block);
                                    }
                                    last_block = current_block;
                                    current_block.unwrap_or(block)
                                };

                                match fill {
                                    Fill::ResourceSprite { .. } => canvas.map_resource(pos, map),
                                    _ => canvas.map(pos, map),
                                };

                                if let Some(sprite_cfg) = sprite_cfg {
                                    canvas.set_sprite_cfg(pos, sprite_cfg);
                                }
                            }
                            if let Some(block) = last_block {
                                spawn(Vec3::new(x, y, aabb.max.z), block);
                            }
                        }
                    }
                }
            }

            for entity in entities {
                canvas.spawn(entity);
            }
        }
    }

    pub fn apply_supplement(
        &self,
        dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        supplement: &mut crate::ChunkSupplement,
    ) {
        for (_, plot) in self.plots.iter() {
            match &plot.kind {
                PlotKind::Gnarling(g) => g.apply_supplement(dynamic_rng, wpos2d, supplement),
                PlotKind::Adlet(a) => a.apply_supplement(dynamic_rng, wpos2d, supplement),
                _ => {},
            }
        }
    }
}

pub fn test_site() -> Site {
    let index = crate::index::Index::new(0);
    let index_ref = IndexRef {
        colors: &index.colors(),
        features: &index.features(),
        index: &index,
    };
    let mut gen_meta = SitesGenMeta::new(0);
    Site::generate_city(
        &Land::empty(),
        index_ref,
        &mut thread_rng(),
        Vec2::zero(),
        0.5,
        None,
        &mut gen_meta,
    )
}

fn wpos_is_hazard(land: &Land, wpos: Vec2<i32>) -> Option<HazardKind> {
    if land
        .get_chunk_wpos(wpos)
        .is_none_or(|c| c.river.near_water())
    {
        Some(HazardKind::Water)
    } else {
        Some(land.get_gradient_approx(wpos))
            .filter(|g| *g > 0.8)
            .map(|gradient| HazardKind::Hill { gradient })
    }
}

fn temp_at_wpos(land: &Land, wpos: Vec2<i32>) -> f32 {
    land.get_chunk_wpos(wpos)
        .map(|c| c.temp)
        .unwrap_or(CONFIG.temperate_temp)
}

pub fn aabr_tiles(aabr: Aabr<i32>) -> impl Iterator<Item = Vec2<i32>> {
    (0..aabr.size().h)
        .flat_map(move |y| (0..aabr.size().w).map(move |x| aabr.min + Vec2::new(x, y)))
}

pub struct Plaza {}
