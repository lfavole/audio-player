use std::{collections::HashMap, fs::File, future::Future, io::{Cursor, Read, Seek}, path::Path, sync::Mutex};
use reqwest::{Client, Url};

/// Return the "real name" of a song, that is to say
/// the part after the song number, if there is one.
fn get_real_name(path: &str) -> Option<&str> {
    let mut name = path;

    // Keep only the basename (everything after the last slash)
    name = name.rsplit_once('/').map_or(name, | x | x.1);

    // Keep only the file name (everything before the last dot)
    name = name.rsplit_once('.').map_or(name, | x | x.0);

    // Return the real name (everything after the first underscore) or None
    name.split_once('_').map(| x | x.1)
}

pub type EBox = Box<dyn std::error::Error + Send + Sync>;
pub trait Song<'name>: Sized {
    async fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox>;
    fn get_path(&self) -> &'name str;

    fn get_real_name(&self) -> Option<&'name str> {
        get_real_name(self.get_path())
    }
    fn preload(&mut self) -> impl Future<Output = Result<(), EBox>> + Send + 'static {
        async {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct CompiledSong<'name: 'static> {
    pub path: &'name str,
    pub contents: &'static [u8],
}
impl<'name> CompiledSong<'name> {
    pub fn new(path: &'name str) -> Self {
        Self { path, contents: &[] }
    }
}
impl<'name> Song<'name> for CompiledSong<'name> {
    async fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        Ok(Cursor::new(self.contents))
    }
    fn get_path(&self) -> &'name str {
        self.path
    }
}

pub struct FileSong<'a> {
    pub path: &'a Path,
}
impl<'name> Song<'name> for FileSong<'name> {
    async fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        Ok(File::open(self.path)?)
    }
    fn get_path(&self) -> &'name str {
        self.path.to_str().unwrap()  // FIXME
    }
}

pub struct WebSong {
    pub url: Url,
    pub client: Client,
    pub data: Vec<u8>,
    preloading: Mutex<()>,
}
impl WebSong {
    pub fn new(url: Url, client: Client) -> Self {
        Self { url, client, data: vec![], preloading: Mutex::new(()) }
    }
}
impl Song<'_> for WebSong {
    async fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        self.preload().await?;
        Ok(Cursor::new(self.data.clone()))
    }
    async fn preload(&mut self) -> impl Future<Output = Result<(), EBox>> + Send + 'static {
        self.preloading.lock().unwrap();
        if !self.data.is_empty() {
            return Ok(());
        }
        let mut resp = self.client.get(self.url.clone()).send().await?;
        while let Some(chunk) = resp.chunk().await? {
            self.data.extend_from_slice(&chunk[..]);
        }
        Ok(())
    }
    fn get_path(&self) -> &'name str {
        self.url.as_str()
    }
}

/// Search for double songs.
/// Return a `HashMap` with the real song name as a key and the count as a value.
fn get_double_songs<'name>(files: &mut [impl Song<'name>]) -> HashMap<&'name str, usize> {
    let mut filenames: HashMap<&str, usize> = HashMap::with_capacity(files.len() / 2);
    for song in files {
        if let Some(real_name) = song.get_real_name() {
            *filenames.entry(real_name).or_default() += 1;
        }
    }
    filenames.into_iter().filter(| x | x.1 >= 2).collect::<HashMap<_, _>>()
}

/// Change the queue order to put double songs far from each other.
pub fn check_double_songs<'name>(files: &mut [impl Song<'name>]) {
    let double_songs = get_double_songs(files);
    // If there are no double songs, stop here
    if double_songs.is_empty() {
        return;
    }
    let length = files.len();

    'outer: loop {
        // Save the last positions of the double songs
        let mut double_songs_position: HashMap<&str, usize> = HashMap::new();

        for (i, song) in files.iter().enumerate() {
            // Check only double songs
            if let Some(real_name) = song.get_real_name() {
                if let Some(double_song_count) = double_songs.get(real_name) {
                    // We accept a distance of (length / double_song_count) ± 25%
                    // so the minimum distance is length / double_song_count * 0.75
                    let min_threshold = length * 3 / (double_song_count * 4);

                    // If there was previously a double song...
                    if let Some(position) = double_songs_position.get(real_name) {
                        // ...and the current double song too near from the previous one
                        if i - position < min_threshold {
                            for j in 0..length {
                                // If there is a song far enough and not another double song...
                                if j.abs_diff(i) >= min_threshold
                                && files[j].get_real_name().unwrap_or_default() != real_name {
                                    // Swap the songs...
                                    files.swap(i, j);
                                    // ...and move the last file to the start
                                    // (to avoid the cases where the first song is after the threshold)
                                    files.rotate_left(1);
                                    continue 'outer;
                                }
                            }
                        }
                    }
                    // Save the last position of the double song
                    double_songs_position.insert(real_name, i);
                }
            }
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::{check_double_songs, Song, CompiledSong};

    #[test]
    fn real_name() {
        assert_eq!(CompiledSong::new("test/00_a.mp3").get_real_name(), Some("a"));
        assert_eq!(CompiledSong::new("test/a/b/00_c.mp3").get_real_name(), Some("c"));
        assert_eq!(CompiledSong::new("test/00_a.test.mp3").get_real_name(), Some("a.test"));
        assert_eq!(CompiledSong::new("test/00_a_test.mp3").get_real_name(), Some("a_test"));

        assert_eq!(CompiledSong::new("test/00.mp3").get_real_name(), None);
        assert_eq!(CompiledSong::new("test/test.mp3").get_real_name(), None);
        assert_eq!(CompiledSong::new("test.mp3").get_real_name(), None);
    }

    #[test]
    fn double_songs() {
        let songs = &mut ["a", "b", "a", "c", "d", "e", "f"].map(CompiledSong::new);
        check_double_songs(&mut songs[..]);
        let paths: Vec<_> = songs.iter().map(| x | x.path).collect();
        assert_eq!(&paths, &["a", "b", "c", "a", "d", "e", "f"]);
    }
}
