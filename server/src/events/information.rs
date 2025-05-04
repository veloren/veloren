use crate::client::Client;
use common::event::RequestSiteInfoEvent;
use common_net::msg::{ServerGeneral, world_msg::EconomyInfo};
#[cfg(feature = "plugins")]
use common_state::plugin::PluginMgr;
#[cfg(feature = "worldgen")]
use specs::ReadExpect;
use specs::{DispatcherBuilder, ReadStorage};
use std::collections::HashMap;
#[cfg(feature = "worldgen")]
use world::IndexOwned;

use super::{ServerEvent, event_dispatch};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<RequestSiteInfoEvent>(builder, &[]);
    #[cfg(feature = "plugins")]
    event_dispatch::<common::event::RequestPluginsEvent>(builder, &[]);
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
                let info = if let Some(site_id) = site_id
                    && let Some(economy) = index.sites.get(site_id).economy.as_ref()
                {
                    economy.get_information(site_id)
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

/// Send missing plugins to the client
#[cfg(feature = "plugins")]
impl ServerEvent for common::event::RequestPluginsEvent {
    type SystemData<'a> = (ReadExpect<'a, PluginMgr>, ReadStorage<'a, Client>);

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (plugin_mgr, clients): Self::SystemData<'_>,
    ) {
        for mut ev in events {
            let Some(client) = clients.get(ev.entity) else {
                continue;
            };

            for hash in ev.plugins.drain(..) {
                if let Some(plugin) = plugin_mgr.find(&hash) {
                    let buf = Vec::from(plugin.data_buf());
                    // TODO: @perf We could possibly make this more performant by caching prepared
                    // messages for each plugin.
                    client
                        .send(ServerGeneral::PluginData(buf))
                        .unwrap_or_else(|e| {
                            tracing::warn!("Error {e} sending plugin {hash:?} to client")
                        });
                }
            }
        }
    }
}
