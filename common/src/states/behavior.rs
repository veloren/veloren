use crate::{
    comp::{
        self,
        character_state::OutputEvents,
        item::{tool::AbilityMap, MaterialStatManifest},
        ActiveAbilities, Beam, Body, CharacterState, Combo, ControlAction, Controller,
        ControllerInputs, Density, Energy, Health, InputAttr, InputKind, Inventory,
        InventoryAction, Mass, Melee, Ori, PhysicsState, Pos, SkillSet, Stance, StateUpdate, Stats,
        Vel,
    },
    link::Is,
    mounting::Rider,
    resources::{DeltaTime, Time},
    terrain::TerrainGrid,
    uid::Uid,
};
use specs::{storage::FlaggedAccessMut, Entity, LazyUpdate};
use vek::*;

pub trait CharacterBehavior {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate;
    // Impl these to provide behavior for these inputs
    fn swap_equipped_weapons(
        &self,
        data: &JoinData,
        _output_events: &mut OutputEvents,
    ) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn manipulate_loadout(
        &self,
        data: &JoinData,
        _output_events: &mut OutputEvents,
        _inv_action: InventoryAction,
    ) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn wield(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn glide_wield(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn unwield(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn sit(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn dance(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn sneak(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn stand(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn talk(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        StateUpdate::from(data)
    }
    // start_input has custom implementation in the following character states that
    // may also need to be modified when changes are made here: ComboMelee2
    fn start_input(
        &self,
        data: &JoinData,
        input: InputKind,
        target_entity: Option<Uid>,
        select_pos: Option<Vec3<f32>>,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.queued_inputs.insert(input, InputAttr {
            select_pos,
            target_entity,
        });
        update
    }
    fn cancel_input(&self, data: &JoinData, input: InputKind) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.removed_inputs.push(input);
        update
    }
    fn handle_event(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        event: ControlAction,
    ) -> StateUpdate {
        match event {
            ControlAction::SwapEquippedWeapons => self.swap_equipped_weapons(data, output_events),
            ControlAction::InventoryAction(inv_action) => {
                self.manipulate_loadout(data, output_events, inv_action)
            },
            ControlAction::Wield => self.wield(data, output_events),
            ControlAction::GlideWield => self.glide_wield(data, output_events),
            ControlAction::Unwield => self.unwield(data, output_events),
            ControlAction::Sit => self.sit(data, output_events),
            ControlAction::Dance => self.dance(data, output_events),
            ControlAction::Sneak => {
                if data.mount_data.is_none() {
                    self.sneak(data, output_events)
                } else {
                    self.stand(data, output_events)
                }
            },
            ControlAction::Stand => self.stand(data, output_events),
            ControlAction::Talk => self.talk(data, output_events),
            ControlAction::StartInput {
                input,
                target_entity,
                select_pos,
            } => self.start_input(data, input, target_entity, select_pos),
            ControlAction::CancelInput(input) => self.cancel_input(data, input),
        }
    }
}

/// Read-Only Data sent from Character Behavior System to behavior fn's
pub struct JoinData<'a> {
    pub entity: Entity,
    pub uid: &'a Uid,
    pub character: &'a CharacterState,
    pub pos: &'a Pos,
    pub vel: &'a Vel,
    pub ori: &'a Ori,
    pub mass: &'a Mass,
    pub density: &'a Density,
    pub dt: &'a DeltaTime,
    pub time: &'a Time,
    pub controller: &'a Controller,
    pub inputs: &'a ControllerInputs,
    pub health: Option<&'a Health>,
    pub energy: &'a Energy,
    pub inventory: Option<&'a Inventory>,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub melee_attack: Option<&'a Melee>,
    pub updater: &'a LazyUpdate,
    pub stats: &'a Stats,
    pub skill_set: &'a SkillSet,
    pub active_abilities: Option<&'a ActiveAbilities>,
    pub ability_map: &'a AbilityMap,
    pub msm: &'a MaterialStatManifest,
    pub combo: Option<&'a Combo>,
    pub alignment: Option<&'a comp::Alignment>,
    pub terrain: &'a TerrainGrid,
    pub mount_data: Option<&'a Is<Rider>>,
    pub stance: Option<&'a Stance>,
}

pub struct JoinStruct<'a> {
    pub entity: Entity,
    pub uid: &'a Uid,
    pub char_state: FlaggedAccessMut<'a, &'a mut CharacterState, CharacterState>,
    pub pos: &'a mut Pos,
    pub vel: &'a mut Vel,
    pub ori: &'a mut Ori,
    pub mass: &'a Mass,
    pub density: FlaggedAccessMut<'a, &'a mut Density, Density>,
    pub energy: FlaggedAccessMut<'a, &'a mut Energy, Energy>,
    pub inventory: Option<&'a Inventory>,
    pub controller: &'a mut Controller,
    pub health: Option<&'a Health>,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub melee_attack: Option<&'a Melee>,
    pub beam: Option<&'a Beam>,
    pub stat: &'a Stats,
    pub skill_set: &'a SkillSet,
    pub active_abilities: Option<&'a ActiveAbilities>,
    pub combo: Option<&'a Combo>,
    pub alignment: Option<&'a comp::Alignment>,
    pub terrain: &'a TerrainGrid,
    pub mount_data: Option<&'a Is<Rider>>,
    pub stance: Option<&'a Stance>,
}

impl<'a> JoinData<'a> {
    pub fn new(
        j: &'a JoinStruct<'a>,
        updater: &'a LazyUpdate,
        dt: &'a DeltaTime,
        time: &'a Time,
        msm: &'a MaterialStatManifest,
        ability_map: &'a AbilityMap,
    ) -> Self {
        Self {
            entity: j.entity,
            uid: j.uid,
            character: &j.char_state,
            pos: j.pos,
            vel: j.vel,
            ori: j.ori,
            mass: j.mass,
            density: &j.density,
            energy: &j.energy,
            inventory: j.inventory,
            controller: j.controller,
            inputs: &j.controller.inputs,
            health: j.health,
            body: j.body,
            physics: j.physics,
            melee_attack: j.melee_attack,
            stats: j.stat,
            skill_set: j.skill_set,
            updater,
            dt,
            time,
            msm,
            ability_map,
            combo: j.combo,
            alignment: j.alignment,
            terrain: j.terrain,
            active_abilities: j.active_abilities,
            mount_data: j.mount_data,
            stance: j.stance,
        }
    }
}
