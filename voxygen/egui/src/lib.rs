#![feature(stmt_expr_attributes)]

#[cfg(all(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
compile_error!("Can't use both \"be-dyn-lib\" and \"use-dyn-lib\" features at once");

mod character_states;

use crate::character_states::draw_char_state_group;
use client::{Client, Join, World, WorldExt};
use common::{
    comp,
    comp::{aura::AuraKind::Buff, Body, Fluid, Poise, PoiseState},
};
use core::mem;
use egui::{
    plot::{Line, Plot, Value, Values},
    CollapsingHeader, Color32, Grid, Label, ScrollArea, Slider, Ui, Window,
};
use egui_winit_platform::Platform;
use std::time::Duration;

#[cfg(feature = "use-dyn-lib")]
use {
    lazy_static::lazy_static, std::ffi::CStr, std::sync::Arc, std::sync::Mutex,
    voxygen_dynlib::LoadedLib,
};

#[cfg(feature = "use-dyn-lib")]
lazy_static! {
    static ref LIB: Arc<Mutex<Option<LoadedLib>>> =
        voxygen_dynlib::init("veloren-voxygen-egui", "veloren-voxygen-egui-dyn", "egui");
}

#[cfg(feature = "use-dyn-lib")]
const MAINTAIN_EGUI_FN: &[u8] = b"maintain_egui_inner\0";

pub struct SelectedEntityInfo {
    entity_id: u32,
    debug_shape_id: Option<u64>,
    character_state_history: Vec<String>,
}

impl SelectedEntityInfo {
    fn new(entity_id: u32) -> Self {
        Self {
            entity_id,
            debug_shape_id: None,
            character_state_history: Vec::new(),
        }
    }
}

pub struct EguiDebugInfo {
    pub frame_time: Duration,
    pub ping_ms: f64,
}

pub struct EguiInnerState {
    selected_entity_info: Option<SelectedEntityInfo>,
    max_entity_distance: f32,
    selected_entity_cylinder_height: f32,
    frame_times: Vec<f32>,
}

#[derive(Clone, Default)]
pub struct EguiWindows {
    egui_inspection: bool,
    egui_settings: bool,
    egui_memory: bool,
    frame_time: bool,
    ecs_entities: bool,
}

impl Default for EguiInnerState {
    fn default() -> Self {
        Self {
            selected_entity_info: None,
            max_entity_distance: 100000.0,
            selected_entity_cylinder_height: 10.0,
            frame_times: Vec::new(),
        }
    }
}

pub enum DebugShapeAction {
    AddCylinder {
        radius: f32,
        height: f32,
    },
    RemoveShape(u64),
    SetPosAndColor {
        id: u64,
        pos: [f32; 4],
        color: [f32; 4],
    },
}

#[derive(Default)]
pub struct EguiActions {
    pub actions: Vec<DebugShapeAction>,
}

#[cfg(feature = "use-dyn-lib")]
pub fn init() { lazy_static::initialize(&LIB); }

pub fn maintain(
    platform: &mut Platform,
    egui_state: &mut EguiInnerState,
    egui_windows: &mut EguiWindows,
    client: &Client,
    debug_info: Option<EguiDebugInfo>,
    added_cylinder_shape_id: Option<u64>,
) -> EguiActions {
    #[cfg(not(feature = "use-dyn-lib"))]
    {
        maintain_egui_inner(
            platform,
            egui_state,
            egui_windows,
            client,
            debug_info,
            added_cylinder_shape_id,
        )
    }

    #[cfg(feature = "use-dyn-lib")]
    {
        let lock = LIB.lock().unwrap();
        let lib = &lock.as_ref().unwrap().lib;

        #[allow(clippy::type_complexity)]
        let maintain_fn: voxygen_dynlib::Symbol<
            fn(
                &mut Platform,
                &mut EguiInnerState,
                &mut EguiWindows,
                &Client,
                Option<EguiDebugInfo>,
                Option<u64>,
            ) -> EguiActions,
        > = unsafe { lib.get(MAINTAIN_EGUI_FN) }.unwrap_or_else(|e| {
            panic!(
                "Trying to use: {} but had error: {:?}",
                CStr::from_bytes_with_nul(MAINTAIN_EGUI_FN)
                    .map(CStr::to_str)
                    .unwrap()
                    .unwrap(),
                e
            )
        });

        maintain_fn(
            platform,
            egui_state,
            egui_windows,
            client,
            debug_info,
            added_cylinder_shape_id,
        )
    }
}

