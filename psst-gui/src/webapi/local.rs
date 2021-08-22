use std::{collections::{HashMap, hash_map::Entry}, fs::File, io, io::SeekFrom, io::prelude::*, str, sync::{Arc, Mutex}, time::Duration, vec::Vec};

use crate::data::{AlbumLink, ArtistLink, Image, Track, TrackId, config::Config};

use druid::im::Vector;
use once_cell::sync::OnceCell;
use serde::{Deserialize, de::Deserializer};
use serde_json::Value;

const MAGIC_BYTES: &[u8] = "SPCO".as_bytes();
const FILE_TYPE: &[u8] = "LocalFilesStorage".as_bytes();

const ARRAY_SIGNATURE: u8 = 0x60;
const STRING_SIGNATURE: u8 = 0x09;
const TRAILER_END: [u8; 2] = [0x78u8, 0x04u8];

#[derive(Clone, Debug)]
pub struct LocalTrack {
    title: String,
    path: String,
    album: String,
    artist: String
}

pub struct LocalTrackManager {
    tracks: HashMap<String, Vec<LocalTrack>>
}

/**
 * All local files registered by the Spotify file can be found in the file located at:
 *  <Spotify config>/Users/<username>-user/local-files.bnk
 * 
 * While this is not a complete reverse engineering of the way it is stored it suffices for
 * now. The file appears to be saved as a custom fork of Google's protobuf format
 * 
 * The file starts with "SPCO" as the magic followed by 0x13, 0x00*4 couldn't tell you
 * what these bytes do
 * 
 * After that there is 0x11 and "LocalFilesStorage". Note the 0x11 corresponds to the strings
 * length and I can only assume this is somehow used to load the correct proto definition
 * 
 * Interestingly enough spotify bnk files are chunked. After the above there is a little endian
 * encoded value of the number of bytes in a chunk maxing out at 0x1FE3. After exactly this many bytes
 * another chunk size will be present. This can even interrupt strings however it does not include the
 * 0x00, 0x00 that ends every file(I think)
 * 
 * Following the first chunk size is 0x60 which I think is used to signify an array in this protobuf language?
 * Doesn't matter too much. The number of elements in the array then follows encoded as a 
 * varint(https://developers.google.com/protocol-buffers/docs/encoding)
 * 
 * The bytes 0x94 0x00 seem to be present in every file I have used, I don't know what the
 * represent.
 * 
 * Then a track entry is presented as follows Title -> Artist -> Album -> Local Path -> Trailer.
 * I have no clue what the trailer represents other than directly after the Local path the bytes
 * 0x08 -> <varint encoding of track length>, not relevant so far. The title, artist, etc are each
 * encoded in the following format 0x09(string identifier) -> <varint string size> -> string
 * If the varint size is 0 this is a null string
 * 
 * The end of a trailer for a given track is signified by 0x78, 0x04 from what I can tell
 */

struct LocalChunkedReader {
    reader: File,
    chunk: Vec<u8>,
    read_in: usize
}

impl LocalChunkedReader {
    fn read_next_chunk(&mut self) -> io::Result<()> {
        let mut chunk_size_bytes = [0u8; 2];
        self.reader.read_exact(&mut chunk_size_bytes)?;

        let chunk_size = (chunk_size_bytes[1] as usize) << 8 | (chunk_size_bytes[0] as usize);
        let mut next_chunk = vec![0; chunk_size];
        self.reader.read_exact(&mut next_chunk)?;
        self.read_in += chunk_size + 2;

        self.chunk.append(&mut next_chunk);
        
        Ok(())
    }

    pub fn new(f: File) -> io::Result<Self> {
        let mut ret = LocalChunkedReader {
            reader: f,
            chunk: vec![0u8; 0],
            read_in: 0x1B
        };

        ret.read_next_chunk()?;
        Ok(ret)
    }

    pub fn get_pos(&mut self) -> usize {
        self.read_in - self.chunk.len()
    }

    pub fn read_exact(&mut self, size: usize) -> io::Result<Vec<u8>> {
        if size >= self.chunk.len() {
            self.read_next_chunk()?;
        }

        Ok(self.chunk.drain(0..size).collect())
    }

