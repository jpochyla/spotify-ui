use std::{convert::TryFrom, ops::Deref, str::FromStr, sync::Arc, time::Duration};

use druid::{im::Vector, lens::Map, Data, Lens};
use psst_core::item_id::{ItemId, ItemIdType};
use serde::Deserialize;

use crate::data::{AlbumLink, ArtistLink};

#[derive(Clone, Debug, Data, Lens, Deserialize)]
pub struct Track {
    #[serde(default)]
    pub id: TrackId,
    pub name: Arc<str>,
    pub album: Option<AlbumLink>,
    pub artists: Vector<ArtistLink>,
    #[serde(rename = "duration_ms")]
    #[serde(deserialize_with = "super::utils::deserialize_millis")]
    pub duration: Duration,
    pub disc_number: usize,
    pub track_number: usize,
    pub explicit: bool,
    pub is_local: bool,
    #[serde(skip_deserializing)]
    pub local_path: Option<Arc<str>>,
    pub is_playable: Option<bool>,
    pub popularity: Option<u32>,
}

impl Track {
    pub fn lens_artist_name() -> impl Lens<Self, Arc<str>> {
        Map::new(
            |track: &Self| track.artist_name(),
            |_, _| {
                // Immutable.
            },
        )
    }

    pub fn lens_album_name() -> impl Lens<Self, Arc<str>> {
        Map::new(
            |track: &Self| track.album_name(),
            |_, _| {
                // Immutable.
            },
        )
    }

    pub fn artist_name(&self) -> Arc<str> {
        self.artists
            .front()
            .map(|artist| artist.name.clone())
            .unwrap_or_else(|| "Unknown".into())
    }

    pub fn album_name(&self) -> Arc<str> {
        self.album
            .as_ref()
            .map(|album| album.name.clone())
            .unwrap_or_else(|| "Unknown".into())
    }

    pub fn url(&self) -> String {
        format!("https://open.spotify.com/track/{}", self.id.to_base62())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct TrackId(ItemId);

impl TrackId {
    pub const INVALID: Self = Self(ItemId::new(0u128, ItemIdType::Unknown));
}

impl Default for TrackId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Data for TrackId {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for TrackId {
    type Target = ItemId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ItemId> for TrackId {
    fn from(id: ItemId) -> Self {
        Self(id)
    }
}

impl FromStr for TrackId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(id) = ItemId::from_base62(s, ItemIdType::Track) {
            Ok(Self(id))
        } else {
            Err("Invalid track ID")
        }
    }
}

impl TryFrom<String> for TrackId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

#[derive(Clone, Data, Debug, Deserialize)]
pub struct AudioAnalysis {
    pub segments: Vector<AudioSegment>,
}

#[derive(Clone, Data, Debug, Deserialize)]
pub struct AudioSegment {
    #[serde(flatten)]
    pub interval: TimeInterval,
    pub loudness_start: f64,
    pub loudness_max: f64,
    pub loudness_max_time: f64,
}

#[derive(Clone, Data, Debug, Deserialize)]
pub struct TimeInterval {
    #[serde(deserialize_with = "super::utils::deserialize_secs")]
    pub start: Duration,
    #[serde(deserialize_with = "super::utils::deserialize_secs")]
    pub duration: Duration,
    pub confidence: f64,
}
