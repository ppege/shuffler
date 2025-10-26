use std::{error::Error, fmt::Display};
use crate::spotify::get_playlist_list;
use dialoguer::{FuzzySelect, Input};
use rspotify::{model::PlaylistId, AuthCodeSpotify, prelude::*, ClientError};

pub fn select_playlist<'a>(
    names: Vec<String>,
    ids: Vec<PlaylistId<'a>>,
) -> (String, PlaylistId<'a>) {
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

#[derive(Debug)]
pub enum UserFuckYouError {
    InvalidId,
    UnavailableInfoWtfDude(ClientError)
}

impl Display for UserFuckYouError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId => write!(f, "InvalidId"),
            Self::UnavailableInfoWtfDude(e) => write!(f, "UnavailableInfoWtfDude({e})")
        }
    }
}

impl Error for UserFuckYouError {}

pub async fn get_from_id<'a>(id: Option<String>, spotify: &'a AuthCodeSpotify) -> Result<(String, PlaylistId<'a>), UserFuckYouError> {
    match id {
        Some(id) => {
            let playlist_id = PlaylistId::from_id(id.clone()).map_err(|_| 
                UserFuckYouError::InvalidId
            )?;
            let playlist_name = spotify
                .playlist(playlist_id.clone(), None, None)
                .await
                .map_err(UserFuckYouError::UnavailableInfoWtfDude)?
                .name;
            Ok((playlist_name, playlist_id))
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
                get_playlist_list(spotify, Some(limit)),
                "Fetching user playlists...",
                "Fetched user playlists"
            )
            .await;

            Ok(select_playlist(names, ids))
        }
    }
}
