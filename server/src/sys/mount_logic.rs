// In a new or existing server-side system file...

use specs::{Builder, Entity, World, WorldExt};
use veloren_common::{
    comp::{
        self, Alignment, EphemeralMount, Health, Inventory, Pet, Poise, Rider, SkillSet, Stats,
    },
    event::TamePetEvent,
    generation::EntityInfo,
    npc,
    uid::Uid,
};

/// This function contains the core logic for spawning, taming, and mounting the antelope.
/// It is designed to be called from our custom ability handler.
pub fn execute_summon_ephemeral_mount(server: &mut crate::Server, player_entity: Entity) -> Result<(), Content> {
    let state = &mut server.state;
    let ecs = state.ecs_mut();

    // --- 1. SPAWN LOGIC (Derived from `handle_spawn` in cmd.rs) ---

    let player_pos = ecs
       .read_storage::<comp::Pos>()
       .get(player_entity)
       .copied()
       .ok_or_else(|| Content::localized("command-position-unavailable", [("target", "player")]))?;

    let player_uid = ecs
       .read_storage::<Uid>()
       .get(player_entity)
       .copied()
       .ok_or_else(|| Content::localized("command-uid-unavailable", [("target", "player")]))?;

    // Define the antelope body from its asset file
    let body = npc::NpcBody::from_asset("common.entity.quadruped.antelope")
       .map_err(|_| Content::localized("command-entity-load-failed", [("config", "antelope")]))?
       .1();

    // Create the NPC entity using the same builder pattern as the /spawn command
    let mount_entity_builder = ecs
       .create_entity_synced()
       .with(player_pos)
       .with(comp::Vel::zero())
       .with(comp::Ori::default())
       .with(comp::Stats::new(
            Content::Plain(npc::get_npc_name(
                "common.entity.quadruped.antelope",
                npc::BodyType::from_body(body),
            )),
            body,
        ))
       .with(SkillSet::default())
       .with(Health::new(body))
       .with(Poise::new(body))
       .with(Inventory::with_empty()) // Mounts don't need a loadout
       .with(body)
       .with(body.scale())
       .with(Alignment::Owned(player_uid)); // This makes the entity a "pet" of the player

    // Build the entity to get its ID
    let mount_entity = mount_entity_builder.build();

    // --- 2. TAGGING LOGIC ---

    // Now that the entity exists, add our custom tag component
    ecs.write_storage::<EphemeralMount>()
       .insert(mount_entity, EphemeralMount)
       .expect("Failed to insert EphemeralMount component.");

    // --- 3. TAMING LOGIC (Derived from `handle_spawn` in cmd.rs) ---

    // The /spawn command emits a TamePetEvent when alignment is "pet", which handles
    // adding the entity to the owner's group and other pet-related setup. We do the same.
    ecs.read_resource::<common::event::EventBus<TamePetEvent>>()
       .emit_now(TamePetEvent {
            owner_entity: player_entity,
            pet_entity: mount_entity,
        });

    // --- 4. MOUNTING LOGIC (Derived from `handle_mount` in cmd.rs) ---

    let mount_uid = ecs
       .read_storage::<Uid>()
       .get(mount_entity)
       .copied()
       .ok_or_else(|| Content::localized("command-uid-unavailable", [("target", "mount")]))?;

    // This `link` call is the core of the /mount command.
    state
       .link(common::mounting::Mounting {
            mount: mount_uid,
            rider: player_uid,
        })
       .map_err(|_| Content::Plain("Failed to mount entities".to_string()))?;

    Ok(())
}