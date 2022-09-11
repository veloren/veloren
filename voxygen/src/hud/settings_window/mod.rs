mod chat;
mod controls;
mod gameplay;
mod interface;
mod language;
mod networking;
mod sound;
mod video;

use crate::{
    hud::{img_ids::Imgs, Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN},
    session::settings_change::SettingsChange,
    ui::fonts::Fonts,
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

use strum::{EnumIter, IntoEnumIterator};

widget_ids! {
    struct Ids {
        frame,
        settings_bg,
        tabs_align,
        icon,
        settings_close,
        settings_title,
        settings_content_align,

        tabs[],
        interface,
        gameplay,
        controls,
        video,
        sound,
        language,
        chat,
        networking,
    }
}

const RESET_BUTTONS_HEIGHT: f64 = 34.0;
const RESET_BUTTONS_WIDTH: f64 = 155.0;

#[derive(Debug, EnumIter, PartialEq, Eq)]
pub enum SettingsTab {
    Interface,
    Chat,
    Video,
    Sound,
    Gameplay,
    Controls,
    Lang,
    Networking,
}
impl SettingsTab {
    fn name_key(&self) -> &str {
        match self {
            SettingsTab::Interface => "common-interface",
            SettingsTab::Chat => "common-chat",
            SettingsTab::Gameplay => "common-gameplay",
            SettingsTab::Controls => "common-controls",
            SettingsTab::Video => "common-video",
            SettingsTab::Sound => "common-sound",
            SettingsTab::Lang => "common-languages",
            SettingsTab::Networking => "common-networking",
        }
    }

    fn title_key(&self) -> &str {
        match self {
            SettingsTab::Interface => "common-interface_settings",
            SettingsTab::Chat => "common-chat_settings",
            SettingsTab::Gameplay => "common-gameplay_settings",
            SettingsTab::Controls => "common-controls_settings",
            SettingsTab::Video => "common-video_settings",
            SettingsTab::Sound => "common-sound_settings",
            SettingsTab::Lang => "common-language_settings",
            SettingsTab::Networking => "common-networking_settings",
        }
    }
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    global_state: &'a GlobalState,
    show: &'a Show,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    server_view_distance_limit: Option<u32>,
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
        server_view_distance_limit: Option<u32>,
        fps: f32,
    ) -> Self {
        Self {
            global_state,
            show,
            imgs,
            fonts,
            localized_strings,
            server_view_distance_limit,
            fps,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ChangeTab(SettingsTab),
    Close,
    SettingsChange(SettingsChange),
    ChangeChatSettingsTab(Option<usize>),
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("SettingsWindow::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let tab_font_scale = 18;

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
            .set(state.ids.settings_content_align, ui);

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
        Text::new(
            &self
                .localized_strings
                .get_msg(self.show.settings_tab.title_key()),
        )
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

        // Tabs
        if state.ids.tabs.len() < SettingsTab::iter().len() {
            state.update(|s| {
                s.ids
                    .tabs
                    .resize(SettingsTab::iter().len(), &mut ui.widget_id_generator())
            });
        }
        for (i, settings_tab) in SettingsTab::iter().enumerate() {
            let tab_name = self.localized_strings.get_msg(settings_tab.name_key());
            let mut button = Button::image(if self.show.settings_tab == settings_tab {
                self.imgs.selection
            } else {
                self.imgs.nothing
            })
            .w_h(230.0, 48.0)
            .hover_image(self.imgs.selection_hover)
            .press_image(self.imgs.selection_press)
            .image_color(color::rgba(1.0, 0.82, 0.27, 1.0))
            .label(&tab_name)
            .label_font_size(self.fonts.cyri.scale(tab_font_scale))
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_color(TEXT_COLOR);

            button = if i == 0 {
                button.mid_top_with_margin_on(state.ids.tabs_align, 28.0)
            } else {
                button.down_from(state.ids.tabs[i - 1], 0.0)
            };

            if button.set(state.ids.tabs[i], ui).was_clicked() {
                events.push(Event::ChangeTab(settings_tab));
            }
        }

        // Content Area
        let global_state = self.global_state;
        let show = self.show;
        let imgs = self.imgs;
        let fonts = self.fonts;
        let localized_strings = self.localized_strings;
        match self.show.settings_tab {
            SettingsTab::Interface => {
                for change in
                    interface::Interface::new(global_state, show, imgs, fonts, localized_strings)
                        .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                        .wh_of(state.ids.settings_content_align)
                        .set(state.ids.interface, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Chat => {
                for event in
                    chat::Chat::new(global_state, self.show, imgs, fonts, localized_strings)
                        .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                        .wh_of(state.ids.settings_content_align)
                        .set(state.ids.chat, ui)
                {
                    match event {
                        chat::Event::ChatChange(change) => {
                            events.push(Event::SettingsChange(change.into()));
                        },
                        chat::Event::ChangeChatSettingsTab(index) => {
                            events.push(Event::ChangeChatSettingsTab(index));
                        },
                    }
                }
            },
            SettingsTab::Gameplay => {
                for change in gameplay::Gameplay::new(global_state, imgs, fonts, localized_strings)
                    .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                    .wh_of(state.ids.settings_content_align)
                    .set(state.ids.gameplay, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Controls => {
                for change in controls::Controls::new(global_state, imgs, fonts, localized_strings)
                    .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                    .wh_of(state.ids.settings_content_align)
                    .set(state.ids.controls, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Video => {
                for change in video::Video::new(
                    global_state,
                    imgs,
                    fonts,
                    localized_strings,
                    self.server_view_distance_limit,
                    self.fps,
                )
                .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                .wh_of(state.ids.settings_content_align)
                .set(state.ids.video, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Sound => {
                for change in sound::Sound::new(global_state, imgs, fonts, localized_strings)
                    .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                    .wh_of(state.ids.settings_content_align)
                    .set(state.ids.sound, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Lang => {
                for change in language::Language::new(global_state, imgs, fonts, localized_strings)
                    .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                    .wh_of(state.ids.settings_content_align)
                    .set(state.ids.language, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
            SettingsTab::Networking => {
                for change in networking::Networking::new(
                    global_state,
                    imgs,
                    fonts,
                    localized_strings,
                    self.server_view_distance_limit,
                )
                .top_left_with_margins_on(state.ids.settings_content_align, 0.0, 0.0)
                .wh_of(state.ids.settings_content_align)
                .set(state.ids.networking, ui)
                {
                    events.push(Event::SettingsChange(change.into()));
                }
            },
        }

        events
    }
}
