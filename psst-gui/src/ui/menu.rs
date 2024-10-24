use druid::{commands, platform_menus, Env, LocalizedString, Menu, MenuItem, SysMods, WindowId};

use crate::{
    cmd,
    data::{AppState, Nav},
};

pub fn main_menu(_window: Option<WindowId>, data: &AppState, _env: &Env) -> Menu<AppState> {
    if cfg!(target_os = "macos") && !data.config.kiosk_mode {
        Menu::empty().entry(mac_app_menu())
    } else {
        Menu::empty()
    }
    .entry(edit_menu())
    .entry(view_menu())
}

fn mac_app_menu() -> Menu<AppState> {
    // macOS-only commands are deprecated on other systems.
    #[cfg_attr(not(target_os = "macos"), allow(deprecated))]
    Menu::new(LocalizedString::new("macos-menu-application-menu"))
        .entry(platform_menus::mac::application::preferences())
        .separator()
        .entry(
            // TODO:
            //  This is just overriding `platform_menus::mac::application::quit()`
            //  because l10n is a bit stupid now.
            MenuItem::new(LocalizedString::new("macos-menu-quit").with_placeholder("Quit Psst"))
                .command(cmd::QUIT_APP_WITH_SAVE)
                .hotkey(SysMods::Cmd, "q"),
        )
        .entry(
            MenuItem::new(LocalizedString::new("macos-menu-hide").with_placeholder("Hide Psst"))
                .command(commands::HIDE_APPLICATION)
                .hotkey(SysMods::Cmd, "h"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("macos-menu-hide-others").with_placeholder("Hide Others"),
            )
            .command(commands::HIDE_OTHERS)
            .hotkey(SysMods::AltCmd, "h"),
        )
}

fn edit_menu() -> Menu<AppState> {
    Menu::new(LocalizedString::new("common-menu-edit-menu").with_placeholder("Edit"))
        .entry(platform_menus::common::cut())
        .entry(platform_menus::common::copy())
        .entry(platform_menus::common::paste())
}

fn view_menu() -> Menu<AppState> {
    Menu::new(LocalizedString::new("menu-view-menu").with_placeholder("View"))
        .entry(
            MenuItem::new(LocalizedString::new("menu-item-home").with_placeholder("Home"))
                .command(cmd::NAVIGATE.with(Nav::Home))
                .hotkey(SysMods::Cmd, "1"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-saved-tracks").with_placeholder("Saved Tracks"),
            )
            .command(cmd::NAVIGATE.with(Nav::SavedTracks))
            .hotkey(SysMods::Cmd, "2"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-saved-albums").with_placeholder("Saved Albums"),
            )
            .command(cmd::NAVIGATE.with(Nav::SavedAlbums))
            .hotkey(SysMods::Cmd, "3"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-saved-shows").with_placeholder("Saved Shows"),
            )
            .command(cmd::NAVIGATE.with(Nav::SavedShows))
            .hotkey(SysMods::Cmd, "4"),
        )
        .entry(
            MenuItem::new(LocalizedString::new("menu-item-search").with_placeholder("Search..."))
                .command(cmd::SET_FOCUS.to(cmd::WIDGET_SEARCH_INPUT))
                .hotkey(SysMods::Cmd, "l"),
        )
        .entry(
            MenuItem::new(LocalizedString::new("menu-item-find").with_placeholder("Find..."))
                .command(cmd::TOGGLE_FINDER)
                .hotkey(SysMods::Cmd, "f"),
        )
}
