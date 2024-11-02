use crate::{client::Client, EditableSettings};
use common::{
    comp::Player,
    event::{ClientDisconnectEvent, EventBus},
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{DisconnectReason, ServerGeneral};
use network::ParticipantEvent;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, WriteStorage};

/// This system consumes events from the `Participant::try_fetch_event`. These
/// currently indicate channels being created and destroyed which potentially
/// corresponds to the client using new addresses.
///
/// New addresses are checked against the existing IP bans. If a match is found
/// that client will be kicked. Otherwise, the IP is added to the set of IPs
/// that client has used. When a new IP ban is created, the set of IP addrs used
/// by each client is scanned and any clients with matches are kicked.
///
/// We could retain addresses of removed channels and use them when banning but
/// that would use a potentially unknown amount of memory (so they are removed).
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Client>,
        Read<'a, EventBus<ClientDisconnectEvent>>,
        ReadExpect<'a, EditableSettings>,
    );

    const NAME: &'static str = "msg::network_events";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, players, mut clients, client_disconnect_event_bus, editable_settings): Self::SystemData,
    ) {
        let now = chrono::Utc::now();
        let mut client_disconnect_emitter = client_disconnect_event_bus.emitter();

        for (entity, client) in (&entities, &mut clients).join() {
            while let Some(Ok(Some(event))) = client
                .participant
                .as_mut()
                .map(|participant| participant.try_fetch_event())
            {
                match event {
                    ParticipantEvent::ChannelCreated(connect_addr) => {
                        // Ignore mpsc connections
                        if let Some(addr) = connect_addr.socket_addr() {
                            client.current_ip_addrs.push(addr);

                            let banned = editable_settings
                                .banlist
                                .get_ip_ban(addr.ip())
                                .and_then(|ban_entry| ban_entry.current.action.ban())
                                .and_then(|ban| {
                                    // Hardcoded admins can always log in.
                                    let admin = players.get(entity).and_then(|player| {
                                        editable_settings.admins.get(&player.uuid())
                                    });
                                    crate::login_provider::ban_applies(ban, admin, now)
                                        .then(|| ban.info())
                                });

                            if let Some(ban_info) = banned {
                                // Kick client
                                client_disconnect_emitter.emit(ClientDisconnectEvent(
                                    entity,
                                    common::comp::DisconnectReason::Kicked,
                                ));
                                let _ = client.send(ServerGeneral::Disconnect(
                                    DisconnectReason::Banned(ban_info),
                                ));
                            }
                        }
                    },
                    ParticipantEvent::ChannelDeleted(connect_addr) => {
                        // Ignore mpsc connections
                        if let Some(addr) = connect_addr.socket_addr() {
                            if let Some(i) = client
                                .current_ip_addrs
                                .iter()
                                .rev()
                                .position(|a| *a == addr)
                            {
                                client.current_ip_addrs.remove(i);
                            } else {
                                tracing::error!(
                                    "Channel deleted but its address isn't present in \
                                     client.current_ip_addrs!"
                                );
                            }
                        }
                    },
                }
            }
        }
    }
}
