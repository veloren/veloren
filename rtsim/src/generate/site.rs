use crate::data::{FactionId, Factions, Site};
use common::store::Id;
use rand::prelude::*;
use vek::*;
use world::{
    IndexRef, World,
    site::{Site as WorldSite, SiteKind},
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
        let wpos = world_site.origin;

        // TODO: This is stupid, do better
        let good_or_evil = match &world_site.kind {
            // Good
            Some(
                SiteKind::Refactor
                | SiteKind::CliffTown
                | SiteKind::DesertCity
                | SiteKind::SavannahTown
                | SiteKind::CoastalTown
                | SiteKind::Citadel,
            ) => Some(true),
            // Evil
            Some(
                SiteKind::Myrmidon
                | SiteKind::ChapelSite
                | SiteKind::Terracotta
                | SiteKind::Gnarling
                | SiteKind::Cultist
                | SiteKind::Sahagin
                | SiteKind::PirateHideout
                | SiteKind::JungleRuin
                | SiteKind::RockCircle
                | SiteKind::TrollCave
                | SiteKind::Camp
                | SiteKind::Haniwa
                | SiteKind::Adlet
                | SiteKind::VampireCastle
                | SiteKind::DwarvenMine,
            ) => Some(false),
            // Neutral
            Some(SiteKind::GiantTree | SiteKind::GliderCourse | SiteKind::Bridge(..)) | None => {
                None
            },
        };

        Self {
            // This is assigned later
            uid: 0,
            seed: rng.random(),
            wpos,
            world_site: Some(world_site_id),
            faction: good_or_evil.and_then(|good_or_evil| {
                nearby_factions
                    .iter()
                    .filter(|(_, faction)| {
                        factions
                            .get(*faction)
                            .is_some_and(|f| f.good_or_evil == good_or_evil)
                    })
                    .min_by_key(|(faction_wpos, _)| {
                        faction_wpos
                            .as_::<i64>()
                            .distance_squared(wpos.as_::<i64>())
                    })
                    .map(|(_, faction)| *faction)
            }),
            count_loaded_chunks: 0,
            population: Default::default(),
            known_reports: Default::default(),
            nearby_sites_by_size: Vec::new(),
        }
    }
}
