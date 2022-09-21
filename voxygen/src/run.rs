use crate::{
    menu::main::MainMenuState,
    settings::get_fps,
    ui,
    window::{Event, EventLoop},
    Direction, GlobalState, PlayState, PlayStateResult,
};
use common_base::{prof_span, span};
use std::{mem, time::Duration};
use tracing::debug;

pub fn run(mut global_state: GlobalState, event_loop: EventLoop, server: Option<String>) {
    // Set up the initial play state.
    let mut states: Vec<Box<dyn PlayState>> =
        vec![Box::new(MainMenuState::new(&mut global_state, server))];
    states.last_mut().map(|current_state| {
        current_state.enter(&mut global_state, Direction::Forwards);
        let current_state = current_state.name();
        debug!(?current_state, "Started game with state");
    });

    // Used to ignore every other `MainEventsCleared`
    // This is a workaround for a bug on macos in which mouse motion events are only
    // reported every other cycle of the event loop
    // See: https://github.com/rust-windowing/winit/issues/1418
    let mut polled_twice = false;

    let mut poll_span = None;
    let mut event_span = None;

    event_loop.run(move |event, _, control_flow| {
        // Continuously run loop since we handle sleeping
        *control_flow = winit::event_loop::ControlFlow::Poll;

        #[cfg(feature = "egui-ui")]
        {
            let enabled_for_current_state =
                states.last().map_or(false, |state| state.egui_enabled());

            // Only send events to the egui UI when it is being displayed.
            if enabled_for_current_state && global_state.settings.interface.egui_enabled() {
                global_state.egui_state.platform.handle_event(&event);
                if global_state.egui_state.platform.captures_event(&event) {
                    return;
                }
            }
        }

        // Don't pass resize events to the ui, `Window` is responsible for:
        // - deduplicating them
        // - generating resize events for the ui
        // - ensuring consistent sizes are passed to the ui and to the renderer
        if !matches!(&event, winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::Resized(_),
            ..
        }) {
            // Get events for the ui.
            if let Some(event) = ui::Event::try_from(&event, global_state.window.window()) {
                global_state.window.send_event(Event::Ui(event));
            }
            // iced ui events
            // TODO: no clone
            if let winit::event::Event::WindowEvent { event, .. } = &event {
                let window = &mut global_state.window;
                if let Some(event) =
                    ui::ice::window_event(event, window.scale_factor(), window.modifiers())
                {
                    window.send_event(Event::IcedUi(event));
                }
            }
        }

        match event {
            winit::event::Event::NewEvents(_) => {
                prof_span!(span, "Process Events");
                event_span = Some(span);
            },
            winit::event::Event::MainEventsCleared => {
                event_span.take();
                poll_span.take();
                if polled_twice {
                    handle_main_events_cleared(&mut states, control_flow, &mut global_state);
                }
                prof_span!(span, "Poll Winit");
                poll_span = Some(span);
                polled_twice = !polled_twice;
            },
            winit::event::Event::WindowEvent { event, .. } => {
                span!(_guard, "Handle WindowEvent");

                if let winit::event::WindowEvent::Focused(focused) = event {
                    global_state.audio.set_master_volume(if focused {
                        global_state.settings.audio.master_volume.get_checked()
                    } else {
                        global_state
                            .settings
                            .audio
                            .inactive_master_volume_perc
                            .get_checked()
                            * global_state.settings.audio.master_volume.get_checked()
                    });
                }

                global_state
                    .window
                    .handle_window_event(event, &mut global_state.settings)
            },
            winit::event::Event::DeviceEvent { event, .. } => {
                span!(_guard, "Handle DeviceEvent");
                global_state.window.handle_device_event(event)
            },
            winit::event::Event::LoopDestroyed => {
                // Save any unsaved changes to settings and profile
                global_state
                    .settings
                    .save_to_file_warn(&global_state.config_dir);
                global_state
                    .profile
                    .save_to_file_warn(&global_state.config_dir);
            },
            _ => {},
        }
    });
}

