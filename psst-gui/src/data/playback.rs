use std::{fmt, sync::Arc, time::Duration};

use druid::{im::Vector, Data, Lens};
use druid_enums::Matcher;
use psst_core::item_id::ItemId;
use serde::{Deserialize, Serialize};

use super::{
    AlbumLink, ArtistLink, Episode, Library, Nav, PlaylistLink, RecommendationsRequest, ShowLink,
    Track,
};

#[derive(Clone, Data, Lens)]
pub struct Playback {
    pub state: PlaybackState,
    pub now_playing: Option<NowPlaying>,
    pub queue_behavior: QueueBehavior,
    pub queue: Vector<QueueEntry>,
    pub volume: f64,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct QueueEntry {
    pub item: Playable,
    pub origin: PlaybackOrigin,
}

#[derive(Clone, Debug, Data, Matcher)]
pub enum Playable {
    Track(Arc<Track>),
    Episode(Arc<Episode>),
}

impl Playable {
    pub fn track(&self) -> Option<&Arc<Track>> {
        if let Self::Track(track) = self {
            Some(track)
        } else {
            None
        }
    }

    pub fn id(&self) -> ItemId {
        match self {
            Playable::Track(track) => track.id.0,
            Playable::Episode(episode) => episode.id.0,
        }
    }

    pub fn name(&self) -> &Arc<str> {
        match self {
            Playable::Track(track) => &track.name,
            Playable::Episode(episode) => &episode.name,
        }
    }

    pub fn artist(&self) -> Arc<String> {
        match self {
            Playable::Track(track) => Arc::new(track.artist_names()),
            Playable::Episode(episode) => Arc::new(episode.show.name.to_string()),
        }
    }

    pub fn duration(&self) -> Duration {
        match self {
            Playable::Track(track) => track.duration,
            Playable::Episode(episode) => episode.duration,
        }
    }
}

#[derive(Default, Copy, Clone, Debug, Data, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueueBehavior {
    #[default]
    Sequential,
    Random,
    LoopTrack,
    LoopAll,
}

#[derive(Copy, Clone, Debug, Data, Eq, PartialEq)]
pub enum PlaybackState {
    Loading,
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Data, Lens)]
pub struct NowPlaying {
    pub item: Playable,
    pub origin: PlaybackOrigin,
    pub progress: Duration,

    // Although keeping a ref to the `Library` here is a bit of a hack, it dramatically
    // simplifies displaying the track context menu in the playback bar.
    pub library: Arc<Library>,
}

impl NowPlaying {
    pub fn cover_image_url(&self, width: f64, height: f64) -> Option<&str> {
        match &self.item {
            Playable::Track(track) => {
                let album = track.album.as_ref().or(match &self.origin {
                    PlaybackOrigin::Album(album) => Some(album),
                    _ => None,
                })?;
                Some(&album.image(width, height)?.url)
            }
            Playable::Episode(episode) => Some(&episode.image(width, height)?.url),
        }
    }
}

#[derive(Clone, Debug, Data)]
pub enum PlaybackOrigin {
    Home,
    Library,
    Queue,
    Album(AlbumLink),
    Artist(ArtistLink),
    Playlist(PlaylistLink),
    Show(ShowLink),
    Search(Arc<str>),
    Recommendations(Arc<RecommendationsRequest>),
}

impl PlaybackOrigin {
    pub fn to_nav(&self) -> Nav {
        match &self {
            PlaybackOrigin::Home => Nav::Home,
            PlaybackOrigin::Library => Nav::SavedTracks,
            // Change this
            PlaybackOrigin::Queue => Nav::Home,
            PlaybackOrigin::Album(link) => Nav::AlbumDetail(link.clone()),
            PlaybackOrigin::Artist(link) => Nav::ArtistDetail(link.clone()),
            PlaybackOrigin::Playlist(link) => Nav::PlaylistDetail(link.clone()),
            PlaybackOrigin::Show(link) => Nav::ShowDetail(link.clone()),
            PlaybackOrigin::Search(query) => Nav::SearchResults(query.clone()),
            PlaybackOrigin::Recommendations(request) => Nav::Recommendations(request.clone()),
        }
    }
}

impl fmt::Display for PlaybackOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            PlaybackOrigin::Home => f.write_str("Home"),
            PlaybackOrigin::Library => f.write_str("Saved Tracks"),
            PlaybackOrigin::Queue => f.write_str("Queue"),
            PlaybackOrigin::Album(link) => link.name.fmt(f),
            PlaybackOrigin::Artist(link) => link.name.fmt(f),
            PlaybackOrigin::Playlist(link) => link.name.fmt(f),
            PlaybackOrigin::Show(link) => link.name.fmt(f),
            PlaybackOrigin::Search(query) => query.fmt(f),
            PlaybackOrigin::Recommendations(_) => f.write_str("Recommended"),
        }
    }
}

#[derive(Clone, Debug, Data)]
pub struct PlaybackPayload {
    pub origin: PlaybackOrigin,
    pub items: Vector<Playable>,
    pub position: usize,
}