    pub fn read_varint(&mut self) -> io::Result<usize> {

        let mut shift: usize = 0;
        let mut ret : usize = 0;

        loop {
            let val = self.read_exact(1)?[0];
            let has_msb: bool = (val & !0b01111111) != 0;
            ret |= ((val & 0b01111111) as usize) << shift;

            if has_msb {
                shift += 7;
            } else {
                break;
            }
        }

        Ok(ret)
    }

    pub fn read_string_with_len(&mut self, len: usize) -> io::Result<String> {
        let str_bytes = self.read_exact(len)?;
        String::from_utf8(str_bytes).map_err(|_| io::Error::from(io::ErrorKind::Other))
    }

    pub fn read_string(&mut self) -> io::Result<String> {
        let magic = self.read_exact(1)?;

        if magic[0] != STRING_SIGNATURE {
            return Err(io::Error::from(io::ErrorKind::Other));
        }

        let str_size = self.read_varint()?;
        self.read_string_with_len(str_size)
    }

    pub fn advance(&mut self, size: usize) -> io::Result<()> {
        if size > self.chunk.len() {
            self.read_next_chunk()?;
        }

        let _drained = self.chunk.drain(0..size);
        Ok(())
    }

    pub fn read_until(&mut self, bytes: Vec<u8>) -> io::Result<()> {
        loop {
            match self.chunk.windows(bytes.len()).position(|window| window == bytes) {
                Some(pos) => {
                    self.chunk.drain(0..pos+bytes.len());
                    return Ok(());
                },
                _ => {
                    self.chunk.clear();
                    self.read_next_chunk()?;
                }
            }
        }
    }
}

/**
 * Spotify can do some weird stuff with local track APIs so serializing with serde requires
 * a good amount of workarounds. Basically the Tracks structs from the data crate with Option 
 * modifications to allow for null values
 */
#[derive(Clone, Debug, Deserialize)]
struct LocalAlbumLinkJSON {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]   
    pub id: Option<Arc<str>>,
    pub name: Arc<str>,
    #[serde(default)]
    pub images: Vector<Image>,
}

#[derive(Clone, Debug, Deserialize)]
struct LocalArtistLinkJSON {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]   
    pub id: Option<Arc<str>>,
    pub name: Arc<str>,
}


#[derive(Clone, Debug, Deserialize)]
struct LocalTrackJSON {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]    
    pub id: Option<TrackId>,
    pub name: Arc<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub album: Option<LocalAlbumLinkJSON>,
    pub artists: Vector<LocalArtistLinkJSON>,
    #[serde(rename = "duration_ms")]
    #[serde(deserialize_with = "deserialize_millis")]
    pub duration: Duration,
    pub disc_number: usize,
    pub track_number: usize,
    pub explicit: bool,
    pub is_local: bool,
    pub is_playable: Option<bool>,
    pub popularity: Option<u32>,
}

pub fn deserialize_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    let duration = Duration::from_millis(millis);
    Ok(duration)
}

static GLOBAL_LOCALTM: OnceCell<Mutex<LocalTrackManager>> = OnceCell::new();

impl LocalTrackManager {

    pub fn install_as_global(self) {
        GLOBAL_LOCALTM
            .set(Mutex::new(self))
            .map_err(|_| "Cannot install more than once")
            .unwrap()
    }

