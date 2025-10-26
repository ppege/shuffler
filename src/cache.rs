use chrono::{DateTime, Utc};
use dialoguer::Confirm;
use humantime::format_duration;
use preferences::{AppInfo, Preferences, PreferencesError, PreferencesMap};
use rspotify::model::{EpisodeId, PlayableId, TrackId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum IdType {
    Track,
    Episode,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CachedPlaylist {
    pub ids: Vec<DeconstructedId>,
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DeconstructedId {
    pub id: String,
    pub kind: IdType,
}

pub fn handle_cache<'a>(
    playlist_id: &str,
    use_cache: Option<bool>,
    config: &'a HashMap<String, CachedPlaylist>,
) -> Option<Vec<PlayableId<'a>>> {
    let cached_playlist = config.get(playlist_id)?;
    let age = Duration::from_secs(
        Utc::now()
            .signed_duration_since(cached_playlist.time)
            .num_seconds() as u64,
    );
    let use_cache = use_cache.unwrap_or(
        Confirm::new()
            .with_prompt(format!(
                "A cached version of this playlist from {} ago was found. Use this?",
                format_duration(age)
            ))
            .interact()
            .unwrap(),
    );
    if use_cache {
        Some(
            cached_playlist
                .ids
                .iter()
                .map(|id| match id.kind {
                    IdType::Episode => EpisodeId::from_id(&id.id).unwrap().into(),
                    IdType::Track => TrackId::from_id(&id.id).unwrap().into(),
                })
                .collect(),
        )
    } else {
        None
    }
}

pub fn cache_playlist(
    app_info: &AppInfo,
    playlist_id: String,
    ids: Vec<DeconstructedId>,
) -> Result<(), PreferencesError> {
    let mut config = PreferencesMap::<CachedPlaylist>::load(app_info, "cache/playlists")
        .unwrap_or(PreferencesMap::<CachedPlaylist>::new());

    config.insert(
        playlist_id,
        CachedPlaylist {
            ids,
            time: Utc::now(),
        },
    );

    config.save(app_info, "cache/playlists")
}
