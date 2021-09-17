use druid::{
    commands,
    widget::{Either, Flex, Label},
    Widget, WidgetExt, Data, 
};

use crate::{
    cmd,
    data::{AppState, UserProfile},
    webapi::WebApi,
    widget::{Async, Empty, MyWidgetExt, icons, icons::SvgIcon}
};

use super::theme;

pub fn user_widget() -> impl Widget<AppState> {
    let is_connected = Either::new(
        // TODO: Avoid the locking here.
        |state: &AppState, _| state.session.is_connected(),
        Label::new("Connected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
        Label::new("Disconnected")
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_size(theme::TEXT_SIZE_SMALL),
    );

    let user_profile = Async::new(
        || Empty,
        || {
            Label::raw()
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(UserProfile::display_name)
        },
        || Empty,
    )
    .on_deferred(|_| WebApi::global().get_user_profile())
    .lens(AppState::user_profile);

    Flex::row()
        .with_child(
            Flex::column()
            .with_child(is_connected)
            .with_child(user_profile)
            .padding((theme::grid(2.0), theme::grid(2.0)))
        )
        .with_child(preferences_widget(&icons::PREFERENCES))
}  

fn preferences_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(3.0), theme::grid(3.0)))
        .padding(theme::grid(2.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| ctx.submit_command(commands::SHOW_PREFERENCES))
}
