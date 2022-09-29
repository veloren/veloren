use super::*;
use crate::{site2::gen::PrimitiveTransform, Land};
use common::terrain::{BiomeKind, Block, BlockKind};
use rand::prelude::*;
use vek::*;

use inline_tweak::tweak;

enum RoofKind {
    Crenelated,
    Hipped,
}

struct HeightenedViaduct {
    slope_inv: i32,
    bridge_start_offset: i32,
    vault_spacing: i32,
    vault_size: (i32, i32),
    side_vault_size: (i32, i32),
    holes: bool,
}

impl HeightenedViaduct {
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            slope_inv: rng.gen_range(6..=8),
            bridge_start_offset: rng.gen_range(5..=12),
            vault_spacing: rng.gen_range(3..=4),
            vault_size: *[(3, 16), (1, 4), (1, 4), (1, 4), (5, 32), (5, 32)].choose(rng).unwrap(),
            side_vault_size: *[(4, 5), (7, 10), (7, 10), (13, 20)].choose(rng).unwrap(),
            holes: rng.gen_bool(0.5),
        }
    }
}

enum BridgeKind {
    Flat,
    Tower(RoofKind),
    Short,
    HeightenedViaduct(HeightenedViaduct),
}

fn aabb(min: Vec3<i32>, max: Vec3<i32>) -> Aabb<i32> {
    let aabb = Aabb { min, max }.made_valid();
    Aabb {
        min: aabb.min,
        max: aabb.max + 1,
    }
}

fn render_short(bridge: &Bridge, painter: &Painter) {
    let (bridge_fill, edge_fill) = match bridge.biome {
        BiomeKind::Desert | BiomeKind::Savannah => (
            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(212, 191, 142))),
            Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(190))),
        ),
        _ => (
            Fill::Brick(BlockKind::Rock, Rgb::gray(70), 25),
            Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(130))),
        ),
    };

    let bridge_width = 3;

    let orth_dir = bridge.dir.orthogonal();

    let orthogonal = orth_dir.to_vec2();
    let forward = bridge.dir.to_vec2();

    let len = (bridge.start.xy() - bridge.end.xy())
        .map(|e| e.abs())
        .reduce_max();
    let inset = 4;

    let height = bridge.end.z + len / 3 - inset;

    let side = orthogonal * bridge_width;

    let remove = painter.vault(
        aabb(
            (bridge.start.xy() - side + forward * inset).with_z(bridge.start.z),
            (bridge.end.xy() + side - forward * inset).with_z(height),
        ),
        orth_dir,
    );

    let ramp_in = len / 3;

    let top = height + 2;

    let outset = 7;

    let up_ramp = |point: Vec3<i32>, dir: Dir, side_len: i32| {
        let forward = dir.to_vec2();
        let side = dir.orthogonal().to_vec2() * side_len;
        painter.ramp(
            aabb(
                (point.xy() - side - forward * (top - point.z - ramp_in + outset))
                    .with_z(bridge.start.z),
                (point.xy() + side + forward * ramp_in).with_z(top),
            ),
            top - point.z + 1 + outset,
            dir,
        )
    };

    let bridge_prim = |side_len: i32| {
        let side = orthogonal * side_len;
        painter
            .aabb(aabb(
                (bridge.start.xy() - side + forward * ramp_in).with_z(bridge.start.z),
                (bridge.end.xy() + side - forward * ramp_in).with_z(top),
            ))
            .union(up_ramp(bridge.start, bridge.dir, side_len).union(up_ramp(
                bridge.end,
                -bridge.dir,
                side_len,
            )))
    };

    let b = bridge_prim(bridge_width);

    let t = 4;
    b.union(
        painter.aabb(aabb(
            (bridge.start.xy() - side - forward * (top - bridge.start.z - ramp_in + outset))
                .with_z(bridge.start.z - t),
            (bridge.end.xy() + side + forward * (top - bridge.end.z - ramp_in + outset))
                .with_z(bridge.start.z),
        )),
    )
    .translate(Vec3::new(0, 0, t))
    .without(b)
    .clear();

    b.without(remove).fill(bridge_fill);

    let prim = bridge_prim(bridge_width + 1);

    prim.translate(Vec3::unit_z())
        .without(prim)
        .without(painter.aabb(aabb(
            bridge.start - side - forward * outset,
            (bridge.end.xy() + side + forward * outset).with_z(top + 1),
        )))
        .fill(edge_fill);
}

