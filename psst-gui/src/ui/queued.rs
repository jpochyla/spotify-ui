use std::sync::Arc;

use crate::{
    cmd,
    data::{AppState, Nav, QueueEntry, QueueFields, RecommendationsRequest},
    ui::Vector,
    widget::{icons, Border, Empty, MyWidgetExt},
};

use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, List, Scroll},
    Data, Env, Lens, LensExt, LocalizedString, Menu, MenuItem, Widget, WidgetExt,
};
use druid_shell::Cursor;

use super::theme;

// Define a struct that includes index for QueueEntry
#[derive(Clone, Data, Lens)]
struct QueueEntryWithIndex {
    entry: QueueEntry,
    index: usize,
}

// Convert Vector<QueueEntry> to Vector<QueueEntryWithIndex>
fn queue_entries_with_index(entries: Vector<QueueEntry>) -> Vector<QueueEntryWithIndex> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, entry)| QueueEntryWithIndex { entry, index: i })
        .collect()
}

// Widget for the queue
pub fn queue_widget() -> impl Widget<AppState> {
    Either::new(
        |data: &AppState, _env: &Env| {
            data.config.window_size.width >= 700.0
                && data.config.show_queue_view
        },
        Flex::column()
            .with_child(queue_header_widget())
            .with_flex_child(
                Scroll::new(queue_list_widget())
                    .vertical()
                    .lens(AppState::added_queue.then(QueueFields::displayed_queue).map(
                        |entries| queue_entries_with_index(entries.clone()),
                        |_, _| (),
                    )),
                1.0,
            )
            .fix_width(185.0)
            .background(Border::Left.with_color(theme::GREY_500)),
        Empty,
    )
}

// Widget for the queue header
fn queue_header_widget() -> impl Widget<AppState> {
    Flex::row()
        .with_flex_child(
            Label::new("Queue")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_text_size(theme::TEXT_SIZE_LARGE)
                .center(),
            0.3,
        )
        .fix_height(32.0)
        .padding(theme::grid(1.0))
        .expand_width()
        .background(Border::Bottom.with_color(theme::GREY_500))
}

// Widget for displaying a list of queue entries with index
fn queue_list_widget() -> impl Widget<Vector<QueueEntryWithIndex>> {
    List::new(|| {
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(
                        Label::new(|item: &QueueEntryWithIndex, _env: &Env| {
                            item.entry.item.name().to_string()
                        })
                        .with_font(theme::UI_FONT_MEDIUM)
                        .with_line_break_mode(LineBreaking::Clip)
                        .expand_width(),
                    )
                    .with_spacer(2.0)
                    .with_child(
                        Label::new(|item: &QueueEntryWithIndex, _env: &Env| {
                            item.entry.item.artist().to_string()
                        })
                        .with_text_size(theme::TEXT_SIZE_SMALL)
                        .with_text_color(theme::PLACEHOLDER_COLOR)
                        .with_line_break_mode(LineBreaking::Clip)
                        .expand_width(),
                    )
                    .context_menu(|item: &QueueEntryWithIndex| {
                        queue_entry_context_menu(item.clone())
                    })
                    .on_click(|ctx, data, _| {
                        ctx.submit_command(cmd::SKIP_TO_PLACE_IN_QUEUE.with(data.index));
                    }),
                1.0,
            )
            .with_default_spacer()
            .with_child(
                icons::CLOSE_CIRCLE
                    .scale((16.0, 16.0))
                    .link()
                    .rounded(100.0)
                    .on_click(|ctx, data: &mut QueueEntryWithIndex, _| {
                        ctx.submit_command(cmd::REMOVE_FROM_QUEUE.with(data.index));
                    })
                    .with_cursor(Cursor::Pointer),
            )
            .padding(theme::grid(1.0))
            .link()
            .rounded(theme::BUTTON_BORDER_RADIUS)
    })
    .padding(theme::grid(1.0))
}

fn queue_entry_context_menu(item: QueueEntryWithIndex) -> Menu<AppState> {
    let mut menu = Menu::empty();

    menu = menu.entry(
        MenuItem::new("Remove from Queue").on_activate(move |ctx, _, _| {
            ctx.submit_command(cmd::REMOVE_FROM_QUEUE.with(item.index));
        }),
    );

    menu = menu.entry(MenuItem::new("Clear Queue").on_activate(move |ctx, _, _| {
        ctx.submit_command(cmd::CLEAR_QUEUE);
    }));

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-show-recommended")
                .with_placeholder("Show Similar Tracks"),
        )
        .command(cmd::NAVIGATE.with(Nav::Recommendations(Arc::new(
            RecommendationsRequest::for_track(crate::data::TrackId(item.entry.item.id())),
        )))),
    );

    menu
}
