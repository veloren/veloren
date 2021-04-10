use crate::{client::Client, Server};
use common_net::msg::{world_msg::EconomyInfo, ServerGeneral};
use specs::{Entity as EcsEntity, WorldExt};
use std::collections::HashMap;

#[cfg(not(feature = "worldgen"))]
pub fn handle_site_info(server: &Server, entity: EcsEntity, id: u64) {
    let info = EconomyInfo {
        id,
        population: 0,
        stock: HashMap::new(),
        labor_values: HashMap::new(),
        values: HashMap::new(),
        labors: Vec::new(),
    };
    let msg = ServerGeneral::SiteEconomy(info);
    server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(entity)
        .map(|c| c.send(msg));
}

#[cfg(feature = "worldgen")]
pub fn handle_site_info(server: &Server, entity: EcsEntity, id: u64) {
    let site_id = server.index.sites.recreate_id(id);
    let info = if let Some(site_id) = site_id {
        let site = server.index.sites.get(site_id);
        EconomyInfo {
            id,
            population: site.economy.pop.floor() as u32,
            stock: site.economy.stocks.iter().map(|(g, a)| (g, *a)).collect(),
            labor_values: site
                .economy
                .labor_values
                .iter()
                .filter_map(|(g, a)| a.map(|a| (g, a)))
                .collect(),
            values: site
                .economy
                .values
                .iter()
                .filter_map(|(g, a)| a.map(|a| (g, a)))
                .collect(),
            labors: site.economy.labors.iter().map(|(_, a)| (*a)).collect(),
            last_exports: site
                .economy
                .last_exports
                .iter()
                .map(|(g, a)| (g, *a))
                .collect(),
            resources: site
                .economy
                .natural_resources
                .chunks_per_resource
                .iter()
                .map(|(g, a)| {
                    (
                        g,
                        ((*a) as f32) * site.economy.natural_resources.average_yield_per_chunk[g],
                    )
                })
                .collect(),
        }
    } else {
        EconomyInfo {
            id,
            population: 0,
            stock: HashMap::new(),
            labor_values: HashMap::new(),
            values: HashMap::new(),
            labors: Vec::new(),
            last_exports: HashMap::new(),
            resources: HashMap::new(),
        }
    };
    let msg = ServerGeneral::SiteEconomy(info);
    server
        .state
        .ecs()
        .read_storage::<Client>()
        .get(entity)
        .map(|c| c.send(msg));
}