fn render_flat(bridge: &Bridge, painter: &Painter) {
    let rock = Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(50)));

    let dz = bridge.end.z - bridge.start.z;
    let orthogonal = bridge.dir.orthogonal().to_vec2();
    let forward = bridge.dir.to_vec2();
    let inset = 5;

    let len = (bridge.end.xy() - bridge.start.xy())
        .map(|e| e.abs())
        .reduce_max();
    let upset = bridge.end.z - bridge.start.z;

    let size = tweak!(8);
    let hole = painter
        .cylinder(aabb(
            bridge.center.with_z(bridge.end.z - 3 - bridge.width * 2) - Vec2::broadcast(size),
            bridge.center.with_z(bridge.end.z + 2) + Vec2::broadcast(size),
        ))
        .rotate_about(
            bridge.dir.orthogonal().from_z_mat3(),
            bridge
                .center
                .as_()
                .with_z(bridge.end.z as f32 - 1.5 - bridge.width as f32),
        )
        .scale(
            bridge.dir.abs().to_vec3().as_()
                * ((len as f32 - upset as f32 * tweak!(2.0)) / (size as f32 * 2.0) - 1.0)
                + 1.0,
        );

    painter
        .ramp(
            aabb(
                bridge.start.with_z(bridge.start.z - inset)
                    - orthogonal * bridge.width
                    - forward * inset,
                bridge.start.with_z(bridge.end.z) + orthogonal * bridge.width + forward * dz,
            ),
            dz + inset + 1,
            bridge.dir,
        )
        .union(
            painter
                .aabb(aabb(
                    bridge.start.with_z(bridge.end.z - 3 - size) - orthogonal * bridge.width
                        + forward * dz,
                    bridge.start.with_z(bridge.end.z)
                        + orthogonal * bridge.width
                        + forward * forward * (bridge.end.xy() - bridge.start.xy()),
                ))
                .without(hole),
        )
        .fill(rock);
}

