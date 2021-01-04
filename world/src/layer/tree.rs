use crate::{
    all::ForestKind,
    block::block_from_structure,
    column::ColumnGen,
    util::{RandomPerm, Sampler, UnitChooser},
    Canvas, CONFIG,
};
use common::{
    assets::AssetHandle,
    terrain::{Block, BlockKind, structure::{Structure, StructureBlock, StructuresGroup}},
    vol::ReadVol,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use std::f32;
use vek::*;
use rand::prelude::*;

lazy_static! {
    static ref OAKS: AssetHandle<StructuresGroup> = Structure::load_group("oaks");
    static ref OAK_STUMPS: AssetHandle<StructuresGroup> = Structure::load_group("oak_stumps");
    static ref PINES: AssetHandle<StructuresGroup> = Structure::load_group("pines");
    static ref PALMS: AssetHandle<StructuresGroup> = Structure::load_group("palms");
    static ref ACACIAS: AssetHandle<StructuresGroup> = Structure::load_group("acacias");
    static ref BAOBABS: AssetHandle<StructuresGroup> = Structure::load_group("baobabs");
    static ref FRUIT_TREES: AssetHandle<StructuresGroup> = Structure::load_group("fruit_trees");
    static ref BIRCHES: AssetHandle<StructuresGroup> = Structure::load_group("birch");
    static ref MANGROVE_TREES: AssetHandle<StructuresGroup> =
        Structure::load_group("mangrove_trees");
    static ref QUIRKY: AssetHandle<StructuresGroup> = Structure::load_group("quirky");
    static ref QUIRKY_DRY: AssetHandle<StructuresGroup> = Structure::load_group("quirky_dry");
    static ref SWAMP_TREES: AssetHandle<StructuresGroup> = Structure::load_group("swamp_trees");
}

static MODEL_RAND: RandomPerm = RandomPerm::new(0xDB21C052);
static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static QUIRKY_RAND: RandomPerm = RandomPerm::new(0xA634460F);

#[allow(clippy::if_same_then_else)]
pub fn apply_trees_to(canvas: &mut Canvas) {
    // TODO: Get rid of this
    enum TreeModel {
        Structure(Structure),
        Procedural(ProceduralTree),
    }

    struct Tree {
        pos: Vec3<i32>,
        model: TreeModel,
        seed: u32,
        units: (Vec2<i32>, Vec2<i32>),
    }

    let mut tree_cache = HashMap::new();

    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let trees = info.land().get_near_trees(wpos2d);

        for (tree_wpos, seed) in trees {
            let tree = if let Some(tree) = tree_cache.entry(tree_wpos).or_insert_with(|| {
                let col = ColumnGen::new(info.land()).get((tree_wpos, info.index()))?;

                let is_quirky = QUIRKY_RAND.chance(seed, 1.0 / 500.0);

                // Ensure that it's valid to place a *thing* here
                if col.alt < col.water_level
                    || col.spawn_rate < 0.9
                    || col.water_dist.map(|d| d < 8.0).unwrap_or(false)
                    || col.path.map(|(d, _, _, _)| d < 12.0).unwrap_or(false)
                {
                    return None;
                }

                // Ensure that it's valid to place a tree here
                if !is_quirky && ((seed.wrapping_mul(13)) & 0xFF) as f32 / 256.0 > col.tree_density
                {
                    return None;
                }

                Some(Tree {
                    pos: Vec3::new(tree_wpos.x, tree_wpos.y, col.alt as i32),
                    model: 'model: {
                        let models: AssetHandle<_> = if is_quirky {
                            if col.temp > CONFIG.desert_temp {
                                *QUIRKY_DRY
                            } else {
                                *QUIRKY
                            }
                        } else {
                            match col.forest_kind {
                                ForestKind::Oak if QUIRKY_RAND.chance(seed + 1, 1.0 / 16.0) => {
                                    *OAK_STUMPS
                                },
                                ForestKind::Oak if QUIRKY_RAND.chance(seed + 2, 1.0 / 20.0) => {
                                    *FRUIT_TREES
                                },
                                ForestKind::Palm => *PALMS,
                                ForestKind::Acacia => *ACACIAS,
                                ForestKind::Baobab => *BAOBABS,
                                // ForestKind::Oak => *OAKS,
                                ForestKind::Oak => {
                                    let mut rng = RandomPerm::new(seed);
                                    break 'model TreeModel::Procedural(ProceduralTree::generate(&mut rng));
                                },
                                ForestKind::Pine => *PINES,
                                ForestKind::Birch => *BIRCHES,
                                ForestKind::Mangrove => *MANGROVE_TREES,
                                ForestKind::Swamp => *SWAMP_TREES,
                            }
                        };

                        let models = models.read();
                        TreeModel::Structure(models[(MODEL_RAND.get(seed.wrapping_mul(17)) / 13) as usize % models.len()]
                            .clone())
                    },
                    seed,
                    units: UNIT_CHOOSER.get(seed),
                })
            }) {
                tree
            } else {
                continue;
            };

            let bounds = match &tree.model {
                TreeModel::Structure(s) => s.get_bounds(),
                TreeModel::Procedural(t) => t.get_bounds().map(|e| e as i32),
            };

            let rpos2d = (wpos2d - tree.pos.xy())
                .map2(Vec2::new(tree.units.0, tree.units.1), |p, unit| {
                    unit * p
                })
                .sum();
            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                continue;
            }

            let mut is_top = true;
            let mut is_leaf_top = true;
            for z in (bounds.min.z..bounds.max.z).rev() {
                let wpos = Vec3::new(wpos2d.x, wpos2d.y, tree.pos.z + z);
                let model_pos = Vec3::from(
                    (wpos - tree.pos)
                        .xy()
                        .map2(Vec2::new(tree.units.0, tree.units.1), |rpos, unit| {
                            unit * rpos
                        })
                        .sum(),
                ) + Vec3::unit_z() * (wpos.z - tree.pos.z);
                block_from_structure(
                    info.index(),
                    if let Some(block) = match &tree.model {
                        TreeModel::Structure(s) => s.get(model_pos).ok().copied(),
                        TreeModel::Procedural(t) => Some(match t.is_branch_or_leaves_at(model_pos.map(|e| e as f32 + 0.5)) {
                            (true, _) => StructureBlock::Normal(Rgb::new(60, 30, 0)),
                            (_, true) => StructureBlock::TemperateLeaves,
                            (_, _) => StructureBlock::None,
                        }),
                    } {
                        block
                    } else {
                        break
                    },
                    wpos,
                    tree.pos.xy(),
                    tree.seed,
                    col,
                    Block::air,
                )
                .map(|block| {
                    // Add a snow covering to the block above under certain circumstances
                    if col.snow_cover
                        && ((block.kind() == BlockKind::Leaves && is_leaf_top)
                            || (is_top && block.is_filled()))
                    {
                        canvas.set(
                            wpos + Vec3::unit_z(),
                            Block::new(BlockKind::Snow, Rgb::new(210, 210, 255)),
                        );
                    }
                    canvas.set(wpos, block);
                    is_leaf_top = false;
                    is_top = false;
                })
                .unwrap_or_else(|| {
                    is_leaf_top = true;
                });
            }
        }
    });
}

