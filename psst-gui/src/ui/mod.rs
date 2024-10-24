use std::time::Duration;

use druid::{
    im::Vector,
    widget::{CrossAxisAlignment, Either, Flex, Label, List, Scroll, Slider, Split, ViewSwitcher},
    Color, Env, Insets, Key, LensExt, Menu, MenuItem, Selector, Widget, WidgetExt, WindowDesc,
    WindowState,
};
use druid_shell::Cursor;

use crate::data::config::SortCriteria;
use crate::{
    cmd,
    controller::{
        AfterDelay, AlertCleanupController, NavController, SessionController, SortController,
    },
    data::{
        config::SortOrder, Alert, AlertStyle, AppState, Config, Nav, Playable, Playback, Route,
        ALERT_DURATION,
    },
    widget::{
        icons, icons::SvgIcon, Border, Empty, MyWidgetExt, Overlay, ThemeScope, ViewDispatcher,
    },
};

pub mod album;
pub mod artist;
pub mod episode;
pub mod find;
pub mod home;
pub mod library;
pub mod lyrics;
pub mod menu;
pub mod playable;
pub mod playback;
pub mod playlist;
pub mod preferences;
pub mod recommend;
pub mod search;
pub mod show;
pub mod theme;
pub mod track;
pub mod user;
pub mod utils;