fn render_heightened_viaduct(bridge: &Bridge, painter: &Painter, data: &HeightenedViaduct) {
    let rock = Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(50)));
    let light_rock = Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(130)));
    let orth_dir = bridge.dir.orthogonal();

    let orthogonal = orth_dir.to_vec2();
    let forward = bridge.dir.to_vec2();

    let slope_inv = data.slope_inv;

    let len = (bridge.start.xy() - bridge.end.xy())
        .map(|e| e.abs())
        .reduce_max();

    let bridge_start_z = bridge.end.z + data.bridge_start_offset;
    let bridge_top = bridge_start_z + len / slope_inv / 2;

    // painter.ramp(aabb(
    //     bridge.start - side,
    //     (bridge.start.xy() + side + forward * (bridge_start_z -
    // bridge.start.z)).with_z(bridge_start_z), ), bridge_start_z -
    // bridge.start.z, bridge.dir).fill(rock);

    let bridge_width = bridge.width;
    let side = orthogonal * bridge_width;

    let aabr = Aabr {
        min: bridge.start.xy() - side,
        max: bridge.end.xy() + side,
    }
    .made_valid();

    let [_start_aabr, rest] = bridge.dir.split_aabr(aabr, bridge_start_z - bridge.start.z);
    let [_end_aabr, bridge_aabr] = (-bridge.dir).split_aabr(rest, bridge_start_z - bridge.end.z);
    let under = bridge.center.z - 15;

    let bridge_prim = |bridge_width: i32| {
        let side = orthogonal * bridge_width;

        let aabr = Aabr {
            min: bridge.start.xy() - side,
            max: bridge.end.xy() + side,
        }
        .made_valid();

        let [start_aabr, rest] = bridge.dir.split_aabr(aabr, bridge_start_z - bridge.start.z);
        let [end_aabr, bridge_aabr] = (-bridge.dir).split_aabr(rest, bridge_start_z - bridge.end.z);
        let [bridge_start, bridge_end] = bridge
            .dir
            .split_aabr(bridge_aabr, bridge.dir.select(bridge_aabr.size()) / 2);

        let ramp_in_aabr = |aabr: Aabr<i32>, dir: Dir, zmin, zmax| {
            let inset = dir.select(aabr.size());
            painter.ramp(
                aabb(aabr.min.with_z(zmin), aabr.max.with_z(zmax)),
                inset,
                dir,
            )
        };

        ramp_in_aabr(start_aabr, bridge.dir, bridge.start.z, bridge_start_z)
            .union(
                ramp_in_aabr(end_aabr, -bridge.dir, bridge.end.z, bridge_start_z)
                    .union(ramp_in_aabr(
                        bridge_start,
                        bridge.dir,
                        bridge_start_z + 1,
                        bridge_top,
                    ))
                    .union(ramp_in_aabr(
                        bridge_end,
                        -bridge.dir,
                        bridge_start_z + 1,
                        bridge_top,
                    )),
            )
            .union(
                painter
                    .aabb(aabb(
                        start_aabr.min.with_z(under),
                        start_aabr.max.with_z(bridge.start.z - 1),
                    ))
                    .union(painter.aabb(aabb(
                        end_aabr.min.with_z(under),
                        end_aabr.max.with_z(bridge.end.z - 1),
                    ))),
            )
            .union(painter.aabb(aabb(
                bridge_aabr.min.with_z(under),
                bridge_aabr.max.with_z(bridge_start_z),
            )))
    };

    let b = bridge_prim(bridge_width - 1);
    let b = b.without(b.translate(-Vec3::unit_z()));

    let c = bridge_aabr.center();
    let len = bridge.dir.select(bridge_aabr.size());
    let vault_size = data.vault_size.0 * len / data.vault_size.1;
    let side_vault = data.side_vault_size.0 * vault_size / data.side_vault_size.1;
    let vertical = 5;
    let spacing = data.vault_spacing;
    let vault_top = bridge_top - vertical;
    let side_vault_top = vault_top - (vault_size + spacing + 1 + side_vault) / slope_inv;
    let side_vault_offset = vault_size + spacing + 1;

    let mut remove = painter.vault(
        aabb(
            (c - side - forward * vault_size).with_z(under),
            (c + side + forward * vault_size).with_z(vault_top),
        ),
        orth_dir,
    );

    if side_vault * 2 + side_vault_offset < len / 2 + 5 {
        remove = remove.union(
            painter
                .vault(
                    aabb(
                        (c - side + forward * side_vault_offset).with_z(under),
                        (c + side + forward * (side_vault * 2 + side_vault_offset))
                            .with_z(side_vault_top),
                    ),
                    orth_dir,
                )
                .union(
                    painter.vault(
                        aabb(
                            (c - side - forward * side_vault_offset).with_z(under),
                            (c + side - forward * (side_vault * 2 + side_vault_offset))
                                .with_z(side_vault_top),
                        ),
                        orth_dir,
                    ),
                ),
        );
        
        if data.holes {
            remove = remove.union(
                painter.vault(aabb(
                    (c - side + forward * (vault_size + 1)).with_z(side_vault_top - 4),
                    (c + side + forward * (vault_size + spacing)).with_z(side_vault_top + 2),
                ), orth_dir).union(
                    painter.vault(aabb(
                        (c - side - forward * (vault_size + 1)).with_z(side_vault_top - 4),
                        (c + side - forward * (vault_size + spacing)).with_z(side_vault_top + 2),
                    ), orth_dir)
                )
            );
        }
    }

    bridge_prim(bridge_width)
        .without(b)
        .without(remove)
        .fill(rock.clone());
    b.translate(-Vec3::unit_z()).fill(light_rock.clone());

    b.translate(Vec3::unit_z() * 5)
        .without(b.translate(-Vec3::unit_z()))
        .clear();
}

