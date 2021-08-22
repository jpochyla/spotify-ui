use crate::{
    data::{
        Album, AlbumType, Artist, ArtistAlbums, AudioAnalysis, Cached, Nav, Page, Playlist, Range,
        Recommendations, RecommendationsRequest, SearchResults, SpotifyUrl, Track, UserProfile,
    },
    error::Error,
};
use druid::{
    im::Vector,
    image::{self, ImageFormat},
    Data,
};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use psst_core::{
    access_token::TokenProvider, session::SessionService, util::default_ureq_agent_builder,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    fmt::Display,
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};
use ureq::{Agent, Request, Response};

use super::{cache::WebApiCache, local::LocalTrackManager};

pub struct WebApi {
    session: SessionService,
    agent: Agent,
    cache: WebApiCache,
    token_provider: TokenProvider,
}

impl WebApi {
    pub fn new(
        session: SessionService,
        proxy_url: Option<&str>,
        cache_base: Option<PathBuf>
    ) -> Self {
        let agent = default_ureq_agent_builder(proxy_url).unwrap().build();
        Self {
            session,
            agent,
            cache: WebApiCache::new(cache_base),
            token_provider: TokenProvider::new(),
        }
    }

    fn access_token(&self) -> Result<String, Error> {
        let token = self
            .token_provider
            .get(&self.session)
            .map_err(|err| Error::WebApiError(err.to_string()))?;
        Ok(token.token)
    }

    fn request(&self, method: &str, path: impl Display) -> Result<Request, Error> {
        let token = self.access_token()?;
        let request = self
            .agent
            .request(method, &format!("https://api.spotify.com/{}", path))
            .set("Authorization", &format!("Bearer {}", &token));
        Ok(request)
    }

    fn get(&self, path: impl Display) -> Result<Request, Error> {
        self.request("GET", path)
    }

    fn put(&self, path: impl Display) -> Result<Request, Error> {
        self.request("PUT", path)
    }

    fn delete(&self, path: impl Display) -> Result<Request, Error> {
        self.request("DELETE", path)
    }

    fn with_retry(f: impl Fn() -> Result<Response, Error>) -> Result<Response, Error> {
        loop {
            let response = f()?;
            match response.status() {
                429 => {
                    let retry_after_secs = response
                        .header("Retry-After")
                        .and_then(|secs| secs.parse().ok())
                        .unwrap_or(2);
                    thread::sleep(Duration::from_secs(retry_after_secs));
                }
                _ => {
                    break Ok(response);
                }
            }
        }
    }

    /// Send a request with a empty JSON object, throw away the response body.
    /// Use for POST/PUT/DELETE requests.
    fn send_empty_json(&self, request: Request) -> Result<(), Error> {
        let _response = Self::with_retry(|| Ok(request.clone().send_string("{}")?))?;
        Ok(())
    }

    /// Send a request and return the deserialized JSON body.  Use for GET
    /// requests.
    fn load<T: DeserializeOwned>(&self, request: Request) -> Result<T, Error> {
        let response = Self::with_retry(|| Ok(request.clone().call()?))?;
        let result = response.into_json()?;
        Ok(result)
    }

    /// Send a request using `self.load()`, but only if it isn't already present
    /// in cache.
    fn load_cached<T: Data + DeserializeOwned>(
        &self,
        request: Request,
        bucket: &str,
        key: &str,
    ) -> Result<Cached<T>, Error> {
        if let Some(file) = self.cache.get(bucket, key) {
            let cached_at = file.metadata()?.modified()?;
            let value = serde_json::from_reader(file)?;
            Ok(Cached::cached(value, cached_at))
        } else {
            let response = Self::with_retry(|| Ok(request.clone().call()?))?;
            let body = {
                let mut reader = response.into_reader();
                let mut body = Vec::new();
                reader.read_to_end(&mut body)?;
                body
            };
            let value = serde_json::from_slice(&body)?;
            self.cache.set(bucket, key, &body);
            Ok(Cached::fresh(value))
        }
    }

