use crate::{
    all::*,
    block::block_from_structure,
    column::ColumnGen,
    util::{RandomPerm, Sampler, UnitChooser},
    Canvas, CONFIG,
};
use common::{
    assets::AssetHandle,
    terrain::{
        structure::{Structure, StructureBlock, StructuresGroup},
        Block, BlockKind, SpriteKind,
    },
    vol::ReadVol,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{f32, ops::Range};
use vek::*;

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
pub fn apply_trees_to(canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
    // TODO: Get rid of this
    enum TreeModel {
        Structure(Structure),
        Procedural(ProceduralTree, StructureBlock),
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
        let trees = info.chunks().get_near_trees(wpos2d);

        for TreeAttr {
            pos,
            seed,
            scale,
            forest_kind,
            inhabited,
        } in trees
        {
            let tree = if let Some(tree) = tree_cache.entry(pos).or_insert_with(|| {
                let col = ColumnGen::new(info.chunks()).get((pos, info.index()))?;

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
                    pos: Vec3::new(pos.x, pos.y, col.alt as i32),
                    model: 'model: {
                        let models: AssetHandle<_> = if is_quirky {
                            if col.temp > CONFIG.desert_temp {
                                *QUIRKY_DRY
                            } else {
                                *QUIRKY
                            }
                        } else {
                            match forest_kind {
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
                                    break 'model TreeModel::Procedural(
                                        ProceduralTree::generate(
                                            TreeConfig::oak(&mut RandomPerm::new(seed), scale),
                                            &mut RandomPerm::new(seed),
                                        ),
                                        StructureBlock::TemperateLeaves,
                                    );
                                },
                                //ForestKind::Pine => *PINES,
                                ForestKind::Pine => {
                                    break 'model TreeModel::Procedural(
                                        ProceduralTree::generate(
                                            TreeConfig::pine(&mut RandomPerm::new(seed), scale),
                                            &mut RandomPerm::new(seed),
                                        ),
                                        StructureBlock::PineLeaves,
                                    );
                                },
                                ForestKind::Birch => *BIRCHES,
                                ForestKind::Mangrove => *MANGROVE_TREES,
                                ForestKind::Swamp => *SWAMP_TREES,
                                ForestKind::Giant => {
                                    break 'model TreeModel::Procedural(
                                        ProceduralTree::generate(
                                            TreeConfig::giant(
                                                &mut RandomPerm::new(seed),
                                                scale,
                                                inhabited,
                                            ),
                                            &mut RandomPerm::new(seed),
                                        ),
                                        StructureBlock::TemperateLeaves,
                                    );
                                },
                            }
                        };

                        let models = models.read();
                        TreeModel::Structure(
                            models[(MODEL_RAND.get(seed.wrapping_mul(17)) / 13) as usize
                                % models.len()]
                            .clone(),
                        )
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
                TreeModel::Procedural(t, _) => t.get_bounds().map(|e| e as i32),
            };

            let rpos2d = (wpos2d - tree.pos.xy())
                .map2(Vec2::new(tree.units.0, tree.units.1), |p, unit| unit * p)
                .sum();
            if !Aabr::from(bounds).contains_point(rpos2d) {
                // Skip this column
                continue;
            }

            let mut is_top = true;
            let mut is_leaf_top = true;
            let mut last_block = Block::empty();
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
                        TreeModel::Procedural(t, leaf_block) => Some(
                            match t.is_branch_or_leaves_at(model_pos.map(|e| e as f32 + 0.5)) {
                                (_, _, true, _) => {
                                    StructureBlock::Block(BlockKind::Wood, Rgb::new(110, 68, 22))
                                },
                                (_, _, _, true) => StructureBlock::None,
                                (true, _, _, _) => StructureBlock::Log,
                                (_, true, _, _) => *leaf_block,
                                _ => StructureBlock::None,
                            },
                        ),
                    } {
                        block
                    } else {
                        break;
                    },
                    wpos,
                    tree.pos.xy(),
                    tree.seed,
                    col,
                    Block::air,
                )
                .map(|block| {
                    // Add lights to the tree
                    if inhabited
                        && last_block.is_air()
                        && block.kind() == BlockKind::Wood
                        && dynamic_rng.gen_range(0..256) == 0
                    {
                        canvas.set(wpos + Vec3::unit_z(), Block::air(SpriteKind::Lantern));
                    // Add a snow covering to the block above under certain
                    // circumstances
                    } else if col.snow_cover
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
                    last_block = block;
                })
                .unwrap_or_else(|| {
                    if last_block.kind() == BlockKind::Wood && dynamic_rng.gen_range(0..512) == 0 {
                        canvas.set(wpos, Block::air(SpriteKind::Beehive));
                    }

                    is_leaf_top = true;
                    last_block = Block::empty();
                });
            }
        }
    });
}

