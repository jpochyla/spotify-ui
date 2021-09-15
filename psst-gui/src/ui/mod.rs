use std::time::Duration;

use druid::{
    im::Vector,
    lens::Unit,
    widget::{CrossAxisAlignment, Either, Flex, Label, Scroll, Slider, Split, ViewSwitcher},
    Env, Insets, LensExt, Menu, MenuItem, Selector, Widget, WidgetExt, WindowDesc,
};

use crate::{
    cmd,
    controller::{NavController, SessionController},
    data::{AppState, Nav, Playback},
    widget::{icons, icons::SvgIcon, Border, Empty, MyWidgetExt, ThemeScope, ViewDispatcher},
};

pub mod album;
pub mod artist;
pub mod home;
pub mod library;
pub mod menu;
pub mod playback;
pub mod playlist;
pub mod preferences;
pub mod recommend;
pub mod search;
pub mod theme;
pub mod track;
pub mod user;
pub mod utils;

pub fn main_window() -> WindowDesc<AppState> {
    let win = WindowDesc::new(root_widget())
        .title(compute_main_window_title)
        .with_min_size((theme::grid(65.0), theme::grid(25.0)))
        .window_size((theme::grid(80.0), theme::grid(100.0)))
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn preferences_window() -> WindowDesc<AppState> {
    let win_size = (theme::grid(50.0), theme::grid(50.0));

    // On Windows, the window size includes the titlebar.
    let win_size = if cfg!(target_os = "windows") {
        const WINDOWS_TITLEBAR_OFFSET: f64 = 6.0;
        (win_size.0, win_size.1 + WINDOWS_TITLEBAR_OFFSET)
    } else {
        win_size
    };

    let win = WindowDesc::new(preferences_widget())
        .title("Preferences")
        .window_size(win_size)
        .resizable(false)
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn account_setup_window() -> WindowDesc<AppState> {
    let win = WindowDesc::new(account_setup_widget())
        .title("Log In")
        .window_size((theme::grid(50.0), theme::grid(45.0)))
        .resizable(false)
        .show_title(false)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

fn preferences_widget() -> impl Widget<AppState> {
    ThemeScope::new(
        preferences::preferences_widget()
            .background(theme::BACKGROUND_DARK)
            .expand(),
    )
}

fn account_setup_widget() -> impl Widget<AppState> {
    ThemeScope::new(
        preferences::account_setup_widget()
            .background(theme::BACKGROUND_DARK)
            .expand(),
    )
}

fn root_widget() -> impl Widget<AppState> {
    let playlists = Scroll::new(playlist::list_widget())
        .vertical()
        .expand_height();
    let sidebar = Flex::column()
        .must_fill_main_axis(true)
        .with_child(sidebar_logo_widget())
        .with_child(sidebar_menu_widget())
        .with_default_spacer()
        .with_flex_child(playlists, 1.0)
        .with_child(volume_slider())
        .with_default_spacer()
        .with_child(user::user_widget())
        .padding(if cfg!(target_os = "macos") {
            // Accommodate the window controls on Mac.
            Insets::new(0.0, 24.0, 0.0, 0.0)
        } else {
            Insets::ZERO
        })
        .background(theme::BACKGROUND_DARK);

    let topbar = Flex::row()
        .must_fill_main_axis(true)
        .with_child(topbar_back_button_widget())
        .with_child(topbar_title_widget())
        .background(Border::Bottom.with_color(theme::BACKGROUND_DARK));

    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(topbar)
        .with_flex_child(route_widget(), 1.0)
        .with_child(playback::panel_widget())
        .background(theme::BACKGROUND_LIGHT);

    let split = Split::columns(sidebar, main)
        .split_point(0.2)
        .bar_size(1.0)
        .min_size(150.0, 300.0)
        .min_bar_area(1.0)
        .solid_bar(true);

    ThemeScope::new(split)
        .controller(SessionController)
        .controller(NavController)
    // .debug_invalidation()
    // .debug_widget_id()
    // .debug_paint_layout()
}

fn route_widget() -> impl Widget<AppState> {
    ViewDispatcher::new(
        |state: &AppState, _| state.route.clone(),
        |route: &Nav, _, _| match route {
            Nav::Home => Scroll::new(home::home_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Nav::SavedTracks => {
                Scroll::new(library::saved_tracks_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::SavedAlbums => {
                Scroll::new(library::saved_albums_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::SearchResults(_) => {
                Scroll::new(search::results_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::AlbumDetail(_) => Scroll::new(album::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Nav::ArtistDetail(_) => Scroll::new(artist::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Nav::PlaylistDetail(_) => {
                Scroll::new(playlist::detail_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Nav::Recommendations(_) => {
                Scroll::new(recommend::results_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
        },
    )
    .expand()
}

fn sidebar_logo_widget() -> impl Widget<AppState> {
    icons::LOGO
        .scale((29.0, 32.0))
        .with_color(theme::GREY_500)
        .padding((0.0, theme::grid(2.0), 0.0, theme::grid(1.0)))
        .center()
        .lens(Unit)
}

fn sidebar_menu_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_default_spacer()
        .with_child(sidebar_link_widget("Home", Nav::Home))
        .with_child(sidebar_link_widget("Tracks", Nav::SavedTracks))
        .with_child(sidebar_link_widget("Albums", Nav::SavedAlbums))
        .with_child(search::input_widget().padding((theme::grid(1.0), theme::grid(1.0))))
}

fn sidebar_link_widget(title: &str, nav: Nav) -> impl Widget<AppState> {
    Label::new(title)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .link()
        .env_scope({
            let nav = nav.clone();
            move |env, route: &Nav| {
                env.set(
                    theme::LINK_COLD_COLOR,
                    if &nav == route {
                        env.get(theme::MENU_BUTTON_BG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_BG_INACTIVE)
                    },
                );
                env.set(
                    theme::TEXT_COLOR,
                    if &nav == route {
                        env.get(theme::MENU_BUTTON_FG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_FG_INACTIVE)
                    },
                );
            }
        })
        .on_click(move |ctx, _, _| {
            ctx.submit_command(cmd::NAVIGATE.with(nav.clone()));
        })
        .lens(AppState::route)
}

fn volume_slider() -> impl Widget<AppState> {
    const SAVE_DELAY: Duration = Duration::from_millis(100);
    const SAVE_TO_CONFIG: Selector = Selector::new("app.volume.save-to-config");

    Flex::column()
        .with_child(
            Label::dynamic(|&volume: &f64, _| format!("Volume: {}%", (volume * 100.0).floor()))
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_text_size(theme::TEXT_SIZE_SMALL),
        )
        .with_default_spacer()
        .with_child(
            Slider::new()
                .with_range(0.0, 1.0)
                .expand_width()
                .env_scope(|env, _| {
                    env.set(theme::BASIC_WIDGET_HEIGHT, theme::grid(1.5));
                    env.set(theme::FOREGROUND_LIGHT, env.get(theme::GREY_400));
                    env.set(theme::FOREGROUND_DARK, env.get(theme::GREY_400));
                }),
        )
        .padding((theme::grid(1.5), 0.0))
        .on_debounce(SAVE_DELAY, |ctx, _, _| ctx.submit_command(SAVE_TO_CONFIG))
        .lens(AppState::playback.then(Playback::volume))
        .on_command(SAVE_TO_CONFIG, |_, _, data| {
            data.config.volume = data.playback.volume;
            data.config.save();
        })
}

fn topbar_back_button_widget() -> impl Widget<AppState> {
    let icon = icons::BACK.scale((10.0, theme::grid(2.0)));
    let disabled = icon
        .clone()
        .with_color(theme::GREY_600)
        .padding(theme::grid(1.0));
    let enabled = icon
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_click(|ctx, _, _| {
            ctx.submit_command(cmd::NAVIGATE_BACK.with(1));
        })
        .context_menu(history_menu);
    Either::new(
        |history: &Vector<Nav>, _| history.is_empty(),
        disabled,
        enabled,
    )
    .padding(theme::grid(1.0))
    .lens(AppState::history)
}

fn history_menu(history: &Vector<Nav>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for (index, history) in history.iter().rev().take(10).enumerate() {
        let skip_back_in_history_n_times = index + 1;
        menu = menu.entry(
            MenuItem::new(history.full_title())
                .command(cmd::NAVIGATE_BACK.with(skip_back_in_history_n_times)),
        );
    }

    menu
}

fn topbar_title_widget() -> impl Widget<AppState> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(route_title_widget())
        .with_spacer(theme::grid(0.5))
        .with_child(route_icon_widget())
        .lens(AppState::route)
}

fn route_icon_widget() -> impl Widget<Nav> {
    ViewSwitcher::new(
        |route: &Nav, _| route.clone(),
        |route: &Nav, _, _| {
            let icon = |icon: &SvgIcon| icon.scale(theme::ICON_SIZE_SMALL);
            match &route {
                Nav::Home => Empty.boxed(),
                Nav::SavedTracks => Empty.boxed(),
                Nav::SavedAlbums => Empty.boxed(),
                Nav::SearchResults(_) => icon(&icons::SEARCH).boxed(),
                Nav::AlbumDetail(_) => icon(&icons::ALBUM).boxed(),
                Nav::ArtistDetail(_) => icon(&icons::ARTIST).boxed(),
                Nav::PlaylistDetail(_) => icon(&icons::PLAYLIST).boxed(),
                Nav::Recommendations(_) => icon(&icons::SEARCH).boxed(),
            }
        },
    )
}

fn route_title_widget() -> impl Widget<Nav> {
    Label::dynamic(|route: &Nav, _| route.title())
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_size(theme::TEXT_SIZE_LARGE)
}

fn compute_main_window_title(data: &AppState, _env: &Env) -> String {
    if let Some(now_playing) = &data.playback.now_playing {
        format!(
            "Psst - {} - {}",
            now_playing.item.artist_name(),
            now_playing.item.name
        )
    } else {
        "Psst".to_owned()
    }
}
