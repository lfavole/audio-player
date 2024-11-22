//! Play the songs from a specified URL.
use reqwest::{Client, Url};
use play::play_songs;
use song::{EBox, WebSong};
use tokio::task::JoinSet;

mod files;
mod play;
mod song;

const URL: &str = "http://127.0.0.1:8000/";

async fn get_files_and_folders(client: Client, url: Url) -> Result<(Vec<Url>, Vec<Url>), EBox> {
    let mut files = vec![];
    let mut folders = vec![];

    // Get the directory listing page
    let body = client.get(url.clone()).send().await?.text().await?;

    let mut last_index = 0;
    let pattern = "<a href=\"";
    let pattern_length = pattern.len();
    // Each time we find the pattern...
    while let Some(index) = body[last_index..].find(pattern) {
        // Add (don't set) because the index is relative to the new substring
        last_index += index + pattern_length;
        // If there is a closing quote...
        if let Some(end_index) = body[last_index..].find('"') {
            // Get the target URL
            let target_url = url.join(&body[last_index..last_index + end_index])?;
            // Save the URL to the files/folders list
            // Don't add parent/current folders
            if target_url.make_relative(&url).is_some_and(| x | !x.is_empty() && !x.starts_with("..")) {
                if target_url.path().ends_with('/') {
                    folders.push(target_url);
                } else {
                    files.push(target_url);
                }
            }
        }
    }

    Ok((files, folders))
}

async fn get_files(client: &reqwest::Client, url: Url) -> Result<Vec<Url>, EBox> {
    let mut files: Vec<Url> = vec![];
    let mut folders: Vec<Url> = vec![url.clone()];

    let mut set = JoinSet::new();

    loop {
        while !folders.is_empty() {
            set.spawn(get_files_and_folders(client.clone(), folders.remove(0)));
        }

        while let Some(res) = set.join_next().await {
            let (mut new_files, mut new_folders) = res??;
            files.append(&mut new_files);
            folders.append(&mut new_folders);
        }

        if folders.is_empty() {
            break;
        }
    }

    Ok(files)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder().build()?;
    let files = get_files(&client, Url::parse(URL)?).await?;

    let mut songs = files.iter().map(| x | WebSong::new(x, &client)).collect::<Vec<_>>();
    play_songs(&mut songs[..]).await
}
