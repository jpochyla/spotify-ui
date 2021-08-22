mod album;
mod artist;
pub mod config;
mod ctx;
mod id;
mod nav;
mod playback;
mod playlist;
mod promise;
mod recommend;
mod search;
mod track;
mod user;
mod utils;

use std::{mem, sync::Arc, time::Duration};

use druid::{
    im::{HashSet, Vector},
    Data, Lens,
};
use psst_core::session::SessionService;

pub use crate::data::{
    album::{Album, AlbumDetail, AlbumLink, AlbumType, Copyright, CopyrightType},
    artist::{Artist, ArtistAlbums, ArtistDetail, ArtistLink, ArtistTracks},
    config::{AudioQuality, Authentication, Config, Preferences, PreferencesTab, Theme},
    ctx::Ctx,
    nav::{Nav, SpotifyUrl},
    playback::{
        NowPlaying, Playback, PlaybackOrigin, PlaybackPayload, PlaybackState, QueueBehavior,
        QueuedTrack,
    },
    playlist::{Playlist, PlaylistDetail, PlaylistLink, PlaylistTracks},
    promise::{Promise, PromiseState},
    recommend::{
        Range, Recommend, Recommendations, RecommendationsKnobs, RecommendationsParams,
        RecommendationsRequest, Toggled,
    },
    search::{Search, SearchResults},
    track::{AudioAnalysis, AudioSegment, TimeInterval, Track, TrackId},
    user::UserProfile,
    utils::{Cached, Float64, Image, Page},
};

#[derive(Clone, Data, Lens)]
pub struct AppState {
    #[data(ignore)]
    pub session: SessionService,

    pub route: Nav,
    pub history: Vector<Nav>,
    pub config: Config,
    pub preferences: Preferences,
    pub playback: Playback,
    pub search: Search,
    pub recommend: Recommend,
    pub album_detail: AlbumDetail,
    pub artist_detail: ArtistDetail,
    pub playlist_detail: PlaylistDetail,
    pub library: Arc<Library>,
    pub common_ctx: Arc<CommonCtx>,
    pub user_profile: Promise<UserProfile>,
    pub personalized: Personalized,
}

impl AppState {
    pub fn default_with_config(config: Config) -> Self {
        let library = Arc::new(Library {
            saved_albums: Promise::Empty,
            saved_tracks: Promise::Empty,
            playlists: Promise::Empty,
        });
        let common_ctx = Arc::new(CommonCtx {
            playback_item: None,
            library: Arc::clone(&library),
        });
        let playback = Playback {
            state: PlaybackState::Stopped,
            now_playing: None,
            queue_behavior: QueueBehavior::Sequential,
            queue: Vector::new(),
            volume: config.volume,
        };
        Self {
            session: SessionService::empty(),
            route: Nav::Home,
            history: Vector::new(),
            config,
            preferences: Preferences {
                active: PreferencesTab::General,
                auth: Authentication {
                    username: String::new(),
                    password: String::new(),
                    result: Promise::Empty,
                },
                cache_size: Promise::Empty,
            },
            playback,
            search: Search {
                input: "".into(),
                results: Promise::Empty,
            },
            recommend: Recommend {
                knobs: Default::default(),
                results: Promise::Empty,
            },
            album_detail: AlbumDetail {
                album: Promise::Empty,
            },
            artist_detail: ArtistDetail {
                artist: Promise::Empty,
                albums: Promise::Empty,
                top_tracks: Promise::Empty,
                related_artists: Promise::Empty,
            },
            playlist_detail: PlaylistDetail {
                playlist: Promise::Empty,
                tracks: Promise::Empty,
            },
            library,
            common_ctx,
            user_profile: Promise::Empty,
            personalized: Personalized {
                made_for_you: Promise::Empty,
            },
        }
    }
}

impl AppState {
    pub fn navigate(&mut self, nav: &Nav) {
        if &self.route != nav {
            let previous = mem::replace(&mut self.route, nav.to_owned());
            self.history.push_back(previous);
        }
    }

    pub fn navigate_back(&mut self) {
        if let Some(nav) = self.history.pop_back() {
            self.route = nav;
        }
    }
}

impl AppState {
    pub fn queued_track(&self, track_id: &TrackId) -> Option<QueuedTrack> {
        self.playback
            .queue
            .iter()
            .find(|queued| queued.track.id.same(track_id))
            .cloned()
    }

