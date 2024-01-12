use crate::{client::Client, events::DispatcherBuilder};
use common::event::{RequestPluginsEvent, RequestSiteInfoEvent};
use common_net::msg::{world_msg::EconomyInfo, ServerGeneral};
use specs::{DispatcherBuilder, ReadExpect, ReadStorage};
use std::collections::HashMap;
use world::IndexOwned;
#[cfg(feature = "plugins")]
use {common_state::plugin::PluginMgr, std::io::Read};

use super::{event_dispatch, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<RequestSiteInfoEvent>(builder);
    event_dispatch::<RequestPluginsEvent>(builder);
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

/// Send missing plugins to the client
#[cfg(feature = "plugins")]
impl ServerEvent for RequestPluginsEvent {
    type SystemData<'a> = (ReadExpect<'a, PluginMgr>, ReadStorage<'a, Client>);

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (plugin_mgr, clients): Self::SystemData<'_>,
    ) {
        for mut ev in events {
            for hash in ev.plugins.drain(..) {
                if let Some(plugin) = plugin_mgr.find(&hash) {
                    match std::fs::File::open(plugin.path()) {
                        Ok(mut reader) => {
                            let mut buf = Vec::new();
                            match reader.read_to_end(&mut buf) {
                                Ok(_) => {
                                    clients.get(ev.entity).map(|c| {
                                        c.send(ServerGeneral::PluginData(buf)).unwrap_or_else(|e| {
                                            tracing::warn!(
                                                "Error {e} sending plugin {hash:?} to client"
                                            )
                                        })
                                    });
                                },
                                Err(e) => {
                                    tracing::warn!(
                                        "Error {e} reading plugin file {:?}",
                                        plugin.path()
                                    );
                                },
                            }
                        },
                        Err(e) => {
                            tracing::warn!("Error {e} opening plugin file {:?}", plugin.path());
                        },
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "plugins"))]
impl ServerEvent for RequestPluginsEvent {
    type SystemData<'a> = ();

    fn handle(events: impl ExactSizeIterator<Item = Self>, _: Self::SystemData<'_>) {}
}