/// A type that specifies the generation properties of a tree.
pub struct TreeConfig {
    /// Length of trunk, also scales other branches.
    pub trunk_len: f32,
    /// Radius of trunk, also scales other branches.
    pub trunk_radius: f32,
    // The scale that child branch lengths should be compared to their parents.
    pub branch_child_len: f32,
    // The scale that child branch radii should be compared to their parents.
    pub branch_child_radius: f32,
    /// The range of radii that leaf-emitting branches might have.
    pub leaf_radius: Range<f32>,
    /// 0 - 1 (0 = chaotic, 1 = straight).
    pub straightness: f32,
    /// Maximum number of branch layers (not including trunk).
    pub max_depth: usize,
    /// The number of branches that form from each branch.
    pub splits: Range<f32>,
    /// The range of proportions along a branch at which a split into another
    /// branch might occur. This value is clamped between 0 and 1, but a
    /// wider range may bias the results towards branch ends.
    pub split_range: Range<f32>,
    /// The bias applied to the length of branches based on the proportion along
    /// their parent that they eminate from. -1.0 = negative bias (branches
    /// at ends are longer, branches at the start are shorter) 0.0 = no bias
    /// (branches do not change their length with regard to parent branch
    /// proportion) 1.0 = positive bias (branches at ends are shorter,
    /// branches at the start are longer)
    pub branch_len_bias: f32,
    /// The scale of leaves in the vertical plane. Less than 1.0 implies a
    /// flattening of the leaves.
    pub leaf_vertical_scale: f32,
    /// How evenly spaced (vs random) sub-branches are along their parent.
    pub proportionality: f32,
    /// Whether the tree is inhabited (adds various features and effects)
    pub inhabited: bool,
}

impl TreeConfig {
    pub fn oak(rng: &mut impl Rng, scale: f32) -> Self {
        let scale = scale * (0.8 + rng.gen::<f32>().powi(4) * 0.75);
        let log_scale = 1.0 + scale.log2().max(0.0);

        Self {
            trunk_len: 9.0 * scale,
            trunk_radius: 2.0 * scale,
            branch_child_len: 0.9,
            branch_child_radius: 0.75,
            leaf_radius: 2.5 * log_scale..3.25 * log_scale,
            straightness: 0.45,
            max_depth: 4,
            splits: 2.25..3.25,
            split_range: 0.75..1.5,
            branch_len_bias: 0.0,
            leaf_vertical_scale: 1.0,
            proportionality: 0.0,
            inhabited: false,
        }
    }

    pub fn pine(rng: &mut impl Rng, scale: f32) -> Self {
        let scale = scale * (1.0 + rng.gen::<f32>().powi(4) * 0.5);
        let log_scale = 1.0 + scale.log2().max(0.0);

        Self {
            trunk_len: 32.0 * scale,
            trunk_radius: 1.25 * scale,
            branch_child_len: 0.35 / scale,
            branch_child_radius: 0.0,
            leaf_radius: 2.5 * log_scale..2.75 * log_scale,
            straightness: 0.0,
            max_depth: 1,
            splits: 40.0..50.0,
            split_range: 0.165..1.2,
            branch_len_bias: 0.75,
            leaf_vertical_scale: 0.3,
            proportionality: 1.0,
            inhabited: false,
        }
    }

