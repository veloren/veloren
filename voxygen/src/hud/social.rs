use super::{img_ids::Imgs, Show, TEXT_COLOR, TEXT_COLOR_3, UI_MAIN};

use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use client::{self, Client};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    pub struct Ids {
        social_frame,
        social_close,
        social_title,
        frame,
        align,
        content_align,
        online_tab,
        friends_tab,
        faction_tab,
        online_title,
        online_no,
        scrollbar,
        friends_test,
        faction_test,
        player_names[],
    }
}

pub enum SocialTab {
    Online,
    Friends,
    Faction,
}

#[derive(WidgetCommon)]
pub struct Social<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Social<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    ChangeSocialTab(SocialTab),
}

impl<'a> Widget for Social<'a> {
    type Event = Vec<Event>;
    type State = Ids;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State { Ids::new(id_gen) }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            /* id, */ state: ids,
            ui,
            ..
        } = args;

        let mut events = Vec::new();

        Image::new(self.imgs.window_3)
            .top_left_with_margins_on(ui.window, 200.0, 25.0)
            .color(Some(UI_MAIN))
            .w_h(103.0 * 4.0, 122.0 * 4.0)
            .set(ids.social_frame, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(ids.social_frame, 0.0, 0.0)
            .set(ids.social_close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(&self.localized_strings.get("hud.social"))
            .mid_top_with_margin_on(ids.social_frame, 6.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .set(ids.social_title, ui);

        // Alignment
        Rectangle::fill_with([99.0 * 4.0, 112.0 * 4.0], color::TRANSPARENT)
            .mid_top_with_margin_on(ids.social_frame, 8.0 * 4.0)
            .set(ids.align, ui);
        // Content Alignment
        Rectangle::fill_with([94.0 * 4.0, 94.0 * 4.0], color::TRANSPARENT)
            .middle_of(ids.frame)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(ids.content_align, ui);
        Scrollbar::y_axis(ids.content_align)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(ids.scrollbar, ui);
        // Frame
        Image::new(self.imgs.social_frame)
            .w_h(99.0 * 4.0, 100.0 * 4.0)
            .mid_bottom_of(ids.align)
            .color(Some(UI_MAIN))
            .set(ids.frame, ui);

        // Online Tab

        if Button::image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .w_h(30.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button_hover
        })
        .press_image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button_press
        })
        .top_left_with_margins_on(ids.align, 4.0, 0.0)
        .label(&self.localized_strings.get("hud.social.online"))
        .label_font_size(self.fonts.cyri.scale(14))
        .label_font_id(self.fonts.cyri.conrod_id)
        .parent(ids.frame)
        .color(UI_MAIN)
        .label_color(TEXT_COLOR)
        .set(ids.online_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Online));
        }

        // Contents

        if let SocialTab::Online = self.show.social_tab {
            // TODO Needs to be a string sent from the server

            // Players list
            // TODO: this list changes infrequently enough that it should not have to be
            // recreated every frame
            let count = self.client.player_list.len();
            if ids.player_names.len() < count {
                ids.update(|ids| {
                    ids.player_names
                        .resize(count, &mut ui.widget_id_generator())
                })
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.social.play_online_fmt")
                    .replace("{nb_player}", &format!("{:?}", count)),
            )
            .top_left_with_margins_on(ids.content_align, -2.0, 7.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(ids.online_title, ui);
            for (i, (_, player_alias)) in self.client.player_list.iter().enumerate() {
                Text::new(player_alias)
                    .down(3.0)
                    .font_size(self.fonts.cyri.scale(15))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(ids.player_names[i], ui);
            }
        }

        // Friends Tab

        if Button::image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .w_h(30.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .press_image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .right_from(ids.online_tab, 0.0)
        .label(&self.localized_strings.get("hud.social.friends"))
        .label_font_size(self.fonts.cyri.scale(14))
        .label_font_id(self.fonts.cyri.conrod_id)
        .parent(ids.frame)
        .color(UI_MAIN)
        .label_color(TEXT_COLOR_3)
        .set(ids.friends_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Friends));
        }

        // Contents

        if let SocialTab::Friends = self.show.social_tab {
            Text::new(&self.localized_strings.get("hud.social.not_yet_available"))
                .middle_of(ids.content_align)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR_3)
                .set(ids.friends_test, ui);
        }

        // Faction Tab
        let button_img = if let SocialTab::Faction = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        };
        if Button::image(button_img)
            .w_h(30.0 * 4.0, 12.0 * 4.0)
            .right_from(ids.friends_tab, 0.0)
            .label(&self.localized_strings.get("hud.social.faction"))
            .parent(ids.frame)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_font_id(self.fonts.cyri.conrod_id)
            .color(UI_MAIN)
            .label_color(TEXT_COLOR_3)
            .set(ids.faction_tab, ui)
            .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Faction));
        }

        // Contents

        if let SocialTab::Faction = self.show.social_tab {
            Text::new(&self.localized_strings.get("hud.social.not_yet_available"))
                .middle_of(ids.content_align)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR_3)
                .set(ids.faction_test, ui);
        }

        events
    }
}
