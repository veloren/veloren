use super::super::SysTimer;
use crate::{
    client::Client,
    metrics::PlayerMetrics,
    streams::{GetStream, PingStream},
    Settings,
};
use common::{
    event::{EventBus, ServerEvent},
    msg::PingMsg,
    span,
    state::Time,
};
use specs::{Entities, Join, Read, ReadExpect, System, Write, WriteStorage};
use tracing::{debug, info};

impl Sys {
    fn handle_ping_msg(
        ping_stream: &mut PingStream,
        msg: PingMsg,
    ) -> Result<(), crate::error::Error> {
        match msg {
            PingMsg::Ping => ping_stream.send(PingMsg::Pong)?,
            PingMsg::Pong => {},
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, PlayerMetrics>,
        Write<'a, SysTimer<Self>>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, PingStream>,
        Read<'a, Settings>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            player_metrics,
            mut timer,
            mut clients,
            mut ping_streams,
            settings,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "msg::ping::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        for (entity, client, ping_stream) in (&entities, &mut clients, &mut ping_streams).join() {
            let res = super::try_recv_all(ping_stream, |ping_stream, msg| {
                Self::handle_ping_msg(ping_stream, msg)
            });

            match res {
                Err(e) => {
                    let reg = client.registered;
                    debug!(
                        ?entity,
                        ?e,
                        ?reg,
                        "network error with client, disconnecting"
                    );
                    if reg {
                        player_metrics
                            .clients_disconnected
                            .with_label_values(&["network_error"])
                            .inc();
                    }
                    server_emitter.emit(ServerEvent::ClientDisconnect(entity));
                },
                Ok(1_u64..=u64::MAX) => {
                    // Update client ping.
                    client.last_ping = time.0
                },
                Ok(0) => {
                    if time.0 - client.last_ping > settings.client_timeout.as_secs() as f64
                    // Timeout
                    {
                        let reg = client.registered;
                        info!(?entity, ?reg, "timeout error with client, disconnecting");
                        if reg {
                            player_metrics
                                .clients_disconnected
                                .with_label_values(&["timeout"])
                                .inc();
                        }
                        server_emitter.emit(ServerEvent::ClientDisconnect(entity));
                    } else if time.0 - client.last_ping
                        > settings.client_timeout.as_secs() as f64 * 0.5
                    {
                        // Try pinging the client if the timeout is nearing.
                        ping_stream.send_fallible(PingMsg::Ping);
                    }
                },
            }
        }

        timer.end()
    }
}