    pub fn giant(_rng: &mut impl Rng, scale: f32, inhabited: bool) -> Self {
        let log_scale = 1.0 + scale.log2().max(0.0);

        Self {
            trunk_len: 11.0 * scale,
            trunk_radius: 6.0 * scale,
            branch_child_len: 0.9,
            branch_child_radius: 0.75,
            leaf_radius: 2.5 * scale..3.75 * scale,
            straightness: 0.36,
            max_depth: (7.0 + log_scale) as usize,
            splits: 1.5..2.5,
            split_range: 1.0..1.1,
            branch_len_bias: 0.0,
            leaf_vertical_scale: 0.6,
            proportionality: 0.0,
            inhabited,
        }
    }
}

// TODO: Rename this to `Tree` when the name conflict is gone
pub struct ProceduralTree {
    branches: Vec<Branch>,
    trunk_idx: usize,
}

impl ProceduralTree {
    /// Generate a new tree using the given configuration and seed.
    pub fn generate(config: TreeConfig, rng: &mut impl Rng) -> Self {
        let mut this = Self {
            branches: Vec::new(),
            trunk_idx: 0, // Gets replaced later
        };

        // Add the tree trunk (and sub-branches) recursively
        let (trunk_idx, _) = this.add_branch(
            &config,
            // Our trunk starts at the origin...
            Vec3::zero(),
            // ...and has a roughly upward direction
            Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 10.0).normalized(),
            config.trunk_len,
            config.trunk_radius,
            0,
            None,
            rng,
        );
        this.trunk_idx = trunk_idx;