#[cfg_attr(feature = "be-dyn-lib", export_name = "maintain_egui_inner")]
pub fn maintain_egui_inner(
    platform: &mut Platform,
    egui_state: &mut EguiInnerState,
    egui_windows: &mut EguiWindows,
    client: &Client,
    debug_info: Option<EguiDebugInfo>,
    added_cylinder_shape_id: Option<u64>,
) -> EguiActions {
    platform.begin_frame();
    let ctx = &platform.context();

    let mut egui_actions = EguiActions::default();
    let mut previous_selected_entity: Option<SelectedEntityInfo> = None;
    let mut max_entity_distance = egui_state.max_entity_distance;
    let mut selected_entity_cylinder_height = egui_state.selected_entity_cylinder_height;

    // If a debug cylinder was added in the last frame, store it against the
    // selected entity
    if let Some(shape_id) = added_cylinder_shape_id {
        if let Some(selected_entity) = &mut egui_state.selected_entity_info {
            selected_entity.debug_shape_id = Some(shape_id);
        }
    }

    if let Some(debug_info) = debug_info.as_ref() {
        egui_state
            .frame_times
            .push(debug_info.frame_time.as_nanos() as f32);
        if egui_state.frame_times.len() > 250 {
            egui_state.frame_times.remove(0);
        }
    };

    egui::Window::new("Debug Control")
        .default_width(200.0)
        .default_height(200.0)
        .show(&platform.context(), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Ping: {:.1}ms",
                    debug_info.as_ref().map_or(0.0, |x| x.ping_ms)
                ));
            });
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut egui_windows.ecs_entities, "ECS Entities");
                    ui.checkbox(&mut egui_windows.frame_time, "Frame Time");
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Show EGUI Windows");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut egui_windows.egui_inspection, "🔍 Inspection");
                        ui.checkbox(&mut egui_windows.egui_settings, "🔧 Settings");
                        ui.checkbox(&mut egui_windows.egui_memory, "📝 Memory");
                    })
                })
            });
        });

    Window::new("🔧 Settings")
        .open(&mut egui_windows.egui_settings)
        .scroll(true)
        .show(ctx, |ui| {
            ctx.settings_ui(ui);
        });
    Window::new("🔍 Inspection")
        .open(&mut egui_windows.egui_inspection)
        .scroll(true)
        .show(ctx, |ui| {
            ctx.inspection_ui(ui);
        });

    Window::new("📝 Memory")
        .open(&mut egui_windows.egui_memory)
        .resizable(false)
        .show(ctx, |ui| {
            ctx.memory_ui(ui);
        });

    Window::new("Frame Time")
        .open(&mut egui_windows.frame_time)
        .default_width(200.0)
        .default_height(200.0)
        .show(ctx, |ui| {
            let line = Line::new(Values::from_values_iter(
                egui_state
                    .frame_times
                    .iter()
                    .enumerate()
                    .map(|(i, x)| Value::new(i as f64, *x)),
            ));
            let plot = Plot::new("Frame Time").line(line);
            ui.add(plot);
        });

    if egui_windows.ecs_entities {
        let ecs = client.state().ecs();

        let positions = client.state().ecs().read_storage::<comp::Pos>();
        let client_pos = positions.get(client.entity());

        egui::Window::new("ECS Entities")
            .open(&mut egui_windows.ecs_entities)
            .default_width(500.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label(format!("Entity count: {}", &ecs.entities().join().count()));
                ui.add(
                    Slider::new(&mut max_entity_distance, 1.0..=100000.0)
                        .logarithmic(true)
                        .clamp_to_range(true)
                        .text("Max entity distance"),
                );

                ui.add(
                    Slider::new(&mut selected_entity_cylinder_height, 0.1..=100.0)
                        .logarithmic(true)
                        .clamp_to_range(true)
                        .text("Cylinder height"),
                );

                let scroll_area = ScrollArea::from_max_height(800.0);
                let (_current_scroll, _max_scroll) = scroll_area.show(ui, |ui| {
                    Grid::new("entities_grid")
                        .spacing([40.0, 4.0])
                        .max_col_width(300.0)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("-");
                            ui.label("ID");
                            ui.label("Pos");
                            ui.label("Vel");
                            ui.label("Name");
                            ui.label("Body");
                            ui.label("Poise");
                            ui.label("Character State");
                            ui.end_row();
                            for (entity, body, stats, pos, _ori, vel, poise, character_state) in (
                                &ecs.entities(),
                                ecs.read_storage::<comp::Body>().maybe(),
                                ecs.read_storage::<comp::Stats>().maybe(),
                                ecs.read_storage::<comp::Pos>().maybe(),
                                ecs.read_storage::<comp::Ori>().maybe(),
                                ecs.read_storage::<comp::Vel>().maybe(),
                                ecs.read_storage::<comp::Poise>().maybe(),
                                ecs.read_storage::<comp::CharacterState>().maybe(),
                            )
                                .join()
                                .filter(|(_, _, _, pos, _, _, _, _)| {
                                    client_pos.map_or(true, |client_pos| {
                                        pos.map_or(0.0, |pos| pos.0.distance_squared(client_pos.0))
                                            < max_entity_distance
                                    })
                                })
                            {
                                if ui.button("View").clicked() {
                                    previous_selected_entity =
                                        mem::take(&mut egui_state.selected_entity_info);

                                    if pos.is_some() {
                                        egui_actions.actions.push(DebugShapeAction::AddCylinder {
                                            radius: 1.0,
                                            height: egui_state.selected_entity_cylinder_height,
                                        });
                                    }
                                    egui_state.selected_entity_info =
                                        Some(SelectedEntityInfo::new(entity.id()));
                                }

                                ui.label(format!("{}", entity.id()));

                                if let Some(pos) = pos {
                                    ui.label(format!(
                                        "{:.0},{:.0},{:.0}",
                                        pos.0.x, pos.0.y, pos.0.z
                                    ));
                                } else {
                                    ui.label("-");
                                }

                                if let Some(vel) = vel {
                                    ui.label(format!("{:.1}u/s", vel.0.magnitude()));
                                } else {
                                    ui.label("-");
                                }
                                if let Some(stats) = stats {
                                    ui.label(&stats.name);
                                } else {
                                    ui.label("-");
                                }
                                if let Some(body) = body {
                                    ui.label(body_species(&body));
                                } else {
                                    ui.label("-");
                                }

                                if let Some(poise) = poise {
                                    poise_state_label(ui, poise);
                                } else {
                                    ui.label("-");
                                }

                                if let Some(character_state) = character_state {
                                    ui.label(character_state.to_string());
                                } else {
                                    ui.label("-");
                                }

                                ui.end_row();
                            }
                        });

                    let margin = ui.visuals().clip_rect_margin;

                    let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
                    let max_scroll =
                        ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
                    (current_scroll, max_scroll)
                });
            });
        if let Some(selected_entity_info) = &mut egui_state.selected_entity_info {
            let selected_entity = ecs.entities().entity(selected_entity_info.entity_id);
            if !selected_entity.gen().is_alive() {
                previous_selected_entity = mem::take(&mut egui_state.selected_entity_info);
            } else {
                selected_entity_window(platform, ecs, selected_entity_info, &mut egui_actions);
            }
        }
    }

    if let Some(previous) = previous_selected_entity {
        if let Some(debug_shape_id) = previous.debug_shape_id {
            egui_actions
                .actions
                .push(DebugShapeAction::RemoveShape(debug_shape_id));
        }
    };

    if let Some(selected_entity) = &egui_state.selected_entity_info {
        if let Some(debug_shape_id) = selected_entity.debug_shape_id {
            if (egui_state.selected_entity_cylinder_height - selected_entity_cylinder_height).abs()
                > f32::EPSILON
            {
                egui_actions
                    .actions
                    .push(DebugShapeAction::RemoveShape(debug_shape_id));
                egui_actions.actions.push(DebugShapeAction::AddCylinder {
                    radius: 1.0,
                    height: selected_entity_cylinder_height,
                });
            }
        }
    };

    egui_state.max_entity_distance = max_entity_distance;
    egui_state.selected_entity_cylinder_height = selected_entity_cylinder_height;
    egui_actions
}

