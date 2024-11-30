use anyhow::{Context, Result};
use console::Term;
use dialoguer::FuzzySelect;
use futures::stream::TryStreamExt;
use futures_util::pin_mut;
use indicatif::{ProgressBar, ProgressStyle};
use rand::distributions::{Alphanumeric, DistString};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rspotify::{model::PlaylistId, prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};
use std::time::Duration;

#[macro_export]
macro_rules! spinner {
    ($fn:expr, $message:expr, $finish_message:expr) => {{
        async {
            let spinner_style = ProgressStyle::with_template("{wide_msg} {spinner}").unwrap();
            let tick_duration = Duration::from_millis(100);
            let spinner = ProgressBar::new_spinner()
                .with_style(spinner_style.clone())
                .with_message(format!("[ ] {}", $message));
            spinner.enable_steady_tick(tick_duration);
            let result = $fn.await;
            spinner.finish_with_message(format!("[\u{2713}] {}", $finish_message));
            result
        }
    }};
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let term = Term::stdout();

    let spotify = get_authorized_session().await.unwrap();

    let (names, ids) = spinner!(
        get_playlist_list(&spotify),
        "Fetching user playlists...",
        "Fetched user playlists"
    )
    .await;

    let (selected_playlist_name, selected_playlist_id) = select_playlist(names, ids);

    let mut track_ids = get_playlist_content(&spotify, selected_playlist_id).await;
    track_ids.shuffle(&mut thread_rng());
    track_ids.truncate(100); // Spotify API has a limit of 100 tracks when adding songs to a playlist

    let user = spotify.current_user().await.unwrap();

    let shuffle_name = format!(
        "generated shuffle {}",
        Alphanumeric.sample_string(&mut thread_rng(), 6)
    );

    let shuffled_playlist = spinner!(
        async {
            spotify
                .user_playlist_create(
                    user.id,
                    &shuffle_name,
                    Some(false),
                    Some(false),
                    Some("Created with openSHUFFLE https://github.com/ppege/openSHUFFLE"),
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

    term.write_line(&format!("[\u{2713}] {} shuffled", selected_playlist_name))
        .context("Couldn't write success message to terminal. Program succeeded anyway... process failed successfully?").unwrap();
}

async fn get_authorized_session() -> Result<AuthCodeSpotify> {
    let creds = Credentials::from_env().context("Failed to get Spotify API credentials from environment variables. Make sure the environment variables RSPOTIFY_CLIENT_ID, RSPOTIFY_CLIENT_SECRET, and RSPOTIFY_REDIRECT_URL are set in accordance to your Spotify application.")?;
    let oauth = OAuth::from_env(scopes!(
        "playlist-read-private",
        "playlist-read-collaborative",
        "playlist-modify-private",
        "playlist-modify-public"
    ))
    .context("Your Spotify application seems to be missing permissions.")?;
    let mut spotify = AuthCodeSpotify::new(creds, oauth);
    // spotify.config.token_cached = true;

    // Obtaining the access token
    let url = spotify
        .get_authorize_url(false)
        .context("Failed to get auth URL. Check your internet connection.")?;

    // This function requires the `cli` feature enabled.
    spotify.prompt_for_token(&url).await.context("Failed to login. Make sure you pasted the full URL from your browser, with the URI parameters.")?;
    Ok(spotify)
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

async fn get_playlist_list(spotify: &AuthCodeSpotify) -> (Vec<String>, Vec<PlaylistId<'_>>) {
    let playlists = spotify.current_user_playlists();
    pin_mut!(playlists);

    let mut names: Vec<String> = vec![];
    let mut ids: Vec<PlaylistId<'_>> = vec![];

    while let Some(item) = playlists.try_next().await.unwrap() {
        names.push(item.name);
        ids.push(item.id);
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
