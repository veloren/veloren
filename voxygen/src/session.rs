use std::{cell::RefCell, rc::Rc, time::Duration};

use ordered_float::OrderedFloat;
use specs::{Join, WorldExt};
use tracing::{error, info};
use vek::*;

use client::{self, Client};
use common::{
    assets::AssetExt,
    comp,
    comp::{
        inventory::slot::Slot, invite::InviteKind, ChatMsg, ChatType, InventoryUpdateEvent, Pos,
        Vel,
    },
    consts::{MAX_MOUNT_RANGE, MAX_PICKUP_RANGE},
    outcome::Outcome,
    span,
    terrain::{Block, BlockKind},
    trade::TradeResult,
    util::{
        find_dist::{Cube, Cylinder, FindDist},
        Dir,
    },
    vol::ReadVol,
};
use common_net::{
    msg::{server::InviteAnswer, PresenceKind},
    sync::WorldSyncExt,
};

use crate::{
    audio::sfx::SfxEvent,
    ecs::MyEntity,
    hud::{DebugInfo, Event as HudEvent, Hud, HudInfo, PressBehavior, PromptDialogSettings},
    i18n::{i18n_asset_key, Localization},
    key_state::KeyState,
    menu::char_selection::CharSelectionState,
    render::Renderer,
    scene::{camera, CameraMode, Scene, SceneData},
    settings::{ControlSettings, Settings},
    window::{AnalogGameInput, Event, GameInput},
    Direction, Error, GlobalState, PlayState, PlayStateResult,
};

/// The action to perform after a tick
enum TickAction {
    // Continue executing
    Continue,
    // Disconnected (i.e. go to main menu)
    Disconnect,
}

pub struct SessionState {
    scene: Scene,
    client: Rc<RefCell<Client>>,
    hud: Hud,
    key_state: KeyState,
    inputs: comp::ControllerInputs,
    selected_block: Block,
    walk_forward_dir: Vec2<f32>,
    walk_right_dir: Vec2<f32>,
    freefly_vel: Vec3<f32>,
    free_look: bool,
    auto_walk: bool,
    is_aiming: bool,
    target_entity: Option<specs::Entity>,
    selected_entity: Option<(specs::Entity, std::time::Instant)>,
    interactable: Option<Interactable>,
}

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        // Create a scene for this session. The scene handles visible elements of the
        // game world.
        let mut scene = Scene::new(
            global_state.window.renderer_mut(),
            &*client.borrow(),
            &global_state.settings,
        );
        scene
            .camera_mut()
            .set_fov_deg(global_state.settings.graphics.fov);
        let hud = Hud::new(global_state, &client.borrow());
        let walk_forward_dir = scene.camera().forward_xy();
        let walk_right_dir = scene.camera().right_xy();

        Self {
            scene,
            client,
            key_state: KeyState::default(),
            inputs: comp::ControllerInputs::default(),
            hud,
            selected_block: Block::new(BlockKind::Misc, Rgb::broadcast(255)),
            walk_forward_dir,
            walk_right_dir,
            freefly_vel: Vec3::zero(),
            free_look: false,
            auto_walk: false,
            is_aiming: false,
            target_entity: None,
            selected_entity: None,
            interactable: None,
        }
    }

    fn stop_auto_walk(&mut self) {
        self.auto_walk = false;
        self.hud.auto_walk(false);
        self.key_state.auto_walk = false;
    }

    /// Tick the session (and the client attached to it).
    fn tick(
        &mut self,
        dt: Duration,
        global_state: &mut GlobalState,
        outcomes: &mut Vec<Outcome>,
    ) -> Result<TickAction, Error> {
        span!(_guard, "tick", "Session::tick");
        self.inputs.tick(dt);

        let mut client = self.client.borrow_mut();
        for event in client.tick(self.inputs.clone(), dt, crate::ecs::sys::add_local_systems)? {
            match event {
                client::Event::Chat(m) => {
                    self.hud.new_message(m);
                },
                client::Event::InviteComplete {
                    target,
                    answer,
                    kind,
                } => {
                    // TODO: i18n (complicated since substituting phrases at this granularity may
                    // not be grammatical in some languages)
                    let kind_str = match kind {
                        InviteKind::Group => "Group",
                        InviteKind::Trade => "Trade",
                    };
                    let target_name = match client.player_list().get(&target) {
                        Some(info) => info.player_alias.clone(),
                        None => format!("<entity {}>", target),
                    };
                    let answer_str = match answer {
                        InviteAnswer::Accepted => "accepted",
                        InviteAnswer::Declined => "declined",
                        InviteAnswer::TimedOut => "timed out",
                    };
                    let msg = format!("{} invite to {} {}", kind_str, target_name, answer_str);
                    self.hud.new_message(ChatType::Meta.chat_msg(msg));
                },
                client::Event::TradeComplete { result, trade: _ } => {
                    let i18n = global_state.i18n.read();
                    let msg = match result {
                        TradeResult::Completed => i18n.get("hud.trade.result.completed"),
                        TradeResult::Declined => i18n.get("hud.trade.result.declined"),
                        TradeResult::NotEnoughSpace => i18n.get("hud.trade.result.nospace"),
                    };
                    self.hud.new_message(ChatType::Meta.chat_msg(msg));
                },
                client::Event::InventoryUpdated(inv_event) => {
                    let sfx_triggers = self.scene.sfx_mgr.triggers.read();

                    let sfx_trigger_item = sfx_triggers.get_key_value(&SfxEvent::from(&inv_event));
                    global_state.audio.emit_sfx_item(sfx_trigger_item);

                    let i18n = global_state.i18n.read();

                    match inv_event {
                        InventoryUpdateEvent::CollectFailed => {
                            self.hud.new_message(ChatMsg {
                                message: i18n.get("hud.chat.loot_fail").to_string(),
                                chat_type: ChatType::CommandError,
                            });
                        },
                        InventoryUpdateEvent::Collected(item) => {
                            self.hud.new_message(ChatMsg {
                                message: i18n
                                    .get("hud.chat.loot_msg")
                                    .replace("{item}", item.name()),
                                chat_type: ChatType::Loot,
                            });
                        },
                        _ => {},
                    };
                },
                client::Event::Disconnect => return Ok(TickAction::Disconnect),
                client::Event::DisconnectionNotification(time) => {
                    let i18n = global_state.i18n.read();

                    let message = match time {
                        0 => String::from(i18n.get("hud.chat.goodbye")),
                        _ => i18n
                            .get("hud.chat.connection_lost")
                            .replace("{time}", time.to_string().as_str()),
                    };

                    self.hud.new_message(ChatMsg {
                        chat_type: ChatType::CommandError,
                        message,
                    });
                },
                client::Event::Kicked(reason) => {
                    global_state.info_message = Some(format!(
                        "{}: {}",
                        global_state
                            .i18n
                            .read()
                            .get("main.login.kicked")
                            .to_string(),
                        reason
                    ));
                    return Ok(TickAction::Disconnect);
                },
                client::Event::Notification(n) => {
                    self.hud.new_notification(n);
                },
                client::Event::SetViewDistance(vd) => {
                    global_state.settings.graphics.view_distance = vd;
                    global_state.settings.save_to_file_warn();
                },
                client::Event::Outcome(outcome) => outcomes.push(outcome),
                client::Event::CharacterCreated(_) => {},
                client::Event::CharacterError(error) => {
                    global_state.client_error = Some(error);
                },
            }
        }

        Ok(TickAction::Continue)
    }

    /// Clean up the session (and the client attached to it) after a tick.
    pub fn cleanup(&mut self) { self.client.borrow_mut().cleanup(); }
}

