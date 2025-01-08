mod adlet;
mod airship_dock;
mod barn;
mod bridge;
mod camp;
mod castle;
mod citadel;
mod cliff_tower;
mod cliff_town_airship_dock;
mod coastal_airship_dock;
mod coastal_house;
mod coastal_workshop;
mod cultist;
mod desert_city_airship_dock;
mod desert_city_arena;
mod desert_city_multiplot;
mod desert_city_temple;
mod dwarven_mine;
mod farm_field;
mod giant_tree;
mod glider_finish;
mod glider_platform;
mod glider_ring;
mod gnarling;
mod haniwa;
mod house;
mod jungle_ruin;
mod myrmidon_arena;
mod myrmidon_house;
mod pirate_hideout;
mod rock_circle;
mod sahagin;
mod savannah_airship_dock;
mod savannah_hut;
mod savannah_workshop;
mod sea_chapel;
pub mod tavern;
mod terracotta_house;
mod terracotta_palace;
mod terracotta_yard;
mod troll_cave;
mod vampire_castle;
mod workshop;

pub use self::{
    adlet::AdletStronghold, airship_dock::AirshipDock, barn::Barn, bridge::Bridge, camp::Camp,
    castle::Castle, citadel::Citadel, cliff_tower::CliffTower,
    cliff_town_airship_dock::CliffTownAirshipDock, coastal_airship_dock::CoastalAirshipDock,
    coastal_house::CoastalHouse, coastal_workshop::CoastalWorkshop, cultist::Cultist,
    desert_city_airship_dock::DesertCityAirshipDock, desert_city_arena::DesertCityArena,
    desert_city_multiplot::DesertCityMultiPlot, desert_city_temple::DesertCityTemple,
    dwarven_mine::DwarvenMine, farm_field::FarmField, giant_tree::GiantTree,
    glider_finish::GliderFinish, glider_platform::GliderPlatform, glider_ring::GliderRing,
    gnarling::GnarlingFortification, haniwa::Haniwa, house::House, jungle_ruin::JungleRuin,
    myrmidon_arena::MyrmidonArena, myrmidon_house::MyrmidonHouse, pirate_hideout::PirateHideout,
    rock_circle::RockCircle, sahagin::Sahagin, savannah_airship_dock::SavannahAirshipDock,
    savannah_hut::SavannahHut, savannah_workshop::SavannahWorkshop, sea_chapel::SeaChapel,
    tavern::Tavern, terracotta_house::TerracottaHouse, terracotta_palace::TerracottaPalace,
    terracotta_yard::TerracottaYard, troll_cave::TrollCave, vampire_castle::VampireCastle,
    workshop::Workshop,
};

use super::*;
use crate::util::DHashSet;
use common::path::Path;
use vek::*;

pub struct Plot {
    pub(crate) kind: PlotKind,
    pub(crate) root_tile: Vec2<i32>,
    pub(crate) tiles: DHashSet<Vec2<i32>>,
    pub(crate) seed: u32,
}

impl Plot {
    pub fn find_bounds(&self) -> Aabr<i32> {
        self.tiles
            .iter()
            .fold(Aabr::new_empty(self.root_tile), |b, t| {
                b.expanded_to_contain_point(*t)
            })
    }

    pub fn z_range(&self) -> Option<Range<i32>> {
        match &self.kind {
            PlotKind::House(house) => Some(house.z_range()),
            _ => None,
        }
    }

    pub fn kind(&self) -> &PlotKind { &self.kind }

    pub fn root_tile(&self) -> Vec2<i32> { self.root_tile }

    pub fn tiles(&self) -> impl ExactSizeIterator<Item = Vec2<i32>> + '_ {
        self.tiles.iter().copied()
    }
}

#[derive(Debug, Clone)]
pub enum PlotKindMeta<'plot> {
    AirshipDock {
        door_tile: Vec2<i32>,
        docking_positions: &'plot Vec<Vec3<i32>>,
    },
    Workshop {
        door_tile: Option<Vec2<i32>>,
    },
    House {
        door_tile: Vec2<i32>,
    },
    Dungeon,
}

