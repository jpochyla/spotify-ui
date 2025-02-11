use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Either, Flex, Label, ViewSwitcher},
    Env, LensExt, LocalizedString, Menu, MenuItem, Size, TextAlignment, Widget, WidgetExt,
};
use psst_core::{
    audio::normalize::NormalizationLevel,
    item_id::{ItemId, ItemIdType},
    player::item::PlaybackItem,
};

use crate::{
    cmd,
    data::{
        AppState, Library, Nav, Playable, PlaybackOrigin, PlaylistAddTrack, PlaylistRemoveTrack,
        QueueEntry, RecommendationsRequest, Track,
    },
    ui::playlist,
    widget::{icons, Empty, MyWidgetExt, RemoteImage},
};

use super::{
    library,
    playable::{self, PlayRow},
    theme,
    utils::{self, placeholder_widget},
};

#[derive(Copy, Clone)]
pub struct Display {
    pub number: bool,
    pub title: bool,
    pub artist: bool,
    pub album: bool,
    pub cover: bool,
    pub popularity: bool,
}

impl Display {
    pub fn empty() -> Self {
        Display {
            number: false,
            title: false,
            artist: false,
            album: false,
            cover: false,
            popularity: false,
        }
    }
}

pub fn playable_widget(track: &Track, display: Display) -> impl Widget<PlayRow<Arc<Track>>> {
    let mut main_row = Flex::row();
    let mut major = Flex::row();
    let mut minor = Flex::row();

    if display.number {
        let track_number = Label::<Arc<Track>>::dynamic(|track, _| track.track_number.to_string())
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_text_alignment(TextAlignment::Center)
            .center()
            .fix_width(theme::grid(2.0))
            .lens(PlayRow::item);
        major.add_child(track_number);
        major.add_default_spacer();

        // Align the bottom line content.
        minor.add_spacer(theme::grid(2.0));
        minor.add_default_spacer();
    }

    if display.cover {
        let album_cover = rounded_cover_widget(theme::grid(4.0))
            .padding_right(theme::grid(1.0)) // Instead of `add_default_spacer`.
            .lens(PlayRow::item);
        main_row.add_child(Either::new(
            |row, _| row.ctx.show_track_cover,
            album_cover,
            Empty,
        ));
    }

    if display.title {
        let track_name = Label::raw()
            .with_font(theme::UI_FONT_MEDIUM)
            .lens(PlayRow::item.then(Track::name.in_arc()));
        major.add_child(track_name);
    }

    if track.explicit && (display.artist || display.album) {
        let icon = icons::EXPLICIT.scale(theme::ICON_SIZE_TINY);
        minor.add_child(icon);
        minor.add_spacer(theme::grid(0.5));
    }

    if display.artist {
        let track_artists = Label::dynamic(|row: &PlayRow<Arc<Track>>, _| row.item.artist_names())
            .with_text_size(theme::TEXT_SIZE_SMALL);
        minor.add_child(track_artists);
    }

    if display.album {
        let track_album = Label::raw()
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(PlayRow::item.then(Track::lens_album_name().in_arc()));
        if display.artist {
            minor.add_default_spacer();
        }
        minor.add_child(track_album);
    }

    let is_playing = playable::is_playing_marker_widget().lens(PlayRow::is_playing);
    major.add_default_spacer();
    major.add_flex_child(is_playing, 1.0);

    if display.popularity {
        let track_popularity = Label::<Arc<Track>>::dynamic(|track, _| {
            track.popularity.map(popularity_stars).unwrap_or_default()
        })
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(PlayRow::item);
        major.add_default_spacer();
        major.add_child(track_popularity);
    }

    let track_duration =
        Label::<Arc<Track>>::dynamic(|track, _| utils::as_minutes_and_seconds(track.duration))
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .lens(PlayRow::item);
    major.add_default_spacer();
    major.add_child(track_duration);

    let saved = ViewSwitcher::new(
        |row: &PlayRow<Arc<Track>>, _| row.ctx.library.saved_tracks.is_resolved(),
        |selector: &bool, _, _| match selector {
            true => ViewSwitcher::new(
                |row: &PlayRow<Arc<Track>>, _| row.ctx.library.contains_track(&row.item),
                |selector: &bool, _, _| {
                    match selector {
                        true => &icons::CIRCLE_CHECK,
                        false => &icons::CIRCLE_PLUS,
                    }
                    .scale(theme::ICON_SIZE_SMALL)
                    .boxed()
                },
            )
            .on_left_click(|ctx, _, row, _| {
                let track = &row.item;
                if row.ctx.library.contains_track(track) {
                    ctx.submit_command(library::UNSAVE_TRACK.with(track.id))
                } else {
                    ctx.submit_command(library::SAVE_TRACK.with(track.clone()));
                    ctx.submit_command(library::LOAD_TRACKS);
                }
            })
            .boxed(),
            false => Box::new(Flex::column()),
        },
    );

    main_row
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(major)
                .with_spacer(2.0)
                .with_child(minor)
                .on_left_click(|ctx, _, row, _| {
                    ctx.submit_notification(cmd::PLAY.with(row.position))
                }),
            1.0,
        )
        .with_default_spacer()
        .with_child(saved)
        .padding(theme::grid(1.0))
        .link()
        .active(|row: &PlayRow<Arc<Track>>, _env: &Env| {
            // Check if this track is the target of album detail navigation
            if let Nav::AlbumDetail(_, Some(target_id)) = &row.ctx.nav {
                return *target_id == row.item.id;
            }
            // Otherwise check if it's playing or is the current track
            row.is_playing || row.ctx.now_playing.as_ref().is_some_and(|playable| {
                matches!(playable, Playable::Track(track) if track.id == row.item.id)
            })
        })
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .context_menu(track_row_menu)
}