impl PlayState for SessionState {
    fn enter(&mut self, global_state: &mut GlobalState, _: Direction) {
        // Trap the cursor.
        global_state.window.grab_cursor(true);

        self.client.borrow_mut().clear_terrain();

        // Send startup commands to the server
        if global_state.settings.send_logon_commands {
            for cmd in &global_state.settings.logon_commands {
                self.client.borrow_mut().send_chat(cmd.to_string());
            }
        }
    }

    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult {
        span!(_guard, "tick", "<Session as PlayState>::tick");
        // TODO: let mut client = self.client.borrow_mut();

        // TODO: can this be a method on the session or are there borrowcheck issues?
        let (client_presence, client_registered) = {
            let client = self.client.borrow();
            (client.presence(), client.registered())
        };
        if client_presence.is_some() {
            // Update MyEntity
            // Note: Alternatively, the client could emit an event when the entity changes
            // which may or may not be more elegant
            {
                let my_entity = self.client.borrow().entity();
                self.client
                    .borrow_mut()
                    .state_mut()
                    .ecs_mut()
                    .insert(MyEntity(my_entity));
            }
            // Compute camera data
            self.scene
                .camera_mut()
                .compute_dependents(&*self.client.borrow().state().terrain());
            let camera::Dependents {
                cam_pos, cam_dir, ..
            } = self.scene.camera().dependents();
            let focus_pos = self.scene.camera().get_focus_pos();
            let focus_off = focus_pos.map(|e| e.trunc());
            let cam_pos = cam_pos + focus_off;

            let (is_aiming, aim_dir_offset) = {
                let client = self.client.borrow();
                let is_aiming = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(client.entity())
                    .map(|cs| cs.is_aimed())
                    .unwrap_or(false);

                (
                    is_aiming,
                    if is_aiming && self.scene.camera().get_mode() == CameraMode::ThirdPerson {
                        Vec3::unit_z() * 0.05
                    } else {
                        Vec3::zero()
                    },
                )
            };
            self.is_aiming = is_aiming;

            // Check to see whether we're aiming at anything
            let (build_pos, select_pos, target_entity) =
                under_cursor(&self.client.borrow(), cam_pos, cam_dir);
            // Throw out distance info, it will be useful in the future
            self.target_entity = target_entity.map(|x| x.0);

            let can_build = self
                .client
                .borrow()
                .state()
                .read_storage::<comp::CanBuild>()
                .get(self.client.borrow().entity())
                .is_some();

            self.interactable = select_interactable(
                &self.client.borrow(),
                target_entity,
                select_pos,
                &self.scene,
            );

            // Only highlight interactables
            // unless in build mode where select_pos highlighted
            self.scene
                .set_select_pos(select_pos.filter(|_| can_build).or_else(
                    || match self.interactable {
                        Some(Interactable::Block(_, block_pos)) => Some(block_pos),
                        _ => None,
                    },
                ));

            // Handle window events.
            for event in events {
                // Pass all events to the ui first.
                if self.hud.handle_event(event.clone(), global_state) {
                    continue;
                }

                match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    },
                    Event::InputUpdate(GameInput::Primary, state) => {
                        // If we can build, use LMB to break blocks, if not, use it to attack
                        let mut client = self.client.borrow_mut();
                        if state && can_build {
                            if let Some(select_pos) = select_pos {
                                client.remove_block(select_pos);
                            }
                        } else {
                            self.inputs.primary.set_state(state);
                        }
                    },

                    Event::InputUpdate(GameInput::Secondary, state) => {
                        self.inputs.secondary.set_state(false); // To be changed later on

                        let mut client = self.client.borrow_mut();

                        if state && can_build {
                            if let Some(build_pos) = build_pos {
                                client.place_block(build_pos, self.selected_block);
                            }
                        } else {
                            self.inputs.secondary.set_state(state);
                        }
                    },

                    Event::InputUpdate(GameInput::Roll, state) => {
                        let client = self.client.borrow();
                        if can_build {
                            if state {
                                if let Some(block) = select_pos
                                    .and_then(|sp| client.state().terrain().get(sp).ok().copied())
                                {
                                    self.selected_block = block;
                                }
                            }
                        } else {
                            self.inputs.roll.set_state(state);
                        }
                    },
                    Event::InputUpdate(GameInput::Respawn, state)
                        if state != self.key_state.respawn =>
                    {
                        self.stop_auto_walk();
                        self.key_state.respawn = state;
                        if state {
                            self.client.borrow_mut().respawn();
                        }
                    }
                    Event::InputUpdate(GameInput::Jump, state) => {
                        self.inputs.jump.set_state(state);
                    },
                    Event::InputUpdate(GameInput::SwimUp, state) => {
                        self.key_state.swim_up = state;
                    },
                    Event::InputUpdate(GameInput::SwimDown, state) => {
                        self.key_state.swim_down = state;
                    },
                    Event::InputUpdate(GameInput::Sit, state)
                        if state != self.key_state.toggle_sit =>
                    {
                        self.key_state.toggle_sit = state;

                        if state {
                            self.stop_auto_walk();
                            self.client.borrow_mut().toggle_sit();
                        }
                    }
                    Event::InputUpdate(GameInput::Dance, state)
                        if state != self.key_state.toggle_dance =>
                    {
                        self.key_state.toggle_dance = state;
                        if state {
                            self.stop_auto_walk();
                            self.client.borrow_mut().toggle_dance();
                        }
                    }
                    Event::InputUpdate(GameInput::Sneak, state)
                        if state != self.key_state.toggle_sneak =>
                    {
                        self.key_state.toggle_sneak = state;
                        if state {
                            self.stop_auto_walk();
                            self.client.borrow_mut().toggle_sneak();
                        }
                    }
                    Event::InputUpdate(GameInput::MoveForward, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            self.stop_auto_walk();
                        }
                        self.key_state.up = state
                    },
                    Event::InputUpdate(GameInput::MoveBack, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            self.stop_auto_walk();
                        }
                        self.key_state.down = state
                    },
                    Event::InputUpdate(GameInput::MoveLeft, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            self.stop_auto_walk();
                        }
                        self.key_state.left = state
                    },
                    Event::InputUpdate(GameInput::MoveRight, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            self.stop_auto_walk();
                        }
                        self.key_state.right = state
                    },
                    Event::InputUpdate(GameInput::Glide, state)
                        if state != self.key_state.toggle_glide =>
                    {
                        self.key_state.toggle_glide = state;
                        if state {
                            self.client.borrow_mut().toggle_glide();
                        }
                    }
                    Event::InputUpdate(GameInput::Fly, state) => {
                        self.key_state.fly ^= state;
                    },
                    Event::InputUpdate(GameInput::Climb, state) => {
                        self.key_state.climb_up = state;
                    },
                    Event::InputUpdate(GameInput::ClimbDown, state) => {
                        self.key_state.climb_down = state;
                    },
                    /*Event::InputUpdate(GameInput::WallLeap, state) => {
                        self.inputs.wall_leap.set_state(state)
                    },*/
                    Event::InputUpdate(GameInput::ToggleWield, state)
                        if state != self.key_state.toggle_wield =>
                    {
                        self.key_state.toggle_wield = state;
                        if state {
                            self.client.borrow_mut().toggle_wield();
                        }
                    }
                    Event::InputUpdate(GameInput::SwapLoadout, state)
                        if state != self.key_state.swap_loadout =>
                    {
                        self.key_state.swap_loadout = state;
                        if state {
                            self.client.borrow_mut().swap_loadout();
                        }
                    }
                    Event::InputUpdate(GameInput::ToggleLantern, true) => {
                        let mut client = self.client.borrow_mut();
                        if client.is_lantern_enabled() {
                            client.disable_lantern();
                        } else {
                            client.enable_lantern();
                        }
                    },
                    Event::InputUpdate(GameInput::Mount, true) => {
                        let mut client = self.client.borrow_mut();
                        if client.is_mounted() {
                            client.unmount();
                        } else {
                            let player_pos = client
                                .state()
                                .read_storage::<comp::Pos>()
                                .get(client.entity())
                                .copied();
                            if let Some(player_pos) = player_pos {
                                // Find closest mountable entity
                                let closest_mountable_entity = (
                                    &client.state().ecs().entities(),
                                    &client.state().ecs().read_storage::<comp::Pos>(),
                                    &client.state().ecs().read_storage::<comp::MountState>(),
                                )
                                    .join()
                                    .filter(|(entity, _, mount_state)| {
                                        *entity != client.entity()
                                            && **mount_state == comp::MountState::Unmounted
                                    })
                                    .map(|(entity, pos, _)| {
                                        (entity, player_pos.0.distance_squared(pos.0))
                                    })
                                    .filter(|(_, dist_sqr)| *dist_sqr < MAX_MOUNT_RANGE.powi(2))
                                    .min_by_key(|(_, dist_sqr)| OrderedFloat(*dist_sqr));
                                if let Some((mountee_entity, _)) = closest_mountable_entity {
                                    client.mount(mountee_entity);
                                }
                            }
                        }
                    },
                    Event::InputUpdate(GameInput::Interact, state)
                        if state != self.key_state.interact =>
                    {
                        self.key_state.interact = state;

                        if state {
                            if let Some(interactable) = self.interactable {
                                let mut client = self.client.borrow_mut();
                                match interactable {
                                    Interactable::Block(block, pos) => {
                                        if block.is_collectible() {
                                            client.collect_block(pos);
                                        }
                                    },
                                    Interactable::Entity(entity) => {
                                        if client
                                            .state()
                                            .ecs()
                                            .read_storage::<comp::Item>()
                                            .get(entity)
                                            .is_some()
                                        {
                                            client.pick_up(entity);
                                        } else {
                                            client.npc_interact(entity);
                                        }
                                    },
                                }
                            }
                        }
                    }
                    Event::InputUpdate(GameInput::Trade, state)
                        if state != self.key_state.trade =>
                    {
                        self.key_state.trade = state;

                        if state {
                            if let Some(interactable) = self.interactable {
                                let mut client = self.client.borrow_mut();
                                match interactable {
                                    Interactable::Block(_, _) => {},
                                    Interactable::Entity(entity) => {
                                        if let Some(uid) =
                                            client.state().ecs().uid_from_entity(entity)
                                        {
                                            let name = client
                                                .player_list()
                                                .get(&uid)
                                                .map(|info| info.player_alias.clone())
                                                .unwrap_or_else(|| format!("<entity {:?}>", uid));
                                            let msg = global_state
                                                .i18n
                                                .read()
                                                .get("hud.trade.invite_sent")
                                                .replace("{playername}", &name);
                                            self.hud.new_message(ChatType::Meta.chat_msg(msg));
                                            client.send_invite(uid, InviteKind::Trade)
                                        };
                                    },
                                }
                            }
                        }
                    }
                    /*Event::InputUpdate(GameInput::Charge, state) => {
                        self.inputs.charge.set_state(state);
                    },*/
                    Event::InputUpdate(GameInput::FreeLook, state) => {
                        match (global_state.settings.gameplay.free_look_behavior, state) {
                            (PressBehavior::Toggle, true) => {
                                self.free_look = !self.free_look;
                                self.hud.free_look(self.free_look);
                            },
                            (PressBehavior::Hold, state) => {
                                self.free_look = state;
                                self.hud.free_look(self.free_look);
                            },
                            _ => {},
                        };
                    },
                    Event::InputUpdate(GameInput::AutoWalk, state) => {
                        match (global_state.settings.gameplay.auto_walk_behavior, state) {
                            (PressBehavior::Toggle, true) => {
                                self.auto_walk = !self.auto_walk;
                                self.key_state.auto_walk = self.auto_walk;
                                self.hud.auto_walk(self.auto_walk);
                            },
                            (PressBehavior::Hold, state) => {
                                self.auto_walk = state;
                                self.key_state.auto_walk = self.auto_walk;
                                self.hud.auto_walk(self.auto_walk);
                            },
                            _ => {},
                        }
                    },
                    Event::InputUpdate(GameInput::CycleCamera, true) => {
                        // Prevent accessing camera modes which aren't available in multiplayer
                        // unless you are an admin. This is an easily bypassed clientside check.
                        // The server should do its own filtering of which entities are sent to
                        // clients to prevent abuse.
                        let camera = self.scene.camera_mut();
                        camera.next_mode(self.client.borrow().is_admin());
                    },
                    Event::InputUpdate(GameInput::Select, state) => {
                        if !state {
                            self.selected_entity =
                                self.target_entity.map(|e| (e, std::time::Instant::now()));
                        }
                    },
                    Event::InputUpdate(GameInput::AcceptGroupInvite, true) => {
                        let mut client = self.client.borrow_mut();
                        if client.invite().is_some() {
                            client.accept_invite();
                        }
                    },
                    Event::InputUpdate(GameInput::DeclineGroupInvite, true) => {
                        let mut client = self.client.borrow_mut();
                        if client.invite().is_some() {
                            client.decline_invite();
                        }
                    },
                    Event::AnalogGameInput(input) => match input {
                        AnalogGameInput::MovementX(v) => {
                            self.key_state.analog_matrix.x = v;
                        },
                        AnalogGameInput::MovementY(v) => {
                            self.key_state.analog_matrix.y = v;
                        },
                        other => {
                            self.scene.handle_input_event(Event::AnalogGameInput(other));
                        },
                    },
                    Event::ScreenshotMessage(screenshot_message) => {
                        self.hud.new_message(comp::ChatMsg {
                            chat_type: comp::ChatType::CommandInfo,
                            message: screenshot_message,
                        })
                    },

                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    }, // TODO: Do something if the event wasn't handled?
                }
            }

            if !self.free_look {
                self.walk_forward_dir = self.scene.camera().forward_xy();
                self.walk_right_dir = self.scene.camera().right_xy();
                self.inputs.look_dir = Dir::from_unnormalized(cam_dir + aim_dir_offset).unwrap();
            }

            // Get the current state of movement related inputs
            let input_vec = self.key_state.dir_vec();
            let (axis_right, axis_up) = (input_vec[0], input_vec[1]);

            match self.scene.camera().get_mode() {
                camera::CameraMode::FirstPerson | camera::CameraMode::ThirdPerson => {
                    // Move the player character based on their walking direction.
                    // This could be different from the camera direction if free look is enabled.
                    self.inputs.move_dir =
                        self.walk_right_dir * axis_right + self.walk_forward_dir * axis_up;
                    self.freefly_vel = Vec3::zero();
                },

                camera::CameraMode::Freefly => {
                    // Move the camera freely in 3d space. Apply acceleration so that
                    // the movement feels more natural and controlled.
                    const FREEFLY_ACCEL: f32 = 120.0;
                    const FREEFLY_DAMPING: f32 = 80.0;
                    const FREEFLY_MAX_SPEED: f32 = 50.0;

                    let forward = self.scene.camera().forward();
                    let right = self.scene.camera().right();
                    let dir = right * axis_right + forward * axis_up;

                    let dt = global_state.clock.get_stable_dt().as_secs_f32();
                    if self.freefly_vel.magnitude_squared() > 0.01 {
                        let new_vel = self.freefly_vel
                            - self.freefly_vel.normalized() * (FREEFLY_DAMPING * dt);
                        if self.freefly_vel.dot(new_vel) > 0.0 {
                            self.freefly_vel = new_vel;
                        } else {
                            self.freefly_vel = Vec3::zero();
                        }
                    }
                    if dir.magnitude_squared() > 0.01 {
                        self.freefly_vel += dir * (FREEFLY_ACCEL * dt);
                        if self.freefly_vel.magnitude() > FREEFLY_MAX_SPEED {
                            self.freefly_vel = self.freefly_vel.normalized() * FREEFLY_MAX_SPEED;
                        }
                    }

                    let pos = self.scene.camera().get_focus_pos();
                    self.scene
                        .camera_mut()
                        .set_focus_pos(pos + self.freefly_vel * dt);

                    // Do not apply any movement to the player character
                    self.inputs.move_dir = Vec2::zero();
                },
            };

            self.inputs.climb = self.key_state.climb();
            self.inputs.fly.set_state(self.key_state.fly);
            self.inputs.move_z =
                self.key_state.swim_up as i32 as f32 - self.key_state.swim_down as i32 as f32;

            let mut outcomes = Vec::new();

            // Runs if either in a multiplayer server or the singleplayer server is unpaused
            if !global_state.paused() {
                // Perform an in-game tick.
                match self.tick(
                    global_state.clock.get_stable_dt(),
                    global_state,
                    &mut outcomes,
                ) {
                    Ok(TickAction::Continue) => {}, // Do nothing
                    Ok(TickAction::Disconnect) => return PlayStateResult::Pop, // Go to main menu
                    Err(err) => {
                        global_state.info_message = Some(
                            global_state
                                .i18n
                                .read()
                                .get("common.connection_lost")
                                .to_owned(),
                        );
                        error!("[session] Failed to tick the scene: {:?}", err);

                        return PlayStateResult::Pop;
                    },
                }
            }

            // Recompute dependents just in case some input modified the camera
            self.scene
                .camera_mut()
                .compute_dependents(&*self.client.borrow().state().terrain());

            // Generate debug info, if needed (it iterates through enough data that we might
            // as well avoid it unless we need it).
            let debug_info = global_state
                .settings
                .gameplay
                .toggle_debug
                .then(|| DebugInfo {
                    tps: global_state.clock.stats().average_tps,
                    frame_time: global_state.clock.stats().average_busy_dt,
                    ping_ms: self.client.borrow().get_ping_ms_rolling_avg(),
                    coordinates: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Pos>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    velocity: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Vel>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    ori: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<comp::Ori>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    num_chunks: self.scene.terrain().chunk_count() as u32,
                    num_lights: self.scene.lights().len() as u32,
                    num_visible_chunks: self.scene.terrain().visible_chunk_count() as u32,
                    num_shadow_chunks: self.scene.terrain().shadow_chunk_count() as u32,
                    num_figures: self.scene.figure_mgr().figure_count() as u32,
                    num_figures_visible: self.scene.figure_mgr().figure_count_visible() as u32,
                    num_particles: self.scene.particle_mgr().particle_count() as u32,
                    num_particles_visible: self.scene.particle_mgr().particle_count_visible()
                        as u32,
                });

            // Extract HUD events ensuring the client borrow gets dropped.
            let mut hud_events = self.hud.maintain(
                &self.client.borrow(),
                global_state,
                &debug_info,
                &self.scene.camera(),
                global_state.clock.get_stable_dt(),
                HudInfo {
                    is_aiming,
                    is_first_person: matches!(
                        self.scene.camera().get_mode(),
                        camera::CameraMode::FirstPerson
                    ),
                    target_entity: self.target_entity,
                    selected_entity: self.selected_entity,
                },
            );

            // Look for changes in the localization files
            if global_state.i18n.reloaded() {
                hud_events.push(HudEvent::ChangeLanguage(Box::new(
                    global_state.i18n.read().metadata.clone(),
                )));
            }

            // Maintain the UI.
            for event in hud_events {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    },
                    HudEvent::CharacterSelection => {
                        self.client.borrow_mut().request_remove_character()
                    },
                    HudEvent::Logout => {
                        self.client.borrow_mut().logout();
                        return PlayStateResult::Pop;
                    },
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    },
                    HudEvent::AdjustMousePan(sensitivity) => {
                        global_state.window.pan_sensitivity = sensitivity;
                        global_state.settings.gameplay.pan_sensitivity = sensitivity;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustMouseZoom(sensitivity) => {
                        global_state.window.zoom_sensitivity = sensitivity;
                        global_state.settings.gameplay.zoom_sensitivity = sensitivity;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleZoomInvert(zoom_inverted) => {
                        global_state.window.zoom_inversion = zoom_inverted;
                        global_state.settings.gameplay.zoom_inversion = zoom_inverted;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::Sct(sct) => {
                        global_state.settings.gameplay.sct = sct;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SctPlayerBatch(sct_player_batch) => {
                        global_state.settings.gameplay.sct_player_batch = sct_player_batch;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleTips(loading_tips) => {
                        global_state.settings.gameplay.loading_tips = loading_tips;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SctDamageBatch(sct_damage_batch) => {
                        global_state.settings.gameplay.sct_damage_batch = sct_damage_batch;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SpeechBubbleDarkMode(sbdm) => {
                        global_state.settings.gameplay.speech_bubble_dark_mode = sbdm;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SpeechBubbleIcon(sbi) => {
                        global_state.settings.gameplay.speech_bubble_icon = sbi;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleDebug(toggle_debug) => {
                        global_state.settings.gameplay.toggle_debug = toggle_debug;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleMouseYInvert(mouse_y_inverted) => {
                        global_state.window.mouse_y_inversion = mouse_y_inverted;
                        global_state.settings.gameplay.mouse_y_inversion = mouse_y_inverted;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleControllerYInvert(controller_y_inverted) => {
                        global_state.window.controller_settings.pan_invert_y =
                            controller_y_inverted;
                        global_state.settings.controller.pan_invert_y = controller_y_inverted;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleSmoothPan(smooth_pan_enabled) => {
                        global_state.settings.gameplay.smooth_pan_enable = smooth_pan_enabled;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustViewDistance(view_distance) => {
                        self.client.borrow_mut().set_view_distance(view_distance);

                        global_state.settings.graphics.view_distance = view_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustLodDetail(lod_detail) => {
                        self.scene.lod.set_detail(lod_detail);

                        global_state.settings.graphics.lod_detail = lod_detail;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustSpriteRenderDistance(sprite_render_distance) => {
                        global_state.settings.graphics.sprite_render_distance =
                            sprite_render_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustFigureLoDRenderDistance(figure_lod_render_distance) => {
                        global_state.settings.graphics.figure_lod_render_distance =
                            figure_lod_render_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::CrosshairTransp(crosshair_transp) => {
                        global_state.settings.gameplay.crosshair_transp = crosshair_transp;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChatTransp(chat_transp) => {
                        global_state.settings.gameplay.chat_transp = chat_transp;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChatCharName(chat_char_name) => {
                        global_state.settings.gameplay.chat_character_name = chat_char_name;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::CrosshairType(crosshair_type) => {
                        global_state.settings.gameplay.crosshair_type = crosshair_type;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::Intro(intro_show) => {
                        global_state.settings.gameplay.intro_show = intro_show;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleXpBar(xp_bar) => {
                        global_state.settings.gameplay.xp_bar = xp_bar;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleBarNumbers(bar_numbers) => {
                        global_state.settings.gameplay.bar_numbers = bar_numbers;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleShortcutNumbers(shortcut_numbers) => {
                        global_state.settings.gameplay.shortcut_numbers = shortcut_numbers;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::BuffPosition(buff_position) => {
                        global_state.settings.gameplay.buff_position = buff_position;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::UiScale(scale_change) => {
                        global_state.settings.gameplay.ui_scale =
                            self.hud.scale_change(scale_change);
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustMusicVolume(music_volume) => {
                        global_state.audio.set_music_volume(music_volume);

                        global_state.settings.audio.music_volume = music_volume;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustSfxVolume(sfx_volume) => {
                        global_state.audio.set_sfx_volume(sfx_volume);

                        global_state.settings.audio.sfx_volume = sfx_volume;
                        global_state.settings.save_to_file_warn();
                    },
                    //HudEvent::ChangeAudioDevice(name) => {
                    //    global_state.audio.set_device(name.clone());

                    //    global_state.settings.audio.output = AudioOutput::Device(name);
                    //    global_state.settings.save_to_file_warn();
                    //},
                    HudEvent::ChangeMaxFPS(fps) => {
                        global_state.settings.graphics.max_fps = fps;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::RemoveBuff(buff_id) => {
                        let mut client = self.client.borrow_mut();
                        client.remove_buff(buff_id);
                    },
                    HudEvent::UnlockSkill(skill) => {
                        let mut client = self.client.borrow_mut();
                        client.unlock_skill(skill);
                    },
                    HudEvent::UseSlot {
                        slot,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;

                        if !bypass_dialog {
                            if let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                            {
                                match slot {
                                    comp::slot::Slot::Inventory(inv_slot) => {
                                        let slot_deficit = inventory.free_after_equip(inv_slot);
                                        if slot_deficit < 0 {
                                            self.hud.set_prompt_dialog(PromptDialogSettings::new(
                                                format!(
                                                    "Equipping this item will result in \
                                                     insufficient inventory space to hold the \
                                                     items in your inventory and {} items will \
                                                     drop on the floor. Do you wish to continue?",
                                                    slot_deficit.abs()
                                                ),
                                                HudEvent::UseSlot {
                                                    slot,
                                                    bypass_dialog: true,
                                                },
                                                None,
                                            ));
                                            move_allowed = false;
                                        }
                                    },
                                    comp::slot::Slot::Equip(equip_slot) => {
                                        // Ensure there is a free slot that is not provided by the
                                        // item being unequipped
                                        let free_slots =
                                            inventory.free_slots_minus_equipped_item(equip_slot);
                                        if free_slots > 0 {
                                            let slot_deficit =
                                                inventory.free_after_unequip(equip_slot);
                                            if slot_deficit < 0 {
                                                self.hud.set_prompt_dialog(
                                                    PromptDialogSettings::new(
                                                        format!(
                                                            "Unequipping this item  will result \
                                                             in insufficient inventory space to \
                                                             hold the items in your inventory and \
                                                             {} items will drop on the floor. Do \
                                                             you wish to continue?",
                                                            slot_deficit.abs()
                                                        ),
                                                        HudEvent::UseSlot {
                                                            slot,
                                                            bypass_dialog: true,
                                                        },
                                                        None,
                                                    ),
                                                );
                                                move_allowed = false;
                                            }
                                        } else {
                                            move_allowed = false;
                                        }
                                    },
                                }
                            };
                        }

                        if move_allowed {
                            self.client.borrow_mut().use_slot(slot);
                        }
                    },
                    HudEvent::SwapSlots {
                        slot_a,
                        slot_b,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;
                        if !bypass_dialog {
                            if let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                            {
                                match (slot_a, slot_b) {
                                    (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
                                    | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
                                        if !inventory.can_swap(inv_slot, equip_slot) {
                                            move_allowed = false;
                                        } else {
                                            let slot_deficit =
                                                inventory.free_after_swap(equip_slot, inv_slot);
                                            if slot_deficit < 0 {
                                                self.hud.set_prompt_dialog(
                                                    PromptDialogSettings::new(
                                                        format!(
                                                            "This will result in dropping {} \
                                                             item(s) on the ground. Are you sure?",
                                                            slot_deficit.abs()
                                                        ),
                                                        HudEvent::SwapSlots {
                                                            slot_a,
                                                            slot_b,
                                                            bypass_dialog: true,
                                                        },
                                                        None,
                                                    ),
                                                );
                                                move_allowed = false;
                                            }
                                        }
                                    },
                                    _ => {},
                                }
                            }
                        }
                        if move_allowed {
                            self.client.borrow_mut().swap_slots(slot_a, slot_b);
                        }
                    },
                    HudEvent::SplitSwapSlots {
                        slot_a,
                        slot_b,
                        bypass_dialog,
                    } => {
                        let mut move_allowed = true;
                        if !bypass_dialog {
                            if let Some(inventory) = self
                                .client
                                .borrow()
                                .state()
                                .ecs()
                                .read_storage::<comp::Inventory>()
                                .get(self.client.borrow().entity())
                            {
                                match (slot_a, slot_b) {
                                    (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
                                    | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
                                        if !inventory.can_swap(inv_slot, equip_slot) {
                                            move_allowed = false;
                                        } else {
                                            let slot_deficit =
                                                inventory.free_after_swap(equip_slot, inv_slot);
                                            if slot_deficit < 0 {
                                                self.hud.set_prompt_dialog(
                                                    PromptDialogSettings::new(
                                                        format!(
                                                            "This will result in dropping {} \
                                                             item(s) on the ground. Are you sure?",
                                                            slot_deficit.abs()
                                                        ),
                                                        HudEvent::SwapSlots {
                                                            slot_a,
                                                            slot_b,
                                                            bypass_dialog: true,
                                                        },
                                                        None,
                                                    ),
                                                );
                                                move_allowed = false;
                                            }
                                        }
                                    },
                                    _ => {},
                                }
                            }
                        };
                        if move_allowed {
                            self.client.borrow_mut().split_swap_slots(slot_a, slot_b);
                        }
                    },
                    HudEvent::DropSlot(x) => {
                        let mut client = self.client.borrow_mut();
                        client.drop_slot(x);
                        if let comp::slot::Slot::Equip(comp::slot::EquipSlot::Lantern) = x {
                            client.disable_lantern();
                        }
                    },
                    HudEvent::SplitDropSlot(x) => {
                        let mut client = self.client.borrow_mut();
                        client.split_drop_slot(x);
                        if let comp::slot::Slot::Equip(comp::slot::EquipSlot::Lantern) = x {
                            client.disable_lantern();
                        }
                    },
                    HudEvent::ChangeHotbarState(state) => {
                        let client = self.client.borrow();

                        let server_name = &client.server_info().name;
                        // If we are changing the hotbar state this CANNOT be None.
                        let character_id = match client.presence().unwrap() {
                            PresenceKind::Character(id) => id,
                            PresenceKind::Spectator => {
                                unreachable!("HUD adaption in Spectator mode!")
                            },
                        };

                        // Get or update the ServerProfile.
                        global_state.profile.set_hotbar_slots(
                            server_name,
                            character_id,
                            state.slots,
                        );

                        global_state.profile.save_to_file_warn();

                        info!("Event! -> ChangedHotbarState")
                    },
                    HudEvent::TradeAction(action) => {
                        let mut client = self.client.borrow_mut();
                        client.perform_trade_action(action);
                    },
                    HudEvent::Ability3(state) => self.inputs.ability3.set_state(state),
                    HudEvent::Ability4(state) => self.inputs.ability4.set_state(state),
                    HudEvent::ChangeFOV(new_fov) => {
                        global_state.settings.graphics.fov = new_fov;
                        global_state.settings.save_to_file_warn();
                        self.scene.camera_mut().set_fov_deg(new_fov);
                        self.scene
                            .camera_mut()
                            .compute_dependents(&*self.client.borrow().state().terrain());
                    },
                    HudEvent::MapZoom(map_zoom) => {
                        global_state.settings.gameplay.map_zoom = map_zoom;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapDrag(map_drag) => {
                        global_state.settings.gameplay.map_drag = map_drag;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowDifficulty(map_show_difficulty) => {
                        global_state.settings.gameplay.map_show_difficulty = map_show_difficulty;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowTowns(map_show_towns) => {
                        global_state.settings.gameplay.map_show_towns = map_show_towns;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowDungeons(map_show_dungeons) => {
                        global_state.settings.gameplay.map_show_dungeons = map_show_dungeons;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowCastles(map_show_castles) => {
                        global_state.settings.gameplay.map_show_castles = map_show_castles;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowCaves(map_show_caves) => {
                        global_state.settings.gameplay.map_show_caves = map_show_caves;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MapShowTrees(map_show_trees) => {
                        global_state.settings.gameplay.map_show_trees = map_show_trees;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeGamma(new_gamma) => {
                        global_state.settings.graphics.gamma = new_gamma;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeExposure(new_exposure) => {
                        global_state.settings.graphics.exposure = new_exposure;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeAmbiance(new_ambiance) => {
                        global_state.settings.graphics.ambiance = new_ambiance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeRenderMode(new_render_mode) => {
                        // Do this first so if it crashes the setting isn't saved :)
                        global_state
                            .window
                            .renderer_mut()
                            .set_render_mode((&*new_render_mode).clone())
                            .unwrap();
                        global_state.settings.graphics.render_mode = *new_render_mode;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeLanguage(new_language) => {
                        global_state.settings.language.selected_language =
                            new_language.language_identifier;
                        global_state.i18n = Localization::load_expect(&i18n_asset_key(
                            &global_state.settings.language.selected_language,
                        ));
                        global_state.i18n.read().log_missing_entries();
                        self.hud.update_fonts(&global_state.i18n.read());
                    },
                    HudEvent::ChangeFullscreenMode(new_fullscreen_settings) => {
                        global_state
                            .window
                            .set_fullscreen_mode(new_fullscreen_settings);
                        global_state.settings.graphics.fullscreen = new_fullscreen_settings;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleParticlesEnabled(particles_enabled) => {
                        global_state.settings.graphics.particles_enabled = particles_enabled;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustWindowSize(new_size) => {
                        global_state.window.set_size(new_size.into());
                        global_state.settings.graphics.window_size = new_size;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeBinding(game_input) => {
                        global_state.window.set_keybinding_mode(game_input);
                    },
                    HudEvent::ResetBindings => {
                        global_state.settings.controls = ControlSettings::default();
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeFreeLookBehavior(behavior) => {
                        global_state.settings.gameplay.free_look_behavior = behavior;
                    },
                    HudEvent::ChangeAutoWalkBehavior(behavior) => {
                        global_state.settings.gameplay.auto_walk_behavior = behavior;
                    },
                    HudEvent::ChangeStopAutoWalkOnInput(state) => {
                        global_state.settings.gameplay.stop_auto_walk_on_input = state;
                    },
                    HudEvent::CraftRecipe(r) => {
                        self.client.borrow_mut().craft_recipe(&r);
                    },
                    HudEvent::InviteMember(uid) => {
                        self.client.borrow_mut().send_invite(uid, InviteKind::Group);
                    },
                    HudEvent::AcceptInvite => {
                        self.client.borrow_mut().accept_invite();
                    },
                    HudEvent::DeclineInvite => {
                        self.client.borrow_mut().decline_invite();
                    },
                    HudEvent::KickMember(uid) => {
                        self.client.borrow_mut().kick_from_group(uid);
                    },
                    HudEvent::LeaveGroup => {
                        self.client.borrow_mut().leave_group();
                    },
                    HudEvent::AssignLeader(uid) => {
                        self.client.borrow_mut().assign_group_leader(uid);
                    },
                    HudEvent::MinimapShow(state) => {
                        global_state.settings.gameplay.minimap_show = state;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::MinimapFaceNorth(state) => {
                        global_state.settings.gameplay.minimap_face_north = state;
                        global_state.settings.save_to_file_warn();
                    },
                }
            }

            {
                let client = self.client.borrow();
                let scene_data = SceneData {
                    state: client.state(),
                    player_entity: client.entity(),
                    // Only highlight if interactable
                    target_entity: self.interactable.and_then(Interactable::entity),
                    loaded_distance: client.loaded_distance(),
                    view_distance: client.view_distance().unwrap_or(1),
                    tick: client.get_tick(),
                    runtime: &client.runtime(),
                    gamma: global_state.settings.graphics.gamma,
                    exposure: global_state.settings.graphics.exposure,
                    ambiance: global_state.settings.graphics.ambiance,
                    mouse_smoothing: global_state.settings.gameplay.smooth_pan_enable,
                    sprite_render_distance: global_state.settings.graphics.sprite_render_distance
                        as f32,
                    particles_enabled: global_state.settings.graphics.particles_enabled,
                    figure_lod_render_distance: global_state
                        .settings
                        .graphics
                        .figure_lod_render_distance
                        as f32,
                    is_aiming,
                };

                // Runs if either in a multiplayer server or the singleplayer server is unpaused
                if !global_state.paused() {
                    self.scene.maintain(
                        global_state.window.renderer_mut(),
                        &mut global_state.audio,
                        &scene_data,
                        &client,
                    );

                    // Process outcomes from client
                    for outcome in outcomes {
                        self.scene
                            .handle_outcome(&outcome, &scene_data, &mut global_state.audio);
                        self.hud.handle_outcome(&outcome);
                    }
                }
            }

            // Clean things up after the tick.
            self.cleanup();

            PlayStateResult::Continue
        } else if client_registered && client_presence.is_none() {
            PlayStateResult::Switch(Box::new(CharSelectionState::new(
                global_state,
                Rc::clone(&self.client),
            )))
        } else {
            error!("Client not in the expected state, exiting session play state");
            PlayStateResult::Pop
        }
    }

    fn name(&self) -> &'static str { "Session" }

    /// Render the session to the screen.
    ///
    /// This method should be called once per frame.
    fn render(&mut self, renderer: &mut Renderer, settings: &Settings) {
        span!(_guard, "render", "<Session as PlayState>::render");
        // Render the screen using the global renderer
        {
            let client = self.client.borrow();

            let scene_data = SceneData {
                state: client.state(),
                player_entity: client.entity(),
                // Only highlight if interactable
                target_entity: self.interactable.and_then(Interactable::entity),
                loaded_distance: client.loaded_distance(),
                view_distance: client.view_distance().unwrap_or(1),
                tick: client.get_tick(),
                runtime: &client.runtime(),
                gamma: settings.graphics.gamma,
                exposure: settings.graphics.exposure,
                ambiance: settings.graphics.ambiance,
                mouse_smoothing: settings.gameplay.smooth_pan_enable,
                sprite_render_distance: settings.graphics.sprite_render_distance as f32,
                figure_lod_render_distance: settings.graphics.figure_lod_render_distance as f32,
                particles_enabled: settings.graphics.particles_enabled,
                is_aiming: self.is_aiming,
            };
            self.scene.render(
                renderer,
                client.state(),
                client.entity(),
                client.get_tick(),
                &scene_data,
            );
        }
        // Draw the UI to the screen
        self.hud.render(renderer, self.scene.globals());
    }
}

/// Max distance an entity can be "targeted"
const MAX_TARGET_RANGE: f32 = 300.0;
/// Calculate what the cursor is pointing at within the 3d scene
#[allow(clippy::type_complexity)]
fn under_cursor(
    client: &Client,
    cam_pos: Vec3<f32>,
    cam_dir: Vec3<f32>,
) -> (
    Option<Vec3<i32>>,
    Option<Vec3<i32>>,
    Option<(specs::Entity, f32)>,
) {
    span!(_guard, "under_cursor");
    // Choose a spot above the player's head for item distance checks
    let player_entity = client.entity();
    let ecs = client.state().ecs();
    let positions = ecs.read_storage::<comp::Pos>();
    let player_pos = match positions.get(player_entity) {
        Some(pos) => pos.0,
        None => cam_pos, // Should never happen, but a safe fallback
    };
    let scales = ecs.read_storage();
    let colliders = ecs.read_storage();
    let char_states = ecs.read_storage();
    // Get the player's cylinder
    let player_cylinder = Cylinder::from_components(
        player_pos,
        scales.get(player_entity).copied(),
        colliders.get(player_entity).copied(),
        char_states.get(player_entity),
    );
    let terrain = client.state().terrain();

    let cam_ray = terrain
        .ray(cam_pos, cam_pos + cam_dir * 100.0)
        .until(|block| block.is_filled() || block.is_collectible())
        .cast();

    let cam_dist = cam_ray.0;

    // The ray hit something, is it within range?
    let (build_pos, select_pos) = if matches!(cam_ray.1, Ok(Some(_)) if
        player_cylinder.min_distance(cam_pos + cam_dir * (cam_dist + 0.01))
        <= MAX_PICKUP_RANGE)
    {
        (
            Some((cam_pos + cam_dir * (cam_dist - 0.01)).map(|e| e.floor() as i32)),
            Some((cam_pos + cam_dir * (cam_dist + 0.01)).map(|e| e.floor() as i32)),
        )
    } else {
        (None, None)
    };

    // See if ray hits entities
    // Currently treated as spheres
    // Don't cast through blocks
    // Could check for intersection with entity from last frame to narrow this down
    let cast_dist = if let Ok(Some(_)) = cam_ray.1 {
        cam_dist.min(MAX_TARGET_RANGE)
    } else {
        MAX_TARGET_RANGE
    };

    // Need to raycast by distance to cam
    // But also filter out by distance to the player (but this only needs to be done
    // on final result)
    let mut nearby = (
        &ecs.entities(),
        &positions,
        scales.maybe(),
        &ecs.read_storage::<comp::Body>()
    )
        .join()
        .filter(|(e, _, _, _)| *e != player_entity)
        .map(|(e, p, s, b)| {
            const RADIUS_SCALE: f32 = 3.0;
            // TODO: use collider radius instead of body radius?
            let radius = s.map_or(1.0, |s| s.0) * b.radius() * RADIUS_SCALE;
            // Move position up from the feet
            let pos = Vec3::new(p.0.x, p.0.y, p.0.z + radius);
            // Distance squared from camera to the entity
            let dist_sqr = pos.distance_squared(cam_pos);
            (e, pos, radius, dist_sqr)
        })
        // Roughly filter out entities farther than ray distance
        .filter(|(_, _, r, d_sqr)| *d_sqr <= cast_dist.powi(2) + 2.0 * cast_dist * r + r.powi(2))
        // Ignore entities intersecting the camera
        .filter(|(_, _, r, d_sqr)| *d_sqr > r.powi(2))
        // Substract sphere radius from distance to the camera
        .map(|(e, p, r, d_sqr)| (e, p, r, d_sqr.sqrt() - r))
        .collect::<Vec<_>>();
    // Sort by distance
    nearby.sort_unstable_by(|a, b| a.3.partial_cmp(&b.3).unwrap());

    let seg_ray = LineSegment3 {
        start: cam_pos,
        end: cam_pos + cam_dir * cam_dist,
    };
    // TODO: fuzzy borders
    let target_entity = nearby
        .iter()
        .map(|(e, p, r, _)| (e, *p, r))
        // Find first one that intersects the ray segment
        .find(|(_, p, r)| seg_ray.projected_point(*p).distance_squared(*p) < r.powi(2))
        .and_then(|(e, p, _)| {
            // Get the entity's cylinder
            let target_cylinder = Cylinder::from_components(
                p,
                scales.get(*e).copied(),
                colliders.get(*e).copied(),
                char_states.get(*e),
            );

            let dist_to_player = player_cylinder.min_distance(target_cylinder);
            (dist_to_player < MAX_TARGET_RANGE).then_some((*e, dist_to_player))
        });

    // TODO: consider setting build/select to None when targeting an entity
    (build_pos, select_pos, target_entity)
}

#[derive(Clone, Copy)]
enum Interactable {
    Block(Block, Vec3<i32>),
    Entity(specs::Entity),
}

impl Interactable {
    fn entity(self) -> Option<specs::Entity> {
        match self {
            Self::Entity(e) => Some(e),
            Self::Block(_, _) => None,
        }
    }
}

/// Select interactable to hightlight, display interaction text for, and to
/// interact with if the interact key is pressed
/// Selected in the following order
/// 1) Targeted entity (if interactable) (entities can't be target through
/// blocks) 2) Selected block  (if interactabl)
/// 3) Closest of nearest interactable entity/block
fn select_interactable(
    client: &Client,
    target_entity: Option<(specs::Entity, f32)>,
    selected_pos: Option<Vec3<i32>>,
    scene: &Scene,
) -> Option<Interactable> {
    span!(_guard, "select_interactable");
    // TODO: once there are multiple distances for different types of interactions
    // this code will need to be revamped to cull things by varying distances
    // based on the types of interactions available for those things
    use common::{spiral::Spiral2d, terrain::TerrainChunk, vol::RectRasterableVol};
    target_entity
        .and_then(|(e, dist_to_player)| (dist_to_player < MAX_PICKUP_RANGE).then_some(Interactable::Entity(e)))
        .or_else(|| selected_pos.and_then(|sp|
                client.state().terrain().get(sp).ok().copied()
                    .filter(Block::is_collectible).map(|b| Interactable::Block(b, sp))
        ))
        .or_else(|| {
            let ecs = client.state().ecs();
            let player_entity = client.entity();
            let positions = ecs.read_storage::<comp::Pos>();
            let player_pos = positions.get(player_entity)?.0;

            let scales = ecs.read_storage::<comp::Scale>();
            let colliders = ecs.read_storage::<comp::Collider>();
            let char_states = ecs.read_storage::<comp::CharacterState>();

            let player_cylinder = Cylinder::from_components(
                player_pos,
                scales.get(player_entity).copied(),
                colliders.get(player_entity).copied(),
                char_states.get(player_entity),
            );

            let closest_interactable_entity = (
                &ecs.entities(),
                &positions,
                scales.maybe(),
                colliders.maybe(),
                char_states.maybe(),
            )
                .join()
                .filter(|(e, _, _, _, _)| *e != player_entity)
                .map(|(e, p, s, c, cs)| {
                    let cylinder = Cylinder::from_components(p.0, s.copied(), c.copied(), cs);
                    (e, cylinder)
                })
                // Roughly filter out entities farther than interaction distance
                .filter(|(_, cylinder)| player_cylinder.approx_in_range(*cylinder, MAX_PICKUP_RANGE))
                .map(|(e, cylinder)| (e, player_cylinder.min_distance(cylinder)))
                .min_by_key(|(_, dist)| OrderedFloat(*dist));

            // Only search as far as closest interactable entity
            let search_dist = closest_interactable_entity
                .map_or(MAX_PICKUP_RANGE, |(_, dist)| dist);
            let player_chunk = player_pos.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
                (e.floor() as i32).div_euclid(sz as i32)
            });
            let terrain = scene.terrain();

            // Find closest interactable block
            // TODO: consider doing this one first?
            let closest_interactable_block_pos = Spiral2d::new()
                // TODO: this formula for the number to take was guessed
                // Note: assume RECT_SIZE.x == RECT_SIZE.y
                .take(((search_dist / TerrainChunk::RECT_SIZE.x as f32).ceil() as usize * 2 + 1).pow(2))
                .flat_map(|offset| {
                    let chunk_pos = player_chunk + offset;
                    let chunk_voxel_pos =
                            Vec3::<i32>::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));
                    terrain.get(chunk_pos).map(|data| (data, chunk_voxel_pos))
                })
                // TODO: maybe we could make this more efficient by putting the
                // interactables is some sort of spatial structure
                .flat_map(|(chunk_data, chunk_pos)| {
                    chunk_data
                        .blocks_of_interest
                        .interactables
                        .iter()
                        .map(move |block_offset| chunk_pos + block_offset)
                })
                .map(|block_pos| (
                        block_pos,
                        block_pos.map(|e| e as f32 + 0.5)
                            .distance_squared(player_pos)
                ))
                .min_by_key(|(_, dist_sqr)| OrderedFloat(*dist_sqr))
                .map(|(block_pos, _)| block_pos);

            // Pick closer one if they exist
            closest_interactable_block_pos
                .filter(|block_pos|  player_cylinder.min_distance(Cube { min: block_pos.as_(), side_length: 1.0}) < search_dist)
                .and_then(|block_pos|
                    client.state().terrain().get(block_pos).ok().copied()
                        .map(|b| Interactable::Block(b, block_pos))
                )
                .or_else(|| closest_interactable_entity.map(|(e, _)| Interactable::Entity(e)))
        })
}