        this
    }

    // Recursively add a branch (with sub-branches) to the tree's branch graph,
    // returning the index and AABB of the branch. This AABB gets propagated
    // down to the parent and is used later during sampling to cull the branches to
    // be sampled.
    #[allow(clippy::too_many_arguments)]
    fn add_branch(
        &mut self,
        config: &TreeConfig,
        start: Vec3<f32>,
        dir: Vec3<f32>,
        branch_len: f32,
        branch_radius: f32,
        depth: usize,
        sibling_idx: Option<usize>,
        rng: &mut impl Rng,
    ) -> (usize, Aabb<f32>) {
        let end = start + dir * branch_len;
        let line = LineSegment3 { start, end };
        let wood_radius = branch_radius;
        let leaf_radius = if depth == config.max_depth {
            rng.gen_range(config.leaf_radius.clone())
        } else {
            0.0
        };

        let has_stairs = config.inhabited
            && depth < config.max_depth
            && branch_radius > 6.5
            && start.xy().distance(end.xy()) < (start.z - end.z).abs() * 1.5;
        let bark_radius = if has_stairs { 5.0 } else { 0.0 } + wood_radius * 0.25;

        // The AABB that covers this branch, along with wood and leaves that eminate
        // from it
        let mut aabb = Aabb {
            min: Vec3::partial_min(start, end) - (wood_radius + bark_radius).max(leaf_radius),
            max: Vec3::partial_max(start, end) + (wood_radius + bark_radius).max(leaf_radius),
        };

        let mut child_idx = None;
        // Don't add child branches if we're already enough layers into the tree
        if depth < config.max_depth {
            let x_axis = dir
                .cross(Vec3::<f32>::zero().map(|_| rng.gen_range(-1.0..1.0)))
                .normalized();
            let y_axis = dir.cross(x_axis).normalized();
            let screw_shift = rng.gen_range(0.0..f32::consts::TAU);

            let splits = rng.gen_range(config.splits.clone()).round() as usize;
            for i in 0..splits {
                let dist = Lerp::lerp(
                    rng.gen_range(0.0..1.0),
                    i as f32 / (splits - 1) as f32,
                    config.proportionality,
                );

                const PHI: f32 = 0.618;
                const RAD_PER_BRANCH: f32 = f32::consts::TAU * PHI;
                let screw = (screw_shift + i as f32 * RAD_PER_BRANCH).sin() * x_axis
                    + (screw_shift + i as f32 * RAD_PER_BRANCH).cos() * y_axis;

                // Choose a point close to the branch to act as the target direction for the
                // branch to grow in let split_factor =
                // rng.gen_range(config.split_range.start, config.split_range.end).clamped(0.0,
                // 1.0);
                let split_factor =
                    Lerp::lerp(config.split_range.start, config.split_range.end, dist);
                let tgt = Lerp::lerp_unclamped(start, end, split_factor)
                    + Lerp::lerp(
                        Vec3::<f32>::zero().map(|_| rng.gen_range(-1.0..1.0)),
                        screw,
                        config.proportionality,
                    );
                // Start the branch at the closest point to the target
                let branch_start = line.projected_point(tgt);
                // Now, interpolate between the target direction and the parent branch's
                // direction to find a direction
                let branch_dir =
                    Lerp::lerp(tgt - branch_start, dir, config.straightness).normalized();

                let (branch_idx, branch_aabb) = self.add_branch(
                    config,
                    branch_start,
                    branch_dir,
                    branch_len
                        * config.branch_child_len
                        * (1.0
                            - (split_factor - 0.5)
                                * 2.0
                                * config.branch_len_bias.clamped(-1.0, 1.0)),
                    branch_radius * config.branch_child_radius,
                    depth + 1,
                    child_idx,
                    rng,
                );
                child_idx = Some(branch_idx);
                // Parent branches AABBs include the AABBs of child branches to allow for
                // culling during sampling
                aabb.expand_to_contain(branch_aabb);
            }
        }

        let idx = self.branches.len(); // Compute the index that this branch is going to have
        self.branches.push(Branch {
            line,
            wood_radius,
            leaf_radius,
            leaf_vertical_scale: config.leaf_vertical_scale,
            aabb,
            sibling_idx,
            child_idx,
            has_stairs,
        });

        (idx, aabb)
    }

    /// Get the bounding box that covers the tree (all branches and leaves)
    pub fn get_bounds(&self) -> Aabb<f32> { self.branches[self.trunk_idx].aabb }

    // Recursively search for branches or leaves by walking the tree's branch graph.
    fn is_branch_or_leaves_at_inner(
        &self,
        pos: Vec3<f32>,
        parent: &Branch,
        branch_idx: usize,
    ) -> (bool, bool, bool, bool) {
        let branch = &self.branches[branch_idx];
        // Always probe the sibling branch, since our AABB doesn't include its bounds
        // (it's not one of our children)
        let branch_or_leaves = branch
            .sibling_idx
            .map(|idx| Vec4::<bool>::from(self.is_branch_or_leaves_at_inner(pos, parent, idx)))
            .unwrap_or_default();

        // Only continue probing this sub-graph of the tree if the sample position falls
        // within its AABB
        if branch.aabb.contains_point(pos) {
            // Probe this branch
            let (this, _d2) = branch.is_branch_or_leaves_at(pos, parent);

            let siblings = branch_or_leaves | Vec4::from(this);

            // Probe the children of this branch
            let children = branch
                .child_idx
                .map(|idx| Vec4::<bool>::from(self.is_branch_or_leaves_at_inner(pos, branch, idx)))
                .unwrap_or_default();

            // Only allow empties for children if there is no solid at the current depth
            (siblings | children).into_tuple()
        } else {
            branch_or_leaves.into_tuple()
        }
    }

    /// Determine whether there are either branches or leaves at the given
    /// position in the tree.
    #[inline(always)]
    pub fn is_branch_or_leaves_at(&self, pos: Vec3<f32>) -> (bool, bool, bool, bool) {
        let (log, leaf, platform, air) =
            self.is_branch_or_leaves_at_inner(pos, &self.branches[self.trunk_idx], self.trunk_idx);
        (log /* & !air */, leaf & !air, platform & !air, air)
    }
}