pub enum PlotKind {
    House(House),
    AirshipDock(AirshipDock),
    GliderRing(GliderRing),
    GliderPlatform(GliderPlatform),
    GliderFinish(GliderFinish),
    Tavern(Tavern),
    CoastalAirshipDock(CoastalAirshipDock),
    CoastalHouse(CoastalHouse),
    CoastalWorkshop(CoastalWorkshop),
    Workshop(Workshop),
    DesertCityMultiPlot(DesertCityMultiPlot),
    DesertCityTemple(DesertCityTemple),
    DesertCityArena(DesertCityArena),
    DesertCityAirshipDock(DesertCityAirshipDock),
    SeaChapel(SeaChapel),
    JungleRuin(JungleRuin),
    Plaza,
    Castle(Castle),
    Cultist(Cultist),
    Road(Path<Vec2<i32>>),
    Gnarling(GnarlingFortification),
    Adlet(AdletStronghold),
    Haniwa(Haniwa),
    GiantTree(GiantTree),
    CliffTower(CliffTower),
    CliffTownAirshipDock(CliffTownAirshipDock),
    Sahagin(Sahagin),
    Citadel(Citadel),
    SavannahAirshipDock(SavannahAirshipDock),
    SavannahHut(SavannahHut),
    SavannahWorkshop(SavannahWorkshop),
    Barn(Barn),
    Bridge(Bridge),
    PirateHideout(PirateHideout),
    RockCircle(RockCircle),
    TrollCave(TrollCave),
    Camp(Camp),
    DwarvenMine(DwarvenMine),
    TerracottaPalace(TerracottaPalace),
    TerracottaHouse(TerracottaHouse),
    TerracottaYard(TerracottaYard),
    FarmField(FarmField),
    VampireCastle(VampireCastle),
    MyrmidonArena(MyrmidonArena),
    MyrmidonHouse(MyrmidonHouse),
}

