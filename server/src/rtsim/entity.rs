use super::*;
use common::{
    comp::inventory::loadout_builder::LoadoutBuilder,
    resources::Time,
    rtsim::{Memory, MemoryItem},
    store::Id,
    terrain::TerrainGrid,
};
use rand_distr::{Distribution, Normal};
use std::f32::consts::PI;
use tracing::warn;
use world::{
    civ::{Site, Track},
    util::RandomPerm,
    IndexRef, World,
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
const PERM_GENUS: u32 = 4;

impl Entity {
    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed + perm) }

    pub fn get_body(&self) -> comp::Body {
        match self.rng(PERM_GENUS).gen::<f32>() {
            // we want 5% airships, 45% birds, 50% humans
            x if x < 0.05 => comp::Body::Ship(comp::ship::Body::DefaultAirship),
            x if x < 0.50 => {
                let species = *(&comp::bird_medium::ALL_SPECIES)
                    .choose(&mut self.rng(PERM_SPECIES))
                    .unwrap();
                comp::bird_medium::Body::random_with(&mut self.rng(PERM_BODY), &species).into()
            },
            _ => {
                let species = *(&comp::humanoid::ALL_SPECIES)
                    .choose(&mut self.rng(PERM_SPECIES))
                    .unwrap();
                comp::humanoid::Body::random_with(&mut self.rng(PERM_BODY), &species).into()
            },
        }
    }

    pub fn get_name(&self) -> String {
        use common::{generation::get_npc_name, npc::NPC_NAMES};
        let npc_names = NPC_NAMES.read();
        match self.get_body() {
            comp::Body::BirdMedium(b) => {
                get_npc_name(&npc_names.bird_medium, b.species).to_string()
            },
            comp::Body::BirdSmall(_) => "Warbler".to_string(),
            comp::Body::Dragon(b) => get_npc_name(&npc_names.dragon, b.species).to_string(),
            comp::Body::Humanoid(b) => get_npc_name(&npc_names.humanoid, b.species).to_string(),
            comp::Body::Ship(_) => "Veloren Air".to_string(),
            //TODO: finish match as necessary
            _ => unimplemented!(),
        }
    }

    pub fn get_level(&self) -> u32 {
        (self.rng(PERM_LEVEL).gen::<f32>().powi(2) * 15.0).ceil() as u32
    }

    pub fn get_loadout(&self) -> comp::inventory::loadout::Loadout {
        let mut rng = self.rng(PERM_LOADOUT);
        let main_tool = comp::Item::new_from_asset_expect(
            (&[
                "common.items.weapons.sword.wood-2",
                "common.items.weapons.sword.starter",
                "common.items.weapons.sword.wood-0",
                "common.items.weapons.bow.starter",
                "common.items.weapons.bow.hardwood-2",
            ])
                .choose(&mut rng)
                .unwrap(),
        );

        let back = match rng.gen_range(0..5) {
            0 => Some(comp::Item::new_from_asset_expect(
                "common.items.armor.agile.back",
            )),
            1 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.backpack",
            )),
            2 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.backpack_blue",
            )),
            3 => Some(comp::Item::new_from_asset_expect(
                "common.items.npc_armor.back.leather_blue",
            )),
            _ => None,
        };

        let lantern = match rng.gen_range(0..3) {
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
            "common.items.npc_armor.chest.leather_blue",
        ));
        let pants = Some(comp::Item::new_from_asset_expect(
            "common.items.npc_armor.pants.leather_blue",
        ));
        let shoulder = Some(comp::Item::new_from_asset_expect(
            "common.items.armor.swift.shoulder",
        ));

        LoadoutBuilder::build_loadout(self.get_body(), Some(main_tool), None, None)
            .back(back)
            .lantern(lantern)
            .chest(chest)
            .pants(pants)
            .shoulder(shoulder)
            .build()
    }

    pub fn tick(&mut self, time: &Time, terrain: &TerrainGrid, world: &World, index: &IndexRef) {
        self.brain.route = match self.brain.route.clone() {
            Travel::Lost => {
                match self.get_body() {
                    comp::Body::Humanoid(_) => {
                        if let Some(nearest_site_id) = world
                            .civs()
                            .sites
                            .iter()
                            .filter(|s| s.1.is_settlement() || s.1.is_castle())
                            .min_by_key(|(_, site)| {
                                let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                                wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32
                            })
                            .map(|(id, _)| id)
                        {
                            // The path choosing code works best when Humanoids can assume they are
                            // in a town that has at least one path. If the Human isn't in a town
                            // with at least one path, we need to get them to a town that does.
                            let nearest_site = &world.civs().sites[nearest_site_id];
                            let site_wpos =
                                nearest_site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                            let dist =
                                site_wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                            if dist < 64_u32.pow(2) {
                                Travel::InSite {
                                    site_id: nearest_site_id,
                                }
                            } else {
                                Travel::Direct {
                                    target_id: nearest_site_id,
                                }
                            }
                        } else {
                            // Somehow no nearest site could be found
                            // Logically this should never happen, but if it does the rtsim entity
                            // will just sit tight
                            warn!("Nearest site could not be found");
                            Travel::Lost
                        }
                    },
                    comp::Body::Ship(_) => {
                        if let Some((target_id, site)) = world
                            .civs()
                            .sites
                            .iter()
                            .filter(|s| match self.get_body() {
                                comp::Body::Ship(_) => s.1.is_settlement(),
                                _ => s.1.is_dungeon(),
                            })
                            .filter(|_| thread_rng().gen_range(0i32..4) == 0)
                            .min_by_key(|(_, site)| {
                                let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                                let dist =
                                    wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                                dist + if dist < 96_u32.pow(2) { 100_000_000 } else { 0 }
                            })
                        {
                            let mut rng = thread_rng();
                            if let (Ok(normalpos), Ok(normaloff)) =
                                (Normal::new(0.0, 64.0), Normal::new(0.0, 256.0))
                            {
                                let mut path = Vec::<Vec2<i32>>::default();
                                let target_site_pos = site.center.map(|e| e as f32)
                                    * TerrainChunk::RECT_SIZE.map(|e| e as f32);
                                let offset_site_pos =
                                    target_site_pos.map(|v| v + normalpos.sample(&mut rng));
                                let offset_dir = (offset_site_pos - self.pos.xy()).normalized();
                                let dist = (offset_site_pos - self.pos.xy()).magnitude();
                                let midpoint = self.pos.xy() + offset_dir * (dist / 2.0);
                                let perp_dir = offset_dir.rotated_z(PI / 2.0);
                                let offset = normaloff.sample(&mut rng);
                                let inbetween_pos = midpoint + (perp_dir * offset);

                                path.push(inbetween_pos.map(|e| e as i32));
                                path.push(target_site_pos.map(|e| e as i32));

                                Travel::CustomPath {
                                    target_id,
                                    path,
                                    progress: 0,
                                }
                            } else {
                                Travel::Direct { target_id }
                            }
                        } else {
                            Travel::Lost
                        }
                    },
                    _ => {
                        if let Some(target_id) = world
                            .civs()
                            .sites
                            .iter()
                            .filter(|s| match self.get_body() {
                                comp::Body::Ship(_) => s.1.is_settlement(),
                                _ => s.1.is_dungeon(),
                            })
                            .filter(|_| thread_rng().gen_range(0i32..4) == 0)
                            .min_by_key(|(_, site)| {
                                let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                                let dist =
                                    wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                                dist + if dist < 96_u32.pow(2) { 100_000 } else { 0 }
                            })
                            .map(|(id, _)| id)
                        {
                            Travel::Direct { target_id }
                        } else {
                            Travel::Lost
                        }
                    },
                }
            },
            Travel::InSite { site_id } => {
                if !self.get_body().is_humanoid() {
                    // Non humanoids don't care if they start at a site
                    Travel::Lost
                } else if let Some(target_id) = world
                    .civs()
                    .neighbors(site_id)
                    .filter(|sid| {
                        let site = world.civs().sites.get(*sid);
                        let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                        let dist = wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                        dist > 96_u32.pow(2)
                    })
                    .filter(|sid| {
                        if let Some(last_visited) = self.brain.last_visited {
                            *sid != last_visited
                        } else {
                            true
                        }
                    })
                    .choose(&mut thread_rng())
                {
                    if let Some(track_id) = world.civs().track_between(site_id, target_id) {
                        self.brain.last_visited = Some(site_id);
                        Travel::Path {
                            target_id,
                            track_id,
                            progress: 0,
                            reversed: false,
                        }
                    } else {
                        // This should never trigger, since neighbors returns a list of sites for
                        // which a track exists going from the current town.
                        warn!("Could not get track after selecting from neighbor list");
                        self.brain.last_visited = Some(site_id);
                        Travel::Direct { target_id }
                    }
                } else if let Some(target_id) = world
                    .civs()
                    .sites
                    .iter()
                    .filter(|s| s.1.is_settlement() | s.1.is_castle())
                    .filter(|_| thread_rng().gen_range(0i32..4) == 0)
                    .min_by_key(|(_, site)| {
                        let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                        let dist = wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                        dist + if dist < 96_u32.pow(2) { 100_000 } else { 0 }
                    })
                    .map(|(id, _)| id)
                {
                    // This code should only trigger when no paths out of the current town exist.
                    // The traveller will attempt to directly travel to another town
                    self.brain.last_visited = Some(site_id);
                    Travel::Direct { target_id }
                } else {
                    // No paths we're picked, so stay in town. This will cause direct travel on the
                    // next tick.
                    self.brain.last_visited = Some(site_id);
                    Travel::InSite { site_id }
                }
            },
            Travel::Direct { target_id } => {
                let site = &world.civs().sites[target_id];
                let destination_name = site
                    .site_tmp
                    .map_or("".to_string(), |id| index.sites[id].name().to_string());

                let wpos = site.center * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                let dist = wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;

                if dist < 64_u32.pow(2) {
                    Travel::InSite { site_id: target_id }
                } else {
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

                    self.controller.travel_to = Some((travel_to, destination_name));
                    self.controller.speed_factor = 0.70;
                    Travel::Direct { target_id }
                }
            },
            Travel::CustomPath {
                target_id,
                path,
                progress,
            } => {
                let site = &world.civs().sites[target_id];
                let destination_name = site
                    .site_tmp
                    .map_or("".to_string(), |id| index.sites[id].name().to_string());

                if let Some(wpos) = &path.get(progress) {
                    let dist = wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;
                    if dist < 16_u32.pow(2) {
                        if progress + 1 < path.len() {
                            Travel::CustomPath {
                                target_id,
                                path,
                                progress: progress + 1,
                            }
                        } else {
                            Travel::InSite { site_id: target_id }
                        }
                    } else {
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

                        self.controller.travel_to = Some((travel_to, destination_name));
                        self.controller.speed_factor = 0.70;
                        Travel::CustomPath {
                            target_id,
                            path,
                            progress,
                        }
                    }
                } else {
                    Travel::Direct { target_id }
                }
            },
            Travel::Path {
                target_id,
                track_id,
                progress,
                reversed,
            } => {
                let track = &world.civs().tracks.get(track_id);
                let site = &world.civs().sites[target_id];
                let destination_name = site
                    .site_tmp
                    .map_or("".to_string(), |id| index.sites[id].name().to_string());
                let nth = if reversed {
                    track.path().len() - progress - 1
                } else {
                    progress
                };

                if let Some(sim_pos) = track.path().iter().nth(nth) {
                    let chunkpos = sim_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32);
                    let wpos = if let Some(pathdata) = world.sim().get_nearest_path(chunkpos) {
                        pathdata.1.map(|e| e as i32)
                    } else {
                        chunkpos
                    };
                    let dist = wpos.map(|e| e as f32).distance_squared(self.pos.xy()) as u32;

                    match dist {
                        d if d < 16_u32.pow(2) => {
                            if progress + 1 >= track.path().len() {
                                Travel::Direct { target_id }
                            } else {
                                Travel::Path {
                                    target_id,
                                    track_id,
                                    progress: progress + 1,
                                    reversed,
                                }
                            }
                        },
                        d if d > 256_u32.pow(2) => {
                            if !reversed && progress == 0 {
                                Travel::Path {
                                    target_id,
                                    track_id,
                                    progress: 0,
                                    reversed: true,
                                }
                            } else {
                                Travel::Lost
                            }
                        },
                        _ => {
                            let travel_to = self.pos.xy()
                                + Vec3::from(
                                    (wpos.map(|e| e as f32 + 0.5) - self.pos.xy())
                                        .try_normalized()
                                        .unwrap_or_else(Vec2::zero),
                                ) * 64.0;
                            let travel_to_alt = world
                                .sim()
                                .get_alt_approx(travel_to.map(|e| e as i32))
                                .unwrap_or(0.0)
                                as i32;
                            let travel_to = terrain
                                .find_space(Vec3::new(
                                    travel_to.x as i32,
                                    travel_to.y as i32,
                                    travel_to_alt,
                                ))
                                .map(|e| e as f32)
                                + Vec3::new(0.5, 0.5, 0.0);
                            self.controller.travel_to = Some((travel_to, destination_name));
                            self.controller.speed_factor = 0.70;
                            Travel::Path {
                                target_id,
                                track_id,
                                progress,
                                reversed,
                            }
                        },
                    }
                } else {
                    // This code should never trigger. If we've gone outside the bounds of the
                    // tracks vec then a logic bug has occured. I actually had
                    // an off by one error that caused this to trigger and
                    // resulted in travellers getting stuck in towns.
                    warn!("Progress out of bounds while following track");
                    Travel::Lost
                }
            },
        };

        // Forget old memories
        self.brain
            .memories
            .retain(|memory| memory.time_to_forget > time.0);
    }
}