fn render_tower(bridge: &Bridge, painter: &Painter, roof_kind: &RoofKind) {
    let rock = Fill::Block(Block::new(BlockKind::Rock, Rgb::gray(50)));
    let wood = Fill::Block(Block::new(BlockKind::Wood, Rgb::new(40, 28, 20)));

    let tower_size = 5;

    let bridge_width = tower_size - 2;

    let orth_dir = bridge.dir.orthogonal();

    let orthogonal = orth_dir.to_vec2();
    let forward = bridge.dir.to_vec2();

    let tower_height_extend = 10;

    let tower_end = bridge.end.z + tower_height_extend;

    let tower_center = bridge.start.xy() + forward * tower_size;
    let tower_aabr = Aabr {
        min: tower_center - tower_size,
        max: tower_center + tower_size,
    };

    let len = (bridge.dir.select(bridge.end.xy()) - bridge.dir.select_aabr(tower_aabr)).abs() - 1;

    painter
        .aabb(aabb(
            tower_aabr.min.with_z(bridge.start.z - 5),
            tower_aabr.max.with_z(tower_end),
        ))
        .fill(rock.clone());

    painter
        .aabb(aabb(
            (tower_aabr.min + 1).with_z(bridge.start.z),
            (tower_aabr.max - 1).with_z(tower_end - 1),
        ))
        .clear();

    let c = (-bridge.dir).select_aabr_with(tower_aabr, tower_aabr.center());
    painter
        .aabb(aabb(
            (c - orthogonal).with_z(bridge.start.z),
            (c + orthogonal).with_z(bridge.start.z + 2),
        ))
        .clear();

    let ramp_height = 8;

    let ramp_aabb = aabb(
        (c - forward - orthogonal).with_z(bridge.start.z - 1),
        (c - forward * ramp_height + orthogonal).with_z(bridge.start.z + ramp_height - 2),
    );

    painter
        .aabb(ramp_aabb)
        .without(painter.ramp(ramp_aabb, ramp_height, -bridge.dir))
        .clear();

    let c = bridge.dir.select_aabr_with(tower_aabr, tower_aabr.center());
    painter
        .aabb(aabb(
            (c - orthogonal).with_z(bridge.end.z),
            (c + orthogonal).with_z(bridge.end.z + 2),
        ))
        .clear();

    let stair_thickness = 2;
    painter
        .staircase_in_aabb(
            aabb(
                (tower_aabr.min + 1).with_z(bridge.start.z),
                (tower_aabr.max - 1).with_z(bridge.end.z - 1),
            ),
            stair_thickness,
            bridge.dir.rotated_ccw(),
        )
        .fill(rock.clone());
    let aabr = bridge
        .dir
        .rotated_cw()
        .split_aabr(tower_aabr, stair_thickness + 1)[1];

    painter
        .aabb(aabb(
            aabr.min.with_z(bridge.end.z - 1),
            aabr.max.with_z(bridge.end.z - 1),
        ))
        .fill(rock.clone());

    painter
        .aabb(aabb(
            tower_aabr.center().with_z(bridge.start.z),
            tower_aabr.center().with_z(bridge.end.z - 1),
        ))
        .fill(rock.clone());

    let offset = tower_size * 2 - 2;
    let d = tweak!(2);
    let n = (bridge.end.z - bridge.start.z - d) / offset;
    let p = (bridge.end.z - bridge.start.z - d) / n;

    for i in 1..=n {
        let c = tower_aabr.center().with_z(bridge.start.z + i * p);

        for dir in Dir::ALL {
            painter.rotated_sprite(c + dir.to_vec2(), SpriteKind::WallSconce, dir.sprite_ori());
        }
    }

    painter.rotated_sprite(
        (tower_aabr.center() + bridge.dir.to_vec2() * (tower_size - 1))
            .with_z(bridge.end.z + tower_height_extend / 2),
        SpriteKind::WallLamp,
        (-bridge.dir).sprite_ori(),
    );

    match roof_kind {
        RoofKind::Crenelated => {
            painter
                .aabb(aabb(
                    (tower_aabr.min - 1).with_z(tower_end + 1),
                    (tower_aabr.max + 1).with_z(tower_end + 2),
                ))
                .fill(rock.clone());

            painter
                .aabbs_around_aabb(
                    aabb(
                        tower_aabr.min.with_z(tower_end + 3),
                        tower_aabr.max.with_z(tower_end + 3),
                    ),
                    1,
                    1,
                )
                .fill(rock.clone());

            painter
                .aabb(aabb(
                    tower_aabr.min.with_z(tower_end + 2),
                    tower_aabr.max.with_z(tower_end + 2),
                ))
                .clear();

            painter
                .aabbs_around_aabb(
                    aabb(
                        (tower_aabr.min + 1).with_z(tower_end + 2),
                        (tower_aabr.max - 1).with_z(tower_end + 2),
                    ),
                    1,
                    4,
                )
                .fill(Fill::Sprite(SpriteKind::FireBowlGround));
        },
        RoofKind::Hipped => {
            painter
                .pyramid(aabb(
                    (tower_aabr.min - 1).with_z(tower_end + 1),
                    (tower_aabr.max + 1).with_z(tower_end + 2 + tower_size),
                ))
                .fill(wood.clone());
        },
    }

    let offset = 15;
    let thickness = 3;

    let size = (offset - thickness) / 2;

    let n = len / offset;
    let p = len / n;

    let offset = forward * p;

    let size = bridge_width * orthogonal + forward * size;
    let start = bridge.dir.select_aabr_with(tower_aabr, tower_aabr.center()) + forward;
    painter
        .aabb(aabb(
            (start - orthogonal * bridge_width).with_z(bridge.center.z - 10),
            (bridge.end + orthogonal * bridge_width).with_z(bridge.end.z - 1),
        ))
        .without(
            painter
                .vault(
                    aabb(
                        (start + offset - size).with_z(bridge.center.z - 10),
                        (start + offset + size).with_z(bridge.end.z - 3),
                    ),
                    orth_dir,
                )
                .repeat(offset.with_z(0), n as u32),
        )
        .fill(rock.clone());

    painter
        .aabb(aabb(
            (start - orthogonal * bridge_width).with_z(bridge.end.z),
            (bridge.end + orthogonal * bridge_width).with_z(bridge.end.z + 5),
        ))
        .clear();

    let light_spacing = 10;
    let n = len / light_spacing;
    let p = len / n;

    let start = bridge.end;
    let offset = -forward * p;
    for i in 1..=n {
        let c = start + i * offset;

        painter.sprite(c + orthogonal * bridge_width, SpriteKind::StreetLamp);
        painter.sprite(c - orthogonal * bridge_width, SpriteKind::StreetLamp);
    }
}

