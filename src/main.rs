use std::time::Duration;
use rspotify::{model::PlaylistId, prelude::*, ClientCredsSpotify, Credentials};

#[derive(Debug)]
struct Song {
    title: String,
    artist: String,
    identifier: String,
    album: String,
    length: Duration,
}

impl Default for Song {
    fn default() -> Self {
        Song {
            title: String::from("forever"),
            artist: String::from("Charli xcx"),
            identifier: String::from("5GsJIVCBFjhCcUwJaTW2sB"),
            album: String::from("how i'm feeling now"),
            length: Duration::from_secs(243),
        }
    }
}

type Playlist = Vec<Song>;

#[tokio::main]
async fn main() {
    env_logger::init();
    
    let creds = Credentials::from_env().unwrap();
    let spotify = ClientCredsSpotify::new(creds);
    spotify.request_token().await.unwrap();
    
    let playlist_uri = PlaylistId::from_uri("spotify:playlist:4ySVKEWGGLRLJMR4tw33no").unwrap();
    let playlist = spotify.playlist(playlist_uri, None, None).await;

    println!("Response: {playlist:#?}"); // this response is 2 mB and 44k lines of json... it will be a piece of work to implement my algorithm, especially having it hook into the Spotify user to be able to track skips and reassign points based on this.
   
    // let my_playlist: Playlist = vec![
    //     Song::default(),
    //     Song {
    //         title: String::from("anthems"),
    //         ..Song::default()
    //     },
    // ];
    // dbg!(my_playlist);
}
