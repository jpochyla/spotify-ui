use std::io::{BufReader, BufWriter};
use std::{env, env::VarError, fs::File, path::PathBuf};

use std::fs::OpenOptions;
#[cfg(target_family = "unix")]
use std::os::unix::fs::OpenOptionsExt;

use druid::{Data, Lens, Size};
use platform_dirs::AppDirs;
use psst_core::{
    cache::mkdir_if_not_exists,
    connection::Credentials,
    player::PlaybackConfig,
    session::{SessionConfig, SessionConnection},
};
use serde::{Deserialize, Serialize};

use crate::ui::theme;

use super::{Nav, Promise, QueueBehavior, SliderScrollScale};

#[derive(Clone, Debug, Data, Lens)]
pub struct Preferences {
    pub active: PreferencesTab,
    pub cache_size: Promise<u64, (), ()>,
    pub auth: Authentication,
}

impl Preferences {
    pub fn reset(&mut self) {
        self.cache_size.clear();
        self.auth.result.clear();
    }

    pub fn measure_cache_usage() -> Option<u64> {
        Config::cache_dir().and_then(|path| fs_extra::dir::get_size(path).ok())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Data)]
pub enum PreferencesTab {
    General,
    Account,
    Cache,
    About,
}

#[derive(Clone, Debug, Data, Lens)]
pub struct Authentication {
    pub username: String,
    pub password: String,
    pub access_token: String,
    pub result: Promise<(), (), String>,
}

impl Authentication {
    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            login_creds: if !self.access_token.is_empty() {
                Credentials::from_access_token(self.access_token.clone())
            } else {
                Credentials::from_username_and_password(
                    self.username.clone(),
                    self.password.clone(),
                )
            },
            proxy_url: Config::proxy(),
        }
    }

    pub fn authenticate_and_get_credentials(config: SessionConfig) -> Result<Credentials, String> {
        let connection = SessionConnection::open(config).map_err(|err| err.to_string())?;
        Ok(connection.credentials)
    }

    pub fn clear(&mut self) {
        self.username.clear();
        self.password.clear();
    }
}

const APP_NAME: &str = "Psst";
const CONFIG_FILENAME: &str = "config.json";
const PROXY_ENV_VAR: &str = "SOCKS_PROXY";

#[derive(Clone, Debug, Data, Lens, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[data(ignore)]
    credentials: Option<Credentials>,
    pub audio_quality: AudioQuality,
    pub theme: Theme,
    pub volume: f64,
    pub last_route: Option<Nav>,
    pub queue_behavior: QueueBehavior,
    pub show_track_cover: bool,
    pub show_queue_view: bool,
    pub window_size: Size,
    pub slider_scroll_scale: SliderScrollScale,
    pub sort_order: SortOrder,
    pub sort_criteria: SortCriteria,
    pub paginated_limit: usize,
    pub seek_duration: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credentials: Default::default(),
            audio_quality: Default::default(),
            theme: Default::default(),
            volume: 1.0,
            last_route: Default::default(),
            queue_behavior: Default::default(),
            show_track_cover: Default::default(),
            show_queue_view: Default::default(),
            window_size: Size::new(theme::grid(80.0), theme::grid(100.0)),
            slider_scroll_scale: Default::default(),
            sort_order: Default::default(),
            sort_criteria: Default::default(),
            paginated_limit: 500,
            seek_duration: 10,
        }
    }
}

impl Config {
    fn app_dirs() -> Option<AppDirs> {
        const USE_XDG_ON_MACOS: bool = false;

        AppDirs::new(Some(APP_NAME), USE_XDG_ON_MACOS)
    }

    pub fn spotify_local_files_file(username: &str) -> Option<PathBuf> {
        AppDirs::new(Some("spotify"), false).map(|dir| {
            let path = format!("Users/{}-user/local-files.bnk", username);
            dir.config_dir.join(path)
        })
    }

    pub fn cache_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.cache_dir)
    }

    pub fn config_dir() -> Option<PathBuf> {
        Self::app_dirs().map(|dirs| dirs.config_dir)
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join(CONFIG_FILENAME))
    }

    pub fn load() -> Option<Config> {
        let path = Self::config_path().expect("Failed to get config path");
        if let Ok(file) = File::open(&path) {
            log::info!("loading config: {:?}", &path);
            let reader = BufReader::new(file);
            Some(serde_json::from_reader(reader).expect("Failed to read config"))
        } else {
            None
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir().expect("Failed to get config dir");
        let path = Self::config_path().expect("Failed to get config path");
        mkdir_if_not_exists(&dir).expect("Failed to create config dir");

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(target_family = "unix")]
        options.mode(0o600);

        let file = options.open(&path).expect("Failed to create config");
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).expect("Failed to write config");
        log::info!("saved config: {:?}", &path);
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    pub fn store_credentials(&mut self, credentials: Credentials) {
        self.credentials.replace(credentials);
    }

    pub fn clear_credentials(&mut self) {
        self.credentials = Default::default();
    }

    pub fn username(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .and_then(|c| c.username.as_deref())
    }

    pub fn session(&self) -> SessionConfig {
        SessionConfig {
            login_creds: self.credentials.clone().expect("Missing credentials"),
            proxy_url: Config::proxy(),
        }
    }

    pub fn playback(&self) -> PlaybackConfig {
        PlaybackConfig {
            bitrate: self.audio_quality.as_bitrate(),
            ..PlaybackConfig::default()
        }
    }

    pub fn proxy() -> Option<String> {
        env::var(PROXY_ENV_VAR).map_or_else(
            |err| match err {
                VarError::NotPresent => None,
                VarError::NotUnicode(_) => {
                    log::error!("proxy URL is not a valid unicode");
                    None
                }
            },
            Some,
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum AudioQuality {
    Low,
    Normal,
    High,
}

impl AudioQuality {
    fn as_bitrate(self) -> usize {
        match self {
            AudioQuality::Low => 96,
            AudioQuality::Normal => 160,
            AudioQuality::High => 320,
        }
    }
}

impl Default for AudioQuality {
    fn default() -> Self {
        Self::High
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}
impl Default for SortOrder {
    fn default() -> Self {
        Self::Ascending
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data, Serialize, Deserialize)]
pub enum SortCriteria {
    Title,
    Artist,
    Album,
    Duration,
    DateAdded,
}
impl Default for SortCriteria {
    fn default() -> Self {
        Self::DateAdded
    }
}
