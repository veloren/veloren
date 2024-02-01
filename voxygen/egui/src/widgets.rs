use egui::{ScrollArea, Ui, Vec2, WidgetText};

pub(crate) fn filterable_list(
    ui: &mut Ui,
    list_items: &[String],
    search_text: &str,
    selected_index: &mut usize,
) {
    let scroll_area = ScrollArea::vertical();
    scroll_area.show(ui, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
        let search_text = search_text.to_lowercase();
        for (i, list_item) in list_items.iter().enumerate().filter_map(|(i, list_item)| {
            if search_text.is_empty() || list_item.to_lowercase().contains(&search_text) {
                Some((i, list_item))
            } else {
                None
            }
        }) {
            if ui
                .selectable_label(i == *selected_index, list_item)
                .clicked()
            {
                *selected_index = i;
            };
        }
    });
}

pub(crate) fn two_col_row(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    content: impl Into<WidgetText>,
) {
    ui.label(label);
    ui.label(content);
    ui.end_row();
}
