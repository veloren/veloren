use crate::settings::{AudioSettings, Settings};
use common::assets::read_dir;
use crossbeam::{
    atomic::AtomicCell,
    channel::{unbounded, Sender},
    queue::SegQueue,
    sync::ShardedLock,
};
use parking_lot::{Condvar, Mutex};
use rodio::{Decoder, Device, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::thread;

trait AudioConfig {
    fn set_volume(&mut self, volume: f32);
    fn set_device(&mut self, name: String);
}

trait MonoMode {
    fn set_mono(tx: Sender<AudioPlayerMsg>) -> Self;
}

trait StereoMode {
    fn set_stereo(tx: Sender<AudioPlayerMsg>) -> Self;
}

trait DebugMode {
    fn set_no_audio(tx: Sender<AudioPlayerMsg>) -> Self;
}

#[derive(Debug)]
pub enum SinkError {
    SinkNotMatch,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Genre {
    Bgm,
    Sfx,
    None,
}

#[derive(Debug)]
pub enum AudioPlayerMsg {
    AudioPlay,
    AudioStop,
    AudioTime(f64),
}

#[derive(Debug)]
pub enum Action {
    Load(String),
    Stop,
    AdjustVolume(f32),
    ChangeDevice(String),
}

#[derive(Clone)]
struct EventLoop {
    condvar: Arc<(Mutex<bool>, Condvar)>,
    queue: Arc<SegQueue<Action>>,
    playing: Arc<ShardedLock<bool>>,
}

impl EventLoop {
    fn new() -> Self {
        Self {
            condvar: Arc::new((Mutex::new(false), Condvar::new())),
            queue: Arc::new(SegQueue::new()),
            playing: Arc::new(ShardedLock::new(false)),
        }
    }
}

/// The `AudioPlayer` handles exactly what player does by playing audio from emitter.
pub struct AudioPlayer {
    event_loop: EventLoop,
    paused: AtomicCell<bool>,
    tx: Sender<AudioPlayerMsg>,
}

impl AudioPlayer {
    pub(crate) fn new(genre: Genre, tx: Sender<AudioPlayerMsg>) -> Self {
        match genre {
            Genre::Bgm => AudioPlayer::set_mono(tx),
            Genre::Sfx => unimplemented!(),
            Genre::None => AudioPlayer::set_no_audio(tx),
        }
    }

    pub(crate) fn load(&self, path: &str) {
        self.emit(Action::Load(path.to_string()));
        self.set_playing(true);
    }

    // pub(crate) fn pause(&mut self) {
    //     self.paused.store(true);
    //     self.send(AudioPlayerMsg::AudioStop);
    //     self.set_playing(false);
    // }

    pub(crate) fn resume(&mut self) {
        self.paused.store(false);
        self.send(AudioPlayerMsg::AudioPlay);
        self.set_playing(true);
    }

    // pub(crate) fn stop(&mut self) {
    //     self.paused.store(false);
    //     self.send(AudioPlayerMsg::AudioStop);
    //     self.emit(Action::Stop);
    //     self.set_playing(false);
    // }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load()
    }

    pub(crate) fn set_volume(&self, value: f32) {
        self.emit(Action::AdjustVolume(value));
    }

    pub(crate) fn set_device(&self, device: &str) {
        self.emit(Action::ChangeDevice(device.to_string()));
    }

    fn emit(&self, action: Action) {
        self.event_loop.queue.push(action);
    }

    fn send(&mut self, msg: AudioPlayerMsg) {
        send_msg(&mut self.tx, msg);
    }

    fn set_playing(&self, playing: bool) {
        *self.event_loop.playing.write().unwrap() = playing;
        let &(ref lock, ref condvar) = &*self.event_loop.condvar;
        let mut started = lock.lock();
        *started = playing;
        if playing {
            condvar.notify_one();
        }
    }
}

impl MonoMode for AudioPlayer {
    /// Playing audio until a receive operation appears on the other side.
    fn set_mono(tx: Sender<AudioPlayerMsg>) -> Self {
        let event_loop = EventLoop::new();

        {
            //let mut tx = tx.clone();
            let event_loop = event_loop.clone();
            let condition = event_loop.condvar.clone();

            thread::spawn(move || {
                let block = || {
                    let (ref lock, ref condvar) = *condition;
                    let mut started = lock.lock();
                    *started = false;
                    while !*started {
                        condvar.wait(&mut started);
                    }
                };
                let mut playback = MonoEmitter::new(&Settings::load().audio);

                // Start the thread if set_playing(true).
                loop {
                    if let Ok(action) = event_loop.queue.pop() {
                        match action {
                            Action::Load(path) => {
                                if playback.stream.empty() {
                                    playback.play_from(&path);
                                    //send_msg(&mut tx, AudioPlayerMsg::AudioPlay);
                                }
                            }
                            Action::Stop => playback.stream.stop(),
                            Action::AdjustVolume(value) => playback.set_volume(value),
                            Action::ChangeDevice(device) => playback.set_device(device),
                        }
                    } else {
                        block();
                    }
                }
            });
        }

        Self {
            event_loop,
            paused: AtomicCell::new(false),
            tx,
        }
    }
}

impl DebugMode for AudioPlayer {
    /// Don't load `rodio` for `no-audio` feature.
    fn set_no_audio(tx: Sender<AudioPlayerMsg>) -> Self {
        Self {
            event_loop: EventLoop::new(),
            paused: AtomicCell::new(true),
            tx,
        }
    }
}

/// TODO: Implement treeview and modellist widgets for GUI design.
pub struct Jukebox {
    genre: AtomicCell<Genre>,
    pub(crate) player: AudioPlayer,
}

impl Jukebox {
    pub(crate) fn new(genre: Genre) -> Self {
        let (tx, _rx) = unbounded();
        Self {
            genre: AtomicCell::new(genre),
            player: AudioPlayer::new(genre, tx),
        }
    }

    // TODO: The `update` function should associate with `conrod` to visualise the audio playlist
    // and settings.
    // pub(crate) fn update(&mut self, _msg: AudioPlayerMsg) {
    //     unimplemented!()
    // }

    /// Display the current genre.
    pub(crate) fn get_genre(&self) -> Genre {
        self.genre.load()
    }
}

struct MonoEmitter {
    device: Device,
    stream: Sink,
}

// struct StereoEmitter {
//     device: Device,
//     stream: SpatialSink,
// }

impl MonoEmitter {
    fn new(settings: &AudioSettings) -> Self {
        let device = match &settings.audio_device {
            Some(dev) => rodio::output_devices()
                .find(|x| &x.name() == dev)
                .or_else(rodio::default_output_device)
                .expect("No Audio devices found!"),
            None => rodio::default_output_device().expect("No Audio devices found!"),
        };
        let sink = Sink::new(&device);
        sink.set_volume(settings.music_volume);

        Self {
            device,
            stream: sink,
        }
    }

    //    /// Returns the name of the current audio device.
    //    /// Does not return rodio Device struct in case our audio backend changes.
    //    fn get_device(&self) -> String {
    //        self.device.name()
    //    }

    fn play_from(&mut self, path: &str) {
        let bufreader = BufReader::new(File::open(path).unwrap());
        let src = Decoder::new(bufreader).unwrap();
        self.stream.append(src);
    }
}

impl AudioConfig for MonoEmitter {
    fn set_volume(&mut self, volume: f32) {
        self.stream.set_volume(volume.min(1.0).max(0.0))
    }

    /// Sets the current audio device from a string.
    /// Does not use the rodio Device struct in case that detail changes.
    /// If the string is an invalid audio device, then no change is made.
    fn set_device(&mut self, name: String) {
        if let Some(dev) = rodio::output_devices().find(|x| x.name() == name) {
            self.device = dev;
            self.stream = Sink::new(&self.device);
        }
    }
}

// impl StereoEmitter {
//     fn new(settings: &AudioSettings) -> Self {
//        let device = match &settings.audio_device {
//            Some(dev) => rodio::output_devices()
//                .find(|x| &x.name() == dev)
//                .or_else(rodio::default_output_device)
//                .expect("No Audio devices found!"),
//            None => rodio::default_output_device().expect("No Audio devices found!"),
//        };
//         let sink = SpatialSink::new(
//             &device.device,
//             [0.0, 0.0, 0.0],
//             [1.0, 0.0, 0.0],
//             [-1.0, 0.0, 0.0],
//         );
//         sink.set_volume(settings.music_volume);

//         Self {
//             device,
//             stream: sink,
//         }
//     }

//     fn play_from(&mut self, path: &str) {
//         let bufreader = load_from_path(path).unwrap();
//         let src = Decoder::new(bufreader).unwrap();
//         self.stream.append(src);
//     }
// }

// impl AudioConfig for StereoEmitter {
//     fn set_volume(&mut self, volume: f32) {
//         self.stream.set_volume(volume.min(1.0).max(0.0))
//     }

//     /// Sets the current audio device from a string.
//     /// Does not use the rodio Device struct in case that detail changes.
//     /// If the string is an invalid audio device, then no change is made.
//     fn set_device(&mut self, name: String) {
//         if let Some(dev) = rodio::output_devices().find(|x| x.name() == name) {
//             self.device = dev;
//             self.stream = SpatialSink::new(
//                 &self.device,
//                 [0.0, 0.0, 0.0],
//                 [1.0, 0.0, 0.0],
//                 [-1.0, 0.0, 0.0],
//             );
//         }
//     }
// }

/// Returns the default audio device.
/// Does not return rodio Device struct in case our audio backend changes.
pub(crate) fn get_default_device() -> String {
    rodio::default_output_device()
        .expect("No audio output devices detected.")
        .name()
}

/// Load the audio file directory selected by genre.
pub(crate) fn load_soundtracks(genre: &Genre) -> Vec<String> {
    match *genre {
        Genre::Bgm => {
            let assets = read_dir("voxygen.audio.soundtrack").unwrap();
            let soundtracks = assets
                .filter_map(|entry| {
                    entry.ok().map(|f| {
                        let path = f.path();
                        path.to_string_lossy().into_owned()
                    })
                })
                .collect::<Vec<String>>();

            soundtracks
        }
        Genre::Sfx => {
            let assets = read_dir("voxygen.audio.soundtrack").unwrap();
            let soundtracks = assets
                //.filter_map(|entry| {
                //    entry.ok().and_then(|f| {
                //        f.path()
                //            .file_name()
                //            .and_then(|n| n.to_str().map(|s| String::from(s)))
                //    })
                //})
                //.collect::<Vec<String>>();
                .filter_map(|entry| {
                    entry.ok().map(|f| {
                        let path = f.path();
                        (*path.into_os_string().to_string_lossy()).to_owned()
                    })
                })
                .collect::<Vec<String>>();

            soundtracks
        }
        Genre::None => {
            let empty_list = Vec::new();
            empty_list
        }
    }
}

pub(crate) fn select_random_music(genre: &Genre) -> String {
    let soundtracks = load_soundtracks(genre);
    let index = rand::random::<usize>() % soundtracks.len();
    soundtracks[index].clone()
}

/// Returns a vec of the audio devices available.
/// Does not return rodio Device struct in case our audio backend changes.
pub(crate) fn list_devices() -> Vec<String> {
    list_devices_raw().iter().map(|x| x.name()).collect()
}

/// Returns vec of devices
fn list_devices_raw() -> Vec<Device> {
    rodio::output_devices().collect()
}

fn send_msg(tx: &mut Sender<AudioPlayerMsg>, msg: AudioPlayerMsg) {
    tx.send(msg)
        .expect("Failed on attempting to send a message into audio channel.");
}

#[test]
fn test_load_soundtracks() {
    use crate::audio::base::{load_soundtracks, Genre};
    for entry in load_soundtracks(&Genre::Bgm).iter() {
        println!("{}", entry)
    }
}
