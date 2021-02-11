use crate::{
    Server,
    comp::inventory::slot::InvSlotId,
    events::group_manip::handle_invite,
};
use common::{
    comp::{
        group::InviteKind,
    },
    trade::{Trades, TradeActionMsg, PendingTrade},
    uid::Uid,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use specs::{world::WorldExt, Entity as EcsEntity};
use tracing::warn;


pub fn handle_initiate_trade(server: &mut Server, interactor: EcsEntity, counterparty: EcsEntity) {
    if let Some(uid) = server.state_mut().ecs().uid_from_entity(counterparty) {
        handle_invite(server, interactor, uid, InviteKind::Trade);
    } else {
        warn!("Entity tried to trade with an entity that lacks an uid");
    }
}

pub fn handle_process_trade_action(server: &mut Server, entity: EcsEntity, trade_id: usize, msg: TradeActionMsg) {
    if let Some(uid) = server.state.ecs().uid_from_entity(entity) {
        let mut trades = server.state.ecs().write_resource::<Trades>();
        if let TradeActionMsg::Decline = msg {
            let to_notify = trades.decline_trade(trade_id, uid);
            to_notify
                .and_then(|u| server.state.ecs().entity_from_uid(u.0))
                .map(|e| server.notify_client(e, ServerGeneral::DeclinedTrade));
        } else {
            trades.process_trade_action(trade_id, uid, msg);
            if let Some(trade) = trades.trades.get(&trade_id) {
                if trade.should_commit() {
                    // TODO: inventory manip
                }
            }
        }
    }
}
