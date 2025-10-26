use crate::cache::{cache_playlist, DeconstructedId, IdType};
use futures_util::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use preferences::{AppInfo, PreferencesError};
use rspotify::{
    model::{Id, PlayableId, PlaylistId},
    prelude::*,
    AuthCodeSpotify,
};

pub async fn get_playlist_list(
    spotify: &AuthCodeSpotify,
    limit: Option<u32>,
) -> (Vec<String>, Vec<PlaylistId<'_>>) {
    let mut names: Vec<String> = vec![];
    let mut ids: Vec<PlaylistId<'_>> = vec![];
    let mut current_offset = 0;
    let limit = limit.unwrap_or(5);

    loop {
        let playlists = spotify
            .current_user_playlists_manual(Some(limit), Some(current_offset))
            .await;

        match playlists {
            Ok(res) => {
                for item in res.items.clone() {
                    names.push(item.name);
                    ids.push(item.id);
                }

                // If the current batch is less than the limit, we're done.
                if res.items.len() < limit as usize {
                    break;
                }

                current_offset += limit; // Increment offset for the next batch
            }
            Err(_) => {
                current_offset += limit;
            }
        }
    }

    (names, ids)
}

pub async fn get_playlist_content<'a>(
    app_info: &AppInfo,
    spotify: &AuthCodeSpotify,
    playlist_id: PlaylistId<'a>,
) -> Result<Vec<PlayableId<'a>>, PreferencesError> {
    let mut playlist = spotify.playlist_items(playlist_id.clone(), None, None);
    let mut track_ids: Vec<PlayableId> = vec![];
    let mut tracks_serializable: Vec<DeconstructedId> = vec![];

    let playlist_length = spotify
        .playlist(playlist_id.clone(), None, None)
        .await
        .unwrap()
        .tracks
        .total;

    let fetch_items_bar = ProgressBar::new(playlist_length.into())
        .with_style(ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len}").unwrap())
        .with_message(String::from("[ ] Fetching playlist items..."));

    while let Some(item) = playlist.try_next().await.unwrap() {
        fetch_items_bar.inc(1);
        let Some(track) = item.track else { continue };
        let Some(id) = track.id() else { continue };
        track_ids.push(id.clone_static());
        let serializable_id = match id.clone_static() {
            PlayableId::Track(id) => DeconstructedId {
                id: String::from(id.id()),
                kind: IdType::Track,
            },
            PlayableId::Episode(id) => DeconstructedId {
                id: String::from(id.id()),
                kind: IdType::Episode,
            },
        };
        tracks_serializable.push(serializable_id);
    }

    cache_playlist(app_info, playlist_id.to_string(), tracks_serializable)?;

    fetch_items_bar.finish_with_message(String::from("[\u{2713}] Fetched playlist items"));
    Ok(track_ids)
}