// TODO: Rename this to `Tree` when the name conflict is gone
struct ProceduralTree {
    branches: Vec<Branch>,
}

impl ProceduralTree {
    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut branches = Vec::new();

        fn add_branches(branches: &mut Vec<Branch>, rng: &mut impl Rng, start: Vec3<f32>, dir: Vec3<f32>, depth: usize) {
            let branch_dir = (dir + Vec3::<f32>::new(rng.gen_range(-1.0, 1.0),rng.gen_range(-1.0, 1.0),rng.gen_range(-0.3, 1.0)).cross(dir).normalized() * 0.45 * (depth as f32 + 0.5)).normalized(); // I wish `vek` had a `Vec3::from_fn`
            let branch_len = 12.0 / (depth as f32 * 0.25 + 1.0); // Zipf, I guess

            let end = start + branch_dir * branch_len;

            branches.push(Branch {
                line: LineSegment3 { start, end },
                radius: 0.3 + 2.5 / (depth + 1) as f32,
                health: if depth == 4 {
                    rng.gen_range(3.0, 5.0)
                } else {
                    0.0
                },
            });

            if depth < 4 {
                let sub_branches = if depth == 0 { 3 } else { rng.gen_range(2, 4) };
                for _ in 0..sub_branches {
                    add_branches(branches, rng, end, branch_dir, depth + 1);
                }
            }
        }

