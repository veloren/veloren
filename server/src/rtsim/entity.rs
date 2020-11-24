use super::*;
use common::{comp::item::tool::AbilityMap, store::Id, terrain::TerrainGrid, LoadoutBuilder};
use world::{
    civ::{Site, Track},
    util::RandomPerm,
    World,
};

pub struct Entity {
    pub is_loaded: bool,
    pub pos: Vec3<f32>,
    pub seed: u32,
    pub last_tick: u64,
    pub controller: RtSimController,

    pub brain: Brain,
}

const PERM_SPECIES: u32 = 0;
const PERM_BODY: u32 = 1;
const PERM_LOADOUT: u32 = 2;
const PERM_LEVEL: u32 = 3;

impl Entity {
    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed + perm) }

    pub fn get_body(&self) -> comp::Body {
        let species = *(&comp::humanoid::ALL_SPECIES)
            .choose(&mut self.rng(PERM_SPECIES))
            .unwrap();
        comp::humanoid::Body::random_with(&mut self.rng(PERM_BODY), &species).into()
    }

    pub fn get_level(&self) -> u32 {
        (self.rng(PERM_LEVEL).gen::<f32>().powf(2.0) * 15.0).ceil() as u32
    }

    pub fn get_loadout(&self, ability_map: &AbilityMap) -> comp::Loadout {
        let mut rng = self.rng(PERM_LOADOUT);
        let main_tool = comp::Item::new_from_asset_expect(
            (&[
                "common.items.weapons.sword.wood_sword",
                "common.items.weapons.sword.starter_sword",
                "common.items.weapons.sword.short_sword_0",
                "common.items.weapons.bow.starter_bow",
                "common.items.weapons.bow.leafy_longbow-0",
            ])
                .choose(&mut rng)
                .unwrap(),
        );

        let back = match rng.gen_range(0, 5) {
            0 => Some(comp::Item::new_from_asset_expect(
                "common.items.armor.back.leather_adventurer",
            )),
            1 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.backpack_0",
            )),
            2 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.backpack_blue_0",
            )),
            3 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.leather_blue_0",
            )),
            _ => None,
        };

        let lantern = match rng.gen_range(0, 3) {
            0 => Some(comp::Item::new_from_asset_expect(
                "common.items.lantern.black_0",
            )),
            1 => Some(comp::Item::new_from_asset_expect(
                "common.items.lantern.blue_0",
            )),
            _ => Some(comp::Item::new_from_asset_expect(
                "common.items.lantern.red_0",
            )),
        };

        let chest = Some(comp::Item::new_from_asset_expect(
            "common.items.npc_armor.chest.leather_blue_0",
        ));
        let pants = Some(comp::Item::new_from_asset_expect(
            "common.items.npc_armor.pants.leather_blue_0",
        ));
        let shoulder = Some(comp::Item::new_from_asset_expect(
            "common.items.armor.shoulder.leather_0",
        ));

        LoadoutBuilder::build_loadout(self.get_body(), Some(main_tool), ability_map, None)
            .back(back)
            .lantern(lantern)
            .chest(chest)
            .pants(pants)
            .shoulder(shoulder)
            .build()
    }

    pub fn tick(&mut self, terrain: &TerrainGrid, world: &World) {
        let tgt_site = self.brain.tgt.or_else(|| {
            world
                .civs()
                .sites
                .iter()
                .filter(|_| thread_rng().gen_range(0i32, 4) == 0)
                .min_by_key(|(_, site)| {
                    let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                    let dist = wpos.map(|e| e as f32).distance(self.pos.xy()) as u32;
                    dist + if dist < 96 { 100_000 } else { 0 }
                })
                .map(|(id, _)| id)
        });
        self.brain.tgt = tgt_site;

        tgt_site.map(|tgt_site| {
            let site = &world.civs().sites[tgt_site];

            let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
            let dist = wpos.map(|e| e as f32).distance(self.pos.xy()) as u32;

            if dist < 64 {
                self.brain.tgt = None;
            }

            let travel_to = self.pos.xy()
                + Vec3::from(
                    (wpos.map(|e| e as f32 + 0.5) - self.pos.xy())
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero),
                ) * 64.0;
            let travel_to_alt = world
                .sim()
                .get_alt_approx(travel_to.map(|e| e as i32))
                .unwrap_or(0.0) as i32;
            let travel_to = terrain
                .find_space(Vec3::new(
                    travel_to.x as i32,
                    travel_to.y as i32,
                    travel_to_alt,
                ))
                .map(|e| e as f32)
                + Vec3::new(0.5, 0.5, 0.0);
            self.controller.travel_to = Some(travel_to);
            self.controller.speed_factor = 0.70;
        });
    }
}

#[derive(Default)]
pub struct Brain {
    tgt: Option<Id<Site>>,
    track: Option<(Track, usize)>,
}
