// Library
use winit;

// Crate
use crate::{
    VoxygenErr,
    render::Renderer,
};

pub struct Window {
    events_loop: winit::EventsLoop,
    renderer: Renderer,
}


impl Window {
    pub fn new() -> Result<Window, VoxygenErr> {
        let events_loop = winit::EventsLoop::new();

        let window = winit::WindowBuilder::new()
            .with_title("Veloren (Voxygen)")
            .with_dimensions(winit::dpi::LogicalSize::new(800.0, 500.0))
            .with_maximized(false)
            .build(&events_loop)
            .map_err(|err| VoxygenErr::WinitCreationErr(err))?;

        let tmp = Ok(Self {
            events_loop,
            renderer: Renderer::new(window)?,
        });
        tmp
    }

    pub fn renderer(&self) -> &Renderer { &self.renderer }
    pub fn renderer_mut(&mut self) -> &mut Renderer { &mut self.renderer }

    pub fn fetch_events(&mut self) -> Vec<Event> {
        let mut events = vec![];
        self.events_loop.poll_events(|event| match event {
            winit::Event::WindowEvent { event, .. } => match event {
                winit::WindowEvent::CloseRequested => events.push(Event::Close),
                _ => {},
            },
            _ => {},
        });
        events
    }

    pub fn display(&self) {
        // TODO
    }
}

pub enum Event {
    Close,
}
