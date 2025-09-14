pub use common::event::{
    AuraEvent, BonkEvent, BuffEvent, ChangeAbilityEvent, ChangeBodyEvent, ChangeStanceEvent,
    ChatEvent, ClientDisconnectEvent, ClientDisconnectWithoutPersistenceEvent, ComboChangeEvent,
    CommandEvent, CreateAuraEntityEvent, CreateItemDropEvent, CreateNpcEvent, CreateNpcGroupEvent,
    CreateObjectEvent, CreateShipEvent, CreateSpecialEntityEvent, CreateSpriteEvent,
    DeleteCharacterEvent, DeleteEvent, DestroyEvent, DialogueEvent, DownedEvent, EnergyChangeEvent,
    EntityAttackedHookEvent, EventBus, ExitIngameEvent, ExplosionEvent, GroupManipEvent,
    HealthChangeEvent, HelpDownedEvent, InitializeCharacterEvent, InitializeSpectatorEvent,
    InitiateInviteEvent, InventoryManipEvent, InviteResponseEvent, KillEvent, KnockbackEvent,
    LandOnGroundEvent, MakeAdminEvent, MineBlockEvent, MountEvent, NpcInteractEvent,
    ParryHookEvent, PoiseChangeEvent, PossessEvent, ProcessTradeActionEvent, RegrowHeadEvent,
    RemoveLightEmitterEvent, RequestPluginsEvent, RequestSiteInfoEvent, RespawnEvent,
    SetBattleModeEvent, SetLanternEvent, SetPetStayEvent, ShockwaveEvent, ShootEvent, SoundEvent,
    StartInteractionEvent, StartTeleportingEvent, SummonBeamPillarsEvent, TamePetEvent,
    TeleportToEvent, TeleportToPositionEvent, ThrowEvent, ToggleSpriteLightEvent, TransformEvent,
    UpdateCharacterDataEvent, UpdateMapMarkerEvent,
};

/// X-macro that provides list of server events to the macro this is called
/// with.
macro_rules! server_events {
    ($macro:ident) => {
        $macro! {
            ClientDisconnectEvent
            ClientDisconnectWithoutPersistenceEvent
            ChatEvent
            CommandEvent
            CreateSpecialEntityEvent
            CreateNpcEvent
            CreateNpcGroupEvent
            CreateShipEvent
            CreateItemDropEvent
            CreateObjectEvent
            ExplosionEvent
            BonkEvent
            HealthChangeEvent
            KillEvent
            HelpDownedEvent
            DownedEvent
            PoiseChangeEvent
            DeleteEvent
            DestroyEvent
            InventoryManipEvent
            GroupManipEvent
            RespawnEvent
            ShootEvent
            ThrowEvent
            ShockwaveEvent
            KnockbackEvent
            LandOnGroundEvent
            SetLanternEvent
            NpcInteractEvent
            DialogueEvent
            InviteResponseEvent
            InitiateInviteEvent
            ProcessTradeActionEvent
            MountEvent
            SetPetStayEvent
            PossessEvent
            InitializeCharacterEvent
            InitializeSpectatorEvent
            UpdateCharacterDataEvent
            ExitIngameEvent
            AuraEvent
            BuffEvent
            EnergyChangeEvent
            ComboChangeEvent
            ParryHookEvent
            RequestSiteInfoEvent
            MineBlockEvent
            TeleportToEvent
            SoundEvent
            CreateSpriteEvent
            TamePetEvent
            EntityAttackedHookEvent
            ChangeAbilityEvent
            UpdateMapMarkerEvent
            MakeAdminEvent
            DeleteCharacterEvent
            ChangeStanceEvent
            ChangeBodyEvent
            RemoveLightEmitterEvent
            TeleportToPositionEvent
            StartTeleportingEvent
            ToggleSpriteLightEvent
            TransformEvent
            StartInteractionEvent
            RequestPluginsEvent
            CreateAuraEntityEvent
            RegrowHeadEvent
            SetBattleModeEvent
            SummonBeamPillarsEvent
        }
    };
}

pub(crate) fn register_event_busses(ecs: &mut specs::World) {
    macro_rules! register_events {
        ($($event:ty)*) => {
            $(
                ecs.insert(EventBus::<$event>::default());
            )*
        };
    }
    server_events!(register_events);
}

/// Checks that every server event has been handled and that there aren't
/// duplicate handlers.
///
/// Also asserts that the event busses have all been registered.
///
/// Needs to be called on the first tick after all event handlers have run.
/// After the initial call this does nothing.
#[cfg(debug_assertions)]
pub(super) fn check_event_handlers(ecs: &mut specs::World) {
    struct CheckedEventHandlers;
    if ecs.get_mut::<CheckedEventHandlers>().is_some() {
        return;
    }
    ecs.insert(CheckedEventHandlers);

    fn not_consumed<T>() -> ! {
        panic!("Server event not consumed: {}", core::any::type_name::<T>());
    }
    fn multiple_handlers<T>() -> ! {
        panic!(
            "Server event has multiple handlers, only the first will receive events: {}",
            core::any::type_name::<T>()
        );
    }

    macro_rules! check_events {
        ($($event:ty)*) => {
            $(
                let recv_count = ecs.get_mut::<EventBus<$event>>().expect("event bus not registered").recv_count();
                match recv_count {
                    0 => not_consumed::<$event>(),
                    1 => {},
                    _ => multiple_handlers::<$event>(),
                }
            )*
        };
    }
    server_events!(check_events);
}