fn handle_main_events_cleared(
    states: &mut Vec<Box<dyn PlayState>>,
    control_flow: &mut winit::event_loop::ControlFlow,
    global_state: &mut GlobalState,
) {
    span!(guard, "Handle MainEventsCleared");
    // Screenshot / Fullscreen toggle
    global_state
        .window
        .resolve_deduplicated_events(&mut global_state.settings, &global_state.config_dir);
    // Run tick here

    // What's going on here?
    // ---------------------
    // The state system used by Voxygen allows for the easy development of
    // stack-based menus. For example, you may want a "title" state
    // that can push a "main menu" state on top of it, which can in
    // turn push a "settings" state or a "game session" state on top of it.
    // The code below manages the state transfer logic automatically so that we
    // don't have to re-engineer it for each menu we decide to add
    // to the game.
    let mut exit = true;
    while let Some(state_result) = states.last_mut().map(|last| {
        let events = global_state.window.fetch_events();
        last.tick(global_state, events)
    }) {
        // Implement state transfer logic.
        match state_result {
            PlayStateResult::Continue => {
                exit = false;
                break;
            },
            PlayStateResult::Shutdown => {
                // Clear the Discord activity before shutting down
                #[cfg(feature = "discord")]
                global_state.discord.clear_activity();

                debug!("Shutting down all states...");
                while states.last().is_some() {
                    states.pop().map(|old_state| {
                        debug!("Popped state '{}'.", old_state.name());
                        global_state.on_play_state_changed();
                    });
                }
            },
            PlayStateResult::Pop => {
                states.pop().map(|old_state| {
                    debug!("Popped state '{}'.", old_state.name());
                    global_state.on_play_state_changed();
                });
                states.last_mut().map(|new_state| {
                    new_state.enter(global_state, Direction::Backwards);
                });
            },
            PlayStateResult::Push(mut new_state) => {
                new_state.enter(global_state, Direction::Forwards);
                debug!("Pushed state '{}'.", new_state.name());
                states.push(new_state);
                global_state.on_play_state_changed();
            },
            PlayStateResult::Switch(mut new_state) => {
                new_state.enter(global_state, Direction::Forwards);
                states.last_mut().map(|old_state| {
                    debug!(
                        "Switching to state '{}' from state '{}'.",
                        new_state.name(),
                        old_state.name()
                    );
                    mem::swap(old_state, &mut new_state);
                    global_state.on_play_state_changed();
                });
            },
        }
    }

    if exit {
        *control_flow = winit::event_loop::ControlFlow::Exit;
    }

    let mut capped_fps = false;

    drop(guard);

    #[cfg(feature = "egui-ui")]
    let scale_factor = global_state.window.window().scale_factor() as f32;

    if let Some(last) = states.last_mut() {
        capped_fps = last.capped_fps();

        span!(guard, "Render");

        // Render the screen using the global renderer
        if let Some(mut drawer) = global_state
            .window
            .renderer_mut()
            .start_recording_frame(last.globals_bind_group())
            .expect("Unrecoverable render error when starting a new frame!")
        {
            if global_state.clear_shadows_next_frame {
                drawer.clear_shadows();
            }

            last.render(&mut drawer, &global_state.settings);

            #[cfg(feature = "egui-ui")]
            if last.egui_enabled() && global_state.settings.interface.egui_enabled() {
                drawer.draw_egui(&mut global_state.egui_state.platform, scale_factor);
            }
        };
        if global_state.clear_shadows_next_frame {
            global_state.clear_shadows_next_frame = false;
        }

        drop(guard);
    }

    if !exit {
        // Wait for the next tick.
        span!(guard, "Main thread sleep");

        // Enforce an FPS cap for the non-game session play states to prevent them
        // running at hundreds/thousands of FPS resulting in high GPU usage for
        // effectively doing nothing.
        let max_fps = get_fps(global_state.settings.graphics.max_fps);
        let max_background_fps = u32::min(
            max_fps,
            get_fps(global_state.settings.graphics.max_background_fps),
        );
        let max_fps_focus_adjusted = if global_state.window.focused {
            max_fps
        } else {
            max_background_fps
        };

        const TITLE_SCREEN_FPS_CAP: u32 = 60;

        let target_fps = if capped_fps {
            u32::min(TITLE_SCREEN_FPS_CAP, max_fps_focus_adjusted)
        } else {
            max_fps_focus_adjusted
        };

        global_state
            .clock
            .set_target_dt(Duration::from_secs_f64(1.0 / target_fps as f64));
        global_state.clock.tick();
        drop(guard);
        #[cfg(feature = "tracy")]
        common_base::tracy_client::frame_mark();

        // Maintain global state.
        global_state.maintain(global_state.clock.dt());
    }
}
