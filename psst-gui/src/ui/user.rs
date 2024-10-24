use druid::{
    commands,
    widget::{Either, Flex, Label},
    Data, LensExt, Selector, Widget, WidgetExt,
};

use crate::{
    data::{AppState, Library, UserProfile},
    webapi::WebApi,
    widget::{
        icons::{self, SvgIcon},
        Async, Empty, MyWidgetExt,
    },
};

use super::theme;

pub const LOAD_PROFILE: Selector = Selector::new("app.user.load-profile");

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
    .lens(AppState::library.then(Library::user_profile.in_arc()))
    .on_command_async(
        LOAD_PROFILE,
        |_| WebApi::global().get_user_profile(),
        |_, data, d| data.with_library_mut(|l| l.user_profile.defer(d)),
        |_, data, r| data.with_library_mut(|l| l.user_profile.update(r)),
    );

    Flex::row()
        .with_child(
            Flex::column()
                .with_child(is_connected)
                .with_default_spacer()
                .with_child(user_profile)
                .padding(theme::grid(1.0)),
        )
        .with_child(Either::new(
            |data: &AppState, _| !data.config.kiosk_mode,
            preferences_widget(&icons::PREFERENCES),
            Empty,
        ))
}

fn preferences_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(3.0), theme::grid(3.0)))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, _, _| ctx.submit_command(commands::SHOW_PREFERENCES))
}