    /// Load a paginated result set by sending `request` with added pagination
    /// parameters and return the aggregated results.  Use with GET requests.
    fn load_all_pages<T: DeserializeOwned + Clone>(
        &self,
        request: Request,
    ) -> Result<Vector<T>, Error> {
        // TODO: Some result sets, like very long playlists and saved tracks/albums can
        // be very big.  Implement virtualized scrolling and lazy-loading of results.
        const PAGED_ITEMS_LIMIT: usize = 200;

        let mut results = Vector::new();
        let mut limit = 50;
        let mut offset = 0;
        loop {
            let req = request
                .clone()
                .query("limit", &limit.to_string())
                .query("offset", &offset.to_string());
            let page: Page<T> = self.load(req)?;

            results.extend(page.items);

            if page.total > results.len() && results.len() < PAGED_ITEMS_LIMIT {
                limit = page.limit;
                offset = page.offset + page.limit;
            } else {
                break;
            }
        }
        Ok(results)
    }
}

static GLOBAL_WEBAPI: OnceCell<Arc<WebApi>> = OnceCell::new();

/// Global instance.
impl WebApi {
    pub fn install_as_global(self) {
        GLOBAL_WEBAPI
            .set(Arc::new(self))
            .map_err(|_| "Cannot install more than once")
            .unwrap()
    }

    pub fn global() -> Arc<Self> {
        GLOBAL_WEBAPI.get().unwrap().clone()
    }
}

/// Other endpoints.
impl WebApi {
    pub fn get_user_profile(&self) -> Result<UserProfile, Error> {
        let request = self.get("v1/me")?;
        let result = self.load(request)?;
        Ok(result)
    }
}

/// Artist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artist/
    pub fn get_artist(&self, id: &str) -> Result<Artist, Error> {
        let request = self.get(format!("v1/artists/{}", id))?;
        let result = self.load_cached(request, "artist", id)?;
        Ok(result.data)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artists-albums/
    pub fn get_artist_albums(&self, id: &str) -> Result<ArtistAlbums, Error> {
        let request = self
            .get(format!("v1/artists/{}/albums", id))?
            .query("market", "from_token");
        let result: Vector<Arc<Album>> = self.load_all_pages(request)?;

        let mut artist_albums = ArtistAlbums {
            albums: Vector::new(),
            singles: Vector::new(),
            compilations: Vector::new(),
            appears_on: Vector::new(),
        };
        for album in result {
            match album.album_type {
                AlbumType::Album => artist_albums.albums.push_back(album),
                AlbumType::Single => artist_albums.singles.push_back(album),
                AlbumType::Compilation => artist_albums.compilations.push_back(album),
                AlbumType::AppearsOn => artist_albums.appears_on.push_back(album),
            }
        }
        Ok(artist_albums)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-artists-top-tracks/
    pub fn get_artist_top_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Deserialize)]
        struct Tracks {
            tracks: Vector<Arc<Track>>,
        }

        let request = self
            .get(format!("v1/artists/{}/top-tracks", id))?
            .query("market", "from_token");
        let result: Tracks = self.load(request)?;
        Ok(result.tracks)
    }

    // https://developer.spotify.com/documentation/web-api/reference/artists/get-related-artists/
    pub fn get_related_artists(&self, id: &str) -> Result<Cached<Vector<Artist>>, Error> {
        #[derive(Clone, Data, Deserialize)]
        struct Artists {
            artists: Vector<Artist>,
        }

        let request = self.get(format!("v1/artists/{}/related-artists", id))?;
        let result: Cached<Artists> = self.load_cached(request, "related-artists", id)?;
        Ok(result.map(|result| result.artists))
    }
}

/// Album endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/albums/get-album/
    pub fn get_album(&self, id: &str) -> Result<Cached<Arc<Album>>, Error> {
        let request = self
            .get(format!("v1/albums/{}", id))?
            .query("market", "from_token");
        let result = self.load_cached(request, "album", id)?;
        Ok(result)
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-track
    pub fn get_track(&self, id: &str) -> Result<Arc<Track>, Error> {
        let request = self
            .get(format!("v1/tracks/{}", id))?
            .query("market", "from_token");
        let result = self.load(request)?;
        Ok(result)
    }
}