fn selected_entity_window(
    platform: &mut Platform,
    ecs: &World,
    selected_entity_info: &mut SelectedEntityInfo,
    egui_actions: &mut EguiActions,
) {
    let entity_id = selected_entity_info.entity_id;
    for (
        _entity,
        body,
        stats,
        pos,
        _ori,
        vel,
        poise,
        buffs,
        auras,
        character_state,
        physics_state,
        alignment,
        scale,
        mass,
        (density, health, energy),
    ) in (
        &ecs.entities(),
        ecs.read_storage::<comp::Body>().maybe(),
        ecs.read_storage::<comp::Stats>().maybe(),
        ecs.read_storage::<comp::Pos>().maybe(),
        ecs.read_storage::<comp::Ori>().maybe(),
        ecs.read_storage::<comp::Vel>().maybe(),
        ecs.read_storage::<comp::Poise>().maybe(),
        ecs.read_storage::<comp::Buffs>().maybe(),
        ecs.read_storage::<comp::Auras>().maybe(),
        ecs.read_storage::<comp::CharacterState>().maybe(),
        ecs.read_storage::<comp::PhysicsState>().maybe(),
        ecs.read_storage::<comp::Alignment>().maybe(),
        ecs.read_storage::<comp::Scale>().maybe(),
        ecs.read_storage::<comp::Mass>().maybe(),
        (
            ecs.read_storage::<comp::Density>().maybe(),
            ecs.read_storage::<comp::Health>().maybe(),
            ecs.read_storage::<comp::Energy>().maybe(),
        ),
    )
        .join()
        .filter(|(e, _, _, _, _, _, _, _, _, _, _, _, _, _, (_, _, _))| e.id() == entity_id)
    {
        if let Some(pos) = pos {
            if let Some(shape_id) = selected_entity_info.debug_shape_id {
                egui_actions.actions.push(DebugShapeAction::SetPosAndColor {
                    id: shape_id,
                    color: [1.0, 1.0, 0.0, 0.5],
                    pos: [pos.0.x, pos.0.y, pos.0.z + 2.0, 0.0],
                });
            }
        };

        egui::Window::new("Selected Entity")
            .default_width(300.0)
            .default_height(200.0)
            .show(&platform.context(), |ui| {
                ui.vertical(|ui| {
                    CollapsingHeader::new("General").default_open(true).show(ui, |ui| {
                        Grid::new("selected_entity_general_grid")
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                two_col_row(ui, "Health", health.map_or("-".to_owned(), |x| format!("{:.1}/{:.1}", x.current(), x.maximum())));
                                two_col_row(ui, "Energy", energy.map_or("-".to_owned(), |x| format!("{:.1}/{:.1}", x.current(), x.maximum())));
                                two_col_row(ui, "Position", pos.map_or("-".to_owned(), |x| format!("({:.1},{:.1},{:.1})", x.0.x, x.0.y, x.0.z)));
                                two_col_row(ui, "Velocity", vel.map_or("-".to_owned(), |x| format!("({:.1},{:.1},{:.1}) ({:.1} u/s)", x.0.x, x.0.y, x.0.z, x.0.magnitude())));
                                two_col_row(ui, "Alignment", alignment.map_or("-".to_owned(), |x| format!("{:?}", x)));
                                two_col_row(ui, "Scale", scale.map_or("-".to_owned(), |x| format!("{:?}", x)));
                                two_col_row(ui, "Mass", mass.map_or("-".to_owned(), |x| format!("{:.1}", x.0)));
                                two_col_row(ui, "Density", density.map_or("-".to_owned(), |x| format!("{:.1}", x.0)));
                            });

                    });
                if let Some(stats) = stats {
                    CollapsingHeader::new("Stats").default_open(true).show(ui, |ui| {
                        Grid::new("selected_entity_stats_grid")
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                two_col_row(ui, "Name", stats.name.to_string());
                                two_col_row(ui, "Damage Reduction", format!("{:.1}", stats.damage_reduction));
                                two_col_row(ui, "Max Health Modifier", format!("{:.1}", stats.max_health_modifier));
                                two_col_row(ui, "Move Speed Modifier", format!("{:.1}", stats.move_speed_modifier));
                            });

                    });
                }
                if let Some(body) = body {
                    CollapsingHeader::new("Body").default_open(false).show(ui, |ui| {
                        Grid::new("selected_entity_body_grid")
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                two_col_row(ui, "Type", body.to_string());
                                two_col_row(ui, "Species", body_species(body));
                            });

                    });
                }
                if let Some(pos) = pos {
                    CollapsingHeader::new("Pos").default_open(false).show(ui, |ui| {
                            Grid::new("selected_entity_pos_grid")
                                .spacing([40.0, 4.0])
                                .max_col_width(100.0)
                                .striped(true)
                                .show(ui, |ui| {
                                    two_col_row(ui, "x", format!("{:.1}", pos.0.x));
                                    two_col_row(ui, "y", format!("{:.1}", pos.0.y));
                                    two_col_row(ui, "z", format!("{:.1}", pos.0.z));
                                });

                    });
                }
                if let Some(poise) = poise {
                    CollapsingHeader::new("Poise").default_open(false).show(ui, |ui| {
                            Grid::new("selected_entity_poise_grid")
                                .spacing([40.0, 4.0])
                                .max_col_width(100.0)
                                .striped(true)
                                .show(ui, |ui| #[rustfmt::skip] {
                                    ui.label("State");
                                    poise_state_label(ui, poise);
                                    ui.end_row();
                                    two_col_row(ui, "Current", format!("{}/{}", poise.current(), poise.maximum()));
                                    two_col_row(ui, "Base Max", poise.base_max().to_string());
                                });
                        });
                }

                if let Some(buffs) = buffs {
                    CollapsingHeader::new("Buffs").default_open(false).show(ui, |ui| {
                        Grid::new("selected_entity_buffs_grid")
                            .spacing([40.0, 4.0])
                            .max_col_width(100.0)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Kind");
                                ui.label("Time");
                                ui.label("Source");
                                ui.end_row();
                                buffs.buffs.iter().for_each(|(_, v)| {
                                    ui.label(format!("{:?}", v.kind));
                                    ui.label(
                                        v.time.map_or("-".to_string(), |time| {
                                            format!("{:?}", time)
                                        }),
                                    );
                                    ui.label(format!("{:?}", v.source));
                                    ui.end_row();
                                });
                            });
                    });
                }

                if let Some(auras) = auras {
                    CollapsingHeader::new("Auras").default_open(false).show(ui, |ui| {
                        Grid::new("selected_entity_auras_grid")
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Kind");
                                ui.label("Radius");
                                ui.label("Duration");
                                ui.label("Target");
                                ui.end_row();
                                auras.auras.iter().for_each(|(_, v)| {
                                    ui.label(match v.aura_kind {
                                        Buff { kind, .. } =>  format!("Buff - {:?}", kind)
                                    });
                                    ui.label(format!("{:1}", v.radius));
                                    ui.label(v.duration.map_or("-".to_owned(), |x| format!("{:1}s", x.as_secs())));
                                    ui.label(format!("{:?}", v.target));
                                    ui.end_row();
                                });
                            });
                    });
                }

                if let Some(character_state) = character_state {
                    if selected_entity_info
                        .character_state_history
                        .first()
                        .unwrap_or(&"-".to_owned())
                        != &character_state.to_string()
                    {
                        selected_entity_info
                            .character_state_history
                            .insert(0, character_state.to_string());
                        if selected_entity_info.character_state_history.len() > 50 {
                            selected_entity_info.character_state_history.pop();
                        }
                    }

                    CollapsingHeader::new("Character State").default_open(false).show(ui, |ui| {
                        draw_char_state_group(ui, selected_entity_info, character_state);
                    });
                }

                if let Some(physics_state) = physics_state {
                    CollapsingHeader::new("Physics State").default_open(false).show(ui, |ui| {
                        Grid::new("selected_entity_physics_state_grid")
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                two_col_row(ui, "On Ground", physics_state.on_ground.map_or("None".to_owned(), |x| format!("{:?}", x)));
                                two_col_row(ui, "On Ceiling", (if physics_state.on_ceiling { "True" } else { "False " }).to_string());
                                two_col_row(ui, "On Wall", physics_state.on_wall.map_or("-".to_owned(), |x| format!("{:.1},{:.1},{:.1}", x.x, x.y, x.z )));
                                two_col_row(ui, "Touching Entities", physics_state.touch_entities.len().to_string());
                                two_col_row(ui, "In Fluid", match physics_state.in_fluid {

                                    Some(Fluid::Air { elevation, .. }) => format!("Air (Elevation: {:.1})", elevation),
                                    Some(Fluid::Liquid { depth, kind, .. }) => format!("{:?} (Depth: {:.1})", kind, depth),
                                    _ => "None".to_owned() });
                                });
                            });
                }
            });
        });
    }
}

