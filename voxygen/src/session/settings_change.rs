use super::SessionState;
use crate::{
    GlobalState,
    audio::SfxChannelSettings,
    game_input::GameInput,
    hud::{
        AutoPressBehavior, BarNumbers, BuffPosition, ChatTab, CrosshairType, Intro, PressBehavior,
        ScaleChange, ShortcutNumbers, XpBar,
    },
    render::RenderMode,
    settings::{
        AudioSettings, ChatSettings, ControlSettings, ControllerSettings, Fps, GameplaySettings,
        GraphicsSettings, InterfaceSettings, audio::AudioVolume,
    },
    window::{FullScreenSettings, MenuInput, Window},
};
use common::comp::inventory::InventorySortOrder;
use i18n::{LanguageMetadata, LocalizationHandle};
use std::rc::Rc;

#[derive(Clone)]
pub enum Audio {
    AdjustMasterVolume(f32),
    MuteMasterVolume(bool),
    AdjustInactiveMasterVolume(f32),
    MuteInactiveMasterVolume(bool),
    AdjustMusicVolume(f32),
    MuteMusicVolume(bool),
    AdjustSfxVolume(f32),
    MuteSfxVolume(bool),
    AdjustAmbienceVolume(f32),
    MuteAmbienceVolume(bool),
    RainAmbience(bool),
    AdjustMusicSpacing(f32),
    ToggleCombatMusic(bool),
    SetNumSfxChannels(SfxChannelSettings),
    ResetAudioSettings,
}
#[derive(Clone)]
pub enum Chat {
    Transparency(f32),
    LockChat(bool),
    CharName(bool),
    ChangeChatTab(Option<usize>),
    ChatTabUpdate(usize, ChatTab),
    ChatTabInsert(usize, ChatTab),
    ChatTabMove(usize, usize), //(i, j) move item from position i, and insert into position j
    ChatTabRemove(usize),
    ResetChatSettings,
}
#[derive(Clone)]
pub enum Control {
    // change keyboard bindings
    ChangeBindingKeyboard(GameInput),
    RemoveBindingKeyboard(GameInput),
    ResetKeyBindingsKeyboard,

    // reset all gamepad bindings
    ResetKeyBindingsGamepad,

    // change gamepad button bindings
    ChangeBindingGamepadButton(GameInput),
    RemoveBindingGamepadButton(GameInput),

    // change gamepad menu button bindings
    ChangeBindingGamepadMenu(MenuInput),
    RemoveBindingGamepadMenu(MenuInput),

    // change gamepad layer bindings
    ChangeBindingGamepadLayer(GameInput),
    RemoveBindingGamepadLayer(GameInput),
    GameLayerMod1(bool),
    GameLayerMod2(bool),
}
#[derive(Clone)]
pub enum Gamepad {}
#[derive(Clone)]
pub enum Gameplay {
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustCameraClamp(u32),
    AdjustWalkingSpeed(f32),

    ToggleControllerYInvert(bool),
    ToggleMouseYInvert(bool),
    ToggleZoomInvert(bool),
    ToggleSmoothPan(bool),

    ChangeFreeLookBehavior(PressBehavior),
    ChangeAutoWalkBehavior(PressBehavior),
    ChangeWalkingSpeedBehavior(PressBehavior),
    ChangeCameraClampBehavior(PressBehavior),
    ChangeZoomLockBehavior(AutoPressBehavior),
    ChangeStopAutoWalkOnInput(bool),
    ChangeAutoCamera(bool),
    ChangeBowZoom(bool),
    ChangeZoomLock(bool),
    ChangeShowAllRecipes(bool),

    AdjustAimOffsetX(f32),
    AdjustAimOffsetY(f32),

    ResetGameplaySettings,
}
#[derive(Clone)]
pub enum Graphics {
    AdjustTerrainViewDistance(u32),
    AdjustEntityViewDistance(u32),
    AdjustLodDistance(u32),
    AdjustLodDetail(u32),
    AdjustSpriteRenderDistance(u32),
    AdjustFigureLoDRenderDistance(u32),

    ChangeMaxFPS(Fps),
    ChangeMaxBackgroundFPS(Fps),
    ChangeFOV(u16),

    ChangeGamma(f32),
    ChangeExposure(f32),
    ChangeAmbiance(f32),