/// Library endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/library/get-users-saved-albums/
    pub fn get_saved_albums(&self) -> Result<Vector<Arc<Album>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedAlbum {
            album: Arc<Album>,
        }

        let request = self.get("v1/me/albums")?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedAlbum| item.album)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/save-albums-user/
    pub fn save_album(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/albums")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/remove-albums-user/
    pub fn unsave_album(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/albums")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/get-users-saved-tracks/
    pub fn get_saved_tracks(&self) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct SavedTrack {
            track: Arc<Track>,
        }

        let request = self.get("v1/me/tracks")?.query("market", "from_token");

        Ok(self
            .load_all_pages(request)?
            .into_iter()
            .map(|item: SavedTrack| item.track)
            .collect())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/save-tracks-user/
    pub fn save_track(&self, id: &str) -> Result<(), Error> {
        let request = self.put("v1/me/tracks")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }

    // https://developer.spotify.com/documentation/web-api/reference/library/remove-tracks-user/
    pub fn unsave_track(&self, id: &str) -> Result<(), Error> {
        let request = self.delete("v1/me/tracks")?.query("ids", id);
        self.send_empty_json(request)?;
        Ok(())
    }
}

/// View endpoints.
impl WebApi {
    pub fn get_made_for_you(&self) -> Result<Vector<Playlist>, Error> {
        #[derive(Deserialize)]
        struct View {
            content: Page<Playlist>,
        }

        let request = self
            .get("v1/views/made-for-x")?
            .query("types", "playlist")
            .query("limit", "20")
            .query("offset", "0");
        let result: View = self.load(request)?;
        Ok(result.content.items)
    }
}

/// Playlist endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-a-list-of-current-users-playlists
    pub fn get_playlists(&self) -> Result<Vector<Playlist>, Error> {
        let request = self.get("v1/me/playlists")?;
        let result = self.load_all_pages(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-playlist
    pub fn get_playlist(&self, id: &str) -> Result<Playlist, Error> {
        let request = self.get(format!("v1/me/playlists/{}", id))?;
        let result = self.load(request)?;
        Ok(result)
    }

    // https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-playlists-tracks
    pub fn get_playlist_tracks(&self, id: &str) -> Result<Vector<Arc<Track>>, Error> {
        #[derive(Clone, Deserialize)]
        struct PlaylistItem {
            is_local: bool,
            track: OptionalTrack,
        }

        // Spotify API likes to return _really_ bogus data for local tracks. Much better would
        // be to ignore parsing this completely if `is_local` is true, but this will do as
        // well.
        #[derive(Clone, Deserialize)]
        #[serde(untagged)]
        enum OptionalTrack {
            Track(Arc<Track>),
            Json(serde_json::Value),
        }

        let request = self
            .get(format!("v1/playlists/{}/tracks", id))?
            .query("marker", "from_token")
            .query("additional_types", "track");
        let result: Vector<PlaylistItem> = self.load_all_pages(request)?;

        let local_manager = LocalTrackManager::global().lock();

        Ok(result
            .into_iter()
            .filter_map(|item| match item {
                PlaylistItem {
                    is_local: _,
                    track: OptionalTrack::Track(track),
                } => Some(track),
                PlaylistItem {
                    is_local: _,
                    track: OptionalTrack::Json(track),
                } => match &local_manager {
                    Ok(l) => l.find_local_track(track),
                    _ => None
                },
            })
            .collect())
    }
}

/// Search endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/search/
    pub fn search(&self, query: &str) -> Result<SearchResults, Error> {
        #[derive(Deserialize)]
        struct ApiSearchResults {
            artists: Option<Page<Artist>>,
            albums: Option<Page<Arc<Album>>>,
            tracks: Option<Page<Arc<Track>>>,
            playlists: Option<Page<Playlist>>,
        }

        let request = self
            .get("v1/search")?
            .query("q", query)
            .query("type", "artist,album,track,playlist")
            .query("marker", "from_token");
        let result: ApiSearchResults = self.load(request)?;

        let artists = result.artists.map_or_else(Vector::new, |page| page.items);
        let albums = result.albums.map_or_else(Vector::new, |page| page.items);
        let tracks = result.tracks.map_or_else(Vector::new, |page| page.items);
        let playlists = result.playlists.map_or_else(Vector::new, |page| page.items);
        Ok(SearchResults {
            query: query.into(),
            artists,
            albums,
            tracks,
            playlists,
        })
    }

    pub fn load_spotify_link(&self, link: &SpotifyUrl) -> Result<Nav, Error> {
        let nav = match link {
            SpotifyUrl::Playlist(id) => Nav::PlaylistDetail(self.get_playlist(id)?.link()),
            SpotifyUrl::Artist(id) => Nav::ArtistDetail(self.get_artist(id)?.link()),
            SpotifyUrl::Album(id) => Nav::AlbumDetail(self.get_album(id)?.data.link()),
            SpotifyUrl::Track(id) => Nav::AlbumDetail(
                // TODO: We should highlight the exact track in the album.
                self.get_track(id)?.album.clone().ok_or_else(|| {
                    Error::WebApiError("Track was found but has no album".to_string())
                })?,
            ),
        };
        Ok(nav)
    }
}

/// Recommendation endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-recommendations
    pub fn get_recommendations(
        &self,
        data: Arc<RecommendationsRequest>,
    ) -> Result<Recommendations, Error> {
        let seed_artists = data.seed_artists.iter().map(|link| &link.id).join(", ");
        let seed_tracks = data
            .seed_tracks
            .iter()
            .map(|track| track.to_base62())
            .join(", ");

        let mut request = self
            .get("v1/recommendations")?
            .query("marker", "from_token")
            .query("limit", "100")
            .query("seed_artists", &seed_artists)
            .query("seed_tracks", &seed_tracks);

        fn add_range_param(mut req: Request, r: Range<impl ToString>, s: &str) -> Request {
            if let Some(v) = r.min {
                req = req.query(&format!("min_{}", s), &v.to_string());
            }
            if let Some(v) = r.max {
                req = req.query(&format!("max_{}", s), &v.to_string());
            }
            if let Some(v) = r.target {
                req = req.query(&format!("target_{}", s), &v.to_string());
            }
            req
        }
        request = add_range_param(request, data.params.duration_ms, "duration_ms");
        request = add_range_param(request, data.params.popularity, "popularity");
        request = add_range_param(request, data.params.key, "key");
        request = add_range_param(request, data.params.mode, "mode");
        request = add_range_param(request, data.params.tempo, "tempo");
        request = add_range_param(request, data.params.time_signature, "time_signature");
        request = add_range_param(request, data.params.acousticness, "acousticness");
        request = add_range_param(request, data.params.danceability, "danceability");
        request = add_range_param(request, data.params.energy, "energy");
        request = add_range_param(request, data.params.instrumentalness, "instrumentalness");
        request = add_range_param(request, data.params.liveness, "liveness");
        request = add_range_param(request, data.params.loudness, "loudness");
        request = add_range_param(request, data.params.speechiness, "speechiness");
        request = add_range_param(request, data.params.valence, "valence");

        let mut result: Recommendations = self.load(request)?;
        result.request = data;
        Ok(result)
    }
}

/// Track endpoints.
impl WebApi {
    // https://developer.spotify.com/documentation/web-api/reference/tracks/get-audio-analysis/
    pub fn _get_audio_analysis(&self, track_id: &str) -> Result<AudioAnalysis, Error> {
        let request = self.get(format!("v1/audio-analysis/{}", track_id))?;
        let result = self.load_cached(request, "audio-analysis", track_id)?;
        Ok(result.data)
    }
}

/// Image endpoints.
impl WebApi {
    pub fn get_image(&self, uri: &str) -> Result<image::DynamicImage, Error> {
        let response = self.agent.get(uri).call()?;
        let format = match response.content_type() {
            "image/jpeg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            _ => None,
        };
        let mut body = Vec::new();
        response.into_reader().read_to_end(&mut body)?;
        let image = if let Some(format) = format {
            image::load_from_memory_with_format(&body, format)?
        } else {
            image::load_from_memory(&body)?
        };
        Ok(image)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::WebApiError(err.to_string())
    }
}

impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Self {
        Error::WebApiError(err.to_string())
    }
}
