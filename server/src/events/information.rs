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
        last_exports: HashMap::new(),
        resources: HashMap::new(),
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
        site.economy.get_information(site_id)
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
