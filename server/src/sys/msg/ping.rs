use crate::{client::Client, Settings};
use common::{
    event::{EventBus, ServerEvent},
    resources::Time,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::PingMsg;
use rayon::prelude::*;
use specs::{Entities, ParJoin, Read, WriteStorage};
use tracing::{debug, info};

impl Sys {
    fn handle_ping_msg(client: &Client, msg: PingMsg) -> Result<(), crate::error::Error> {
        match msg {
            PingMsg::Ping => client.send(PingMsg::Pong)?,
            PingMsg::Pong => {},
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        WriteStorage<'a, Client>,
        Read<'a, Settings>,
    );

    const NAME: &'static str = "msg::ping";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, server_event_bus, time, mut clients, settings): Self::SystemData,
    ) {
        (&entities, &mut clients).par_join().for_each_init(
            || server_event_bus.emitter(),
            |server_emitter, (entity, client)| {
                // ignore network events
                while let Some(Ok(Some(_))) =
                    client.participant.as_mut().map(|p| p.try_fetch_event())
                {}

                let res = super::try_recv_all(client, 4, Self::handle_ping_msg);

                match res {
                    Err(e) => {
                        debug!(?entity, ?e, "network error with client, disconnecting");
                        server_emitter.emit(ServerEvent::ClientDisconnect(
                            entity,
                            common::comp::DisconnectReason::NetworkError,
                        ));
                    },
                    Ok(1_u64..=u64::MAX) => {
                        // Update client ping.
                        client.last_ping = time.0
                    },
                    Ok(0) => {
                        let last_ping: f64 = client.last_ping;
                        if time.0 - last_ping > settings.client_timeout.as_secs() as f64
                        // Timeout
                        {
                            info!(?entity, "timeout error with client, disconnecting");
                            server_emitter.emit(ServerEvent::ClientDisconnect(
                                entity,
                                common::comp::DisconnectReason::Timeout,
                            ));
                        } else if time.0 - last_ping
                            > settings.client_timeout.as_secs() as f64 * 0.5
                        {
                            // Try pinging the client if the timeout is nearing.
                            client.send_fallible(PingMsg::Ping);
                        }
                    },
                }
            },
        );
    }
}
