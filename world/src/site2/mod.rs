mod gen;
pub mod plot;
mod tile;
pub mod util;

use self::tile::{HazardKind, KeepKind, RoofKind, Tile, TileGrid, TileKind, TILE_SIZE};
pub use self::{
    gen::{aabr_with_z, Fill, Painter, Primitive, PrimitiveRef, Structure},
    plot::{Plot, PlotKind},
    util::Dir,
};
use crate::{
    sim::Path,
    site::{namegen::NameGen, SpawnRules},
    util::{attempt, DHashSet, Grid, CARDINALS, SQUARE_4, SQUARE_9},
    Canvas, IndexRef, Land,
};
use common::{
    astar::Astar,
    lottery::Lottery,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::{Block, BlockKind, SpriteKind, TerrainChunkSize},
    vol::RectVolSize,
};
use hashbrown::hash_map::DefaultHashBuilder;
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
    pub(crate) origin: Vec2<i32>,
    name: String,
    tiles: TileGrid,
    plots: Store<Plot>,
    plazas: Vec<Id<Plot>>,
    roads: Vec<Id<Plot>>,
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
                PlotKind::Dungeon(d) => Some(d.spawn_rules(wpos)),
                PlotKind::Gnarling(g) => Some(g.spawn_rules(wpos)),
                _ => None,
            })
            .fold(base_spawn_rules, |a, b| a.combine(b))
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let border = 1;
        Aabr {
            min: self.origin + self.tile_wpos(self.tiles.bounds.min - border),
            max: self.origin + self.tile_wpos(self.tiles.bounds.max + 1 + border),
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
        let range = -(w as i32) / 2..w as i32 - (w as i32 + 1) / 2;
        let heuristic = |tile: &Vec2<i32>| {
            let mut max_cost = (tile.distance_squared(b) as f32).sqrt();
            for y in range.clone() {
                for x in range.clone() {
                    if self.tiles.get(*tile + Vec2::new(x, y)).is_obstacle() {
                        max_cost = max_cost.max(1000.0);
                    } else if !self.tiles.get(*tile + Vec2::new(x, y)).is_empty() {
                        max_cost = max_cost.max(25.0);
                    }
                }
            }
            max_cost
        };
        let path = Astar::new(MAX_ITERS, a, heuristic, DefaultHashBuilder::default())
            .poll(
                MAX_ITERS,
                &heuristic,
                |tile| {
                    let tile = *tile;
                    CARDINALS.iter().map(move |dir| tile + *dir)
                },
                |a, b| {
                    let alt_a = land.get_alt_approx(self.tile_center_wpos(*a));
                    let alt_b = land.get_alt_approx(self.tile_center_wpos(*b));
                    (alt_a - alt_b).abs() / TILE_SIZE as f32
                },
                |tile| *tile == b,
            )
            .into_path()?;

        let plot = self.create_plot(Plot {
            kind: PlotKind::Road(path.clone()),
            root_tile: a,
            tiles: path.clone().into_iter().collect(),
            seed: rng.gen(),
        });

        self.roads.push(plot);

        for (i, &tile) in path.iter().enumerate() {
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

    pub fn make_plaza(&mut self, land: &Land, rng: &mut impl Rng) -> Option<Id<Plot>> {
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

        let plaza_alt = land.get_alt_approx(self.tile_center_wpos(pos)) as i32;

        let aabr = Aabr {
            min: pos + Vec2::broadcast(-plaza_radius),
            max: pos + Vec2::broadcast(plaza_radius + 1),
        };
        let plaza = self.create_plot(Plot {
            kind: PlotKind::Plaza,
            root_tile: pos,
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
                .min_by_key(|&&p| self.plot(p).root_tile.distance_squared(pos))
            {
                self.create_road(land, rng, self.plot(p).root_tile, pos, 2 /* + i */);
                already_pathed.push(p);
            } else {
                break;
            }
        }

        Some(plaza)
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
                if let Some((dist, _, Path { width }, _)) = land.get_nearest_path(wpos) {
                    if dist < 2.0 * width {
                        self.tiles
                            .get_mut(tile)
                            .map(|tile| tile.kind = TileKind::Path);
                    }
                }
            });
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn dungeon_difficulty(&self) -> Option<u32> {
        self.plots
            .iter()
            .filter_map(|(_, plot)| {
                if let PlotKind::Dungeon(d) = &plot.kind {
                    Some(d.difficulty())
                } else {
                    None
                }
            })
            .max()
    }

    pub fn generate_dungeon(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);

        let mut site = Site {
            origin,
            ..Site::default()
        };

        site.demarcate_obstacles(land);
        let dungeon = plot::Dungeon::generate(origin, land, &mut rng);
        site.name = dungeon.name().to_string();
        let size = (dungeon.radius() / TILE_SIZE as f32).ceil() as i32;

        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };

        let plot = site.create_plot(Plot {
            kind: PlotKind::Dungeon(dungeon),
            root_tile: aabr.center(),
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
        });

        site.blit_aabr(aabr, Tile {
            kind: TileKind::Empty,
            plot: Some(plot),
            hard_alt: None,
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

    pub fn generate_city(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);

        let mut site = Site {
            origin,
            name: NameGen::location(&mut rng).generate_town(),
            ..Site::default()
        };

        site.demarcate_obstacles(land);

        site.make_plaza(land, &mut rng);

        let build_chance = Lottery::from(vec![(64.0, 1), (5.0, 2), (8.0, 3), (5.0, 4), (5.0, 5)]);

        let mut castles = 0;

        for _ in 0..120 {
            match *build_chance.choose_seeded(rng.gen()) {
                // House
                1 => {
                    let size = (1.5 + rng.gen::<f32>().powf(5.0) * 1.0).round() as u32;
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
                    } else {
                        site.make_plaza(land, &mut rng);
                    }
                },
                // Workshop
                5 => {
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
                    } else {
                        site.make_plaza(land, &mut rng);
                    }
                },
                // Guard tower
                2 => {
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
                /*
                3 => {
                    attempt(10, || {
                        let search_pos = attempt(16, || {
                            let tile =
                                (Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0))
                                    .normalized()
                                    * rng.gen_range(16.0..48.0))
                                .map(|e| e as i32);

                            Some(tile).filter(|_| {
                                site.plazas.iter().all(|&p| {
                                    site.plot(p).root_tile.distance_squared(tile) > 20i32.pow(2)
                                }) && rng.gen_range(0..48) > tile.map(|e| e.abs()).reduce_max()
                            })
                        })
                        .unwrap_or_else(Vec2::zero);

                        site.tiles.find_near(search_pos, |center, _| {
                            site.tiles.grow_organic(&mut rng, center, 12..64).ok()
                        })
                    })
                    .map(|(tiles, _)| {
                        for tile in tiles {
                            site.tiles.set(tile, Tile {
                                kind: TileKind::Field,
                                plot: None,
                                hard_alt: None,
                            });
                        }
                    });
                },
                */
                // Castle
                4 if castles < 1 => {
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
                    }
                },
                _ => {},
            }
        }

        site
    }

    pub fn generate_cliff_town(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: NameGen::location(&mut rng).generate_arabic(),
            ..Site::default()
        };
        let mut campfires = 0;
        site.make_plaza(land, &mut rng);
        for _ in 0..30 {
            // CliffTower
            let size = (6.0 + rng.gen::<f32>().powf(5.0) * 1.0).round() as u32;
            let campfire = campfires < 4;
            if let Some((aabr, door_tile, door_dir)) = attempt(32, || {
                site.find_roadside_aabr(&mut rng, 6..(size + 1).pow(2), Extent2::broadcast(size))
            }) {
                let cliff_tower = plot::CliffTower::generate(
                    land,
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
            } else {
                site.make_plaza(land, &mut rng);
            }
        }

        site
    }

    pub fn generate_savannah_pit(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);
        let mut site = Site {
            origin,
            name: NameGen::location(&mut rng).generate_savannah_custom(),
            ..Site::default()
        };
        let size = 11.0 as i32;
        let aabr = Aabr {
            min: Vec2::broadcast(-size),
            max: Vec2::broadcast(size),
        };
        {
            let savannah_pit =
                plot::SavannahPit::generate(land, &mut reseed(&mut rng), &site, aabr);
            let savannah_pit_alt = savannah_pit.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::SavannahPit(savannah_pit),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
                seed: rng.gen(),
            });

            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(savannah_pit_alt),
            });
        }
        site
    }

    pub fn generate_desert_city(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut rng = reseed(rng);

        let mut site = Site {
            origin,
            name: NameGen::location(&mut rng).generate_arabic(),
            ..Site::default()
        };

        site.demarcate_obstacles(land);

        site.make_plaza(land, &mut rng);

        let build_chance = Lottery::from(vec![(20.0, 1), (10.0, 2)]);

        let mut temples = 0;
        let mut campfires = 0;

        for _ in 0..30 {
            match *build_chance.choose_seeded(rng.gen()) {
                // DesertCityMultiplot
                1 => {
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
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
                    } else {
                        site.make_plaza(land, &mut rng);
                    }
                },
                // DesertCityTemple
                2 if temples < 1 => {
                    let size = (9.0 + rng.gen::<f32>().powf(5.0) * 1.5).round() as u32;
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
                    }
                },
                _ => {},
            }
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
            kind: TileKind::Building,
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

    pub fn render_tile(&self, canvas: &mut Canvas, _dynamic_rng: &mut impl Rng, tpos: Vec2<i32>) {
        let tile = self.tiles.get(tpos);
        let twpos = self.tile_wpos(tpos);
        let border = TILE_SIZE as i32;
        let cols = (-border..TILE_SIZE as i32 + border).flat_map(|y| {
            (-border..TILE_SIZE as i32 + border)
                .map(move |x| (twpos + Vec2::new(x, y), Vec2::new(x, y)))
        });

        #[allow(clippy::single_match)]
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

                    if dist.map_or(false, |d| d <= 1.5) {
                        let alt = canvas.col(wpos2d).map_or(0, |col| col.alt as i32);
                        let sub_surface_color = canvas
                            .col(wpos2d)
                            .map_or(Rgb::zero(), |col| col.sub_surface_color * 0.5);
                        let mut underground = true;
                        for z in -8..6 {
                            canvas.map(Vec3::new(wpos2d.x, wpos2d.y, alt + z), |b| {
                                if b.kind() == BlockKind::Snow {
                                    underground = false;
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
                                    underground = false;
                                    b.into_vacant()
                                }
                            })
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
            #[allow(clippy::single_match)]
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
                PlotKind::Workshop(workshop) => workshop.render_collect(self, canvas),
                PlotKind::Castle(castle) => castle.render_collect(self, canvas),
                PlotKind::SeaChapel(sea_chapel) => sea_chapel.render_collect(self, canvas),
                PlotKind::Dungeon(dungeon) => dungeon.render_collect(self, canvas),
                PlotKind::Gnarling(gnarling) => gnarling.render_collect(self, canvas),
                PlotKind::GiantTree(giant_tree) => giant_tree.render_collect(self, canvas),
                PlotKind::CliffTower(cliff_tower) => cliff_tower.render_collect(self, canvas),
                PlotKind::SavannahPit(savannah_pit) => savannah_pit.render_collect(self, canvas),
                PlotKind::DesertCityMultiPlot(desert_city_multi_plot) => {
                    desert_city_multi_plot.render_collect(self, canvas)
                },
                PlotKind::DesertCityTemple(desert_city_temple) => {
                    desert_city_temple.render_collect(self, canvas)
                },
                PlotKind::Citadel(citadel) => citadel.render_collect(self, canvas),
                PlotKind::Bridge(bridge) => bridge.render_collect(self, canvas),
                _ => continue,
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
                            let col_tile = self.wpos_tile(Vec2::new(x, y));
                            if
                            /* col_tile.is_building() && */
                            col_tile
                                .plot
                                .and_then(|p| self.plots[p].z_range())
                                .zip(self.plots[plot].z_range())
                                .map_or(false, |(a, b)| a.end > b.end)
                            {
                                continue;
                            }
                            let mut last_block = None;
                            for z in aabb.min.z..aabb.max.z {
                                let pos = Vec3::new(x, y, z);

                                canvas.map(pos, |block| {
                                    let current_block =
                                        fill.sample_at(&prim_tree, prim, pos, &info, block);
                                    if let (Some(last_block), None) = (last_block, current_block) {
                                        spawn(pos, last_block);
                                    }
                                    last_block = current_block;
                                    current_block.unwrap_or(block)
                                });
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
                PlotKind::Dungeon(d) => d.apply_supplement(dynamic_rng, wpos2d, supplement),
                PlotKind::Gnarling(g) => g.apply_supplement(dynamic_rng, wpos2d, supplement),
                _ => {},
            }
        }
    }
}

pub fn test_site() -> Site { Site::generate_city(&Land::empty(), &mut thread_rng(), Vec2::zero()) }

fn wpos_is_hazard(land: &Land, wpos: Vec2<i32>) -> Option<HazardKind> {
    if land
        .get_chunk_wpos(wpos)
        .map_or(true, |c| c.river.near_water())
    {
        Some(HazardKind::Water)
    } else {
        Some(land.get_gradient_approx(wpos))
            .filter(|g| *g > 0.8)
            .map(|gradient| HazardKind::Hill { gradient })
    }
}

pub fn aabr_tiles(aabr: Aabr<i32>) -> impl Iterator<Item = Vec2<i32>> {
    (0..aabr.size().h)
        .flat_map(move |y| (0..aabr.size().w).map(move |x| aabr.min + Vec2::new(x, y)))
}

pub struct Plaza {}