pub struct Bridge {
    pub(crate) start: Vec3<i32>,
    pub(crate) end: Vec3<i32>,
    pub(crate) width: i32,
    pub(crate) dir: Dir,
    center: Vec3<i32>,
    kind: BridgeKind,
    biome: BiomeKind,
}

impl Bridge {
    pub fn generate(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        site: &Site,
        start: Vec2<i32>,
        end: Vec2<i32>,
    ) -> Self {
        let start = site.tile_wpos(start);
        let end = site.tile_wpos(end);

        let min_water_dist = 5;
        let find_edge = |start: Vec2<i32>, end: Vec2<i32>| {
            let mut test_start = start;
            let dir = Dir::from_vector(end - start).to_vec2();
            let mut last_alt = if let Some(col) = land.column_sample(start, index) {
                col.alt as i32
            } else {
                return test_start.with_z(land.get_alt_approx(start) as i32);
            };
            let mut step = 0;
            loop {
                if let Some(sample) = land.column_sample(test_start + step * dir, index) {
                    let alt = sample.alt as i32;
                    if last_alt - alt > 1 + (step + 3) / 4
                        || sample.riverless_alt - sample.alt > 2.0
                    {
                        break test_start.with_z(last_alt);
                    } else {
                        let water_dist = sample.water_dist.unwrap_or(16.0) as i32;

                        test_start += step * dir;

                        if water_dist <= min_water_dist {
                            break test_start.with_z(alt);
                        }

                        step = water_dist - min_water_dist;

                        last_alt = alt;
                    }
                } else {
                    break test_start.with_z(last_alt);
                }

                /*
                let alt = land.get_alt_approx(test_start + dir);
                let is_water = land.get_chunk_wpos(test_start + dir * 5).map_or(false, |chunk| chunk.river.river_kind.is_some());
                if is_water || last_alt - alt as i32 > 1 || start.z - alt as i32 > 5 {
                    break test_start.with_z(last_alt);
                }
                last_alt = alt as i32;

                test_start += dir;
                */
            }
        };

        let test_start = find_edge(start, end);

        let test_end = find_edge(end, start);

        let (start, end) = if test_start.z < test_end.z {
            (test_start, test_end)
        } else {
            (test_end, test_start)
        };

        let center = (start.xy() + end.xy()) / 2;
        let center = center.with_z(land.get_alt_approx(center) as i32);

        let len = (start.xy() - end.xy()).map(|e| e.abs()).reduce_max();

        Self {
            start,
            end,
            center,
            dir: Dir::from_vector(end.xy() - start.xy()),
            kind: if end.z - start.z > 17 {
                BridgeKind::Tower(match rng.gen_range(0..=2) {
                    0 => RoofKind::Crenelated,
                    _ => RoofKind::Hipped,
                })
            } else if len < 40 {
                BridgeKind::Short
            } else if end.z - start.z < 5 && start.z - center.z < 16 {
                BridgeKind::HeightenedViaduct(HeightenedViaduct::random(rng))
            } else {
                BridgeKind::Flat
            },
            width: 8,
            biome: land
                .get_chunk_wpos(center.xy())
                .map_or(BiomeKind::Void, |chunk| chunk.get_biome()),
        }
    }
}

impl Structure for Bridge {
    fn render(&self, _site: &Site, _land: &Land, painter: &Painter) {
        match &self.kind {
            BridgeKind::Flat => render_flat(self, painter),
            BridgeKind::Tower(roof) => render_tower(self, painter, roof),
            BridgeKind::Short => render_short(self, painter),
            BridgeKind::HeightenedViaduct(data) => render_heightened_viaduct(self, painter, data),
        }
    }
}