    pub fn global() -> &'static Mutex<Self> {
        GLOBAL_LOCALTM.get().unwrap()
    }

    pub fn new() -> Self {
        LocalTrackManager {
            tracks: HashMap::<String, Vec<LocalTrack>>::new()
        }
    }

    fn validate_file(f: &mut File) -> bool {
        let mut magic_buf = [0u8; 4];
        f.read_exact(&mut magic_buf).unwrap();

        if magic_buf != MAGIC_BYTES {
            return false;
        }

        if f.seek(SeekFrom::Start(0x9)).is_err() {
            return false;
        }

        let mut file_type: [u8; 0x12] = [0; 0x12];
        if f.read_exact(&mut file_type).is_err() 
                || file_type[0] != 0x11 || !file_type[1..].eq(FILE_TYPE) {
            return false;
        }

        return true;
    }

    pub fn read_new_user(&mut self, username: String) {
        // Clear all previous tracks
        self.tracks.clear();

        let file_path = match Config::spotify_local_files(username) {
            Some(p) => p,
            _ => return
        };

        let mut local_file_raw = match File::open(&file_path) {
            Ok(f) => f,
            _ => return
        };

        if !LocalTrackManager::validate_file(&mut local_file_raw) {
            log::warn!("Could not validate local file");
            return;
        }

        let mut chunked_reader = match LocalChunkedReader::new(local_file_raw) {
            Ok(r) => r,
            _ => return
        };

        let array_signature = chunked_reader.read_exact(1);
        if array_signature.is_err() || array_signature.unwrap()[0] != ARRAY_SIGNATURE {
            return
        }

        let num_tracks =  match chunked_reader.read_varint() {
            Ok(nt) => nt,
            _ => return
        };

        if chunked_reader.advance(2).is_err() {
            return;
        }

        for x in 1..=num_tracks {
            let title = match chunked_reader.read_string() {
                Ok(s) => s,
                _ => break
            };

            let artist = match chunked_reader.read_string() {
                Ok(s) => s,
                _ => break
            };

            let album = match chunked_reader.read_string() {
                Ok(s) => s,
                _ => break
            };

            let path = match chunked_reader.read_string() {
                Ok(s) => s,
                _ => break
            };

            let track = LocalTrack{
                title,
                path,
                album,
                artist,
            };

            match self.tracks.entry(track.title.clone()) {
                Entry::Vacant(e) => {e.insert(vec!(track));},
                Entry::Occupied(mut e) => {e.get_mut().push(track)}
            }

            if chunked_reader.read_until(TRAILER_END.to_vec()).is_err() {
                if x != num_tracks {
                    log::warn!("Found EOF but missing {missin} tracks...", missin=num_tracks-x);
                }
                break;
            }
        }
    }

    fn is_matching(t1: &LocalTrack, t2: &LocalTrackJSON) -> bool {
        // TODO: more checks on if a local track may return multiple artists from spotify's web facing API
        // if (has_artists => first artists is equal to local track) && (has_album => album equality) 
        !(!t2.artists.is_empty() && *t2.artists[0].name != t1.artist) 
                && !(t2.album.as_ref().is_some() && *t2.album.as_ref().unwrap().name != t1.album)
    }

    pub fn find_local_track(&self, track: Value) -> Option<Arc<Track>> {
        let local_track: LocalTrackJSON = match serde_json::from_value(track) {
            Ok(t) => t,
            Err(e) => {log::error!("Error parsing track {:?}", e); return None;}
        };

        let known_track = match self.tracks.get(&*local_track.name) {
            Some(kt) => kt,
            _ => return None
        };

        for check_track in known_track {
            if LocalTrackManager::is_matching(check_track, &local_track) {
                return Some(Arc::<Track>::new(Track {
                    id: TrackId::INVALID,
                    name: local_track.name,
                    album: Some(AlbumLink {
                        id: match local_track.album {
                            Some(ref e) => match &e.id {
                                Some(id) => id.clone(),
                                _ => Arc::from(check_track.album.clone())
                            },
                            _ => Arc::from("<null>")
                        },
                        name: match local_track.album {
                            Some(ref e) => e.name.clone(),
                            None => Arc::from("<Unknown>"),
                        },
                        images: match local_track.album {
                            Some(ref e) => e.images.clone(),
                            _ => Vector::<Image>::new(),
                        }
                    }),
                    artists: local_track.artists
                                    .into_iter()
                                    .map(|artist| ArtistLink {
                                        id: artist.id.unwrap_or(Arc::from(check_track.artist.clone())),
                                        name: artist.name
                                    })
                                    .collect(),
                    duration: local_track.duration,
                    disc_number: local_track.disc_number,
                    track_number: local_track.track_number,
                    explicit: local_track.explicit,
                    is_local: local_track.is_local,
                    local_path: Some(Arc::from(check_track.path.clone())),
                    // TODO: Change this to true once playback is supported
                    is_playable: Some(false),
                    popularity: local_track.popularity,
                }));
            }
        }

        None
    }
}