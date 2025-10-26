use anyhow::{Context, Result};
use dialoguer::Input;
use preferences::{AppInfo, Preferences, PreferencesMap};
use rspotify::Credentials;
use rspotify::{prelude::*, scopes, AuthCodeSpotify, OAuth};

pub async fn get_authorized_session(app_info: &AppInfo) -> Result<AuthCodeSpotify> {
    let (creds, redirect_uri) =
        match PreferencesMap::<String>::load(app_info, "preferences/credentials") {
            Ok(config) => {
                if let (Some(id), Some(secret), Some(redirect_uri)) = (
                    config.get("id"),
                    config.get("secret"),
                    config.get("redirect_uri"),
                ) {
                    (Credentials::new(id, secret), redirect_uri.clone())
                } else {
                    initialize_config(app_info)
                }
            }
            Err(_) => initialize_config(app_info),
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

    let mut spotify = AuthCodeSpotify::new(creds, oauth);
    spotify.config.token_cached = true;

    let url = spotify
        .get_authorize_url(false)
        .context("Failed to get auth URL. Check your internet connection.")?;

    spotify.prompt_for_token(&url).await.context("Failed to login. Make sure you pasted the full URL from your browser, with the URI parameters.")?;
    Ok(spotify)
}

fn initialize_config(app_info: &AppInfo) -> (Credentials, String) {
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
    let _ = config.save(app_info, "preferences/credentials");
    (
        Credentials::new(&config["id"], &config["secret"]),
        config["redirect_uri"].clone(),
    )
}