impl PlotKind {
    pub fn meta(&self) -> Option<PlotKindMeta<'_>> {
        match self {
            PlotKind::SavannahAirshipDock(d) => Some(PlotKindMeta::AirshipDock {
                door_tile: d.door_tile,
                docking_positions: &d.docking_positions,
            }),
            PlotKind::AirshipDock(d) => Some(PlotKindMeta::AirshipDock {
                door_tile: d.door_tile,
                docking_positions: &d.docking_positions,
            }),
            PlotKind::CoastalAirshipDock(d) => Some(PlotKindMeta::AirshipDock {
                door_tile: d.door_tile,
                docking_positions: &d.docking_positions,
            }),
            PlotKind::DesertCityAirshipDock(d) => Some(PlotKindMeta::AirshipDock {
                door_tile: d.door_tile,
                docking_positions: &d.docking_positions,
            }),
            PlotKind::CliffTownAirshipDock(d) => Some(PlotKindMeta::AirshipDock {
                door_tile: d.door_tile,
                docking_positions: &d.docking_positions,
            }),
            PlotKind::House(h) => Some(PlotKindMeta::House {
                door_tile: h.door_tile,
            }),
            PlotKind::CoastalHouse(h) => Some(PlotKindMeta::House {
                door_tile: h.door_tile,
            }),
            PlotKind::DesertCityTemple(h) => Some(PlotKindMeta::House {
                door_tile: h.door_tile,
            }),
            PlotKind::Sahagin(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::SavannahHut(h) => Some(PlotKindMeta::House {
                door_tile: h.door_tile,
            }),
            PlotKind::CoastalWorkshop(w) => Some(PlotKindMeta::Workshop {
                door_tile: Some(w.door_tile),
            }),
            PlotKind::Workshop(_) => Some(PlotKindMeta::Workshop { door_tile: None }),
            PlotKind::SavannahWorkshop(w) => Some(PlotKindMeta::Workshop {
                door_tile: Some(w.door_tile),
            }),
            PlotKind::SeaChapel(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::Cultist(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::Gnarling(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::Adlet(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::Haniwa(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::DwarvenMine(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::TerracottaPalace(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::VampireCastle(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::MyrmidonArena(_) => Some(PlotKindMeta::Dungeon),
            PlotKind::GliderRing(_)
            | PlotKind::GliderPlatform(_)
            | PlotKind::GliderFinish(_)
            | PlotKind::Tavern(_)
            | PlotKind::JungleRuin(_)
            | PlotKind::DesertCityArena(_)
            | PlotKind::DesertCityMultiPlot(_)
            | PlotKind::Plaza
            | PlotKind::Castle(_)
            | PlotKind::Road(_)
            | PlotKind::GiantTree(_)
            | PlotKind::CliffTower(_)
            | PlotKind::Citadel(_)
            | PlotKind::Barn(_)
            | PlotKind::Bridge(_)
            | PlotKind::PirateHideout(_)
            | PlotKind::RockCircle(_)
            | PlotKind::TrollCave(_)
            | PlotKind::Camp(_)
            | PlotKind::TerracottaHouse(_)
            | PlotKind::TerracottaYard(_)
            | PlotKind::FarmField(_)
            | PlotKind::MyrmidonHouse(_) => None,
        }
    }
}

#[macro_export]
macro_rules! foreach_plot {
    ($p:expr, $x:ident => $y:expr, $z:expr) => {
        match $p {
            PlotKind::House($x) => $y,
            PlotKind::AirshipDock($x) => $y,
            PlotKind::CoastalAirshipDock($x) => $y,
            PlotKind::CoastalHouse($x) => $y,
            PlotKind::CoastalWorkshop($x) => $y,
            PlotKind::Workshop($x) => $y,
            PlotKind::DesertCityAirshipDock($x) => $y,
            PlotKind::DesertCityMultiPlot($x) => $y,
            PlotKind::DesertCityTemple($x) => $y,
            PlotKind::DesertCityArena($x) => $y,
            PlotKind::SeaChapel($x) => $y,
            PlotKind::JungleRuin($x) => $y,
            PlotKind::Plaza => $z,
            PlotKind::Castle($x) => $y,
            PlotKind::Road(_) => $z,
            PlotKind::Gnarling($x) => $y,
            PlotKind::Adlet($x) => $y,
            PlotKind::GiantTree($x) => $y,
            PlotKind::CliffTower($x) => $y,
            PlotKind::CliffTownAirshipDock($x) => $y,
            PlotKind::Citadel($x) => $y,
            PlotKind::SavannahAirshipDock($x) => $y,
            PlotKind::SavannahHut($x) => $y,
            PlotKind::SavannahWorkshop($x) => $y,
            PlotKind::Barn($x) => $y,
            PlotKind::Bridge($x) => $y,
            PlotKind::PirateHideout($x) => $y,
            PlotKind::Tavern($x) => $y,
            PlotKind::Cultist($x) => $y,
            PlotKind::Haniwa($x) => $y,
            PlotKind::Sahagin($x) => $y,
            PlotKind::RockCircle($x) => $y,
            PlotKind::TrollCave($x) => $y,
            PlotKind::Camp($x) => $y,
            PlotKind::DwarvenMine($x) => $y,
            PlotKind::TerracottaPalace($x) => $y,
            PlotKind::TerracottaHouse($x) => $y,
            PlotKind::TerracottaYard($x) => $y,
            PlotKind::FarmField($x) => $y,
            PlotKind::VampireCastle($x) => $y,
            PlotKind::GliderRing($x) => $y,
            PlotKind::GliderPlatform($x) => $y,
            PlotKind::GliderFinish($x) => $y,
            PlotKind::MyrmidonArena($x) => $y,
            PlotKind::MyrmidonHouse($x) => $y,
        }
    };
}

pub use foreach_plot;
