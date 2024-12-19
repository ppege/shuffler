use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::{FuzzySelect, Input};
use futures::stream::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use preferences::{AppInfo, Preferences, PreferencesMap};
use rand::distributions::{Alphanumeric, DistString};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rspotify::{model::PlaylistId, prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};
use std::time::Duration;

const APP_INFO: AppInfo = AppInfo {
    name: "shuffler",
    author: "ppege",
};

#[macro_export]
macro_rules! spinner {
    ($fn:expr, $message:expr, $finish_message:expr) => {
        async {
            let spinner_style = ProgressStyle::with_template("{msg} {spinner}").unwrap();
            let tick_duration = Duration::from_millis(100);
            let spinner = ProgressBar::new_spinner()
                .with_style(spinner_style.clone())
                .with_message($message);
            spinner.enable_steady_tick(tick_duration);
            let result = $fn.await;
            spinner.finish_with_message(format!("[\u{2713}] {}", $finish_message));
            result
        }
    };
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// ID of the playlist to shuffle
    #[arg(short, long)]
    from: Option<String>,

    /// Name of the playlist to generate
    #[arg(short, long)]
    to: Option<String>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let spotify = get_authorized_session().await.unwrap();

    let (name, id) = get_from_id(args.from.clone(), &spotify).await;

    let mut track_ids = get_playlist_content(&spotify, id).await;
    track_ids.shuffle(&mut thread_rng());
    track_ids.truncate(100); // Spotify API has a limit of 100 tracks when adding songs to a playlist

    let user = spotify.current_user().await.unwrap();

    let shuffle_name = match args.to {
        Some(name) => name,
        None => Input::new()
            .with_prompt("New playlist name?")
            .default(format!(
                "shuffler::{}",
                Alphanumeric.sample_string(&mut thread_rng(), 8)
            ))
            .interact_text()
            .unwrap(),
    };

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
}

fn initialize_config() -> (Credentials, String) {
    let mut config = PreferencesMap::new();
    println!("You're running shuffler for the first time! In order to use this tool, you have to create a Spotify app, which you can do at https://developer.spotify.com/dashboard. Once you've done this, provide the client ID and secret here.");
    config.insert(
        "id".into(),
        Input::<String>::new()
            .with_prompt("Enter the client ID")
            .interact_text()
            .unwrap(),
    );
    config.insert(
        "secret".into(),
        Input::<String>::new()
            .with_prompt("Enter the client secret")
            .interact_text()
            .unwrap(),
    );
    config.insert(
        "redirect_uri".into(),
        Input::<String>::new()
            .with_prompt("Enter the redirect URI (this can be anything, really)")
            .interact_text()
            .unwrap(),
    );
    let _ = config.save(&APP_INFO, "preferences/credentials");
    (
        Credentials::new(&config["id"], &config["secret"]),
        config["redirect_uri"].clone(),
    )
}

async fn get_authorized_session() -> Result<AuthCodeSpotify> {
    let (creds, redirect_uri) =
        match PreferencesMap::<String>::load(&APP_INFO, "preferences/credentials") {
            Ok(config) => {
                if let (Some(id), Some(secret), Some(redirect_uri)) = (
                    config.get("id"),
                    config.get("secret"),
                    config.get("redirect_uri"),
                ) {
                    (Credentials::new(&id, &secret), redirect_uri.clone())
                } else {
                    initialize_config()
                }
            }
            Err(_) => initialize_config(),
        };

    let oauth = OAuth {
        redirect_uri,
        state: OAuth::default().state,
        scopes: scopes!(
            "playlist-read-private",
            "playlist-read-collaborative",
            "playlist-modify-private",
            "playlist-modify-public"
        ),
        proxies: None,
    };
    // let oauth = OAuth::from_env(scopes!(
    //     "playlist-read-private",
    //     "playlist-read-collaborative",
    //     "playlist-modify-private",
    //     "playlist-modify-public"
    // ))
    // .context("Your Spotify application seems to be missing permissions.")?;
    let mut spotify = AuthCodeSpotify::new(creds, oauth);
    spotify.config.token_cached = true;

    let url = spotify
        .get_authorize_url(false)
        .context("Failed to get auth URL. Check your internet connection.")?;

    spotify.prompt_for_token(&url).await.context("Failed to login. Make sure you pasted the full URL from your browser, with the URI parameters.")?;
    Ok(spotify)
}

async fn get_from_id(id: Option<String>, spotify: &AuthCodeSpotify) -> (String, PlaylistId) {
    match id {
        Some(id) => {
            let playlist_id = PlaylistId::from_id(id.clone()).expect("Invalid playlist ID.");
            let playlist_name = spotify
                .playlist(playlist_id.clone(), None, None)
                .await
                .expect("Could not get source playlist info.")
                .name;
            (playlist_name, playlist_id)
        }
        None => {
            println!("Specify a limit below 50 for the user playlist fetch.\nIf you don't see the playlist you wish to shuffle after this, abort with ^C and try a lower limit.");

            let limit: u32 = Input::<u32>::new()
                .with_prompt("Choose a limit")
                .default(5)
                .report(false)
                .interact_text()
                .unwrap_or(5);

            let (names, ids) = spinner!(
                get_playlist_list(&spotify, Some(limit)),
                "Fetching user playlists...",
                "Fetched user playlists"
            )
            .await;

            select_playlist(names, ids)
        }
    }
}

fn select_playlist<'a>(names: Vec<String>, ids: Vec<PlaylistId<'a>>) -> (String, PlaylistId<'a>) {
    let selection = FuzzySelect::new()
        .with_prompt("Which playlist to shuffle?")
        .items(&names)
        .interact()
        .unwrap();

    (
        names.get(selection).unwrap().clone(),
        ids.get(selection).unwrap().clone(),
    )
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

async fn get_playlist_list(
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
                // eprintln!(
                //     "Error fetching playlists at offset {}, skipping to next batch",
                //     current_offset
                // );
                current_offset += limit;
            }
        }
    }

    (names, ids)
}

async fn get_playlist_content<'a>(
    spotify: &AuthCodeSpotify,
    playlist_id: PlaylistId<'_>,
) -> Vec<PlayableId<'a>> {
    let mut playlist = spotify.playlist_items(playlist_id.clone(), None, None);
    let mut track_ids: Vec<PlayableId> = vec![];

    let playlist_length = spotify
        .playlist(playlist_id, None, None)
        .await
        .unwrap()
        .tracks
        .total;

    let fetch_items_bar = ProgressBar::new(playlist_length.into())
        .with_style(ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len}").unwrap())
        .with_message(String::from("[ ] Fetching playlist items..."));

    while let Some(item) = playlist.try_next().await.unwrap() {
        fetch_items_bar.inc(1);
        if let Some(track) = item.track {
            if let Some(id) = track.id() {
                track_ids.push(id.clone_static());
            }
        }
    }

    fetch_items_bar.finish_with_message(String::from("[\u{2713}] Fetched playlist items"));
    track_ids
}