#[derive(Clone, Debug)]
enum Travel {
    Lost,
    InSite {
        site_id: Id<Site>,
    },
    Direct {
        target_id: Id<Site>,
    },
    CustomPath {
        target_id: Id<Site>,
        path: Vec<Vec2<i32>>,
        progress: usize,
    },
    Path {
        target_id: Id<Site>,
        track_id: Id<Track>,
        progress: usize,
        reversed: bool,
    },
}

impl Default for Travel {
    fn default() -> Self { Self::Lost }
}

#[derive(Default)]
pub struct Brain {
    begin: Option<Id<Site>>,
    tgt: Option<Id<Site>>,
    route: Travel,
    last_visited: Option<Id<Site>>,
    memories: Vec<Memory>,
}

impl Brain {
    pub fn add_memory(&mut self, memory: Memory) { self.memories.push(memory); }

    pub fn remembers_mood(&self) -> bool {
        self.memories
            .iter()
            .any(|memory| matches!(&memory.item, MemoryItem::Mood { .. }))
    }

    pub fn set_mood(&mut self, memory: Memory) {
        if let MemoryItem::Mood { .. } = memory.item {
            if self.remembers_mood() {
                while let Some(position) = self
                    .memories
                    .iter()
                    .position(|mem| matches!(&mem.item, MemoryItem::Mood { .. }))
                {
                    self.memories.remove(position);
                }
            }
            self.add_memory(memory);
        };
    }

    pub fn get_mood(&self) -> Option<&Memory> {
        self.memories
            .iter()
            .find(|memory| matches!(&memory.item, MemoryItem::Mood { .. }))
    }

    pub fn remembers_character(&self, name_to_remember: &str) -> bool {
        self.memories.iter().any(|memory| matches!(&memory.item, MemoryItem::CharacterInteraction { name, .. } if name == name_to_remember))
    }

    pub fn remembers_fight_with_character(&self, name_to_remember: &str) -> bool {
        self.memories.iter().any(|memory| matches!(&memory.item, MemoryItem::CharacterFight { name, .. } if name == name_to_remember))
    }
}
