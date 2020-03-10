use crate::{
    menu::main::MainMenuState,
    ui,
    window::{Event, EventLoop},
    Direction, GlobalState, PlayState, PlayStateResult,
};
use std::{mem, time::Duration};
use tracing::debug;

pub fn run(mut global_state: GlobalState, event_loop: EventLoop) {
    // Set up the initial play state.
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(MainMenuState::new(&mut global_state))];
    states.last_mut().map(|current_state| {
        current_state.enter(&mut global_state, Direction::Forwards);
        let current_state = current_state.name();
        debug!(?current_state, "Started game with state");
    });

    // Used to ignore every other `MainEvenstCleared`
    // This is a workaround for a bug on macos in which mouse motion events are only
    // reported every other cycle of the event loop
    // See: https://github.com/rust-windowing/winit/issues/1418
    let mut polled_twice = false;

    event_loop.run(move |event, _, control_flow| {
        // Continously run loop since we handle sleeping
        *control_flow = winit::event_loop::ControlFlow::Poll;

        // Get events for the ui.
        if let Some(event) = ui::Event::try_from(&event, global_state.window.window()) {
            global_state.window.send_event(Event::Ui(event));
        }

        match event {
            winit::event::Event::MainEventsCleared => {
                if polled_twice {
                    handle_main_events_cleared(&mut states, control_flow, &mut global_state);
                }
                polled_twice = !polled_twice;
            },
            winit::event::Event::WindowEvent { event, .. } => global_state
                .window
                .handle_window_event(event, &mut global_state.settings),
            winit::event::Event::DeviceEvent { event, .. } => {
                global_state.window.handle_device_event(event)
            },
            winit::event::Event::LoopDestroyed => {
                // Save any unsaved changes to settings and profile
                global_state.settings.save_to_file_warn();
                global_state.profile.save_to_file_warn();
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
                // Wait for the next tick.
                global_state.clock.tick(Duration::from_millis(
                    1000 / global_state.settings.graphics.max_fps as u64,
                ));

                // Maintain global state.
                global_state.maintain(global_state.clock.get_last_delta().as_secs_f32());

                exit = false;
                break;
            },
            PlayStateResult::Shutdown => {
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

    if let Some(last) = states.last_mut() {
        global_state.window.renderer_mut().clear();
        last.render(global_state.window.renderer_mut(), &global_state.settings);
        // Finish the frame.
        global_state.window.renderer_mut().flush();
        global_state
            .window
            .swap_buffers()
            .expect("Failed to swap window buffers!");
    }
}
