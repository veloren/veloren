use super::SessionState;
use crate::{
    controller::ControllerSettings,
    hud::{
        BarNumbers, BuffPosition, CrosshairType, Intro, PressBehavior, ScaleChange,
        ShortcutNumbers, XpBar,
    },
    i18n::{i18n_asset_key, LanguageMetadata, LocalizationHandle},
    render::RenderMode,
    settings::{
        AudioSettings, ControlSettings, Fps, GamepadSettings, GameplaySettings, GraphicsSettings,
        InterfaceSettings,
    },
    window::{FullScreenSettings, GameInput},
    GlobalState,
};
use vek::*;

#[derive(Clone)]
pub enum Audio {
    AdjustMasterVolume(f32),
    AdjustInactiveMasterVolume(f32),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    //ChangeAudioDevice(String),
    ResetAudioSettings,
}
#[derive(Clone)]
pub enum Control {
    ChangeBinding(GameInput),
    ResetKeyBindings,
}
#[derive(Clone)]
pub enum Gamepad {}
#[derive(Clone)]
pub enum Gameplay {
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustCameraClamp(u32),

    ToggleControllerYInvert(bool),
    ToggleMouseYInvert(bool),
    ToggleZoomInvert(bool),

    ToggleSmoothPan(bool),

    ChangeFreeLookBehavior(PressBehavior),
    ChangeAutoWalkBehavior(PressBehavior),
    ChangeCameraClampBehavior(PressBehavior),
    ChangePlayerPhysicsBehavior { server_authoritative: bool },
    ChangeStopAutoWalkOnInput(bool),
    ChangeAutoCamera(bool),

    ResetGameplaySettings,
}
#[derive(Clone)]
pub enum Graphics {
    AdjustViewDistance(u32),
    AdjustLodDetail(u32),
    AdjustSpriteRenderDistance(u32),
    AdjustFigureLoDRenderDistance(u32),

    ChangeMaxFPS(Fps),
    ChangeFOV(u16),

    ChangeGamma(f32),
    ChangeExposure(f32),
    ChangeAmbiance(f32),

    ChangeRenderMode(Box<RenderMode>),

    ChangeFullscreenMode(FullScreenSettings),
    ToggleParticlesEnabled(bool),
    AdjustWindowSize([u16; 2]),

    ResetGraphicsSettings,
}
#[derive(Clone)]
pub enum Interface {
    Sct(bool),
    SctPlayerBatch(bool),
    SctDamageBatch(bool),
    SpeechBubbleDarkMode(bool),
    SpeechBubbleIcon(bool),
    ToggleHelp(bool),
    ToggleDebug(bool),
    ToggleTips(bool),

    CrosshairTransp(f32),
    ChatTransp(f32),
    ChatCharName(bool),
    CrosshairType(CrosshairType),
    Intro(Intro),
    ToggleXpBar(XpBar),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    BuffPosition(BuffPosition),

    UiScale(ScaleChange),
    //Minimap
    MinimapShow(bool),
    MinimapFaceNorth(bool),
    MinimapZoom(f64),
    //Map settings
    MapZoom(f64),
    MapDrag(Vec2<f64>),
    MapShowTopoMap(bool),
    MapShowDifficulty(bool),
    MapShowTowns(bool),
    MapShowDungeons(bool),
    MapShowCastles(bool),
    MapShowCaves(bool),
    MapShowTrees(bool),
    MapShowPeaks(bool),

    ResetInterfaceSettings,
}
#[derive(Clone)]
pub enum Language {
    ChangeLanguage(Box<LanguageMetadata>),
    ToggleEnglishFallback(bool),
}
#[derive(Clone)]
pub enum Networking {}

#[derive(Clone)]
pub enum SettingsChange {
    Audio(Audio),
    Control(Control),
    Gamepad(Gamepad),
    Gameplay(Gameplay),
    Graphics(Graphics),
    Interface(Interface),
    Language(Language),
    Networking(Networking),
}

macro_rules! settings_change_from {
    ($i: ident) => {
        impl From<$i> for SettingsChange {
            fn from(change: $i) -> Self { SettingsChange::$i(change) }
        }
    };
}
settings_change_from!(Audio);
settings_change_from!(Control);
settings_change_from!(Gamepad);
settings_change_from!(Gameplay);
settings_change_from!(Graphics);
settings_change_from!(Interface);
settings_change_from!(Language);
settings_change_from!(Networking);

