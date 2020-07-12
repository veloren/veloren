use super::{img_ids::Imgs, Show, TEXT_COLOR, TEXT_COLOR_3, TEXT_COLOR_GREY, UI_MAIN};

use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use client::{self, Client};
use common::{
    comp::Stats,
    sync::{Uid, WorldSyncExt},
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::WorldExt;
use std::time::Instant;

widget_ids! {
    pub struct Ids {
        bg,
        title,
        close,
        btn_bg,
        btn_friend,
        btn_leader,
        btn_link,
        btn_kick,
        btn_leave,
        members[],
        invite_bubble,
        bubble_frame,
        btn_accept,
        btn_decline,
        // TEST
        test_leader,
        test_member1,
        test_member2,
        test_member3,
        test_member4,
        test_member5,
    }
}

pub struct State {
    ids: Ids,
    // Holds the time when selection is made since this selection can be overriden
    // by selecting an entity in-game
    selected_uid: Option<(Uid, Instant)>,
    // Selected group member
    selected_member: Option<Uid>,
}

#[derive(WidgetCommon)]
pub struct Group<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    selected_entity: Option<(specs::Entity, Instant)>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Group<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        selected_entity: Option<(specs::Entity, Instant)>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            fonts,
            localized_strings,
            selected_entity,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    Accept,
    Reject,
    Kick(Uid),
    LeaveGroup,
    AssignLeader(Uid),
}

impl<'a> Widget for Group<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
            selected_uid: None,
            selected_member: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        let player_leader = true;
        let in_group = true;
        let open_invite = false;

        if in_group || open_invite {
            // Frame
            Rectangle::fill_with([220.0, 230.0], color::Color::Rgba(0.0, 0.0, 0.0, 0.8))
                .bottom_left_with_margins_on(ui.window, 220.0, 10.0)
                .set(state.ids.bg, ui);
                if open_invite { 
                    // yellow animated border
                }
        }

        // Buttons
        if in_group {
            Text::new("Group Name")
                .mid_top_with_margin_on(state.ids.bg, 2.0)
                .font_size(20)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.title, ui);
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .top_right_with_margins_on(state.ids.bg, 30.0, 5.0)
                .hover_image(self.imgs.button)
                .press_image(self.imgs.button)
                .label("Add to Friends")
                .label_color(TEXT_COLOR_GREY) // Change this when the friendslist is working
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))                
                .set(state.ids.btn_friend, ui)
                .was_clicked()
            {};
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_right_with_margins_on(state.ids.bg, 5.0, 5.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Leave Group")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))                
                .set(state.ids.btn_leave, ui)
                .was_clicked()
            {};
            // Group leader functions
            if player_leader {                       
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .mid_bottom_with_margin_on(state.ids.btn_friend, -27.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Assign Leader")
                .label_color(TEXT_COLOR) // Grey when no player is selected
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))                
                .set(state.ids.btn_leader, ui)
                .was_clicked()
            {};            
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)                
                .mid_bottom_with_margin_on(state.ids.btn_leader, -27.0)
                .hover_image(self.imgs.button)
                .press_image(self.imgs.button)
                .label("Link Group")
                .label_color(TEXT_COLOR_GREY) // Change this when the linking is working
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))                
                .set(state.ids.btn_link, ui)
                .was_clicked()
            {};            
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .mid_bottom_with_margin_on(state.ids.btn_link, -27.0)
                .down_from(state.ids.btn_link, 5.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Kick")
                .label_color(TEXT_COLOR) // Grey when no player is selected
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))                
                .set(state.ids.btn_kick, ui)
                .was_clicked()
            {};
        }            
            // Group Members, only character names, cut long names when they exceed the button size
            // TODO Insert loop here
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .top_left_with_margins_on(state.ids.bg, 30.0, 5.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Leader") // Grey when no player is selected
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_leader, ui)
                .was_clicked()
            {
                //Select the Leader
            };
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .down_from(state.ids.test_leader, 10.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Other Player")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_member1, ui)
                .was_clicked()
            {
                // Select the group member
            };
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .down_from(state.ids.test_member1, 10.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Other Player")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_member2, ui)
                .was_clicked()
            {
                // Select the group member
            };
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .down_from(state.ids.test_member2, 10.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Other Player")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_member3, ui)
                .was_clicked()
            {
                // Select the group member
            };
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .down_from(state.ids.test_member3, 10.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Other Player")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_member4, ui)
                .was_clicked()
            {
                // Select the group member
            };
            if Button::image(self.imgs.nothing) // if selected self.imgs.selection
                .w_h(90.0, 22.0)
                .down_from(state.ids.test_member4, 10.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label("Other Player")
                .label_color(TEXT_COLOR) 
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))                
                .set(state.ids.test_member5, ui)
                .was_clicked()
            {
                // Select the group member
            };
            // Maximum of 6 Players/Npcs per Group
            // Player pets count as group members, too. They are not counted into the maximum group size.

        }
        if open_invite {
            //self.show.group = true; Auto open group menu
            Text::new("Player wants to invite you!")
                .mid_top_with_margin_on(state.ids.bg, 20.0)
                .font_size(20)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.title, ui);
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_left_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button)
                .press_image(self.imgs.button)
                .label("[U] Accept")
                .label_color(TEXT_COLOR_GREY) // Change this when the friendslist is working
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))                
                .set(state.ids.btn_friend, ui)
                .was_clicked()
            {};
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_right_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button)
                .press_image(self.imgs.button)
                .label("[I] Decline")
                .label_color(TEXT_COLOR_GREY) // Change this when the friendslist is working
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))                
                .set(state.ids.btn_friend, ui)
                .was_clicked()
            {};

        }
        events
    }
}
