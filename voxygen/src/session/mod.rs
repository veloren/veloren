pub mod interactable;
pub mod settings_change;
mod target;

use std::{cell::RefCell, collections::HashSet, rc::Rc, result::Result, time::Duration};

#[cfg(not(target_os = "macos"))]
use mumble_link::SharedLink;
use ordered_float::OrderedFloat;
use specs::{Join, WorldExt};
use tracing::{error, info};
use vek::*;

use client::{self, Client};
use common::{
    comp,
    comp::{
        inventory::slot::{EquipSlot, Slot},
        invite::InviteKind,
        item::{
            tool::{AbilityMap, ToolKind},
            ItemDesc, MaterialStatManifest,
        },
        ChatMsg, ChatType, InputKind, InventoryUpdateEvent, Pos, Stats, UtteranceKind, Vel,
    },
    consts::MAX_MOUNT_RANGE,
    link::Is,
    mounting::Mount,
    outcome::Outcome,
    recipe,
    terrain::{Block, BlockKind},
    trade::TradeResult,
    util::{Dir, Plane},
    vol::ReadVol,
};
use common_base::{prof_span, span};
use common_net::{
    msg::{server::InviteAnswer, PresenceKind},
    sync::WorldSyncExt,
};

use crate::{
    audio::sfx::SfxEvent,
    cmd::run_command,
    error::Error,
    game_input::GameInput,
    hud::{
        DebugInfo, Event as HudEvent, Hud, HudCollectFailedReason, HudInfo, LootMessage,
        PromptDialogSettings,
    },
    key_state::KeyState,
    menu::char_selection::CharSelectionState,
    render::{Drawer, GlobalsBindGroup},
    scene::{camera, terrain::Interaction, CameraMode, DebugShapeId, Scene, SceneData},
    settings::Settings,
    window::{AnalogGameInput, Event},
    Direction, GlobalState, PlayState, PlayStateResult,
};
use hashbrown::HashMap;
use interactable::{select_interactable, Interactable};
use settings_change::Language::ChangeLanguage;
use target::targets_under_cursor;
#[cfg(feature = "egui-ui")]
use voxygen_egui::EguiDebugInfo;

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
    inputs_state: HashSet<GameInput>,
    selected_block: Block,
    walk_forward_dir: Vec2<f32>,
    walk_right_dir: Vec2<f32>,
    freefly_vel: Vec3<f32>,
    free_look: bool,
    auto_walk: bool,
    camera_clamp: bool,
    is_aiming: bool,
    target_entity: Option<specs::Entity>,
    selected_entity: Option<(specs::Entity, std::time::Instant)>,
    interactable: Option<Interactable>,
    #[cfg(not(target_os = "macos"))]
    mumble_link: SharedLink,
    hitboxes: HashMap<specs::Entity, DebugShapeId>,
}

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        // Create a scene for this session. The scene handles visible elements of the
        // game world.
        let mut scene = Scene::new(
            global_state.window.renderer_mut(),
            &mut global_state.lazy_init,
            &*client.borrow(),
            &global_state.settings,
        );
        scene
            .camera_mut()
            .set_fov_deg(global_state.settings.graphics.fov);
        #[cfg(not(target_os = "macos"))]
        let mut mumble_link = SharedLink::new("veloren", "veloren-voxygen");
        {
            let mut client = client.borrow_mut();
            client.request_player_physics(global_state.settings.gameplay.player_physics_behavior);
            client.request_lossy_terrain_compression(
                global_state.settings.graphics.lossy_terrain_compression,
            );
            #[cfg(not(target_os = "macos"))]
            if let Some(uid) = client.uid() {
                let identiy = if let Some(info) = client.player_list().get(&uid) {
                    format!("{}-{}", info.player_alias, uid)
                } else {
                    format!("unknown-{}", uid)
                };
                mumble_link.set_identity(&identiy);
                // TODO: evaluate context
            }
        }
        let hud = Hud::new(global_state, &client.borrow());
        let walk_forward_dir = scene.camera().forward_xy();
        let walk_right_dir = scene.camera().right_xy();

        Self {
            scene,
            client,
            key_state: KeyState::default(),
            inputs: comp::ControllerInputs::default(),
            inputs_state: HashSet::new(),
            hud,
            selected_block: Block::new(BlockKind::Misc, Rgb::broadcast(255)),
            walk_forward_dir,
            walk_right_dir,
            freefly_vel: Vec3::zero(),
            free_look: false,
            auto_walk: false,
            camera_clamp: false,
            is_aiming: false,
            target_entity: None,
            selected_entity: None,
            interactable: None,
            #[cfg(not(target_os = "macos"))]
            mumble_link,
            hitboxes: HashMap::new(),
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

        let mut client = self.client.borrow_mut();
        self.scene
            .maintain_debug_hitboxes(&client, &global_state.settings, &mut self.hitboxes);

        #[cfg(not(target_os = "macos"))]
        {
            // Update mumble positional audio
            let pos = client.position().unwrap_or_default();
            let ori = client
                .state()
                .read_storage::<comp::Ori>()
                .get(client.entity())
                .map_or_else(comp::Ori::default, |o| *o);
            let front = ori.look_dir().to_vec();
            let top = ori.up().to_vec();
            // converting from veloren z = height axis, to mumble y = height axis
            let player_pos = mumble_link::Position {
                position: [pos.x, pos.z, pos.y],
                front: [front.x, front.z, front.y],
                top: [top.x, top.z, top.y],
            };
            self.mumble_link.update(player_pos, player_pos);
        }

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
                        None => match client.state().ecs().entity_from_uid(target.0) {
                            Some(entity) => {
                                let stats = client.state().read_storage::<Stats>();
                                stats
                                    .get(entity)
                                    .map_or(format!("<entity {}>", target), |e| e.name.to_owned())
                            },
                            None => format!("<uid {}>", target),
                        },
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
                    self.hud.clear_cursor();
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

                    match inv_event {
                        InventoryUpdateEvent::Dropped 
                        | InventoryUpdateEvent::Swapped
                        | InventoryUpdateEvent::Given
                        | InventoryUpdateEvent::Collected(_)
                        | InventoryUpdateEvent::EntityCollectFailed(_)
                        | InventoryUpdateEvent::BlockCollectFailed(_)
                        | InventoryUpdateEvent::Craft => {
                            global_state
                                .audio
                                .emit_sfx_item(sfx_trigger_item, Some(1.0));

                        }
                        _ => global_state
                                .audio
                                .emit_sfx(sfx_trigger_item, client.position().unwrap_or_default(), Some(1.0), false)
                    }


                    match inv_event {
                        InventoryUpdateEvent::BlockCollectFailed { pos, reason } => {
                            self.hud.add_failed_block_pickup(
                                pos,
                                HudCollectFailedReason::from_server_reason(
                                    &reason,
                                    client.state().ecs(),
                                ),
                            );
                        },
                        InventoryUpdateEvent::EntityCollectFailed {
                            entity: uid,
                            reason,
                        } => {
                            if let Some(entity) = client.state().ecs().entity_from_uid(uid.into()) {
                                self.hud.add_failed_entity_pickup(
                                    entity,
                                    HudCollectFailedReason::from_server_reason(
                                        &reason,
                                        client.state().ecs(),
                                    ),
                                );
                            }
                        },
                        InventoryUpdateEvent::Collected(item) => {
                            let ability_map = AbilityMap::load().read();
                            let msm = MaterialStatManifest::load().read();
                            self.hud.new_loot_message(LootMessage {
                                item: item.duplicate(&ability_map, &msm),
                                amount: item.amount(),
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
                        global_state.i18n.read().get("main.login.kicked"),
                        reason
                    ));
                    return Ok(TickAction::Disconnect);
                },
                client::Event::Notification(n) => {
                    self.hud.new_notification(n);
                },
                client::Event::SetViewDistance(vd) => {
                    global_state.settings.graphics.view_distance = vd;
                    global_state
                        .settings
                        .save_to_file_warn(&global_state.config_dir);
                },
                client::Event::Outcome(outcome) => outcomes.push(outcome),
                client::Event::CharacterCreated(_) => {},
                client::Event::CharacterEdited(_) => {},
                client::Event::CharacterError(error) => {
                    global_state.client_error = Some(error);
                },
                client::Event::MapMarker(event) => {
                    self.hud.show.update_map_markers(event);
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
            let camera = self.scene.camera_mut();

            // Clamp camera's vertical angle if the toggle is enabled
            if self.camera_clamp {
                let mut cam_dir = camera.get_orientation();
                let cam_dir_clamp =
                    (global_state.settings.gameplay.camera_clamp_angle as f32).to_radians();
                cam_dir.y = (-cam_dir_clamp).max(cam_dir.y).min(cam_dir_clamp);
                camera.set_orientation(cam_dir);
            }

            let client = self.client.borrow();
            let player_entity = client.entity();

            if global_state.settings.gameplay.bow_zoom {
                let mut fov_scaling = 1.0;
                if let Some(comp::CharacterState::ChargedRanged(cr)) = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(player_entity)
                {
                    if cr.charge_frac() > 0.5 {
                        fov_scaling -= 3.0 * cr.charge_frac() / 5.0;
                    }
                }
                camera.set_fixate(fov_scaling);
            } else {
                camera.set_fixate(1.0);
            }

            // Compute camera data
            camera.compute_dependents(&*self.client.borrow().state().terrain());
            let camera::Dependents {
                cam_pos, cam_dir, ..
            } = self.scene.camera().dependents();
            let focus_pos = self.scene.camera().get_focus_pos();
            let focus_off = focus_pos.map(|e| e.trunc());
            let cam_pos = cam_pos + focus_off;

            let (is_aiming, aim_dir_offset) = {
                let is_aiming = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(player_entity)
                    .map(|cs| cs.is_aimed())
                    .unwrap_or(false);

                (
                    is_aiming,
                    if is_aiming && self.scene.camera().get_mode() == CameraMode::ThirdPerson {
                        Vec3::unit_z() * 0.025
                    } else {
                        Vec3::zero()
                    },
                )
            };
            self.is_aiming = is_aiming;

            let can_build = client
                .state()
                .read_storage::<comp::CanBuild>()
                .get(player_entity)
                .map_or_else(|| false, |cb| cb.enabled);

            let is_mining = client
                .inventories()
                .get(player_entity)
                .and_then(|inv| inv.equipped(EquipSlot::ActiveMainhand))
                .and_then(|item| item.tool_info())
                .map_or(false, |tool_kind| tool_kind == ToolKind::Pick)
                && client.is_wielding() == Some(true);

            // Check to see whether we're aiming at anything
            let (build_target, collect_target, entity_target, mine_target, terrain_target) =
                targets_under_cursor(&client, cam_pos, cam_dir, can_build, is_mining);

            self.interactable = select_interactable(
                &client,
                collect_target,
                entity_target,
                mine_target,
                &self.scene,
            );

            drop(client);

            // Nearest block to consider with GameInput primary or secondary key.
            let nearest_block_dist = find_shortest_distance(&[
                mine_target.filter(|_| is_mining).map(|t| t.distance),
                build_target.filter(|_| can_build).map(|t| t.distance),
            ]);
            // Nearest block to be highlighted in the scene (self.scene.set_select_pos).
            let nearest_scene_dist = find_shortest_distance(&[
                nearest_block_dist,
                collect_target.filter(|_| !is_mining).map(|t| t.distance),
            ]);
            // Set break_block_pos only if mining is closest.
            self.inputs.break_block_pos = if let Some(mt) =
                mine_target.filter(|mt| is_mining && nearest_scene_dist == Some(mt.distance))
            {
                self.scene.set_select_pos(Some(mt.position_int()));
                Some(mt.position)
            } else if let Some(bt) =
                build_target.filter(|bt| can_build && nearest_scene_dist == Some(bt.distance))
            {
                self.scene.set_select_pos(Some(bt.position_int()));
                None
            } else if let Some(ct) =
                collect_target.filter(|ct| nearest_scene_dist == Some(ct.distance))
            {
                self.scene.set_select_pos(Some(ct.position_int()));
                None
            } else {
                self.scene.set_select_pos(None);
                None
            };

            // filled block in line of sight
            let default_select_pos = terrain_target.map(|tt| tt.position);

            // Throw out distance info, it will be useful in the future
            self.target_entity = entity_target.map(|t| t.kind.0);

            // Handle window events.
            for event in events {
                // Pass all events to the ui first.
                {
                    let client = self.client.borrow();
                    let inventories = client.inventories();
                    let inventory = inventories.get(client.entity());
                    if self
                        .hud
                        .handle_event(event.clone(), global_state, inventory)
                    {
                        continue;
                    }
                }

                match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    },
                    Event::InputUpdate(input, state)
                        if state != self.inputs_state.contains(&input) =>
                    {
                        if !self.inputs_state.insert(input) {
                            self.inputs_state.remove(&input);
                        }

                        match input {
                            GameInput::Primary => {
                                let mut client = self.client.borrow_mut();
                                // Mine and build targets can be the same block. make building take
                                // precedence.
                                // Order of precedence: build, then mining, then attack.
                                if let Some(build_target) = build_target.filter(|bt| {
                                    state && can_build && nearest_block_dist == Some(bt.distance)
                                }) {
                                    client.remove_block(build_target.position_int());
                                } else {
                                    client.handle_input(
                                        InputKind::Primary,
                                        state,
                                        default_select_pos,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::Secondary => {
                                let mut client = self.client.borrow_mut();
                                if let Some(build_target) = build_target.filter(|bt| {
                                    state && can_build && nearest_block_dist == Some(bt.distance)
                                }) {
                                    let selected_pos = build_target.kind.0;
                                    client.place_block(
                                        selected_pos.map(|p| p.floor() as i32),
                                        self.selected_block,
                                    );
                                } else {
                                    client.handle_input(
                                        InputKind::Secondary,
                                        state,
                                        default_select_pos,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::Block => {
                                self.client.borrow_mut().handle_input(
                                    InputKind::Block,
                                    state,
                                    None,
                                    self.target_entity,
                                );
                            },
                            GameInput::Roll => {
                                let mut client = self.client.borrow_mut();
                                if can_build {
                                    if state {
                                        if let Some(block) = build_target.and_then(|bt| {
                                            client
                                                .state()
                                                .terrain()
                                                .get(bt.position_int())
                                                .ok()
                                                .copied()
                                        }) {
                                            self.selected_block = block;
                                        }
                                    }
                                } else {
                                    client.handle_input(
                                        InputKind::Roll,
                                        state,
                                        None,
                                        self.target_entity,
                                    );
                                }
                            },
                            GameInput::Respawn => {
                                self.stop_auto_walk();
                                if state {
                                    self.client.borrow_mut().respawn();
                                }
                            },
                            GameInput::Jump => {
                                self.client.borrow_mut().handle_input(
                                    InputKind::Jump,
                                    state,
                                    None,
                                    self.target_entity,
                                );
                            },
                            GameInput::SwimUp => {
                                self.key_state.swim_up = state;
                            },
                            GameInput::SwimDown => {
                                self.key_state.swim_down = state;
                            },
                            GameInput::Sit => {
                                if state {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_sit();
                                }
                            },
                            GameInput::Dance => {
                                if state {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_dance();
                                }
                            },
                            GameInput::Greet => {
                                if state {
                                    self.client.borrow_mut().utter(UtteranceKind::Greeting);
                                }
                            },
                            GameInput::Sneak => {
                                let is_trading = self.client.borrow().is_trading();
                                if state && !is_trading {
                                    self.stop_auto_walk();
                                    self.client.borrow_mut().toggle_sneak();
                                }
                            },
                            GameInput::MoveForward => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.up = state
                            },
                            GameInput::MoveBack => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.down = state
                            },
                            GameInput::MoveLeft => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.left = state
                            },
                            GameInput::MoveRight => {
                                if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                                    self.stop_auto_walk();
                                }
                                self.key_state.right = state
                            },
                            GameInput::Glide => {
                                let is_trading = self.client.borrow().is_trading();
                                if state && !is_trading {
                                    if global_state.settings.gameplay.stop_auto_walk_on_input {
                                        self.stop_auto_walk();
                                    }
                                    self.client.borrow_mut().toggle_glide();
                                }
                            },
                            GameInput::Fly => {
                                // Not sure where to put comment, but I noticed
                                // when testing flight.
                                //
                                // Syncing of inputs between mounter and mountee
                                // broke with controller change
                                self.key_state.fly ^= state;
                                self.client.borrow_mut().handle_input(
                                    InputKind::Fly,
                                    self.key_state.fly,
                                    None,
                                    self.target_entity,
                                );
                            },
                            GameInput::Climb => {
                                self.key_state.climb_up = state;
                            },
                            GameInput::ClimbDown => {
                                self.key_state.climb_down = state;
                            },
                            GameInput::ToggleWield => {
                                if state {
                                    self.client.borrow_mut().toggle_wield();
                                }
                            },
                            GameInput::SwapLoadout => {
                                if state {
                                    self.client.borrow_mut().swap_loadout();
                                }
                            },
                            GameInput::ToggleLantern if state => {
                                let mut client = self.client.borrow_mut();
                                if client.is_lantern_enabled() {
                                    client.disable_lantern();
                                } else {
                                    client.enable_lantern();
                                }
                            },
                            GameInput::Mount if state => {
                                let mut client = self.client.borrow_mut();
                                if client.is_riding() {
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
                                            // TODO: More cleverly filter by things that can actually be mounted
                                            !&client.state().ecs().read_storage::<Is<Mount>>(),
                                            client.state().ecs().read_storage::<comp::Alignment>().maybe(),
                                        )
                                            .join()
                                            .filter(|(entity, _, _, _)| *entity != client.entity())
                                            .filter(|(_, _, _, alignment)| matches!(alignment, Some(comp::Alignment::Owned(owner)) if Some(*owner) == client.uid()))
                                            .map(|(entity, pos, _, _)| {
                                                (entity, player_pos.0.distance_squared(pos.0))
                                            })
                                            .filter(|(_, dist_sqr)| {
                                                *dist_sqr < MAX_MOUNT_RANGE.powi(2)
                                            })
                                            .min_by_key(|(_, dist_sqr)| OrderedFloat(*dist_sqr));
                                        if let Some((mountee_entity, _)) = closest_mountable_entity
                                        {
                                            client.mount(mountee_entity);
                                        }
                                    }
                                }
                            },
                            GameInput::Interact => {
                                if state {
                                    if let Some(interactable) = self.interactable {
                                        let mut client = self.client.borrow_mut();
                                        match interactable {
                                            Interactable::Block(block, pos, interaction) => {
                                                match interaction {
                                                    Interaction::Collect => {
                                                        if block.is_collectible() {
                                                            client.collect_block(pos);
                                                        }
                                                    },
                                                    Interaction::Craft(tab) => {
                                                        self.hud.show.open_crafting_tab(
                                                            tab,
                                                            block.get_sprite().map(|s| (pos, s)),
                                                        )
                                                    },
                                                    Interaction::Mine => {},
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
                                                } else if client
                                                    .state()
                                                    .ecs()
                                                    .read_storage::<comp::Body>()
                                                    .get(entity)
                                                    .map_or(false, |b| b.is_campfire())
                                                {
                                                    // TODO: maybe start crafting instead?
                                                    client.toggle_sit();
                                                } else {
                                                    client.npc_interact(entity);
                                                }
                                            },
                                        }
                                    }
                                }
                            },
                            GameInput::Trade => {
                                if state {
                                    if let Some(interactable) = self.interactable {
                                        let mut client = self.client.borrow_mut();
                                        match interactable {
                                            Interactable::Block(_, _, _) => {},
                                            Interactable::Entity(entity) => {
                                                if let Some(uid) =
                                                    client.state().ecs().uid_from_entity(entity)
                                                {
                                                    let name = client
                                                        .player_list()
                                                        .get(&uid)
                                                        .map(|info| info.player_alias.clone())
                                                        .unwrap_or_else(|| {
                                                            let stats = client
                                                                .state()
                                                                .read_storage::<Stats>();
                                                            stats.get(entity).map_or(
                                                                format!("<entity {:?}>", uid),
                                                                |e| e.name.to_owned(),
                                                            )
                                                        });
                                                    let msg = global_state
                                                        .i18n
                                                        .read()
                                                        .get("hud.trade.invite_sent")
                                                        .replace("{playername}", &name);
                                                    self.hud
                                                        .new_message(ChatType::Meta.chat_msg(msg));
                                                    client.send_invite(uid, InviteKind::Trade)
                                                };
                                            },
                                        }
                                    }
                                }
                            },
                            GameInput::FreeLook => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.free_look_behavior.update(
                                    state,
                                    &mut self.free_look,
                                    |b| hud.free_look(b),
                                );
                            },
                            GameInput::AutoWalk => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.auto_walk_behavior.update(
                                    state,
                                    &mut self.auto_walk,
                                    |b| hud.auto_walk(b),
                                );

                                self.key_state.auto_walk =
                                    self.auto_walk && !self.client.borrow().is_gliding();
                            },
                            GameInput::CameraClamp => {
                                let hud = &mut self.hud;
                                global_state.settings.gameplay.camera_clamp_behavior.update(
                                    state,
                                    &mut self.camera_clamp,
                                    |b| hud.camera_clamp(b),
                                );
                            },
                            GameInput::CycleCamera if state => {
                                // Prevent accessing camera modes which aren't available in
                                // multiplayer unless you are an
                                // admin. This is an easily bypassed clientside check.
                                // The server should do its own filtering of which entities are sent
                                // to clients to prevent abuse.
                                let camera = self.scene.camera_mut();
                                camera.next_mode(self.client.borrow().is_moderator());
                            },
                            GameInput::Select => {
                                if !state {
                                    self.selected_entity =
                                        self.target_entity.map(|e| (e, std::time::Instant::now()));
                                }
                            },
                            GameInput::AcceptGroupInvite if state => {
                                let mut client = self.client.borrow_mut();
                                if client.invite().is_some() {
                                    client.accept_invite();
                                }
                            },
                            GameInput::DeclineGroupInvite if state => {
                                let mut client = self.client.borrow_mut();
                                if client.invite().is_some() {
                                    client.decline_invite();
                                }
                            },
                            _ => {},
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
                            self.scene.handle_input_event(
                                Event::AnalogGameInput(other),
                                &self.client.borrow(),
                            );
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
                        self.scene.handle_input_event(event, &self.client.borrow());
                    }, // TODO: Do something if the event wasn't handled?
                }
            }

            // If auto-gliding, point camera into the wind
            if let Some(dir) = self
                .auto_walk
                .then_some(self.client.borrow())
                .filter(|client| client.is_gliding())
                .and_then(|client| {
                    let ecs = client.state().ecs();
                    let entity = client.entity();
                    let fluid = ecs
                        .read_storage::<comp::PhysicsState>()
                        .get(entity)?
                        .in_fluid?;
                    ecs.read_storage::<comp::Vel>()
                        .get(entity)
                        .map(|vel| fluid.relative_flow(vel).0)
                        .map(|rel_flow| {
                            let is_wind_downwards = rel_flow.dot(Vec3::unit_z()).is_sign_negative();
                            if !self.free_look {
                                if is_wind_downwards {
                                    self.scene.camera().forward_xy().into()
                                } else {
                                    let windwards = rel_flow
                                        * self
                                            .scene
                                            .camera()
                                            .forward_xy()
                                            .dot(rel_flow.xy())
                                            .signum();
                                    Plane::from(Dir::new(self.scene.camera().right()))
                                        .projection(windwards)
                                }
                            } else if is_wind_downwards {
                                Vec3::from(-rel_flow.xy())
                            } else {
                                -rel_flow
                            }
                        })
                        .and_then(Dir::from_unnormalized)
                })
            {
                self.key_state.auto_walk = false;
                self.inputs.move_dir = Vec2::zero();
                self.inputs.look_dir = dir;
            } else {
                self.key_state.auto_walk = self.auto_walk;
                if !self.free_look {
                    self.walk_forward_dir = self.scene.camera().forward_xy();
                    self.walk_right_dir = self.scene.camera().right_xy();
                    self.inputs.look_dir =
                        Dir::from_unnormalized(cam_dir + aim_dir_offset).unwrap();
                }
            }
            self.inputs.strafing = matches!(
                self.scene.camera().get_mode(),
                camera::CameraMode::FirstPerson
            );

            // Get the current state of movement related inputs
            let input_vec = self.key_state.dir_vec();
            let (axis_right, axis_up) = (input_vec[0], input_vec[1]);
            let dt = global_state.clock.get_stable_dt().as_secs_f32();

            // Auto camera mode
            if global_state.settings.gameplay.auto_camera
                && matches!(
                    self.scene.camera().get_mode(),
                    camera::CameraMode::ThirdPerson | camera::CameraMode::FirstPerson
                )
                && input_vec.magnitude_squared() > 0.0
            {
                let camera = self.scene.camera_mut();
                let ori = camera.get_orientation();
                camera.set_orientation_instant(Vec3::new(
                    ori.x
                        + input_vec.x
                            * (3.0 - input_vec.y * 1.5 * if is_aiming { 1.5 } else { 1.0 })
                            * dt,
                    std::f32::consts::PI * if is_aiming { 0.015 } else { 0.1 },
                    0.0,
                ));
            }

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

            // Generate debug info, if needed
            // (it iterates through enough data that we might
            // as well avoid it unless we need it).
            let debug_info = global_state.settings.interface.toggle_debug.then(|| {
                let client = self.client.borrow();
                let ecs = client.state().ecs();
                let entity = client.entity();
                let coordinates = ecs.read_storage::<Pos>().get(entity).cloned();
                let velocity = ecs.read_storage::<Vel>().get(entity).cloned();
                let ori = ecs.read_storage::<comp::Ori>().get(entity).cloned();
                let look_dir = self.inputs.look_dir;
                let in_fluid = ecs
                    .read_storage::<comp::PhysicsState>()
                    .get(entity)
                    .and_then(|state| state.in_fluid);
                let character_state = ecs
                    .read_storage::<comp::CharacterState>()
                    .get(entity)
                    .cloned();

                DebugInfo {
                    tps: global_state.clock.stats().average_tps,
                    frame_time: global_state.clock.stats().average_busy_dt,
                    ping_ms: self.client.borrow().get_ping_ms_rolling_avg(),
                    coordinates,
                    velocity,
                    ori,
                    look_dir,
                    character_state,
                    in_fluid,
                    num_chunks: self.scene.terrain().chunk_count() as u32,
                    num_lights: self.scene.lights().len() as u32,
                    num_visible_chunks: self.scene.terrain().visible_chunk_count() as u32,
                    num_shadow_chunks: self.scene.terrain().shadow_chunk_count() as u32,
                    num_figures: self.scene.figure_mgr().figure_count() as u32,
                    num_figures_visible: self.scene.figure_mgr().figure_count_visible() as u32,
                    num_particles: self.scene.particle_mgr().particle_count() as u32,
                    num_particles_visible: self.scene.particle_mgr().particle_count_visible()
                        as u32,
                }
            });

            // Extract HUD events ensuring the client borrow gets dropped.
            let mut hud_events = self.hud.maintain(
                &self.client.borrow(),
                global_state,
                &debug_info,
                self.scene.camera(),
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
                self.interactable,
            );

            // Maintain egui (debug interface)
            #[cfg(feature = "egui-ui")]
            if global_state.settings.interface.egui_enabled() {
                let settings_change = global_state.egui_state.maintain(
                    &mut self.client.borrow_mut(),
                    &mut self.scene,
                    debug_info.map(|debug_info| EguiDebugInfo {
                        frame_time: debug_info.frame_time,
                        ping_ms: debug_info.ping_ms,
                    }),
                    &global_state.settings,
                );

                if let Some(settings_change) = settings_change {
                    settings_change.process(global_state, self);
                }
            }

            // Look for changes in the localization files
            if global_state.i18n.reloaded() {
                hud_events.push(HudEvent::SettingsChange(
                    ChangeLanguage(Box::new(global_state.i18n.read().metadata().clone())).into(),
                ));
            }

            // Maintain the UI.
            for event in hud_events {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    },
                    HudEvent::SendCommand(name, args) => {
                        match run_command(&mut self.client.borrow_mut(), global_state, &name, args)
                        {
                            Ok(Some(info)) => {
                                self.hud.new_message(ChatType::CommandInfo.chat_msg(&info))
                            },
                            Ok(None) => {}, // Server will provide an info message
                            Err(error) => {
                                self.hud.new_message(ChatType::CommandError.chat_msg(error))
                            },
                        };
                    },
                    HudEvent::CharacterSelection => {
                        self.client.borrow_mut().request_remove_character()
                    },
                    HudEvent::Logout => {
                        self.client.borrow_mut().logout();
                        global_state.audio.stop_ambient_sounds();
                        global_state.audio.stop_all_sfx();
                        return PlayStateResult::Pop;
                    },
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    },

                    HudEvent::RemoveBuff(buff_id) => {
                        self.client.borrow_mut().remove_buff(buff_id);
                    },
                    HudEvent::UnlockSkill(skill) => {
                        self.client.borrow_mut().unlock_skill(skill);
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
                    HudEvent::SwapEquippedWeapons => {
                        self.client.borrow_mut().swap_loadout();
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
                    HudEvent::SortInventory => {
                        self.client.borrow_mut().sort_inventory();
                    },
                    HudEvent::ChangeHotbarState(state) => {
                        let client = self.client.borrow();

                        let server_name = &client.server_info().name;
                        // If we are changing the hotbar state this CANNOT be None.
                        let character_id = match client.presence().unwrap() {
                            PresenceKind::Character(id) => Some(id),
                            PresenceKind::Spectator => {
                                unreachable!("HUD adaption in Spectator mode!")
                            },
                            PresenceKind::Possessor => None,
                        };

                        // Get or update the ServerProfile.
                        global_state.profile.set_hotbar_slots(
                            server_name,
                            character_id,
                            state.slots,
                        );

                        global_state
                            .profile
                            .save_to_file_warn(&global_state.config_dir);

                        info!("Event! -> ChangedHotbarState")
                    },
                    HudEvent::TradeAction(action) => {
                        self.client.borrow_mut().perform_trade_action(action);
                    },
                    HudEvent::Ability(i, state) => {
                        self.client.borrow_mut().handle_input(
                            InputKind::Ability(i),
                            state,
                            default_select_pos,
                            self.target_entity,
                        );
                    },

                    HudEvent::RequestSiteInfo(id) => {
                        self.client.borrow_mut().request_site_economy(id);
                    },

                    HudEvent::CraftRecipe {
                        recipe,
                        craft_sprite,
                    } => {
                        let slots = {
                            let client = self.client.borrow();
                            if let Some(recipe) = client.recipe_book().get(&recipe) {
                                client
                                    .inventories()
                                    .get(client.entity())
                                    .and_then(|inv| recipe.inventory_contains_ingredients(inv).ok())
                            } else {
                                None
                            }
                        };
                        if let Some(slots) = slots {
                            self.client
                                .borrow_mut()
                                .craft_recipe(&recipe, slots, craft_sprite);
                        }
                    },
                    HudEvent::CraftModularWeapon {
                        primary_slot,
                        secondary_slot,
                        craft_sprite,
                    } => {
                        self.client.borrow_mut().craft_modular_weapon(
                            primary_slot,
                            secondary_slot,
                            craft_sprite,
                        );
                    },
                    HudEvent::CraftModularWeaponComponent {
                        toolkind,
                        material,
                        modifier,
                        craft_sprite,
                    } => {
                        let additional_slots = {
                            let client = self.client.borrow();
                            let item_id = |slot| {
                                client
                                    .inventories()
                                    .get(client.entity())
                                    .and_then(|inv| inv.get(slot))
                                    .and_then(|item| {
                                        item.item_definition_id().itemdef_id().map(String::from)
                                    })
                            };
                            if let Some(material_id) = item_id(material) {
                                let key = recipe::ComponentKey {
                                    toolkind,
                                    material: material_id,
                                    modifier: modifier.and_then(item_id),
                                };
                                if let Some(recipe) = client.component_recipe_book().get(&key) {
                                    client.inventories().get(client.entity()).and_then(|inv| {
                                        recipe.inventory_contains_additional_ingredients(inv).ok()
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        if let Some(additional_slots) = additional_slots {
                            self.client.borrow_mut().craft_modular_weapon_component(
                                toolkind,
                                material,
                                modifier,
                                additional_slots,
                                craft_sprite,
                            );
                        }
                    },
                    HudEvent::SalvageItem { slot, salvage_pos } => {
                        self.client.borrow_mut().salvage_item(slot, salvage_pos);
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
                    HudEvent::ChangeAbility(slot, new_ability) => {
                        self.client.borrow_mut().change_ability(slot, new_ability);
                    },
                    HudEvent::SettingsChange(settings_change) => {
                        settings_change.process(global_state, self);
                    },
                    HudEvent::AcknowledgePersistenceLoadError => {
                        self.client
                            .borrow_mut()
                            .acknolwedge_persistence_load_error();
                    },
                    HudEvent::MapMarkerEvent(event) => {
                        self.client.borrow_mut().map_marker_event(event);
                    },
                }
            }

            {
                let client = self.client.borrow();
                let scene_data = SceneData {
                    client: &client,
                    state: client.state(),
                    player_entity: client.entity(),
                    // Only highlight if interactable
                    target_entity: self.interactable.and_then(Interactable::entity),
                    loaded_distance: client.loaded_distance(),
                    view_distance: client.view_distance().unwrap_or(1),
                    tick: client.get_tick(),
                    gamma: global_state.settings.graphics.gamma,
                    exposure: global_state.settings.graphics.exposure,
                    ambiance: global_state.settings.graphics.ambiance,
                    mouse_smoothing: global_state.settings.gameplay.smooth_pan_enable,
                    sprite_render_distance: global_state.settings.graphics.sprite_render_distance
                        as f32,
                    particles_enabled: global_state.settings.graphics.particles_enabled,
                    weapon_trails_enabled: global_state.settings.graphics.weapon_trails_enabled,
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
                        self.scene.handle_outcome(
                            &outcome,
                            &scene_data,
                            &mut global_state.audio,
                            &client.state(),
                            cam_pos,
                        );
                        self.hud
                            .handle_outcome(&outcome, scene_data.client, global_state);
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

    fn capped_fps(&self) -> bool { false }

    fn globals_bind_group(&self) -> &GlobalsBindGroup { self.scene.global_bind_group() }

    /// Render the session to the screen.
    ///
    /// This method should be called once per frame.
    fn render<'a>(&'a self, drawer: &mut Drawer<'a>, settings: &Settings) {
        span!(_guard, "render", "<Session as PlayState>::render");

        let client = self.client.borrow();

        let scene_data = SceneData {
            client: &client,
            state: client.state(),
            player_entity: client.entity(),
            // Only highlight if interactable
            target_entity: self.interactable.and_then(Interactable::entity),
            loaded_distance: client.loaded_distance(),
            view_distance: client.view_distance().unwrap_or(1),
            tick: client.get_tick(),
            gamma: settings.graphics.gamma,
            exposure: settings.graphics.exposure,
            ambiance: settings.graphics.ambiance,
            mouse_smoothing: settings.gameplay.smooth_pan_enable,
            sprite_render_distance: settings.graphics.sprite_render_distance as f32,
            figure_lod_render_distance: settings.graphics.figure_lod_render_distance as f32,
            particles_enabled: settings.graphics.particles_enabled,
            weapon_trails_enabled: settings.graphics.weapon_trails_enabled,
            is_aiming: self.is_aiming,
        };

        // Render world
        self.scene.render(
            drawer,
            client.state(),
            client.entity(),
            client.get_tick(),
            &scene_data,
        );

        if let Some(mut second_pass) = drawer.second_pass() {
            // Clouds
            {
                prof_span!("clouds");
                second_pass.draw_clouds();
            }
            // Trails
            {
                prof_span!("trails");
                if let Some(mut trail_drawer) = second_pass.draw_trails() {
                    self.scene
                        .trail_mgr()
                        .render(&mut trail_drawer, &scene_data);
                }
            }
        }
        // Bloom (call does nothing if bloom is off)
        {
            prof_span!("bloom");
            drawer.run_bloom_passes()
        }
        // PostProcess and UI
        {
            prof_span!("post-process and ui");
            let mut third_pass = drawer.third_pass();
            third_pass.draw_postprocess();
            // Draw the UI to the screen
            if let Some(mut ui_drawer) = third_pass.draw_ui() {
                self.hud.render(&mut ui_drawer);
            }; // Note: this semicolon is needed for the third_pass borrow to be dropped before it's lifetime ends
        }
    }

    fn egui_enabled(&self) -> bool { true }
}

fn find_shortest_distance(arr: &[Option<f32>]) -> Option<f32> {
    arr.iter()
        .filter_map(|x| *x)
        .min_by(|d1, d2| OrderedFloat(*d1).cmp(&OrderedFloat(*d2)))
}
