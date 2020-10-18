use super::super::SysTimer;
use crate::{
    client::Client,
    login_provider::LoginProvider,
    metrics::PlayerMetrics,
    streams::{GeneralStream, GetStream, RegisterStream},
    EditableSettings,
};
use common::{
    comp::{Admin, Player, Stats},
    msg::{
        CharacterInfo, ClientRegister, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneral,
        ServerRegisterAnswer,
    },
    span,
    state::Time,
    sync::Uid,
};
use hashbrown::HashMap;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect, WriteStorage,
};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_register_msg(
        player_list: &HashMap<Uid, PlayerInfo>,
        new_players: &mut Vec<specs::Entity>,
        entity: specs::Entity,
        client: &mut Client,
        register_stream: &mut RegisterStream,
        general_stream: &mut GeneralStream,
        player_metrics: &ReadExpect<'_, PlayerMetrics>,
        login_provider: &mut WriteExpect<'_, LoginProvider>,
        admins: &mut WriteStorage<'_, Admin>,
        players: &mut WriteStorage<'_, Player>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        msg: ClientRegister,
    ) -> Result<(), crate::error::Error> {
        let (username, uuid) = match login_provider.try_login(
            &msg.token_or_username,
            &*editable_settings.admins,
            &*editable_settings.whitelist,
            &*editable_settings.banlist,
        ) {
            Err(err) => {
                register_stream.send(ServerRegisterAnswer::Err(err))?;
                return Ok(());
            },
            Ok((username, uuid)) => (username, uuid),
        };

        const INITIAL_VD: Option<u32> = Some(5); //will be changed after login
        let player = Player::new(username, None, INITIAL_VD, uuid);
        let is_admin = editable_settings.admins.contains(&uuid);

        if !player.is_valid() {
            // Invalid player
            register_stream.send(ServerRegisterAnswer::Err(RegisterError::InvalidCharacter))?;
            return Ok(());
        }

        if !client.registered && client.in_game.is_none() {
            // Add Player component to this client
            let _ = players.insert(entity, player);
            player_metrics.players_connected.inc();

            // Give the Admin component to the player if their name exists in
            // admin list
            if is_admin {
                let _ = admins.insert(entity, Admin);
            }

            // Tell the client its request was successful.
            client.registered = true;
            register_stream.send(ServerRegisterAnswer::Ok(()))?;

            // Send initial player list
            general_stream.send(ServerGeneral::PlayerListUpdate(PlayerListUpdate::Init(
                player_list.clone(),
            )))?;

            // Add to list to notify all clients of the new player
            new_players.push(entity);
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
        Read<'a, Time>,
        ReadExpect<'a, PlayerMetrics>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        WriteExpect<'a, LoginProvider>,
        WriteStorage<'a, Admin>,
        WriteStorage<'a, RegisterStream>,
        WriteStorage<'a, GeneralStream>,
        ReadExpect<'a, EditableSettings>,
    );

    fn run(
        &mut self,
        (
            entities,
            time,
            player_metrics,
            mut timer,
            uids,
            mut clients,
            mut players,
            stats,
            mut login_provider,
            mut admins,
            mut register_streams,
            mut general_streams,
            editable_settings,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "msg::register::Sys::run");
        timer.start();

        // Player list to send new players.
        let player_list = (&uids, &players, stats.maybe(), admins.maybe())
            .join()
            .map(|(uid, player, stats, admin)| {
                (*uid, PlayerInfo {
                    is_online: true,
                    is_admin: admin.is_some(),
                    player_alias: player.alias.clone(),
                    character: stats.map(|stats| CharacterInfo {
                        name: stats.name.clone(),
                        level: stats.level.level(),
                    }),
                })
            })
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        for (entity, client, register_stream, general_stream) in (
            &entities,
            &mut clients,
            &mut register_streams,
            &mut general_streams,
        )
            .join()
        {
            let res = super::try_recv_all(register_stream, |register_stream, msg| {
                Self::handle_register_msg(
                    &player_list,
                    &mut new_players,
                    entity,
                    client,
                    register_stream,
                    general_stream,
                    &player_metrics,
                    &mut login_provider,
                    &mut admins,
                    &mut players,
                    &editable_settings,
                    msg,
                )
            });

            if let Ok(1_u64..=u64::MAX) = res {
                // Update client ping.
                client.last_ping = time.0
            }
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let msg =
                    ServerGeneral::PlayerListUpdate(PlayerListUpdate::Add(*uid, PlayerInfo {
                        player_alias: player.alias.clone(),
                        is_online: true,
                        is_admin: admins.get(entity).is_some(),
                        character: None, // new players will be on character select.
                    }));
                for (_, general_stream) in (&mut clients, &mut general_streams)
                    .join()
                    .filter(|(c, _)| c.registered)
                {
                    let _ = general_stream.send(msg.clone());
                }
            }
        }

        timer.end()
    }
}