fn cover_widget(size: f64) -> impl Widget<Arc<Track>> {
    RemoteImage::new(placeholder_widget(), move |track: &Arc<Track>, _| {
        track
            .album
            .as_ref()
            .and_then(|al| al.image(size, size).map(|image| image.url.clone()))
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Arc<Track>> {
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

fn popularity_stars(popularity: u32) -> String {
    const COUNT: usize = 5;

    let popularity_coef = popularity as f32 / 100.0;
    let popular = (COUNT as f32 * popularity_coef).round() as usize;
    let unpopular = COUNT - popular;

    let mut stars = String::with_capacity(COUNT);
    for _ in 0..popular {
        stars.push('★');
    }
    for _ in 0..unpopular {
        stars.push('☆');
    }
    stars
}

fn track_row_menu(row: &PlayRow<Arc<Track>>) -> Menu<AppState> {
    track_menu(&row.item, &row.ctx.library, &row.origin, row.item.track_pos)
}

pub fn track_menu(
    track: &Arc<Track>,
    library: &Library,
    origin: &PlaybackOrigin,
    track_pos: usize,
) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for artist_link in &track.artists {
        let more_than_one_artist = track.artists.len() > 1;
        let title = if more_than_one_artist {
            LocalizedString::new("menu-item-show-artist-name")
                .with_placeholder(format!("Go to Artist \"{}\"", artist_link.name))
        } else {
            LocalizedString::new("menu-item-show-artist").with_placeholder("Go to Artist")
        };
        menu = menu.entry(
            MenuItem::new(title)
                .command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist_link.to_owned()))),
        );
    }

    if let Some(album_link) = track.album.as_ref() {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-show-album").with_placeholder("Go to Album"),
            )
            .command(cmd::NAVIGATE.with(Nav::AlbumDetail(album_link.to_owned(), None))),
        );
    }

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-show-recommended")
                .with_placeholder("Show Similar Tracks"),
        )
        .command(cmd::NAVIGATE.with(Nav::Recommendations(Arc::new(
            RecommendationsRequest::for_track(track.id),
        )))),
    );

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-show-credits").with_placeholder("Show Track Credits"),
        )
        .command(cmd::SHOW_CREDITS_WINDOW.with(track.clone())),
    );

    menu = menu.separator();

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Track"),
        )
        .command(cmd::COPY.with(track.url())),
    );

    if library.contains_track(track) {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-remove-from-library")
                    .with_placeholder("Remove Track from Library"),
            )
            .command(library::UNSAVE_TRACK.with(track.id)),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library")
                    .with_placeholder("Save Track to Library"),
            )
            .command(library::SAVE_TRACK.with(track.clone())),
        );
    }

    if let PlaybackOrigin::Playlist(playlist) = origin {
        // Do some (hopefully) quick checks to determine if we should give the
        // option to remove items from this playlist, only allowing it if the
        // playlist is collaborative or we are the owner of it
        let should_show = {
            if let Some(details) = library
                .playlists
                .resolved()
                .and_then(|pl| pl.iter().find(|p| p.id == playlist.id))
            {
                if details.collaborative {
                    true
                } else if let Some(user) = library.user_profile.resolved() {
                    user.id == details.owner.id
                } else {
                    // If we can find the playlist, but for some reason can't
                    // resolve our own user, just show the option anyways and
                    // we'll see an error at the bottom if it doesn't work
                    // when they try to remove a track
                    true
                }
            } else {
                // If this playlist doesn't exist in our library,
                // just assume that we can't edit it since we probably
                // searched for it or something
                false
            }
        };

        if should_show {
            menu = menu.entry(
                MenuItem::new(
                    LocalizedString::new("menu-item-remove-from-playlist")
                        .with_placeholder("Remove from Current Playlist"),
                )
                .command(playlist::REMOVE_TRACK.with(PlaylistRemoveTrack {
                    link: playlist.to_owned(),
                    track_pos,
                })),
            );
        }
    }

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-add-to-queue").with_placeholder("Add Track to Queue"),
        )
        // PlayerCommand
        .command(cmd::ADD_TO_QUEUE.with((
            QueueEntry {
                item: crate::ui::Playable::Track(track.clone()),
                origin: origin.clone(),
            },
            PlaybackItem {
                item_id: ItemId::from_base62(&String::from(track.id), ItemIdType::Track).unwrap(),
                norm_level: NormalizationLevel::Track,
            },
        ))),
    );

    let mut playlist_menu = Menu::new(
        LocalizedString::new("menu-item-add-to-playlist").with_placeholder("Add to Playlist"),
    );
    for playlist in library.writable_playlists() {
        playlist_menu = playlist_menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-playlist")
                    .with_placeholder(format!("{}", playlist.name)),
            )
            .command(playlist::ADD_TRACK.with(PlaylistAddTrack {
                link: playlist.link(),
                track_id: track.id,
            })),
        );
    }
    menu = menu.entry(playlist_menu);

    menu
}
