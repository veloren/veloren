use crate::data::{FactionId, Factions, Site};
use common::store::Id;
use rand::prelude::*;
use vek::*;
use world::{
    site::{Site as WorldSite, SiteKind},
    IndexRef, World,
};

impl Site {
    pub fn generate(
        world_site_id: Id<WorldSite>,
        _world: &World,
        index: IndexRef,
        nearby_factions: &[(Vec2<i32>, FactionId)],
        factions: &Factions,
        rng: &mut impl Rng,
    ) -> Self {
        let world_site = index.sites.get(world_site_id);
        let wpos = world_site.get_origin();

        // TODO: This is stupid, do better
        let good_or_evil = match &world_site.kind {
            // Good
            SiteKind::Refactor(_)
            | SiteKind::CliffTown(_)
            | SiteKind::DesertCity(_)
            | SiteKind::SavannahPit(_)
            | SiteKind::CoastalTown(_) => Some(true),
            // Evil
            SiteKind::Dungeon(_)
            | SiteKind::ChapelSite(_)
            | SiteKind::Terracotta(_)
            | SiteKind::Gnarling(_)
            | SiteKind::PirateHideout(_)
            | SiteKind::JungleRuin(_)
            | SiteKind::RockCircle(_)
            | SiteKind::TrollCave(_)
            | SiteKind::Camp(_)
            | SiteKind::Haniwa(_)
            | SiteKind::Adlet(_) => Some(false),
            SiteKind::DwarvenMine(_) => Some(false),
            // Neutral
            SiteKind::Settlement(_)
            | SiteKind::Castle(_)
            | SiteKind::Tree(_)
            | SiteKind::GiantTree(_)
            | SiteKind::Bridge(_) => None,
        };

        Self {
            seed: rng.gen(),
            wpos,
            world_site: Some(world_site_id),
            faction: good_or_evil.and_then(|good_or_evil| {
                nearby_factions
                    .iter()
                    .filter(|(_, faction)| {
                        factions
                            .get(*faction)
                            .map_or(false, |f| f.good_or_evil == good_or_evil)
                    })
                    .min_by_key(|(faction_wpos, _)| {
                        faction_wpos
                            .as_::<i64>()
                            .distance_squared(wpos.as_::<i64>())
                    })
                    .map(|(_, faction)| *faction)
            }),
            population: Default::default(),
            known_reports: Default::default(),
            nearby_sites_by_size: Vec::new(),
        }
    }
}
