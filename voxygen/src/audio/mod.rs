pub mod frontend;

use frontend::AudioFrontend;

pub struct Audio {
    frontend: AudioFrontend,
}

impl Audio {
    pub fn new() -> Self {
        Audio {
            frontend: AudioFrontend::new(),
        }
    }

    pub fn play_music(&self, filename: String) {
        let buffer = frontend.get_buffer(filename);
        frontend.gen_stream()
    }
}