fn body_species(body: &Body) -> String {
    match body {
        Body::Humanoid(body) => format!("{:?}", body.species),
        Body::QuadrupedSmall(body) => format!("{:?}", body.species),
        Body::QuadrupedMedium(body) => format!("{:?}", body.species),
        Body::BirdMedium(body) => format!("{:?}", body.species),
        Body::FishMedium(body) => format!("{:?}", body.species),
        Body::Dragon(body) => format!("{:?}", body.species),
        Body::BirdLarge(body) => format!("{:?}", body.species),
        Body::FishSmall(body) => format!("{:?}", body.species),
        Body::BipedLarge(body) => format!("{:?}", body.species),
        Body::BipedSmall(body) => format!("{:?}", body.species),
        Body::Object(body) => format!("{:?}", body),
        Body::Golem(body) => format!("{:?}", body.species),
        Body::Theropod(body) => format!("{:?}", body.species),
        Body::QuadrupedLow(body) => format!("{:?}", body.species),
        Body::Ship(body) => format!("{:?}", body),
    }
}

fn two_col_row(ui: &mut Ui, label: impl Into<Label>, content: impl Into<Label>) {
    ui.label(label);
    ui.label(content);
    ui.end_row();
}

fn poise_state_label(ui: &mut Ui, poise: &Poise) {
    match poise.poise_state() {
        PoiseState::Normal => {
            ui.label("Normal");
        },
        PoiseState::Interrupted => {
            ui.colored_label(Color32::YELLOW, "Interrupted");
        },
        PoiseState::Stunned => {
            ui.colored_label(Color32::RED, "Stunned");
        },
        PoiseState::Dazed => {
            ui.colored_label(Color32::RED, "Dazed");
        },
        PoiseState::KnockedDown => {
            ui.colored_label(Color32::BLUE, "Knocked Down");
        },
    };
}