    ChangeRenderMode(Box<RenderMode>),

    ChangeFullscreenMode(FullScreenSettings),
    ToggleParticlesEnabled(bool),
    ToggleWeaponTrailsEnabled(bool),

    ResetGraphicsSettings,
    ChangeGraphicsSettings(Rc<dyn Fn(GraphicsSettings) -> GraphicsSettings>),
}
#[derive(Clone)]
pub enum Interface {
    Sct(bool),
    SctRoundDamage(bool),
    SctDamageAccumDuration(f32),
    SctIncomingDamage(bool),
    SctIncomingDamageAccumDuration(f32),
    SpeechBubbleSelf(bool),
    SpeechBubbleDarkMode(bool),
    SpeechBubbleIcon(bool),
    ToggleDebug(bool),
    ToggleHitboxes(bool),
    ToggleChat(bool),
    ToggleTips(bool),
    ToggleHotkeyHints(bool),

    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    Intro(Intro),
    ToggleXpBar(XpBar),
    ToggleHealthPrefixes(bool),
    ToggleBarNumbers(BarNumbers),
    ToggleAlwaysShowBars(bool),
    TogglePoiseBar(bool),
    ToggleShortcutNumbers(ShortcutNumbers),
    BuffPosition(BuffPosition),
    RowBackgroundOpacity(f32),

    UiScale(ScaleChange),
    //Minimap
    MinimapShow(bool),
    MinimapFaceNorth(bool),
    MinimapZoom(f64),
    MinimapScale(f64),
    MinimapColoredPlayerMarker(bool),
    //Map settings
    MapZoom(f64),
    MapShowTopoMap(bool),
    MapShowDifficulty(bool),
    MapShowTowns(bool),
    MapShowDungeons(bool),
    MapShowCastles(bool),
    MapShowBridges(bool),
    MapShowGliderCourses(bool),
    MapShowCaves(bool),
    MapShowTrees(bool),
    MapShowPeaks(bool),
    MapShowBiomes(bool),
    MapShowVoxelMap(bool),
    MapShowQuests(bool),
    AccumExperience(bool),
    //Slots
    SlotsUsePrefixes(bool),
    SlotsPrefixSwitchPoint(u32),

    ResetInterfaceSettings,
}
#[derive(Clone)]
pub enum Inventory {
    ChangeSortOrder(InventorySortOrder),
}
#[derive(Clone)]
pub enum Language {
    ChangeLanguage(Box<LanguageMetadata>),
    ToggleSendToServer(bool),
    ToggleEnglishFallback(bool),
}
#[derive(Clone)]
pub enum Networking {
    AdjustTerrainViewDistance(u32),
    AdjustEntityViewDistance(u32),
    ChangePlayerPhysicsBehavior {
        server_authoritative: bool,
    },
    ToggleLossyTerrainCompression(bool),

    #[cfg(feature = "discord")]
    ToggleDiscordIntegration(bool),
    // TODO: reset option (ensure it handles the entity/terrain vd the same as graphics reset
    // option)
}

#[derive(Clone)]
pub enum Accessibility {
    ChangeRenderMode(Box<RenderMode>),
    SetSubtitles(bool),
}

#[derive(Clone)]
pub enum SettingsChange {
    Audio(Audio),
    Chat(Chat),
    Control(Control),
    Gamepad(Gamepad),
    Gameplay(Gameplay),
    Graphics(Graphics),
    Interface(Interface),
    Inventory(Inventory),
    Language(Language),
    Networking(Networking),
    Accessibility(Accessibility),
}

macro_rules! settings_change_from {
    ($i: ident) => {
        impl From<$i> for SettingsChange {
            fn from(change: $i) -> Self { SettingsChange::$i(change) }
        }
    };
}
settings_change_from!(Audio);
settings_change_from!(Chat);
settings_change_from!(Control);
settings_change_from!(Gamepad);
settings_change_from!(Gameplay);
settings_change_from!(Graphics);
settings_change_from!(Interface);
settings_change_from!(Language);
settings_change_from!(Networking);
settings_change_from!(Accessibility);

