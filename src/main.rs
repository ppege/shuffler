use clap::Parser;
use dialoguer::Input;
use preferences::{AppInfo, Preferences, PreferencesMap};
use rand::distributions::{Alphanumeric, DistString};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rspotify::prelude::*;

const APP_INFO: AppInfo = AppInfo {
    name: "shuffler",
    author: "ppege",
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// ID of the playlist to shuffle
    #[arg(short, long)]
    pub from: Option<String>,

    /// Name of the playlist to generate
    #[arg(short, long)]
    pub to: Option<String>,

    /// Use a cached version of playlist if possible
    #[arg(short, long)]
    pub use_cache: Option<bool>,
}

#[macro_use]
mod macros;
mod auth;
mod cache;
mod spotify;
mod user_input;

use auth::get_authorized_session;
use cache::handle_cache;
use spotify::get_playlist_content;
use user_input::get_from_id;

use crate::cache::CachedPlaylist;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let args = Args::parse();

    let spotify = get_authorized_session(&APP_INFO).await.unwrap();

    let (name, id) = get_from_id(args.from.clone(), &spotify)
        .await
        .map_err(std::io::Error::other)?;

    let config = PreferencesMap::<CachedPlaylist>::load(&APP_INFO, "cache/playlists")
        .unwrap_or(PreferencesMap::<CachedPlaylist>::new());

    let mut track_ids = match handle_cache(&id.to_string(), args.use_cache, &config) {
        Some(ids) => ids,
        None => get_playlist_content(&APP_INFO, &spotify, id)
            .await
            .map_err(std::io::Error::other)?,
    };

    track_ids.shuffle(&mut thread_rng());
    const TRACK_LIMIT: usize = 100;
    track_ids.truncate(TRACK_LIMIT);

    let user = spotify.current_user().await.unwrap();

    let shuffle_name = args.to.unwrap_or(
        Input::new()
            .with_prompt("New playlist name?")
            .default(format!(
                "shuffler::{}",
                Alphanumeric.sample_string(&mut thread_rng(), 8)
            ))
            .interact_text()
            .unwrap(),
    );

    let shuffled_playlist = spinner!(
        async {
            spotify
                .user_playlist_create(
                    user.id,
                    &shuffle_name,
                    Some(false),
                    Some(false),
                    Some(&format!("True shuffle of {}, created with shuffler https://github.com/ppege/shuffler", &name)),
                )
                .await
                .unwrap()
        },
        "Creating new playlist...",
        "Created playlist"
    )
    .await;

    let _ = spinner!(
        async {
            spotify
                .playlist_add_items(shuffled_playlist.id, track_ids, Some(0))
                .await
                .unwrap()
        },
        "Filling playlist...",
        "Filled playlist"
    )
    .await;

    println!("[\u{2713}] {} shuffled", name);
    Ok(())
}

// async fn get_playlist_list(
//     spotify: &AuthCodeSpotify,
//     offset: Option<u32>,
// ) -> (Vec<String>, Vec<PlaylistId<'_>>) {
//     let limit = Some(50);
//     let playlists = spotify.current_user_playlists_manual(limit, offset).await;

//     let mut names: Vec<String> = vec![];
//     let mut ids: Vec<PlaylistId<'_>> = vec![];

//     if let Ok(res) = playlists {
//         for item in res.items {
//             names.push(item.name);
//             ids.push(item.id);
//         }
//     } else {
//         (names, ids) = get_playlist_list(spotify, Some(offset.unwrap_or(0) + limit.unwrap())).await;
//     }

//     (names, ids)
// }
