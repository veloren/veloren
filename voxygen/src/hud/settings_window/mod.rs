mod controls;
mod gameplay;
mod interface;
mod language;
mod sound;
mod video;

use crate::{
    hud::{
        img_ids::Imgs, BarNumbers, BuffPosition, CrosshairType, PressBehavior, ShortcutNumbers,
        Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
    },
    i18n::{LanguageMetadata, Localization},
    render::RenderMode,
    settings::Fps,
    ui::fonts::Fonts,
    window::{FullScreenSettings, GameInput},
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        frame,
        tabs_align,
        icon,
        settings_content,
        settings_content_r,
        settings_close,
        settings_title,
        gameplay,
        controls,
        interface,
        settings_bg,
        sound,
        video,
        language,

        interface_window,
        gameplay_window,
        controls_window,
        video_window,
        sound_window,
        language_window,
    }
}

const RESET_BUTTONS_HEIGHT: f64 = 34.0;
const RESET_BUTTONS_WIDTH: f64 = 155.0;

pub enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
    Lang,
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    global_state: &'a GlobalState,
    show: &'a Show,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    fps: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        show: &'a Show,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        fps: f32,
    ) -> Self {
        Self {
            global_state,
            show,
            imgs,
            fonts,
            localized_strings,
            fps,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ToggleHelp,
    ToggleDebug,
    ToggleTips(bool),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    BuffPosition(BuffPosition),
    ChangeTab(SettingsTab),
    Close,
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustCameraClamp(u32),
    ToggleZoomInvert(bool),
    ToggleMouseYInvert(bool),
    ToggleControllerYInvert(bool),
    ToggleSmoothPan(bool),
    AdjustViewDistance(u32),
    AdjustSpriteRenderDistance(u32),
    AdjustFigureLoDRenderDistance(u32),
    AdjustFOV(u16),
    AdjustLodDetail(u32),
    AdjustGamma(f32),
    AdjustExposure(f32),
    AdjustAmbiance(f32),
    AdjustWindowSize([u16; 2]),
    ChangeFullscreenMode(FullScreenSettings),
    ToggleParticlesEnabled(bool),
    ChangeRenderMode(Box<RenderMode>),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    //ChangeAudioDevice(String),
    MaximumFPS(Fps),
    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    UiScale(ScaleChange),
    ChatTransp(f32),
    ChatCharName(bool),
    Sct(bool),
    SctPlayerBatch(bool),
    SctDamageBatch(bool),
    SpeechBubbleDarkMode(bool),
    SpeechBubbleIcon(bool),
    ChangeLanguage(Box<LanguageMetadata>),
    ChangeBinding(GameInput),
    ResetInterfaceSettings,
    ResetGameplaySettings,
    ResetKeyBindings,
    ResetGraphicsSettings,
    ResetAudioSettings,
    ChangeFreeLookBehavior(PressBehavior),
    ChangeAutoWalkBehavior(PressBehavior),
    ChangeCameraClampBehavior(PressBehavior),
    ChangeStopAutoWalkOnInput(bool),
    ChangeAutoCamera(bool),
}

#[derive(Clone)]
pub enum ScaleChange {
    ToAbsolute,
    ToRelative,
    Adjust(f64),
}

impl<'a> Widget for SettingsWindow<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let tab_font_scale = 18;

        //let mut xp_bar = self.global_state.settings.interface.xp_bar;

        // Frame
        Image::new(self.imgs.settings_bg)
            .w_h(1052.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.ids.settings_bg, ui);

        Image::new(self.imgs.settings_frame)
            .w_h(1052.0, 886.0)
            .middle_of(state.ids.settings_bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame, ui);

        // Content Alignment
        Rectangle::fill_with([814.0, 834.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, 46.0, 2.0)
            .set(state.ids.settings_content, ui);
        Rectangle::fill_with([814.0 / 2.0, 834.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.settings_content, 0.0, 0.0)
            .set(state.ids.settings_content_r, ui);

        // Tabs Content Alignment
        Rectangle::fill_with([232.0, 814.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 44.0, 2.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.tabs_align, ui);

        // Icon
        Image::new(self.imgs.settings)
            .w_h(29.0 * 1.5, 25.0 * 1.5)
            .top_left_with_margins_on(state.ids.frame, 2.0, 1.0)
            .set(state.ids.icon, ui);
        // Title
        Text::new(match self.show.settings_tab {
            SettingsTab::Interface => self.localized_strings.get("common.interface_settings"),
            SettingsTab::Gameplay => self.localized_strings.get("common.gameplay_settings"),
            SettingsTab::Controls => self.localized_strings.get("common.controls_settings"),
            SettingsTab::Video => self.localized_strings.get("common.video_settings"),
            SettingsTab::Sound => self.localized_strings.get("common.sound_settings"),
            SettingsTab::Lang => self.localized_strings.get("common.language_settings"),
        })
        .mid_top_with_margin_on(state.ids.frame, 3.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(29))
        .color(TEXT_COLOR)
        .set(state.ids.settings_title, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.settings_close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // 1) Interface Tab -------------------------------
        if Button::image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .mid_top_with_margin_on(state.ids.tabs_align, 28.0)
        .label(&self.localized_strings.get("common.interface"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.interface, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Interface));
        }

        // Contents Left Side
        if let SettingsTab::Interface = self.show.settings_tab {
            for event in interface::Interface::new(
                self.global_state,
                self.show,
                self.imgs,
                self.fonts,
                self.localized_strings,
            )
            .parent(state.ids.settings_content)
            .top_left()
            .wh_of(state.ids.settings_content)
            .set(state.ids.interface_window, ui)
            {
                events.push(event);
            }
        }

        // 2) Gameplay Tab --------------------------------
        if Button::image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.interface, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.gameplay"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.gameplay, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Gameplay));
        }

        // Contents
        if let SettingsTab::Gameplay = self.show.settings_tab {
            for event in gameplay::Gameplay::new(
                self.global_state,
                self.imgs,
                self.fonts,
                self.localized_strings,
            )
            .parent(state.ids.settings_content)
            .top_left()
            .wh_of(state.ids.settings_content)
            .set(state.ids.gameplay_window, ui)
            {
                events.push(event);
            }
        }

        // 3) Controls Tab --------------------------------
        if Button::image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.gameplay, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.controls"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.controls, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Controls));
        }

        // Contents
        if let SettingsTab::Controls = self.show.settings_tab {
            for event in controls::Controls::new(
                self.global_state,
                self.imgs,
                self.fonts,
                self.localized_strings,
            )
            .parent(state.ids.settings_content)
            .top_left()
            .wh_of(state.ids.settings_content)
            .set(state.ids.controls_window, ui)
            {
                events.push(event);
            }
        }

        // 4) Video Tab -----------------------------------
        if Button::image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.controls, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.video"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.video, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Video));
        }

        // Contents
        if let SettingsTab::Video = self.show.settings_tab {
            for event in video::Video::new(
                self.global_state,
                self.imgs,
                self.fonts,
                self.localized_strings,
                self.fps,
            )
            .parent(state.ids.settings_content)
            .top_left()
            .wh_of(state.ids.settings_content)
            .set(state.ids.video_window, ui)
            {
                events.push(event);
            }
        }

        // 5) Sound Tab -----------------------------------
        if Button::image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.video, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.sound"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.sound, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Sound));
        }

        // Contents
        if let SettingsTab::Sound = self.show.settings_tab {
            for event in sound::Sound::new(
                self.global_state,
                self.imgs,
                self.fonts,
                self.localized_strings,
            )
            .parent(state.ids.settings_content)
            .top_left()
            .wh_of(state.ids.settings_content)
            .set(state.ids.sound_window, ui)
            {
                events.push(event);
            }
        }

        // 5) Languages Tab -----------------------------------
        if Button::image(if let SettingsTab::Lang = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.sound, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.languages"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.language, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Lang));
        }

        // Contents
        if let SettingsTab::Lang = self.show.settings_tab {
            for event in language::Language::new(self.global_state, self.imgs, self.fonts)
                .parent(state.ids.settings_content)
                .top_left()
                .wh_of(state.ids.settings_content)
                .set(state.ids.language_window, ui)
            {
                events.push(event);
            }
        };

        events
    }
}