impl SettingsChange {
    pub fn process(self, global_state: &mut GlobalState, session_state: &mut SessionState) {
        let settings = &mut global_state.settings;

        match self {
            SettingsChange::Audio(audio_change) => {
                fn update_volume(audio: &mut AudioVolume, volume: f32) -> f32 {
                    audio.volume = volume;
                    audio.get_checked()
                }
                fn update_muted(audio: &mut AudioVolume, muted: bool) -> f32 {
                    audio.muted = muted;
                    audio.get_checked()
                }

                match audio_change {
                    Audio::AdjustMasterVolume(master_volume) => {
                        let volume_checked =
                            update_volume(&mut settings.audio.master_volume, master_volume);

                        global_state.audio.set_master_volume(volume_checked);
                    },
                    Audio::MuteMasterVolume(master_muted) => {
                        let volume_checked =
                            update_muted(&mut settings.audio.master_volume, master_muted);

                        global_state.audio.set_master_volume(volume_checked);
                    },
                    Audio::AdjustInactiveMasterVolume(inactive_master_volume_perc) => {
                        settings.audio.inactive_master_volume_perc.volume =
                            inactive_master_volume_perc;
                    },
                    Audio::MuteInactiveMasterVolume(inactive_master_volume_muted) => {
                        settings.audio.inactive_master_volume_perc.muted =
                            inactive_master_volume_muted;
                    },
                    Audio::AdjustMusicVolume(music_volume) => {
                        let volume_checked =
                            update_volume(&mut settings.audio.music_volume, music_volume);

                        global_state.audio.set_music_volume(volume_checked);
                    },
                    Audio::MuteMusicVolume(music_muted) => {
                        let volume_checked =
                            update_muted(&mut settings.audio.music_volume, music_muted);

                        global_state.audio.set_music_volume(volume_checked);
                    },
                    Audio::AdjustSfxVolume(sfx_volume) => {
                        let volume_checked =
                            update_volume(&mut settings.audio.sfx_volume, sfx_volume);

                        global_state.audio.set_sfx_volume(volume_checked);
                    },
                    Audio::MuteSfxVolume(sfx_muted) => {
                        let volume_checked =
                            update_muted(&mut settings.audio.sfx_volume, sfx_muted);

                        global_state.audio.set_sfx_volume(volume_checked);
                    },
                    Audio::AdjustAmbienceVolume(ambience_volume) => {
                        global_state.audio.set_ambience_volume(ambience_volume);

                        settings.audio.ambience_volume.volume = ambience_volume;
                    },
                    Audio::MuteAmbienceVolume(ambience_muted) => {
                        let volume_checked =
                            update_muted(&mut settings.audio.ambience_volume, ambience_muted);

                        global_state.audio.set_ambience_volume(volume_checked);
                    },
                    Audio::RainAmbience(rain_ambience_enabled) => {
                        settings.audio.rain_ambience_enabled = rain_ambience_enabled;
                    },
                    Audio::AdjustMusicSpacing(multiplier) => {
                        global_state.audio.set_music_spacing(multiplier);

                        settings.audio.music_spacing = multiplier;
                    },
                    Audio::ToggleCombatMusic(combat_music_enabled) => {
                        global_state.audio.combat_music_enabled = combat_music_enabled
                    },
                    Audio::SetNumSfxChannels(level) => {
                        let num = SfxChannelSettings::to_usize(&level);
                        global_state.audio.set_num_sfx_channels(num);
                        settings.audio.num_sfx_channels = num;
                    },
                    Audio::ResetAudioSettings => {
                        let previous_sample_rate = settings.audio.sample_rate;
                        let previous_buffer_size = settings.audio.buffer_size;
                        settings.audio = AudioSettings::default();
                        settings.audio.sample_rate = previous_sample_rate;
                        settings.audio.buffer_size = previous_buffer_size;

                        let audio = &mut global_state.audio;

                        audio.set_master_volume(settings.audio.master_volume.get_checked());
                        audio.set_music_volume(settings.audio.music_volume.get_checked());
                        audio.set_ambience_volume(settings.audio.ambience_volume.get_checked());
                        audio.set_sfx_volume(settings.audio.sfx_volume.get_checked());
                        audio.set_music_spacing(settings.audio.music_spacing);
                    },
                }
            },
            SettingsChange::Chat(chat_change) => {
                let chat_tabs = &mut settings.chat.chat_tabs;
                match chat_change {
                    Chat::Transparency(chat_opacity) => {
                        settings.chat.chat_opacity = chat_opacity;
                    },
                    Chat::LockChat(lock_chat) => {
                        settings.chat.lock_chat = lock_chat;
                    },
                    Chat::CharName(chat_char_name) => {
                        settings.chat.chat_character_name = chat_char_name;
                    },
                    Chat::ChangeChatTab(chat_tab_index) => {
                        settings.chat.chat_tab_index =
                            chat_tab_index.filter(|i| *i < chat_tabs.len());
                    },
                    Chat::ChatTabUpdate(i, chat_tab) => {
                        if i < chat_tabs.len() {
                            chat_tabs[i] = chat_tab;
                        }
                    },
                    Chat::ChatTabInsert(i, chat_tab) => {
                        if i <= chat_tabs.len() {
                            settings.chat.chat_tabs.insert(i, chat_tab);
                        }
                    },
                    Chat::ChatTabMove(i, j) => {
                        if i < chat_tabs.len() && j < chat_tabs.len() {
                            let chat_tab = settings.chat.chat_tabs.remove(i);
                            settings.chat.chat_tabs.insert(j, chat_tab);
                        }
                    },
                    Chat::ChatTabRemove(i) => {
                        if i < chat_tabs.len() {
                            settings.chat.chat_tabs.remove(i);
                        }
                    },
                    Chat::ResetChatSettings => {
                        settings.chat = ChatSettings::default();
                    },
                }
            },
            SettingsChange::Control(control_change) => match control_change {
                // keyboard
                Control::ChangeBindingKeyboard(game_input) => {
                    global_state.window.set_remapping_mode(
                        crate::window::RemappingMode::RemapKeyboard(game_input),
                    );
                },
                Control::RemoveBindingKeyboard(game_input) => {
                    settings.controls.remove_binding(game_input);
                },
                Control::ResetKeyBindingsKeyboard => {
                    settings.controls = ControlSettings::default();
                },

                // reset gamepad
                Control::ResetKeyBindingsGamepad => {
                    // Currently resets everything on the controller to the default controls, this
                    // should be intended behavior
                    settings.controller = ControllerSettings::default();
                },

                // change gamepad button bindings
                Control::ChangeBindingGamepadButton(game_input) => {
                    global_state.window.set_remapping_mode(
                        crate::window::RemappingMode::RemapGamepadButtons(game_input),
                    );
                },
                Control::RemoveBindingGamepadButton(game_input) => {
                    settings.controller.remove_button_binding(game_input);
                },

                // change gamepad menu button bindings
                Control::ChangeBindingGamepadMenu(menu_input) => {
                    global_state.window.set_remapping_mode(
                        crate::window::RemappingMode::RemapGamepadMenu(menu_input),
                    );
                },
                Control::RemoveBindingGamepadMenu(menu_input) => {
                    settings.controller.remove_menu_binding(menu_input);
                },

                // change gamepad layer bindings
                Control::ChangeBindingGamepadLayer(game_input) => {
                    global_state.window.set_remapping_mode(
                        crate::window::RemappingMode::RemapGamepadLayers(game_input),
                    );
                },
                Control::RemoveBindingGamepadLayer(layer_input) => {
                    settings.controller.remove_layer_binding(layer_input);
                },
                Control::GameLayerMod1(glm1) => {
                    global_state.window.gamelayer_mod1 = glm1;
                },
                Control::GameLayerMod2(glm2) => {
                    global_state.window.gamelayer_mod2 = glm2;
                },
            },
            SettingsChange::Gamepad(gamepad_change) => match gamepad_change {},
            SettingsChange::Gameplay(gameplay_change) => {
                let window = &mut global_state.window;
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
                    Gameplay::AdjustWalkingSpeed(speed) => {
                        settings.gameplay.walking_speed = speed;
                    },
                    Gameplay::ToggleControllerYInvert(controller_y_inverted) => {
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
                    Gameplay::ChangeWalkingSpeedBehavior(behavior) => {
                        settings.gameplay.walking_speed_behavior = behavior;
                    },
                    Gameplay::ChangeCameraClampBehavior(behavior) => {
                        settings.gameplay.camera_clamp_behavior = behavior;
                    },
                    Gameplay::ChangeZoomLockBehavior(state) => {
                        settings.gameplay.zoom_lock_behavior = state;
                    },
                    Gameplay::ChangeStopAutoWalkOnInput(state) => {
                        settings.gameplay.stop_auto_walk_on_input = state;
                    },
                    Gameplay::ChangeAutoCamera(state) => {
                        settings.gameplay.auto_camera = state;
                    },
                    Gameplay::ChangeBowZoom(state) => {
                        settings.gameplay.bow_zoom = state;
                    },
                    Gameplay::ChangeZoomLock(state) => {
                        settings.gameplay.zoom_lock = state;
                    },
                    Gameplay::AdjustAimOffsetX(offset) => {
                        settings.gameplay.aim_offset_x = offset;
                    },
                    Gameplay::AdjustAimOffsetY(offset) => {
                        settings.gameplay.aim_offset_y = offset;
                    },
                    Gameplay::ResetGameplaySettings => {
                        // Reset Gameplay Settings
                        settings.gameplay = GameplaySettings::default();
                        // Reset Controller Settings
                        settings.controller = ControllerSettings::default();
                        // Pan Sensitivity
                        window.pan_sensitivity = settings.gameplay.pan_sensitivity;
                        // Zoom Sensitivity
                        window.zoom_sensitivity = settings.gameplay.zoom_sensitivity;
                        // Invert Scroll Zoom
                        window.zoom_inversion = settings.gameplay.zoom_inversion;
                        // Invert Mouse Y Axis
                        window.mouse_y_inversion = settings.gameplay.mouse_y_inversion;
                    },
                    Gameplay::ChangeShowAllRecipes(state) => {
                        settings.gameplay.show_all_recipes = state;
                    },
                }
            },
            SettingsChange::Graphics(graphics_change) => {
                let mut change_preset = false;

                match graphics_change {
                    Graphics::AdjustTerrainViewDistance(terrain_vd) => {
                        adjust_terrain_view_distance(terrain_vd, settings, session_state)
                    },
                    Graphics::AdjustEntityViewDistance(entity_vd) => {
                        adjust_entity_view_distance(entity_vd, settings, session_state)
                    },
                    Graphics::AdjustLodDistance(lod_distance) => {
                        session_state
                            .client
                            .borrow_mut()
                            .set_lod_distance(lod_distance);

                        settings.graphics.lod_distance = lod_distance;
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
                    Graphics::ChangeMaxBackgroundFPS(fps) => {
                        settings.graphics.max_background_fps = fps;
                    },
                    Graphics::ChangeFOV(new_fov) => {
                        settings.graphics.fov = new_fov;
                        session_state.scene.camera_mut().set_fov_deg(new_fov);
                        session_state
                            .scene
                            .camera_mut()
                            .compute_dependents(&session_state.client.borrow().state().terrain());
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
                        change_render_mode(*new_render_mode, &mut global_state.window, settings);
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
                    Graphics::ToggleWeaponTrailsEnabled(weapon_trails_enabled) => {
                        settings.graphics.weapon_trails_enabled = weapon_trails_enabled;
                    },
                    Graphics::ResetGraphicsSettings => {
                        settings.graphics = GraphicsSettings::default();
                        change_preset = true;
                        // Fullscreen mode
                        global_state
                            .window
                            .set_fullscreen_mode(settings.graphics.fullscreen);
                        // Window size
                        global_state
                            .window
                            .set_size(settings.graphics.window.size.into());
                    },
                    Graphics::ChangeGraphicsSettings(f) => {
                        settings.graphics = f(settings.graphics.clone());
                        change_preset = true;
                    },
                }

                if change_preset {
                    let graphics = &settings.graphics;
                    // View distance
                    client_set_view_distance(settings, session_state);
                    // FOV
                    session_state.scene.camera_mut().set_fov_deg(graphics.fov);
                    session_state
                        .scene
                        .camera_mut()
                        .compute_dependents(&session_state.client.borrow().state().terrain());
                    // LoD
                    session_state.scene.lod.set_detail(graphics.lod_detail);
                    // LoD distance
                    session_state
                        .client
                        .borrow_mut()
                        .set_lod_distance(graphics.lod_distance);
                    // Render mode
                    global_state
                        .window
                        .renderer_mut()
                        .set_render_mode(graphics.render_mode.clone())
                        .unwrap();
                }
            },
            SettingsChange::Interface(interface_change) => {
                match interface_change {
                    Interface::Sct(sct) => {
                        settings.interface.sct = sct;
                    },
                    Interface::SctRoundDamage(sct_round_damage) => {
                        settings.interface.sct_damage_rounding = sct_round_damage;
                    },
                    Interface::SctDamageAccumDuration(sct_dmg_accum_duration) => {
                        settings.interface.sct_dmg_accum_duration = sct_dmg_accum_duration;
                    },
                    Interface::SctIncomingDamage(sct_inc_dmg) => {
                        settings.interface.sct_inc_dmg = sct_inc_dmg;
                    },
                    Interface::SctIncomingDamageAccumDuration(sct_inc_dmg_accum_duration) => {
                        settings.interface.sct_inc_dmg_accum_duration = sct_inc_dmg_accum_duration;
                    },
                    Interface::SpeechBubbleSelf(sbdm) => {
                        settings.interface.speech_bubble_self = sbdm;
                    },
                    Interface::SpeechBubbleDarkMode(sbdm) => {
                        settings.interface.speech_bubble_dark_mode = sbdm;
                    },
                    Interface::SpeechBubbleIcon(sbi) => {
                        settings.interface.speech_bubble_icon = sbi;
                    },
                    Interface::ToggleDebug(toggle_debug) => {
                        settings.interface.toggle_debug = toggle_debug;
                    },
                    Interface::ToggleHitboxes(toggle_hitboxes) => {
                        settings.interface.toggle_hitboxes = toggle_hitboxes;
                    },
                    Interface::ToggleChat(toggle_chat) => {
                        settings.interface.toggle_chat = toggle_chat;
                    },
                    Interface::ToggleTips(loading_tips) => {
                        settings.interface.loading_tips = loading_tips;
                    },
                    Interface::ToggleHotkeyHints(toggle_hotkey_hints) => {
                        settings.interface.toggle_hotkey_hints = toggle_hotkey_hints;
                    },
                    Interface::CrosshairTransp(crosshair_opacity) => {
                        settings.interface.crosshair_opacity = crosshair_opacity;
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
                    Interface::ToggleAlwaysShowBars(always_show_bars) => {
                        settings.interface.always_show_bars = always_show_bars;
                    },
                    Interface::TogglePoiseBar(enable_poise_bar) => {
                        settings.interface.enable_poise_bar = enable_poise_bar;
                    },
                    Interface::ToggleHealthPrefixes(use_health_prefixes) => {
                        settings.interface.use_health_prefixes = use_health_prefixes;
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
                    Interface::MinimapScale(minimap_scale) => {
                        settings.interface.minimap_scale = minimap_scale;
                    },
                    Interface::MinimapColoredPlayerMarker(state) => {
                        settings.interface.minimap_colored_player_marker = state;
                    },
                    Interface::MapZoom(map_zoom) => {
                        settings.interface.map_zoom = map_zoom;
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
                    Interface::MapShowBridges(map_show_bridges) => {
                        settings.interface.map_show_bridges = map_show_bridges;
                    },
                    Interface::MapShowGliderCourses(map_show_glider_courses) => {
                        settings.interface.map_show_glider_courses = map_show_glider_courses;
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
                    Interface::MapShowQuests(map_show_quests) => {
                        settings.interface.map_show_quests = map_show_quests;
                    },
                    Interface::MapShowBiomes(map_show_biomes) => {
                        settings.interface.map_show_biomes = map_show_biomes;
                    },
                    Interface::MapShowVoxelMap(map_show_voxel_map) => {
                        settings.interface.map_show_voxel_map = map_show_voxel_map;
                    },
                    Interface::AccumExperience(accum_experience) => {
                        settings.interface.accum_experience = accum_experience;
                    },
                    Interface::SlotsUsePrefixes(slots_use_prefixes) => {
                        settings.interface.slots_use_prefixes = slots_use_prefixes;
                        session_state.hud.set_slots_use_prefixes(slots_use_prefixes);
                    },
                    Interface::SlotsPrefixSwitchPoint(slots_prefix_switch_point) => {
                        settings.interface.slots_prefix_switch_point = slots_prefix_switch_point;
                        session_state
                            .hud
                            .set_slots_prefix_switch_point(slots_prefix_switch_point);
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
                    Interface::RowBackgroundOpacity(opacity) => {
                        settings.interface.row_background_opacity = opacity;
                    },
                }
            },
            SettingsChange::Inventory(inventory_change) => match inventory_change {
                Inventory::ChangeSortOrder(sort_order) => {
                    settings.inventory.sort_order = sort_order
                },
            },
            SettingsChange::Language(language_change) => match language_change {
                Language::ChangeLanguage(new_language) => {
                    settings.language.selected_language = new_language.language_identifier;
                    global_state.i18n =
                        LocalizationHandle::load_expect(&settings.language.selected_language);
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
                Language::ToggleSendToServer(share) => {
                    settings.language.send_to_server = share;
                },
            },
            SettingsChange::Networking(networking_change) => match networking_change {
                Networking::AdjustTerrainViewDistance(terrain_vd) => {
                    adjust_terrain_view_distance(terrain_vd, settings, session_state)
                },
                Networking::AdjustEntityViewDistance(entity_vd) => {
                    adjust_entity_view_distance(entity_vd, settings, session_state)
                },
                Networking::ChangePlayerPhysicsBehavior {
                    server_authoritative,
                } => {
                    settings.networking.player_physics_behavior = server_authoritative;
                    session_state
                        .client
                        .borrow_mut()
                        .request_player_physics(server_authoritative);
                },
                Networking::ToggleLossyTerrainCompression(lossy_terrain_compression) => {
                    settings.networking.lossy_terrain_compression = lossy_terrain_compression;
                    session_state
                        .client
                        .borrow_mut()
                        .request_lossy_terrain_compression(lossy_terrain_compression);
                },
                #[cfg(feature = "discord")]
                Networking::ToggleDiscordIntegration(enabled) => {
                    use crate::discord::Discord;

                    settings.networking.enable_discord_integration = enabled;
                    if enabled {
                        global_state.discord = Discord::start(&global_state.tokio_runtime);

                        #[cfg(feature = "singleplayer")]
                        let singleplayer = global_state.singleplayer.is_running();
                        #[cfg(not(feature = "singleplayer"))]
                        let singleplayer = false;

                        if singleplayer {
                            global_state.discord.join_singleplayer();
                        } else {
                            global_state.discord.join_server(
                                session_state.client.borrow().server_info().name.clone(),
                            );
                        }
                    } else {
                        global_state.discord.clear_activity();
                        global_state.discord = Discord::Inactive;
                    }
                },
            },
            SettingsChange::Accessibility(accessibility_change) => match accessibility_change {
                Accessibility::ChangeRenderMode(new_render_mode) => {
                    change_render_mode(*new_render_mode, &mut global_state.window, settings);
                },
                Accessibility::SetSubtitles(enabled) => {
                    global_state.settings.audio.subtitles = enabled;
                    global_state.audio.set_subtitles(enabled);
                },
            },
        }
        global_state
            .settings
            .save_to_file_warn(&global_state.config_dir);
    }
}

use crate::settings::Settings;

pub fn change_render_mode(
    new_render_mode: RenderMode,
    window: &mut Window,
    settings: &mut Settings,
) {
    // Do this first so if it crashes the setting isn't saved :)
    window
        .renderer_mut()
        .set_render_mode(new_render_mode.clone())
        .unwrap();
    settings.graphics.render_mode = new_render_mode;
}

fn adjust_terrain_view_distance(
    terrain_vd: u32,
    settings: &mut Settings,
    session_state: &mut SessionState,
) {
    settings.graphics.terrain_view_distance = terrain_vd;
    client_set_view_distance(settings, session_state);
}

fn adjust_entity_view_distance(
    entity_vd: u32,
    settings: &mut Settings,
    session_state: &mut SessionState,
) {
    settings.graphics.entity_view_distance = entity_vd;
    client_set_view_distance(settings, session_state);
}

fn client_set_view_distance(settings: &Settings, session_state: &mut SessionState) {
    let view_distances = common::ViewDistances {
        terrain: settings.graphics.terrain_view_distance,
        entity: settings.graphics.entity_view_distance,
    };
    session_state
        .client
        .borrow_mut()
        .set_view_distances(view_distances);
}
