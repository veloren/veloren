use std::time::Duration;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, Entities, Read, ReadStorage, WriteStorage};

use crate::{
    comp::{Alignment, CharacterState, Health, Pos},
    consts::{MAX_INTERACT_RANGE, MAX_MOUNT_RANGE},
    link::{Is, Link, LinkHandle, Role},
    uid::{IdMaps, Uid},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Interactor;

impl Role for Interactor {
    type Link = Interaction;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Interactors {
    interactors: HashMap<Uid, LinkHandle<Interaction>>,
}

impl Interactors {
    pub fn get(&self, uid: Uid) -> Option<&LinkHandle<Interaction>> { self.interactors.get(&uid) }

    pub fn iter(&self) -> impl Iterator<Item = &LinkHandle<Interaction>> {
        self.interactors.values()
    }

    pub fn has_interaction(&self, kind: InteractionKind) -> bool {
        self.iter().any(|i| i.kind == kind)
    }
}

impl Component for Interactors {
    type Storage = DerefFlaggedStorage<Interactors>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionKind {
    HelpDowned,
    Pet,
}

// TODO: Do we want to use this for sprite interactions too?
#[derive(Serialize, Deserialize, Debug)]
pub struct Interaction {
    pub interactor: Uid,
    pub target: Uid,
    pub kind: InteractionKind,
}

#[derive(Debug)]
pub enum InteractionError {
    NoSuchEntity,
    NotInteractable,
    CannotInteract,
}

pub fn can_help_downed(pos: Pos, target_pos: Pos, target_state: Option<&CharacterState>) -> bool {
    let within_distance = pos.0.distance_squared(target_pos.0) <= MAX_INTERACT_RANGE.powi(2);
    let valid_state = matches!(target_state, Some(CharacterState::Crawl));

    within_distance && valid_state
}

pub fn can_pet(pos: Pos, target_pos: Pos, target_alignment: Option<&Alignment>) -> bool {
    let within_distance = pos.0.distance_squared(target_pos.0) <= MAX_MOUNT_RANGE.powi(2);
    let valid_alignment = matches!(
        target_alignment,
        Some(Alignment::Owned(_) | Alignment::Tame)
    );

    within_distance && valid_alignment
}

impl Link for Interaction {
    type CreateData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Is<Interactor>>,
        WriteStorage<'a, Interactors>,
        WriteStorage<'a, CharacterState>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Alignment>,
    );
    type DeleteData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Is<Interactor>>,
        WriteStorage<'a, Interactors>,
        WriteStorage<'a, CharacterState>,
    );
    type Error = InteractionError;
    type PersistData<'a> = (
        Read<'a, IdMaps>,
        Entities<'a>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Is<Interactor>>,
        ReadStorage<'a, Interactors>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Alignment>,
    );

    fn create(
        this: &crate::link::LinkHandle<Self>,
        (id_maps, is_interactors, interactors, character_states, positions, alignments): &mut Self::CreateData<
            '_,
        >,
    ) -> Result<(), Self::Error> {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        if this.interactor == this.target {
            // Can't interact with itself
            Err(InteractionError::NotInteractable)
        } else if let Some(interactor) = entity(this.interactor)
            && let Some(target) = entity(this.target)
        {
            // Can only interact with one thing at a time.
            if !is_interactors.contains(interactor)
                && character_states
                    .get(interactor)
                    .map_or(true, |state| state.can_interact())
                && let Some(pos) = positions.get(interactor)
                && let Some(target_pos) = positions.get(target)
                && match this.kind {
                    InteractionKind::HelpDowned => {
                        can_help_downed(*pos, *target_pos, character_states.get(target))
                    },
                    InteractionKind::Pet => can_pet(*pos, *target_pos, alignments.get(target)),
                }
            {
                if let Some(mut character_state) = character_states.get_mut(interactor) {
                    let (buildup_duration, use_duration, recover_duration) = this.kind.durations();
                    *character_state = CharacterState::Interact(crate::states::interact::Data {
                        static_data: crate::states::interact::StaticData {
                            buildup_duration,
                            use_duration,
                            recover_duration,
                            interact: crate::states::interact::InteractKind::Entity {
                                target: this.target,
                                kind: this.kind,
                            },
                            was_wielded: character_state.is_wield(),
                            was_sneak: character_state.is_stealthy(),
                            required_item: None,
                        },
                        timer: Duration::default(),
                        stage_section: crate::states::utils::StageSection::Buildup,
                    });

                    let _ = is_interactors.insert(interactor, this.make_role());
                    if let Some(mut interactors) = interactors.get_mut(target) {
                        interactors
                            .interactors
                            .insert(this.interactor, this.clone());
                    } else {
                        let _ = interactors.insert(target, Interactors {
                            interactors: std::iter::once((this.interactor, this.clone())).collect(),
                        });
                    }

                    Ok(())
                } else {
                    Err(InteractionError::CannotInteract)
                }
            } else {
                Err(InteractionError::CannotInteract)
            }
        } else {
            Err(InteractionError::NoSuchEntity)
        }
    }

    fn persist(
        this: &crate::link::LinkHandle<Self>,
        (
            id_maps,
            entities,
            healths,
            is_interactors,
            interactors,
            character_states,
            positions,
            alignments,
        ): &mut Self::PersistData<'_>,
    ) -> bool {
        let entity = |uid: Uid| id_maps.uid_entity(uid);
        let is_alive =
            |entity| entities.is_alive(entity) && healths.get(entity).map_or(true, |h| !h.is_dead);

        if let Some(interactor) = entity(this.interactor)
            && let Some(target) = entity(this.target)
            && is_interactors.contains(interactor)
            && let Some(interactors) = interactors.get(target)
            && interactors.interactors.contains_key(&this.interactor)
            && is_alive(interactor)
            && is_alive(target)
            && let Some(pos) = positions.get(interactor)
            && let Some(target_pos) = positions.get(target)
            && match this.kind {
                InteractionKind::HelpDowned => {
                    can_help_downed(*pos, *target_pos, character_states.get(target))
                },
                InteractionKind::Pet => can_pet(*pos, *target_pos, alignments.get(target)),
            }
            && let Some(CharacterState::Interact(crate::states::interact::Data {
                static_data:
                    crate::states::interact::StaticData {
                        interact:
                            crate::states::interact::InteractKind::Entity {
                                target: state_target,
                                kind: state_kind,
                            },
                        ..
                    },
                ..
            })) = character_states.get(interactor)
            && *state_target == this.target
            && *state_kind == this.kind
        {
            true
        } else {
            false
        }
    }

    fn delete(
        this: &crate::link::LinkHandle<Self>,
        (id_maps, is_interactors, interactors, character_states): &mut Self::DeleteData<'_>,
    ) {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        let interactor = entity(this.interactor);
        let target = entity(this.target);

        interactor.map(|interactor| is_interactors.remove(interactor));
        target.map(|target| {
            if let Some(mut i) = interactors.get_mut(target) {
                i.interactors.remove(&this.interactor);

                if i.interactors.is_empty() {
                    interactors.remove(target);
                }
            }
        });

        // yay pattern matching 🦀
        if let Some(character_state) = interactor
            .and_then(|interactor| character_states.get_mut(interactor))
            .as_deref_mut()
            && let CharacterState::Interact(crate::states::interact::Data {
                static_data:
                    crate::states::interact::StaticData {
                        interact:
                            ref mut interact @ crate::states::interact::InteractKind::Entity {
                                target: state_target,
                                kind: state_kind,
                                ..
                            },
                        ..
                    },
                ..
            }) = *character_state
            && state_target == this.target
            && state_kind == this.kind
        {
            // If the character state we created with this link still persists, the target
            // has become invalid so we set it to that. And the character state decides how
            // it handles that, be it ending or something else.
            *interact = crate::states::interact::InteractKind::Invalid;
        }
    }
}