pub fn main_window(config: &Config) -> WindowDesc<AppState> {
    let mut win = WindowDesc::new(root_widget(config))
        .title(compute_main_window_title)
        .show_title(false);

    if config.kiosk_mode {
        win = win
            .set_window_state(WindowState::Maximized)
            .resizable(false)
            .show_titlebar(false);

        // Set the window size to the primary monitor's work area and position it at (0, 0)
        if let Some(monitor) = druid::Screen::get_monitors().first() {
            let work_area = monitor.virtual_work_rect();
            win = win
                .window_size(work_area.size())
                .set_position(druid::Point::new(0.0, 0.0));
        }
    } else {
        win = win
            .window_size(config.window_size)
            .with_min_size((theme::grid(65.0), theme::grid(50.0)))
            .transparent_titlebar(true);
    }

    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn preferences_window() -> WindowDesc<AppState> {
    let win_size = (theme::grid(50.0), theme::grid(55.0));

    // On Windows, the window size includes the titlebar.
    let win_size = if cfg!(target_os = "windows") {
        const WINDOWS_TITLEBAR_OFFSET: f64 = 56.0;
        (win_size.0, win_size.1 + WINDOWS_TITLEBAR_OFFSET)
    } else {
        win_size
    };

    let win = WindowDesc::new(preferences_widget())
        .title("Preferences")
        .window_size(win_size)
        .resizable(false)
        .show_title(false)
        .set_always_on_top(true)
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn account_setup_window() -> WindowDesc<AppState> {
    let win = WindowDesc::new(account_setup_widget())
        .title("Login")
        .resizable(false)
        .show_title(false)
        .window_size((theme::grid(50.0), theme::grid(45.0)))
        .transparent_titlebar(true);
    if cfg!(target_os = "macos") {
        win.menu(menu::main_menu)
    } else {
        win
    }
}

pub fn kiosk_setup_window() -> WindowDesc<AppState> {
    let win = WindowDesc::new(kiosk_setup_widget())
        .title("Setup")
        .resizable(false)
        .show_title(false)
        .window_size((theme::grid(50.0), theme::grid(45.0)))
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

fn kiosk_setup_widget() -> impl Widget<AppState> {
    ThemeScope::new(
        preferences::kiosk_setup_widget()
            .background(theme::BACKGROUND_DARK)
            .expand(),
    )
}

fn root_widget(config: &Config) -> impl Widget<AppState> {
    let playlists = Scroll::new(playlist::list_widget())
        .vertical()
        .expand_height();

    let playlists = Flex::column()
        .must_fill_main_axis(true)
        .with_child(sidebar_menu_widget())
        .with_default_spacer()
        .with_flex_child(playlists, 1.0)
        .padding(if cfg!(target_os = "macos") && !config.kiosk_mode {
            // Accommodate the window controls on macOS
            Insets::new(0.0, 24.0, 0.0, 0.0)
        } else {
            Insets::ZERO
        });

    let controls = Flex::column()
        .with_default_spacer()
        .with_child(volume_slider())
        .with_default_spacer()
        .with_child(user::user_widget())
        .center()
        .fix_height(88.0)
        .background(Border::Top.with_color(theme::GREY_500));

    let sidebar = Flex::column()
        .with_flex_child(playlists, 1.0)
        .with_child(controls)
        .background(theme::BACKGROUND_DARK);

    let topbar = Flex::row()
        .must_fill_main_axis(true)
        .with_child(topbar_back_button_widget())
        .with_child(topbar_title_widget())
        .with_child(topbar_sort_widget())
        .background(Border::Bottom.with_color(theme::BACKGROUND_DARK));

    let main = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(topbar)
        .with_flex_child(Overlay::bottom(route_widget(), alert_widget()), 1.0)
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
        .controller(SortController)
    // .debug_invalidation()
    // .debug_widget_id()
    // .debug_paint_layout()
}

fn alert_widget() -> impl Widget<AppState> {
    const BG: Key<Color> = Key::new("app.alert.BG");
    const DISMISS_ALERT: Selector<usize> = Selector::new("app.alert.dismiss");

    List::new(|| {
        Flex::row()
            .with_child(
                Label::dynamic(|alert: &Alert, _| match alert.style {
                    AlertStyle::Error => "Error:".to_string(),
                    AlertStyle::Info => String::new(),
                })
                .with_font(theme::UI_FONT_MEDIUM),
            )
            .with_default_spacer()
            .with_flex_child(Label::raw().lens(Alert::message), 1.0)
            .padding(theme::grid(2.0))
            .background(BG)
            .env_scope(|env, alert: &Alert| {
                env.set(
                    BG,
                    match alert.style {
                        AlertStyle::Error => env.get(theme::RED),
                        AlertStyle::Info => env.get(theme::GREY_600),
                    },
                )
            })
            .controller(AfterDelay::new(
                ALERT_DURATION,
                |ctx, alert: &mut Alert, _| {
                    ctx.submit_command(DISMISS_ALERT.with(alert.id));
                },
            ))
    })
    .lens(AppState::alerts)
    .on_command(DISMISS_ALERT, |_, &id, state| {
        state.dismiss_alert(id);
    })
    .controller(AlertCleanupController)
}

fn route_widget() -> impl Widget<AppState> {
    ViewDispatcher::new(
        |state: &AppState, _| state.nav.route(),
        |route: &Route, _, _| match route {
            Route::Home => Scroll::new(home::home_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::Lyrics => Scroll::new(lyrics::lyrics_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::SavedTracks => Flex::column()
                .with_child(
                    find::finder_widget(cmd::FIND_IN_SAVED_TRACKS, "Find in Saved Tracks...")
                        .lens(AppState::finder),
                )
                .with_flex_child(
                    Scroll::new(library::saved_tracks_widget().padding(theme::grid(1.0)))
                        .vertical(),
                    1.0,
                )
                .boxed(),
            Route::SavedAlbums => {
                Scroll::new(library::saved_albums_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Route::SavedShows => {
                Scroll::new(library::saved_shows_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
            Route::SearchResults => Scroll::new(search::results_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::AlbumDetail => Scroll::new(album::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::ArtistDetail => Scroll::new(artist::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::PlaylistDetail => Flex::column()
                .with_child(
                    find::finder_widget(cmd::FIND_IN_PLAYLIST, "Find in Playlist...")
                        .lens(AppState::finder),
                )
                .with_flex_child(
                    Scroll::new(playlist::detail_widget().padding(theme::grid(1.0))).vertical(),
                    1.0,
                )
                .boxed(),
            Route::ShowDetail => Scroll::new(show::detail_widget().padding(theme::grid(1.0)))
                .vertical()
                .boxed(),
            Route::Recommendations => {
                Scroll::new(recommend::results_widget().padding(theme::grid(1.0)))
                    .vertical()
                    .boxed()
            }
        },
    )
    .expand()
}

fn sidebar_menu_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_default_spacer()
        .with_child(sidebar_link_widget("Home", Nav::Home))
        .with_child(sidebar_link_widget("Tracks", Nav::SavedTracks))
        .with_child(sidebar_link_widget("Albums", Nav::SavedAlbums))
        .with_child(sidebar_link_widget("Podcasts", Nav::SavedShows))
        .with_child(search::input_widget().padding((theme::grid(1.0), theme::grid(1.0))))
}

fn sidebar_link_widget(title: &str, link_nav: Nav) -> impl Widget<AppState> {
    Label::new(title)
        .padding((theme::grid(2.0), theme::grid(1.0)))
        .expand_width()
        .link()
        .env_scope({
            let link_nav = link_nav.clone();
            move |env, nav: &Nav| {
                env.set(
                    theme::LINK_COLD_COLOR,
                    if &link_nav == nav {
                        env.get(theme::MENU_BUTTON_BG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_BG_INACTIVE)
                    },
                );
                env.set(
                    theme::TEXT_COLOR,
                    if &link_nav == nav {
                        env.get(theme::MENU_BUTTON_FG_ACTIVE)
                    } else {
                        env.get(theme::MENU_BUTTON_FG_INACTIVE)
                    },
                );
            }
        })
        .on_left_click(move |ctx, _, _, _| {
            ctx.submit_command(cmd::NAVIGATE.with(link_nav.clone()));
        })
        .lens(AppState::nav)
}

fn volume_slider() -> impl Widget<AppState> {
    const SAVE_DELAY: Duration = Duration::from_millis(100);
    const SAVE_TO_CONFIG: Selector = Selector::new("app.volume.save-to-config");

    Flex::row()
        .with_flex_child(
            Slider::new()
                .with_range(0.0, 1.0)
                .expand_width()
                .env_scope(|env, _| {
                    env.set(theme::BASIC_WIDGET_HEIGHT, theme::grid(1.5));
                    env.set(theme::FOREGROUND_LIGHT, env.get(theme::GREY_400));
                    env.set(theme::FOREGROUND_DARK, env.get(theme::GREY_400));
                })
                .with_cursor(Cursor::Pointer),
            1.0,
        )
        .with_default_spacer()
        .with_child(
            Label::dynamic(|&volume: &f64, _| format!("{}%", (volume * 100.0).floor()))
                .with_text_color(theme::PLACEHOLDER_COLOR)
                .with_text_size(theme::TEXT_SIZE_SMALL),
        )
        .padding((theme::grid(2.0), 0.0))
        .on_debounce(SAVE_DELAY, |ctx, _, _| ctx.submit_command(SAVE_TO_CONFIG))
        .lens(AppState::playback.then(Playback::volume))
        .on_scroll(
            |data| &data.config.slider_scroll_scale,
            |_, data, _, scaled_delta| {
                data.playback.volume = (data.playback.volume + scaled_delta).clamp(0.0, 1.0);
            },
        )
}

fn topbar_sort_widget() -> impl Widget<AppState> {
    let up_icon = icons::UP.scale((10.0, theme::grid(2.0)));
    let down_icon = icons::DOWN.scale((10.0, theme::grid(2.0)));

    let ascending_icon = up_icon
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, _, _| {
            ctx.submit_command(cmd::TOGGLE_SORT_ORDER);
        })
        .context_menu(sorting_menu);

    let descending_icon = down_icon
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, _, _| {
            ctx.submit_command(cmd::TOGGLE_SORT_ORDER);
        })
        .context_menu(sorting_menu);
    let enabled = Either::new(
        |data: &AppState, _| {
            // check if the current nav is PlaylistDetail
            data.config.sort_order == SortOrder::Ascending
        },
        ascending_icon,
        descending_icon,
    );

    //a "dynamic" widget that is always disabled.
    let disabled = Either::new(|_, _| true, Empty.boxed(), Empty.boxed());

    Either::new(
        |nav: &AppState, _| {
            // check if the current nav is PlaylistDetail
            matches!(nav.nav, Nav::PlaylistDetail(_))
        },
        enabled,
        disabled,
    )
    .padding(theme::grid(1.0)) //.lens(AppState::nav)
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
        .on_left_click(|ctx, _, _, _| {
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

fn sorting_menu(app_state: &AppState) -> Menu<AppState> {
    let mut menu = Menu::new("Sort by");

    // Create menu items for sorting options
    let mut sort_by_title = MenuItem::new("Title").command(cmd::SORT_BY_TITLE);
    let mut sort_by_album = MenuItem::new("Album").command(cmd::SORT_BY_ALBUM);
    let mut sort_by_date_added = MenuItem::new("Date Added").command(cmd::SORT_BY_DATE_ADDED);
    let mut sort_by_duration = MenuItem::new("Duration").command(cmd::SORT_BY_DURATION);
    let mut sort_by_artist = MenuItem::new("Artist").command(cmd::SORT_BY_ARTIST);

    match app_state.config.sort_criteria {
        SortCriteria::Title => sort_by_title = sort_by_title.selected(true),
        SortCriteria::Album => sort_by_album = sort_by_album.selected(true),
        SortCriteria::DateAdded => sort_by_date_added = sort_by_date_added.selected(true),
        SortCriteria::Duration => sort_by_duration = sort_by_duration.selected(true),
        SortCriteria::Artist => sort_by_artist = sort_by_artist.selected(true),
    };

    // Add the items and checkboxes to the menu
    menu = menu.entry(sort_by_album);
    menu = menu.entry(sort_by_artist);
    menu = menu.entry(sort_by_date_added);
    menu = menu.entry(sort_by_duration);
    menu = menu.entry(sort_by_title);

    menu
}

fn topbar_title_widget() -> impl Widget<AppState> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(route_title_widget())
        .with_spacer(theme::grid(0.5))
        .with_child(route_icon_widget())
        .lens(AppState::nav)
}

fn route_icon_widget() -> impl Widget<Nav> {
    ViewSwitcher::new(
        |nav: &Nav, _| nav.clone(),
        |nav: &Nav, _, _| {
            let icon = |icon: &SvgIcon| icon.scale(theme::ICON_SIZE_MEDIUM);
            match &nav {
                Nav::Home | Nav::Lyrics | Nav::SavedTracks | Nav::SavedAlbums | Nav::SavedShows => Empty.boxed(),
                Nav::SearchResults(_) | Nav::Recommendations(_) => icon(&icons::SEARCH).boxed(),
                Nav::AlbumDetail(_) => icon(&icons::ALBUM).boxed(),
                Nav::ArtistDetail(_) => icon(&icons::ARTIST).boxed(),
                Nav::PlaylistDetail(_) => icon(&icons::PLAYLIST).boxed(),
                Nav::ShowDetail(_) => icon(&icons::PODCAST).boxed(),
            }
        },
    )
}

fn route_title_widget() -> impl Widget<Nav> {
    Label::dynamic(|nav: &Nav, _| nav.title())
        .with_font(theme::UI_FONT_MEDIUM)
        .with_text_size(theme::TEXT_SIZE_LARGE)
}

fn compute_main_window_title(data: &AppState, _env: &Env) -> String {
    if let Some(now_playing) = &data.playback.now_playing {
        match &now_playing.item {
            Playable::Track(track) => {
                format!("{} - {}", track.artist_name(), track.name)
            }
            Playable::Episode(episode) => episode.name.to_string(),
        }
    } else {
        "Psst".to_owned()
    }
}