    pub fn loading_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin) {
        self.common_ctx_mut().playback_item.take();
        self.playback.state = PlaybackState::Loading;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress: Duration::default(),
            analysis: Promise::default(),
        });
    }

    pub fn start_playback(&mut self, item: Arc<Track>, origin: PlaybackOrigin, progress: Duration) {
        self.common_ctx_mut().playback_item.replace(item.clone());
        self.playback.state = PlaybackState::Playing;
        self.playback.now_playing.replace(NowPlaying {
            item,
            origin,
            progress,
            analysis: Promise::default(),
        });
    }

    pub fn progress_playback(&mut self, progress: Duration) {
        if let Some(now_playing) = &mut self.playback.now_playing {
            now_playing.progress = progress;
        }
    }

    pub fn pause_playback(&mut self) {
        self.playback.state = PlaybackState::Paused;
    }

    pub fn resume_playback(&mut self) {
        self.playback.state = PlaybackState::Playing;
    }

    pub fn block_playback(&mut self) {
        // TODO: Figure out how to signal blocked playback properly.
    }

    pub fn stop_playback(&mut self) {
        self.playback.state = PlaybackState::Stopped;
        self.playback.now_playing.take();
        self.common_ctx_mut().playback_item.take();
    }
}

impl AppState {
    pub fn update_common_ctx(&mut self) {
        self.common_ctx_mut().library = Arc::clone(&self.library);
    }

    pub fn common_ctx_mut(&mut self) -> &mut CommonCtx {
        Arc::make_mut(&mut self.common_ctx)
    }

    pub fn library_mut(&mut self) -> &mut Library {
        Arc::make_mut(&mut self.library)
    }
}

#[derive(Clone, Data, Lens)]
pub struct Library {
    pub playlists: Promise<Vector<Playlist>>,
    pub saved_albums: Promise<SavedAlbums>,
    pub saved_tracks: Promise<SavedTracks>,
}

impl Library {
    pub fn add_track(&mut self, track: Arc<Track>) {
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            saved.set.insert(track.id);
            saved.tracks.push_front(track);
        }
    }

    pub fn remove_track(&mut self, track_id: &TrackId) {
        if let Some(saved) = self.saved_tracks.resolved_mut() {
            saved.set.remove(track_id);
            saved.tracks.retain(|t| &t.id != track_id);
        }
    }

    pub fn contains_track(&self, track: &Track) -> bool {
        if let Some(saved) = self.saved_tracks.resolved() {
            saved.set.contains(&track.id)
        } else {
            false
        }
    }

    pub fn add_album(&mut self, album: Arc<Album>) {
        if let Some(saved) = self.saved_albums.resolved_mut() {
            saved.set.insert(album.id.clone());
            saved.albums.push_front(album);
        }
    }

    pub fn remove_album(&mut self, album_id: &Arc<str>) {
        if let Some(saved) = self.saved_albums.resolved_mut() {
            saved.set.remove(album_id);
            saved.albums.retain(|a| &a.id != album_id);
        }
    }

    pub fn contains_album(&self, album: &Album) -> bool {
        if let Some(saved) = self.saved_albums.resolved() {
            saved.set.contains(&album.id)
        } else {
            false
        }
    }
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedTracks {
    pub tracks: Vector<Arc<Track>>,
    pub set: HashSet<TrackId>,
}

impl SavedTracks {
    pub fn new(tracks: Vector<Arc<Track>>) -> Self {
        let set = tracks.iter().map(|t| t.id).collect();
        Self { tracks, set }
    }
}

#[derive(Clone, Default, Data, Lens)]
pub struct SavedAlbums {
    pub albums: Vector<Arc<Album>>,
    pub set: HashSet<Arc<str>>,
}

impl SavedAlbums {
    pub fn new(albums: Vector<Arc<Album>>) -> Self {
        let set = albums.iter().map(|a| a.id.clone()).collect();
        Self { albums, set }
    }
}

#[derive(Clone, Data)]
pub struct CommonCtx {
    pub playback_item: Option<Arc<Track>>,
    pub library: Arc<Library>,
}

impl CommonCtx {
    pub fn is_track_playing(&self, track: &Track) -> bool {
        self.playback_item
            .as_ref()
            .map(|t| t.id.same(&track.id))
            .unwrap_or(false)
    }

    pub fn is_track_saved(&self, track: &Track) -> bool {
        self.library.contains_track(track)
    }

    pub fn is_album_saved(&self, album: &Album) -> bool {
        self.library.contains_album(album)
    }
}

pub type WithCtx<T> = Ctx<Arc<CommonCtx>, T>;

#[derive(Clone, Data, Lens)]
pub struct Personalized {
    pub made_for_you: Promise<Vector<Playlist>>,
}
