use super::SessionState;
use crate::{
    controller::ControllerSettings,
    hud::{
        BarNumbers, BuffPosition, CrosshairType, Intro, PressBehavior, ScaleChange,
        ShortcutNumbers, XpBar,
    },
    i18n::{i18n_asset_key, LanguageMetadata, Localization},
    render::RenderMode,
    settings::{
        AudioSettings, ControlSettings, Fps, GamepadSettings, GameplaySettings, GraphicsSettings,
        InterfaceSettings,
    },
    window::{FullScreenSettings, GameInput},
    GlobalState,
};
use common::assets::AssetExt;
use vek::*;

#[derive(Clone)]
pub enum Audio {
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

    ResetInterfaceSettings,
}
#[derive(Clone)]
pub enum Language {
    ChangeLanguage(Box<LanguageMetadata>),
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
        // let mut settings = &mut global_state.settings;
        // let mut window = &mut global_state.window;
        match self {
            SettingsChange::Audio(audio_change) => match audio_change {
                Audio::AdjustMusicVolume(music_volume) => {
                    global_state.audio.set_music_volume(music_volume);

                    global_state.settings.audio.music_volume = music_volume;
                    global_state.settings.save_to_file_warn();
                },
                Audio::AdjustSfxVolume(sfx_volume) => {
                    global_state.audio.set_sfx_volume(sfx_volume);

                    global_state.settings.audio.sfx_volume = sfx_volume;
                    global_state.settings.save_to_file_warn();
                },
                //Audio::ChangeAudioDevice(name) => {
                //    global_state.audio.set_device(name.clone());

                //    global_state.settings.audio.output = AudioOutput::Device(name);
                //    global_state.settings.save_to_file_warn();
                //},
                Audio::ResetAudioSettings => {
                    global_state.settings.audio = AudioSettings::default();
                    global_state.settings.save_to_file_warn();
                    let audio = &global_state.settings.audio;
                    global_state.audio.set_music_volume(audio.music_volume);
                    global_state.audio.set_sfx_volume(audio.sfx_volume);
                },
            },
            SettingsChange::Control(control_change) => match control_change {
                Control::ChangeBinding(game_input) => {
                    global_state.window.set_keybinding_mode(game_input);
                },
                Control::ResetKeyBindings => {
                    global_state.settings.controls = ControlSettings::default();
                    global_state.settings.save_to_file_warn();
                },
            },
            SettingsChange::Gamepad(gamepad_change) => match gamepad_change {},
            SettingsChange::Gameplay(gameplay_change) => match gameplay_change {
                Gameplay::AdjustMousePan(sensitivity) => {
                    global_state.window.pan_sensitivity = sensitivity;
                    global_state.settings.gameplay.pan_sensitivity = sensitivity;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::AdjustMouseZoom(sensitivity) => {
                    global_state.window.zoom_sensitivity = sensitivity;
                    global_state.settings.gameplay.zoom_sensitivity = sensitivity;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::AdjustCameraClamp(angle) => {
                    global_state.settings.gameplay.camera_clamp_angle = angle;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ToggleControllerYInvert(controller_y_inverted) => {
                    global_state.window.controller_settings.pan_invert_y = controller_y_inverted;
                    global_state.settings.controller.pan_invert_y = controller_y_inverted;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ToggleMouseYInvert(mouse_y_inverted) => {
                    global_state.window.mouse_y_inversion = mouse_y_inverted;
                    global_state.settings.gameplay.mouse_y_inversion = mouse_y_inverted;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ToggleZoomInvert(zoom_inverted) => {
                    global_state.window.zoom_inversion = zoom_inverted;
                    global_state.settings.gameplay.zoom_inversion = zoom_inverted;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ToggleSmoothPan(smooth_pan_enabled) => {
                    global_state.settings.gameplay.smooth_pan_enable = smooth_pan_enabled;
                    global_state.settings.save_to_file_warn();
                },

                Gameplay::ChangeFreeLookBehavior(behavior) => {
                    global_state.settings.gameplay.free_look_behavior = behavior;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ChangeAutoWalkBehavior(behavior) => {
                    global_state.settings.gameplay.auto_walk_behavior = behavior;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ChangeCameraClampBehavior(behavior) => {
                    global_state.settings.gameplay.camera_clamp_behavior = behavior;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ChangeStopAutoWalkOnInput(state) => {
                    global_state.settings.gameplay.stop_auto_walk_on_input = state;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ChangeAutoCamera(state) => {
                    global_state.settings.gameplay.auto_camera = state;
                    global_state.settings.save_to_file_warn();
                },
                Gameplay::ResetGameplaySettings => {
                    // Reset Gameplay Settings
                    global_state.settings.gameplay = GameplaySettings::default();
                    // Reset Gamepad and Controller Settings
                    global_state.settings.controller = GamepadSettings::default();
                    global_state.window.controller_settings =
                        ControllerSettings::from(&global_state.settings.controller);
                    // Pan Sensitivity
                    global_state.window.pan_sensitivity =
                        global_state.settings.gameplay.pan_sensitivity;
                    // Zoom Sensitivity
                    global_state.window.zoom_sensitivity =
                        global_state.settings.gameplay.zoom_sensitivity;
                    // Invert Scroll Zoom
                    global_state.window.zoom_inversion =
                        global_state.settings.gameplay.zoom_inversion;
                    // Invert Mouse Y Axis
                    global_state.window.mouse_y_inversion =
                        global_state.settings.gameplay.mouse_y_inversion;
                    // Save to File
                    global_state.settings.save_to_file_warn();
                },
            },
            SettingsChange::Graphics(graphics_change) => match graphics_change {
                Graphics::AdjustViewDistance(view_distance) => {
                    session_state
                        .client
                        .borrow_mut()
                        .set_view_distance(view_distance);

                    global_state.settings.graphics.view_distance = view_distance;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::AdjustLodDetail(lod_detail) => {
                    session_state.scene.lod.set_detail(lod_detail);

                    global_state.settings.graphics.lod_detail = lod_detail;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::AdjustSpriteRenderDistance(sprite_render_distance) => {
                    global_state.settings.graphics.sprite_render_distance = sprite_render_distance;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::AdjustFigureLoDRenderDistance(figure_lod_render_distance) => {
                    global_state.settings.graphics.figure_lod_render_distance =
                        figure_lod_render_distance;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeMaxFPS(fps) => {
                    global_state.settings.graphics.max_fps = fps;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeFOV(new_fov) => {
                    global_state.settings.graphics.fov = new_fov;
                    global_state.settings.save_to_file_warn();
                    session_state.scene.camera_mut().set_fov_deg(new_fov);
                    session_state
                        .scene
                        .camera_mut()
                        .compute_dependents(&*session_state.client.borrow().state().terrain());
                },
                Graphics::ChangeGamma(new_gamma) => {
                    global_state.settings.graphics.gamma = new_gamma;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeExposure(new_exposure) => {
                    global_state.settings.graphics.exposure = new_exposure;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeAmbiance(new_ambiance) => {
                    global_state.settings.graphics.ambiance = new_ambiance;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeRenderMode(new_render_mode) => {
                    // Do this first so if it crashes the setting isn't saved :)
                    global_state
                        .window
                        .renderer_mut()
                        .set_render_mode((&*new_render_mode).clone())
                        .unwrap();
                    global_state.settings.graphics.render_mode = *new_render_mode;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ChangeFullscreenMode(new_fullscreen_settings) => {
                    global_state
                        .window
                        .set_fullscreen_mode(new_fullscreen_settings);
                    global_state.settings.graphics.fullscreen = new_fullscreen_settings;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ToggleParticlesEnabled(particles_enabled) => {
                    global_state.settings.graphics.particles_enabled = particles_enabled;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::AdjustWindowSize(new_size) => {
                    global_state.window.set_size(new_size.into());
                    global_state.settings.graphics.window_size = new_size;
                    global_state.settings.save_to_file_warn();
                },
                Graphics::ResetGraphicsSettings => {
                    global_state.settings.graphics = GraphicsSettings::default();
                    global_state.settings.save_to_file_warn();
                    let graphics = &global_state.settings.graphics;
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
            },
            SettingsChange::Interface(interface_change) => match interface_change {
                Interface::Sct(sct) => {
                    global_state.settings.interface.sct = sct;
                    global_state.settings.save_to_file_warn();
                },
                Interface::SctPlayerBatch(sct_player_batch) => {
                    global_state.settings.interface.sct_player_batch = sct_player_batch;
                    global_state.settings.save_to_file_warn();
                },
                Interface::SctDamageBatch(sct_damage_batch) => {
                    global_state.settings.interface.sct_damage_batch = sct_damage_batch;
                    global_state.settings.save_to_file_warn();
                },
                Interface::SpeechBubbleDarkMode(sbdm) => {
                    global_state.settings.interface.speech_bubble_dark_mode = sbdm;
                    global_state.settings.save_to_file_warn();
                },
                Interface::SpeechBubbleIcon(sbi) => {
                    global_state.settings.interface.speech_bubble_icon = sbi;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ToggleHelp(_) => {
                    // implemented in hud
                },
                Interface::ToggleDebug(toggle_debug) => {
                    global_state.settings.interface.toggle_debug = toggle_debug;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ToggleTips(loading_tips) => {
                    global_state.settings.interface.loading_tips = loading_tips;
                    global_state.settings.save_to_file_warn();
                },

                Interface::CrosshairTransp(crosshair_transp) => {
                    global_state.settings.interface.crosshair_transp = crosshair_transp;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ChatTransp(chat_transp) => {
                    global_state.settings.interface.chat_transp = chat_transp;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ChatCharName(chat_char_name) => {
                    global_state.settings.interface.chat_character_name = chat_char_name;
                    global_state.settings.save_to_file_warn();
                },
                Interface::CrosshairType(crosshair_type) => {
                    global_state.settings.interface.crosshair_type = crosshair_type;
                    global_state.settings.save_to_file_warn();
                },
                Interface::Intro(intro_show) => {
                    global_state.settings.interface.intro_show = intro_show;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ToggleXpBar(xp_bar) => {
                    global_state.settings.interface.xp_bar = xp_bar;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ToggleBarNumbers(bar_numbers) => {
                    global_state.settings.interface.bar_numbers = bar_numbers;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ToggleShortcutNumbers(shortcut_numbers) => {
                    global_state.settings.interface.shortcut_numbers = shortcut_numbers;
                    global_state.settings.save_to_file_warn();
                },
                Interface::BuffPosition(buff_position) => {
                    global_state.settings.interface.buff_position = buff_position;
                    global_state.settings.save_to_file_warn();
                },

                Interface::UiScale(scale_change) => {
                    global_state.settings.interface.ui_scale =
                        session_state.hud.scale_change(scale_change);
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapZoom(map_zoom) => {
                    global_state.settings.interface.map_zoom = map_zoom;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapDrag(map_drag) => {
                    global_state.settings.interface.map_drag = map_drag;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowTopoMap(map_show_topo_map) => {
                    global_state.settings.interface.map_show_topo_map = map_show_topo_map;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowDifficulty(map_show_difficulty) => {
                    global_state.settings.interface.map_show_difficulty = map_show_difficulty;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowTowns(map_show_towns) => {
                    global_state.settings.interface.map_show_towns = map_show_towns;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowDungeons(map_show_dungeons) => {
                    global_state.settings.interface.map_show_dungeons = map_show_dungeons;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowCastles(map_show_castles) => {
                    global_state.settings.interface.map_show_castles = map_show_castles;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowCaves(map_show_caves) => {
                    global_state.settings.interface.map_show_caves = map_show_caves;
                    global_state.settings.save_to_file_warn();
                },
                Interface::MapShowTrees(map_show_trees) => {
                    global_state.settings.interface.map_show_trees = map_show_trees;
                    global_state.settings.save_to_file_warn();
                },
                Interface::ResetInterfaceSettings => {
                    // Reset Interface Settings
                    let tmp = global_state.settings.interface.intro_show;
                    global_state.settings.interface = InterfaceSettings::default();
                    global_state.settings.interface.intro_show = tmp;
                    // Update Current Scaling Mode
                    session_state
                        .hud
                        .set_scaling_mode(global_state.settings.interface.ui_scale);

                    // Save to File
                    global_state.settings.save_to_file_warn();
                },
            },
            SettingsChange::Language(language_change) => match language_change {
                Language::ChangeLanguage(new_language) => {
                    global_state.settings.language.selected_language =
                        new_language.language_identifier;
                    global_state.i18n = Localization::load_expect(&i18n_asset_key(
                        &global_state.settings.language.selected_language,
                    ));
                    global_state.i18n.read().log_missing_entries();
                    session_state.hud.update_fonts(&global_state.i18n.read());
                },
            },
            SettingsChange::Networking(networking_change) => match networking_change {},
        }
    }
}