// Branches are arranged in a graph shape. Each branch points to both its first
// child (if any) and also to the next branch in the list of child branches
// associated with the parent. This means that the entire tree is laid out in a
// walkable graph where each branch refers only to two other branches. As a
// result, walking the tree is simply a case of performing double recursion.
struct Branch {
    line: LineSegment3<f32>,
    wood_radius: f32,
    leaf_radius: f32,
    leaf_vertical_scale: f32,
    aabb: Aabb<f32>,

    sibling_idx: Option<usize>,
    child_idx: Option<usize>,

    has_stairs: bool,
}

impl Branch {
    /// Determine whether there are either branches or leaves at the given
    /// position in the branch.
    /// (branch, leaves, stairs, forced_air)
    pub fn is_branch_or_leaves_at(
        &self,
        pos: Vec3<f32>,
        parent: &Branch,
    ) -> ((bool, bool, bool, bool), f32) {
        // fn finvsqrt(x: f32) -> f32 {
        //     let y = f32::from_bits(0x5f375a86 - (x.to_bits() >> 1));
        //     y * (1.5 - ( x * 0.5 * y * y ))
        // }

        fn length_factor(line: LineSegment3<f32>, p: Vec3<f32>) -> f32 {
            let len_sq = line.start.distance_squared(line.end);
            if len_sq < 0.001 {
                0.0
            } else {
                (p - line.start).dot(line.end - line.start) / len_sq
            }
        }

        // fn smooth(a: f32, b: f32, k: f32) -> f32 {
        //     // let h = (0.5 + 0.5 * (b - a) / k).clamped(0.0, 1.0);
        //     // Lerp::lerp(b, a, h) - k * h * (1.0 - h)

        //     let h = (k-(a-b).abs()).max(0.0);
        //     a.min(b) - h * h * 0.25 / k
        // }

        let p = self.line.projected_point(pos);
        let d2 = p.distance_squared(pos);

        let length_factor = length_factor(self.line, pos);
        let wood_radius = Lerp::lerp(parent.wood_radius, self.wood_radius, length_factor);

        let mask = if d2 < wood_radius.powi(2) {
            (true, false, false, false) // Wood
        } else if {
            let diff = (p - pos) / Vec3::new(1.0, 1.0, self.leaf_vertical_scale);
            diff.magnitude_squared() < self.leaf_radius.powi(2)
        } {
            (false, true, false, false) // Leaves
        } else {
            let stair_width = 5.0;
            let stair_thickness = 2.0;
            let stair_space = 5.0;
            if self.has_stairs {
                let (platform, air) = if pos.z >= self.line.start.z.min(self.line.end.z) - 1.0
                    && pos.z
                        <= self.line.start.z.max(self.line.end.z) + stair_thickness + stair_space
                    && d2 < (wood_radius + stair_width).powi(2)
                {
                    let rpos = pos.xy() - p;
                    let stretch = 32.0;
                    let stair_section =
                        ((rpos.x as f32).atan2(rpos.y as f32) / (f32::consts::PI * 2.0) * stretch
                            + pos.z)
                            .rem_euclid(stretch);
                    (
                        stair_section < stair_thickness,
                        stair_section >= stair_thickness
                            && stair_section < stair_thickness + stair_space,
                    ) // Stairs
                } else {
                    (false, false)
                };

                let platform = platform
                    || (self.has_stairs
                        && self.wood_radius > 4.0
                        && !air
                        && d2 < (wood_radius + 10.0).powi(2)
                        && pos.z % 48.0 < stair_thickness);

                (false, false, platform, air)
            } else {
                (false, false, false, false)
            }
        };

        (mask, d2)
    }
}