impl SettingsChange {
    pub fn process(self, global_state: &mut GlobalState, session_state: &mut SessionState) {
        let mut settings = &mut global_state.settings;
        match self {
            SettingsChange::Audio(audio_change) => {
                match audio_change {
                    Audio::AdjustMasterVolume(master_volume) => {
                        global_state.audio.set_master_volume(master_volume);

                        settings.audio.master_volume = master_volume;
                    },
                    Audio::AdjustInactiveMasterVolume(inactive_master_volume) => {
                        settings.audio.inactive_master_volume = inactive_master_volume;
                    },
                    Audio::AdjustMusicVolume(music_volume) => {
                        global_state.audio.set_music_volume(music_volume);

                        settings.audio.music_volume = music_volume;
                    },
                    Audio::AdjustSfxVolume(sfx_volume) => {
                        global_state.audio.set_sfx_volume(sfx_volume);

                        settings.audio.sfx_volume = sfx_volume;
                    },
                    //Audio::ChangeAudioDevice(name) => {
                    //    global_state.audio.set_device(name.clone());

                    //    settings.audio.output = AudioOutput::Device(name);
                    //},
                    Audio::ResetAudioSettings => {
                        settings.audio = AudioSettings::default();
                        let audio = &settings.audio;
                        global_state.audio.set_music_volume(audio.music_volume);
                        global_state.audio.set_sfx_volume(audio.sfx_volume);
                    },
                }
                settings.save_to_file_warn();
            },
            SettingsChange::Control(control_change) => match control_change {
                Control::ChangeBinding(game_input) => {
                    global_state.window.set_keybinding_mode(game_input);
                },
                Control::ResetKeyBindings => {
                    settings.controls = ControlSettings::default();
                    settings.save_to_file_warn();
                },
            },
            SettingsChange::Gamepad(gamepad_change) => match gamepad_change {},
            SettingsChange::Gameplay(gameplay_change) => {
                let mut window = &mut global_state.window;
                match gameplay_change {
                    Gameplay::AdjustMousePan(sensitivity) => {
                        window.pan_sensitivity = sensitivity;
                        settings.gameplay.pan_sensitivity = sensitivity;
                    },
                    Gameplay::AdjustMouseZoom(sensitivity) => {
                        window.zoom_sensitivity = sensitivity;
                        settings.gameplay.zoom_sensitivity = sensitivity;
                    },
                    Gameplay::AdjustCameraClamp(angle) => {
                        settings.gameplay.camera_clamp_angle = angle;
                    },
                    Gameplay::ToggleControllerYInvert(controller_y_inverted) => {
                        window.controller_settings.pan_invert_y = controller_y_inverted;
                        settings.controller.pan_invert_y = controller_y_inverted;
                    },
                    Gameplay::ToggleMouseYInvert(mouse_y_inverted) => {
                        window.mouse_y_inversion = mouse_y_inverted;
                        settings.gameplay.mouse_y_inversion = mouse_y_inverted;
                    },
                    Gameplay::ToggleZoomInvert(zoom_inverted) => {
                        window.zoom_inversion = zoom_inverted;
                        settings.gameplay.zoom_inversion = zoom_inverted;
                    },
                    Gameplay::ToggleSmoothPan(smooth_pan_enabled) => {
                        settings.gameplay.smooth_pan_enable = smooth_pan_enabled;
                    },
                    Gameplay::ChangeFreeLookBehavior(behavior) => {
                        settings.gameplay.free_look_behavior = behavior;
                    },
                    Gameplay::ChangeAutoWalkBehavior(behavior) => {
                        settings.gameplay.auto_walk_behavior = behavior;
                    },
                    Gameplay::ChangeCameraClampBehavior(behavior) => {
                        settings.gameplay.camera_clamp_behavior = behavior;
                    },
                    Gameplay::ChangePlayerPhysicsBehavior {
                        server_authoritative,
                    } => {
                        settings.gameplay.player_physics_behavior = server_authoritative;
                        session_state
                            .client
                            .borrow_mut()
                            .request_player_physics(server_authoritative);
                    },
                    Gameplay::ChangeStopAutoWalkOnInput(state) => {
                        settings.gameplay.stop_auto_walk_on_input = state;
                    },
                    Gameplay::ChangeAutoCamera(state) => {
                        settings.gameplay.auto_camera = state;
                    },
                    Gameplay::ResetGameplaySettings => {
                        // Reset Gameplay Settings
                        settings.gameplay = GameplaySettings::default();
                        // Reset Gamepad and Controller Settings
                        settings.controller = GamepadSettings::default();
                        window.controller_settings = ControllerSettings::from(&settings.controller);
                        // Pan Sensitivity
                        window.pan_sensitivity = settings.gameplay.pan_sensitivity;
                        // Zoom Sensitivity
                        window.zoom_sensitivity = settings.gameplay.zoom_sensitivity;
                        // Invert Scroll Zoom
                        window.zoom_inversion = settings.gameplay.zoom_inversion;
                        // Invert Mouse Y Axis
                        window.mouse_y_inversion = settings.gameplay.mouse_y_inversion;
                    },
                }
                settings.save_to_file_warn();
            },
            SettingsChange::Graphics(graphics_change) => {
                match graphics_change {
                    Graphics::AdjustViewDistance(view_distance) => {
                        session_state
                            .client
                            .borrow_mut()
                            .set_view_distance(view_distance);

                        settings.graphics.view_distance = view_distance;
                    },
                    Graphics::AdjustLodDetail(lod_detail) => {
                        session_state.scene.lod.set_detail(lod_detail);

                        settings.graphics.lod_detail = lod_detail;
                    },
                    Graphics::AdjustSpriteRenderDistance(sprite_render_distance) => {
                        settings.graphics.sprite_render_distance = sprite_render_distance;
                    },
                    Graphics::AdjustFigureLoDRenderDistance(figure_lod_render_distance) => {
                        settings.graphics.figure_lod_render_distance = figure_lod_render_distance;
                    },
                    Graphics::ChangeMaxFPS(fps) => {
                        settings.graphics.max_fps = fps;
                    },
                    Graphics::ChangeFOV(new_fov) => {
                        settings.graphics.fov = new_fov;
                        session_state.scene.camera_mut().set_fov_deg(new_fov);
                        session_state
                            .scene
                            .camera_mut()
                            .compute_dependents(&*session_state.client.borrow().state().terrain());
                    },
                    Graphics::ChangeGamma(new_gamma) => {
                        settings.graphics.gamma = new_gamma;
                    },
                    Graphics::ChangeExposure(new_exposure) => {
                        settings.graphics.exposure = new_exposure;
                    },
                    Graphics::ChangeAmbiance(new_ambiance) => {
                        settings.graphics.ambiance = new_ambiance;
                    },
                    Graphics::ChangeRenderMode(new_render_mode) => {
                        // Do this first so if it crashes the setting isn't saved :)
                        global_state
                            .window
                            .renderer_mut()
                            .set_render_mode((&*new_render_mode).clone())
                            .unwrap();
                        settings.graphics.render_mode = *new_render_mode;
                    },
                    Graphics::ChangeFullscreenMode(new_fullscreen_settings) => {
                        global_state
                            .window
                            .set_fullscreen_mode(new_fullscreen_settings);
                        settings.graphics.fullscreen = new_fullscreen_settings;
                    },
                    Graphics::ToggleParticlesEnabled(particles_enabled) => {
                        settings.graphics.particles_enabled = particles_enabled;
                    },
                    Graphics::AdjustWindowSize(new_size) => {
                        global_state.window.set_size(new_size.into());
                        settings.graphics.window_size = new_size;
                    },
                    Graphics::ResetGraphicsSettings => {
                        settings.graphics = GraphicsSettings::default();
                        let graphics = &settings.graphics;
                        // View distance
                        session_state
                            .client
                            .borrow_mut()
                            .set_view_distance(graphics.view_distance);
                        // FOV
                        session_state.scene.camera_mut().set_fov_deg(graphics.fov);
                        session_state
                            .scene
                            .camera_mut()
                            .compute_dependents(&*session_state.client.borrow().state().terrain());
                        // LoD
                        session_state.scene.lod.set_detail(graphics.lod_detail);
                        // Render mode
                        global_state
                            .window
                            .renderer_mut()
                            .set_render_mode(graphics.render_mode.clone())
                            .unwrap();
                        // Fullscreen mode
                        global_state.window.set_fullscreen_mode(graphics.fullscreen);
                        // Window size
                        global_state.window.set_size(graphics.window_size.into());
                    },
                }
                settings.save_to_file_warn();
            },
            SettingsChange::Interface(interface_change) => {
                match interface_change {
                    Interface::Sct(sct) => {
                        settings.interface.sct = sct;
                    },
                    Interface::SctPlayerBatch(sct_player_batch) => {
                        settings.interface.sct_player_batch = sct_player_batch;
                    },
                    Interface::SctDamageBatch(sct_damage_batch) => {
                        settings.interface.sct_damage_batch = sct_damage_batch;
                    },
                    Interface::SpeechBubbleDarkMode(sbdm) => {
                        settings.interface.speech_bubble_dark_mode = sbdm;
                    },
                    Interface::SpeechBubbleIcon(sbi) => {
                        settings.interface.speech_bubble_icon = sbi;
                    },
                    Interface::ToggleHelp(_) => {
                        // implemented in hud
                    },
                    Interface::ToggleDebug(toggle_debug) => {
                        settings.interface.toggle_debug = toggle_debug;
                    },
                    Interface::ToggleTips(loading_tips) => {
                        settings.interface.loading_tips = loading_tips;
                    },
                    Interface::CrosshairTransp(crosshair_transp) => {
                        settings.interface.crosshair_transp = crosshair_transp;
                    },
                    Interface::ChatTransp(chat_transp) => {
                        settings.interface.chat_transp = chat_transp;
                    },
                    Interface::ChatCharName(chat_char_name) => {
                        settings.interface.chat_character_name = chat_char_name;
                    },
                    Interface::CrosshairType(crosshair_type) => {
                        settings.interface.crosshair_type = crosshair_type;
                    },
                    Interface::Intro(intro_show) => {
                        settings.interface.intro_show = intro_show;
                    },
                    Interface::ToggleXpBar(xp_bar) => {
                        settings.interface.xp_bar = xp_bar;
                    },
                    Interface::ToggleBarNumbers(bar_numbers) => {
                        settings.interface.bar_numbers = bar_numbers;
                    },
                    Interface::ToggleShortcutNumbers(shortcut_numbers) => {
                        settings.interface.shortcut_numbers = shortcut_numbers;
                    },
                    Interface::BuffPosition(buff_position) => {
                        settings.interface.buff_position = buff_position;
                    },
                    Interface::UiScale(scale_change) => {
                        settings.interface.ui_scale = session_state.hud.scale_change(scale_change);
                    },
                    Interface::MinimapShow(state) => {
                        settings.interface.minimap_show = state;
                    },
                    Interface::MinimapFaceNorth(state) => {
                        settings.interface.minimap_face_north = state;
                    },
                    Interface::MinimapZoom(minimap_zoom) => {
                        settings.interface.minimap_zoom = minimap_zoom;
                    },
                    Interface::MapZoom(map_zoom) => {
                        settings.interface.map_zoom = map_zoom;
                    },
                    Interface::MapDrag(map_drag) => {
                        settings.interface.map_drag = map_drag;
                    },
                    Interface::MapShowTopoMap(map_show_topo_map) => {
                        settings.interface.map_show_topo_map = map_show_topo_map;
                    },
                    Interface::MapShowDifficulty(map_show_difficulty) => {
                        settings.interface.map_show_difficulty = map_show_difficulty;
                    },
                    Interface::MapShowTowns(map_show_towns) => {
                        settings.interface.map_show_towns = map_show_towns;
                    },
                    Interface::MapShowDungeons(map_show_dungeons) => {
                        settings.interface.map_show_dungeons = map_show_dungeons;
                    },
                    Interface::MapShowCastles(map_show_castles) => {
                        settings.interface.map_show_castles = map_show_castles;
                    },
                    Interface::MapShowCaves(map_show_caves) => {
                        settings.interface.map_show_caves = map_show_caves;
                    },
                    Interface::MapShowTrees(map_show_trees) => {
                        settings.interface.map_show_trees = map_show_trees;
                    },
                    Interface::MapShowPeaks(map_show_peaks) => {
                        settings.interface.map_show_peaks = map_show_peaks;
                    },
                    Interface::ResetInterfaceSettings => {
                        // Reset Interface Settings
                        let tmp = settings.interface.intro_show;
                        settings.interface = InterfaceSettings::default();
                        settings.interface.intro_show = tmp;
                        // Update Current Scaling Mode
                        session_state
                            .hud
                            .set_scaling_mode(settings.interface.ui_scale);
                    },
                }
                settings.save_to_file_warn();
            },
            SettingsChange::Language(language_change) => match language_change {
                Language::ChangeLanguage(new_language) => {
                    settings.language.selected_language = new_language.language_identifier;
                    global_state.i18n = LocalizationHandle::load_expect(&i18n_asset_key(
                        &settings.language.selected_language,
                    ));
                    global_state.i18n.read().log_missing_entries();
                    global_state
                        .i18n
                        .set_english_fallback(settings.language.use_english_fallback);
                    session_state.hud.update_fonts(&global_state.i18n.read());
                },
                Language::ToggleEnglishFallback(toggle_fallback) => {
                    settings.language.use_english_fallback = toggle_fallback;
                    global_state
                        .i18n
                        .set_english_fallback(settings.language.use_english_fallback);
                },
            },
            SettingsChange::Networking(networking_change) => match networking_change {},
        }
    }
}