        let height = rng.gen_range(13, 30) as f32;
        let dx = rng.gen_range(-5, 5) as f32;
        let dy = rng.gen_range(-5, 5) as f32;

        // Generate the trunk
        branches.push(Branch {
            line: LineSegment3 { start: Vec3::zero(), end: Vec3::new(dx, dy, height)},
            radius: 3.0,
            health: 0.0,
        });

        // Generate branches
        let branches_count = rng.gen_range(7, 10);

        let angle_division = 360.0 / branches_count as f32;
        let angle_padding = rng.gen_range(0, 360) as f32;

        for i in 0..branches_count {
            for _ in 0..2 {
                let branch_size = height;

                let subdivision = rng.gen_range(1, 2);

                let branch_size = (0..subdivision).fold(branch_size, |x, _| x / 3.0 * 2.0);

                let radians = ((angle_padding
                    + angle_division * i as f32
                    + rng.gen_range(15.0, angle_division - 15.0))
                    % 360.0)
                    .to_radians();

                let branchendx = dx + branch_size * radians.cos();
                let branchendy = dy + branch_size * radians.sin();

                let height_dif = rng.gen_range(0.0, branch_size * 2.0);

                let trunk_margin = rng.gen_range(0.0, height / 3.0);

                add_branches(
                    &mut branches,
                    rng,
                    Vec3::new(dx,  dy, height - trunk_margin),
                    Vec3::new(branchendx,  branchendy, branch_size + height_dif).normalized(),
                    1
                );

                if rng.gen_range(0, 4) != 2 {
                    break;
                }
            }
        }

        Self {
            branches,
        }
    }

    pub fn get_bounds(&self) -> Aabb<f32> {
        let bounds = self.branches
            .iter()
            .fold(Aabb::default(), |Aabb { min, max }, branch| Aabb {
                min: Vec3::partial_min(min, Vec3::partial_min(branch.line.start, branch.line.end) - branch.radius - 8.0),
                max: Vec3::partial_max(max, Vec3::partial_max(branch.line.start, branch.line.end) + branch.radius + 8.0),
            });

        self.branches
            .iter()
            .for_each(|branch| {
                assert!(bounds.contains_point(branch.line.start));
                assert!(bounds.contains_point(branch.line.end));
            });

        bounds
    }

    pub fn is_branch_or_leaves_at(&self, pos: Vec3<f32>) -> (bool, bool) {
        let mut is_branch = false;
        let mut health = 0.0f32;
        for branch in &self.branches {
            let p_d2 = branch.line.projected_point(pos).distance_squared(pos);

            #[allow(unsafe_code)]
            fn finvsqrt(x: f32) -> f32 {
                // Magic number based on Chris Lomont work:
                // const MAGIC_U32: u32 = 0x5f375a86;
                // The Original Magic Number:
                // const MAGIC_32: u32 = 0x5f3759df;
                const threehalfs: f32 = 1.5f32;
                let x2: f32 = x * 0.5f32;
                let mut i: u32 = unsafe { std::mem::transmute(x) };// evil floating point bit level hacking
                i = 0x5f375a86 - (i >> 1); // what the fuck?
                let y: f32 = unsafe { std::mem::transmute(i) };
                let y  = y * ( threehalfs - ( x2 * y * y ) ); // 1st iteration
                // let y = y * (threehalfs - (x2 * y * y)); // 2nd iteration, this can be removed

                y
            }

            is_branch |= p_d2 < branch.radius.powi(2);
            health = health.max(branch.health * finvsqrt(p_d2));
        }
        (is_branch, health > 1.0)
    }
}

struct Branch {
    line: LineSegment3<f32>,
    radius: f32,
    health: f32,
}
