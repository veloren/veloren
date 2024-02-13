use crate::client::Client;
use common::event::RequestSiteInfoEvent;
use common_net::msg::{world_msg::EconomyInfo, ServerGeneral};
use specs::{DispatcherBuilder, ReadExpect, ReadStorage};
use std::collections::HashMap;
use world::IndexOwned;

use super::{event_dispatch, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<RequestSiteInfoEvent>(builder);
}

#[cfg(not(feature = "worldgen"))]
impl ServerEvent for RequestSiteInfoEvent {
    type SystemData<'a> = ReadStorage<'a, Client>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, clients: Self::SystemData<'_>) {
        for ev in events {
            if let Some(client) = clients.get(ev.entity) {
                let info = EconomyInfo {
                    id: ev.id,
                    population: 0,
                    stock: HashMap::new(),
                    labor_values: HashMap::new(),
                    values: HashMap::new(),
                    labors: Vec::new(),
                    last_exports: HashMap::new(),
                    resources: HashMap::new(),
                };
                let msg = ServerGeneral::SiteEconomy(info);
                client.send_fallible(msg);
            }
        }
    }
}

#[cfg(feature = "worldgen")]
impl ServerEvent for RequestSiteInfoEvent {
    type SystemData<'a> = (ReadExpect<'a, IndexOwned>, ReadStorage<'a, Client>);

    fn handle(events: impl ExactSizeIterator<Item = Self>, (index, clients): Self::SystemData<'_>) {
        for ev in events {
            if let Some(client) = clients.get(ev.entity) {
                let site_id = index.sites.recreate_id(ev.id);
                let info = if let Some(site_id) = site_id {
                    let site = index.sites.get(site_id);
                    site.economy.get_information(site_id)
                } else {
                    EconomyInfo {
                        id: ev.id,
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
                client.send_fallible(msg);
            }
        }
    }
}
