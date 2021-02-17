mod plot;
mod tile;

use self::{
    plot::{Plot, PlotKind},
    tile::{TileGrid, Tile, TileKind, TILE_SIZE},
};
use crate::{
    site::SpawnRules,
    util::{Grid, attempt, CARDINALS, SQUARE_9},
    Canvas,
    Land,
};
use common::{
    terrain::{Block, BlockKind, SpriteKind},
    store::{Id, Store},
    astar::Astar,
    lottery::Lottery,
};
use hashbrown::hash_map::DefaultHashBuilder;
use rand::prelude::*;
use vek::*;
use std::ops::Range;

#[derive(Default)]
pub struct Site {
    pub(crate) origin: Vec2<i32>,
    tiles: TileGrid,
    plots: Store<Plot>,
    plazas: Vec<Id<Plot>>,
    roads: Vec<Id<Plot>>,
}

impl Site {
    pub fn radius(&self) -> f32 {
        (tile::MAX_BLOCK_RADIUS.pow(2) as f32 * 2.0).sqrt()
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: SQUARE_9
                .iter()
                .all(|&rpos| self.wpos_tile(wpos + rpos * tile::TILE_SIZE as i32).is_empty()),
            ..SpawnRules::default()
        }
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let radius = tile::MAX_BLOCK_RADIUS;
        Aabr {
            min: -Vec2::broadcast(radius as i32),
            max: Vec2::broadcast(radius as i32),
        }
    }

    pub fn plot(&self, id: Id<Plot>) -> &Plot { &self.plots[id] }

    pub fn plots(&self) -> impl Iterator<Item = &Plot> + '_ { self.plots.values() }

    pub fn create_plot(&mut self, plot: Plot) -> Id<Plot> { self.plots.insert(plot) }

    pub fn blit_aabr(&mut self, aabr: Aabr<i32>, tile: Tile) {
        for y in 0..aabr.size().h {
            for x in 0..aabr.size().w {
                self.tiles.set(aabr.min + Vec2::new(x, y), tile.clone());
            }
        }
    }

    pub fn create_road(&mut self, land: &Land, rng: &mut impl Rng, a: Vec2<i32>, b: Vec2<i32>) -> Option<Id<Plot>> {
        const MAX_ITERS: usize = 4096;
        let heuristic = |tile: &Vec2<i32>| if self.tiles.get(*tile).is_obstacle() { 100.0 } else { 0.0 };
        let path = Astar::new(MAX_ITERS, a, &heuristic, DefaultHashBuilder::default()).poll(
            MAX_ITERS,
            &heuristic,
            |tile| { let tile = *tile; CARDINALS.iter().map(move |dir| tile + *dir) },
            |a, b| (a.distance_squared(*b) as f32).sqrt(),
            |tile| *tile == b,
        ).into_path()?;

        let plot = self.create_plot(Plot {
            kind: PlotKind::Road(path.clone()),
            root_tile: a,
            tiles: path.clone().into_iter().collect(),
            seed: rng.gen(),
            base_alt: 0,
        });

        self.roads.push(plot);

        for &tile in path.iter() {
            self.tiles.set(tile, Tile {
                kind: TileKind::Road,
                plot: Some(plot),
            });
        }

        Some(plot)
    }

    pub fn find_aabr(&mut self, search_pos: Vec2<i32>, area_range: Range<u32>, min_dims: Extent2<u32>) -> Option<(Aabr<i32>, Vec2<i32>)> {
        self.tiles.find_near(
            search_pos,
            |center, _| if CARDINALS.iter().any(|&dir| self.tiles.get(center + dir).kind == TileKind::Road) {
                self.tiles.grow_aabr(center, area_range.clone(), min_dims).ok()
            } else {
                None
            },
        )
    }

    pub fn find_roadside_aabr(&mut self, rng: &mut impl Rng, area_range: Range<u32>, min_dims: Extent2<u32>) -> Option<(Aabr<i32>, Vec2<i32>)> {
        let dir = Vec2::<f32>::zero().map(|_| rng.gen_range(-1.0..1.0)).normalized();
        let search_pos = if rng.gen() {
            self.plot(*self.plazas.choose(rng)?).root_tile + (dir * 5.0).map(|e: f32| e.round() as i32)
        } else {
            if let PlotKind::Road(path) = &self.plot(*self.roads.choose(rng)?).kind {
                *path.nodes().choose(rng)? + (dir * 1.5).map(|e: f32| e.round() as i32)
            } else {
                unreachable!()
            }
        };

        self.find_aabr(search_pos, area_range, min_dims)
    }

    pub fn make_plaza(&mut self, land: &Land, rng: &mut impl Rng) -> Id<Plot> {
        let pos = attempt(32, || {
            self.plazas
                .choose(rng)
                .map(|&p| self.plot(p).root_tile + Vec2::new(rng.gen_range(-20..20), rng.gen_range(-20..20)))
                .filter(|&tile| self
                    .plazas
                    .iter()
                    .all(|&p| self.plot(p).root_tile.distance_squared(tile) > 16i32.pow(2))
                    && rng.gen_range(0..48) > tile.map(|e| e.abs()).reduce_max())
        })
            .unwrap_or_else(Vec2::zero);

        let aabr = Aabr { min: pos + Vec2::broadcast(-3), max: pos + Vec2::broadcast(4) };
        let plaza = self.create_plot(Plot {
            kind: PlotKind::Plaza,
            root_tile: pos,
            tiles: aabr_tiles(aabr).collect(),
            seed: rng.gen(),
            base_alt: land.get_alt_approx(self.tile_center_wpos(aabr.center())) as i32,
        });
        self.plazas.push(plaza);
        self.blit_aabr(aabr, Tile {
            kind: TileKind::Road,
            plot: Some(plaza),
        });

        let mut already_pathed = vec![plaza];
        for _ in 0..2 {
            if let Some(&p) = self.plazas
                .iter()
                .filter(|p| !already_pathed.contains(p))
                .min_by_key(|&&p| self.plot(p).root_tile.distance_squared(pos))
            {
                self.create_road(land, rng, self.plot(p).root_tile, pos);
                already_pathed.push(p);
            } else {
                break;
            }
        }

        plaza
    }

    pub fn generate(land: &Land, rng: &mut impl Rng, origin: Vec2<i32>) -> Self {
        let mut site = Site {
            origin,
            ..Site::default()
        };

        site.make_plaza(land, rng);

        let build_chance = Lottery::from(vec![
            (1.0, 0),
            (48.0, 1),
            (2.0, 2),
            (1.0, 3),
        ]);

        let mut castles = 0;

        for _ in 0..1000 {
            if site.plots.len() - site.plazas.len() > 80 {
                break;
            }

            match *build_chance.choose_seeded(rng.gen()) {
                // Plaza
                0 => {
                    site.make_plaza(land, rng);
                },
                // House
                1 => {
                    let size = (2.0 + rng.gen::<f32>().powf(4.0) * 3.0).round() as u32;
                    if let Some((aabr, _)) = attempt(10, || site.find_roadside_aabr(rng, 4..(size + 1).pow(2), Extent2::broadcast(size))) {
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::House,
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                            base_alt: land.get_alt_approx(site.tile_center_wpos(aabr.center())) as i32,
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Building { levels: size - 1 + rng.gen_range(0..2) },
                            plot: Some(plot),
                        });
                    }
                },
                // Guard tower
                2 => {
                    if let Some((aabr, _)) = attempt(10, || site.find_roadside_aabr(rng, 4..4, Extent2::new(2, 2))) {
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::Castle,
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                            base_alt: land.get_alt_approx(site.tile_center_wpos(aabr.center())) as i32,
                        });

                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Castle,
                            plot: Some(plot),
                        });
                    }
                },
                // Castle
                _ if castles < 1 => {
                    if let Some((aabr, _)) = attempt(10, || site.find_roadside_aabr(rng, 16 * 16..18 * 18, Extent2::new(16, 16))) {
                        let plot = site.create_plot(Plot {
                            kind: PlotKind::Castle,
                            root_tile: aabr.center(),
                            tiles: aabr_tiles(aabr).collect(),
                            seed: rng.gen(),
                            base_alt: land.get_alt_approx(site.tile_center_wpos(aabr.center())) as i32,
                        });

                        // Walls
                        site.blit_aabr(aabr, Tile {
                            kind: TileKind::Wall,
                            plot: Some(plot),
                        });

                        let tower = Tile {
                            kind: TileKind::Castle,
                            plot: Some(plot),
                        };
                        site.tiles.set(Vec2::new(aabr.min.x, aabr.min.y), tower.clone());
                        site.tiles.set(Vec2::new(aabr.max.x - 1, aabr.min.y), tower.clone());
                        site.tiles.set(Vec2::new(aabr.min.x, aabr.max.y - 1), tower.clone());
                        site.tiles.set(Vec2::new(aabr.max.x - 1, aabr.max.y - 1), tower.clone());

                        // Courtyard
                        site.blit_aabr(Aabr { min: aabr.min + 1, max: aabr.max - 1 } , Tile {
                            kind: TileKind::Road,
                            plot: Some(plot),
                        });

                        // Keep
                        site.blit_aabr(Aabr { min: aabr.center() - 3, max: aabr.center() + 3 }, Tile {
                            kind: TileKind::Castle,
                            plot: Some(plot),
                        });

                        castles += 1;
                    }
                },
                _ => {},
            }
        }

        site
    }

    pub fn wpos_tile(&self, wpos2d: Vec2<i32>) -> &Tile {
        self.tiles.get((wpos2d - self.origin).map(|e| e.div_euclid(TILE_SIZE as i32)))
    }

    pub fn tile_center_wpos(&self, tile: Vec2<i32>) -> Vec2<i32> {
        self.origin + tile * tile::TILE_SIZE as i32 + tile::TILE_SIZE as i32 / 2
    }

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        canvas.foreach_col(|canvas, wpos2d, col| {
            let tile = self.wpos_tile(wpos2d);
            let seed = tile.plot.map_or(0, |p| self.plot(p).seed);
            match tile.kind {
                TileKind::Field | TileKind::Road => (-4..5).for_each(|z| canvas.map(
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
                            TileKind::Road => Block::new(BlockKind::Rock, Rgb::new(55, 45, 65)),
                            _ => unreachable!(),
                        }
                    } else {
                        b.with_sprite(SpriteKind::Empty)
                    },
                )),
                TileKind::Building { levels } => {
                    let base_alt = tile.plot.map(|p| self.plot(p)).map_or(col.alt as i32, |p| p.base_alt);
                    for z in base_alt - 12..base_alt + 4 + 6 * levels as i32 {
                        canvas.set(
                            Vec3::new(wpos2d.x, wpos2d.y, z),
                            Block::new(BlockKind::Wood, Rgb::new(180, 90 + (seed % 64) as u8, 120))
                        );
                    }
                },
                TileKind::Castle | TileKind::Wall => {
                    let base_alt = tile.plot.map(|p| self.plot(p)).map_or(col.alt as i32, |p| p.base_alt);
                    for z in base_alt - 12..base_alt + if tile.kind == TileKind::Wall { 24 } else { 40 } {
                        canvas.set(
                            Vec3::new(wpos2d.x, wpos2d.y, z),
                            Block::new(BlockKind::Wood, Rgb::new(40, 40, 55))
                        );
                    }
                },
                _ => {},
            }
        });
    }
}

pub fn test_site() -> Site { Site::generate(&Land::empty(), &mut thread_rng(), Vec2::zero()) }

pub fn aabr_tiles(aabr: Aabr<i32>) -> impl Iterator<Item=Vec2<i32>> {
    (0..aabr.size().h)
        .map(move |y| (0..aabr.size().w)
            .map(move |x| aabr.min + Vec2::new(x, y)))
        .flatten()
}

pub struct Plaza {}
