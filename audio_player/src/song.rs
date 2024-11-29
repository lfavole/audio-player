//! Structures representing songs.
use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Seek},
    path::Path,
    sync::Mutex,
};
use ureq::Agent;
use url::Url;

/// The `Box` type that contains `Error`s.
pub type EBox = Box<dyn std::error::Error + Send + Sync>;

/// Return the "real name" of a song, that is to say
/// the part after the song number, if there is one.
pub fn get_real_name(path: &str) -> Option<&str> {
    let mut name = path;

    // Keep only the basename (everything after the last slash)
    name = name.rsplit_once('/').map_or(name, |x| x.1);

    // Keep only the file name (everything before the last dot)
    name = name.rsplit_once('.').map_or(name, |x| x.0);

    // Return the real name (everything after the first underscore) or None
    name.split_once('_').map(|x| x.1)
}

/// A song that has data and a path.
pub trait Song<'name>: Sized {
    /// Return the song data.
    ///
    /// # Errors
    /// Fails if the song cannot be fetched.
    fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox>;
    /// Return the song path.
    fn get_path(&self) -> &'name str;

    /// Return the "real name" of the song, depending on the path.
    /// (See [`get_real_name`])
    fn get_real_name(&self) -> Option<&'name str> {
        get_real_name(self.get_path())
    }
    /// Download the song data so it will be available immediatly later.
    ///
    /// # Errors
    /// Fails if the song cannot be preloaded (i.e. cannot be fetched).
    fn preload(&mut self) -> Result<(), EBox> {
        Ok(())
    }
}

/// A song whose name is the real name.
pub struct TestSong<'name> {
    /// The song name.
    pub name: &'name str,
}
impl<'name> Song<'name> for TestSong<'name> {
    fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        Ok(Cursor::new(""))
    }
    fn get_path(&self) -> &'name str {
        self.name
    }
    fn get_real_name(&self) -> Option<&'name str> {
        Some(self.name)
    }
}
impl<'name> From<&'name str> for TestSong<'name> {
    fn from(name: &'name str) -> Self {
        Self { name }
    }
}

#[derive(Clone)]
/// A song compiled into the program.
pub struct CompiledSong<'name: 'static> {
    /// The path to the song before compiling it.
    pub path: &'name str,
    /// The song data.
    pub data: &'static [u8],
}
impl<'name> CompiledSong<'name> {
    /// Create a new, empty [`CompiledSong`].
    pub fn new(path: &'name str) -> Self {
        Self { path, data: &[] }
    }
}
impl<'name> Song<'name> for CompiledSong<'name> {
    fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        Ok(Cursor::new(self.data))
    }
    fn get_path(&self) -> &'name str {
        self.path
    }
}

/// A song available in some file.
pub struct FileSong<'name> {
    /// The path to the file containing the song.
    pub path: &'name Path,
}
impl<'name> Song<'name> for FileSong<'name> {
    fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        Ok(File::open(self.path)?)
    }
    fn get_path(&self) -> &'name str {
        self.path.to_str().unwrap() // FIXME
    }
}

/// A song available on the web.
pub struct WebSong<'name, 'agent> {
    /// The URL of the song.
    url: &'name Url,
    /// The [`Agent`] that will be used to fetch the song.
    agent: &'agent Agent,
    /// The fetched song data.
    data: Vec<u8>,
    preloading: Mutex<()>,
}
impl<'name, 'agent> WebSong<'name, 'agent> {
    /// Create a new [`WebSong`].
    pub fn new(url: &'name Url, agent: &'agent Agent) -> Self {
        Self {
            url,
            agent,
            data: vec![],
            preloading: Mutex::new(()),
        }
    }
}
impl<'name, 'agent> Song<'name> for WebSong<'name, 'agent> {
    fn get_data(&mut self) -> Result<impl Read + Seek + Send + Sync + 'static, EBox> {
        self.preload()?;
        Ok(Cursor::new(self.data.clone()))
    }
    fn preload(&mut self) -> Result<(), EBox> {
        let _lock = self.preloading.lock().unwrap();
        if !self.data.is_empty() {
            return Ok(());
        }
        let mut resp = self
            .agent
            .request_url("GET", self.url)
            .call()?
            .into_reader();
        resp.read_to_end(&mut self.data)?;
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
    filenames
        .into_iter()
        .filter(|x| x.1 >= 2)
        .collect::<HashMap<_, _>>()
}

/// Change the queue order to put double songs far from each other.
pub fn check_double_songs<'name>(files: &mut [impl Song<'name>]) {
    let double_songs = get_double_songs(files);
    // If there are no double songs, stop here
    if double_songs.is_empty() {
        return;
    }
    let length = files.len();
    let mut last_swaps = vec![];

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
                                if j.abs_diff(*position) >= min_threshold
                                    && files[j].get_real_name().unwrap_or_default() != real_name
                                {
                                    // Swap the songs
                                    files.swap(i, j);
                                    if last_swaps.contains(&(i, j)) {
                                        last_swaps.clear();
                                        // Move the last file to the start
                                        // (to avoid the cases where the first song is after the threshold)
                                        files.rotate_left(1);
                                    }
                                    last_swaps.push((i, j));
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
    use super::{check_double_songs, CompiledSong, Song, TestSong};

    fn check<const N: usize>(a: &mut [&str; N], b: &[&str; N]) {
        let mut songs = a.map(TestSong::from);
        check_double_songs(&mut songs[..]);
        let paths: Vec<_> = songs.iter().map(|x| x.name).collect();
        assert_eq!(&paths, b);
    }

    #[test]
    fn real_name() {
        assert_eq!(
            CompiledSong::new("test/00_a.mp3").get_real_name(),
            Some("a")
        );
        assert_eq!(
            CompiledSong::new("test/a/b/00_c.mp3").get_real_name(),
            Some("c")
        );
        assert_eq!(
            CompiledSong::new("test/00_a.test.mp3").get_real_name(),
            Some("a.test")
        );
        assert_eq!(
            CompiledSong::new("test/00_a_test.mp3").get_real_name(),
            Some("a_test")
        );

        assert_eq!(CompiledSong::new("test/00.mp3").get_real_name(), None);
        assert_eq!(CompiledSong::new("test/test.mp3").get_real_name(), None);
        assert_eq!(CompiledSong::new("test.mp3").get_real_name(), None);
    }

    #[test]
    fn no_double_songs() {
        let a = &mut ["a", "b", "c", "d", "e"];
        let b = &["a", "b", "c", "d", "e"];
        // The songs should not move (there are no double songs)
        check(a, b);
    }

    #[test]
    fn double_songs() {
        let a = &mut ["a", "b", "a", "c", "d", "e", "f", "g"];
        let b = &["a", "b", "c", "a", "d", "e", "f", "g"];
        // The double songs (the two "a") should be at a distance of 3
        // (length / number of doubles * minimum threshold i.e. 8 / 2 * 0.75)
        // i.e. we allow 4 ± 25% = 4 ± 1 = 3/4/5
        check(a, b);
    }

    #[test]
    fn songs_already_far() {
        let a = &mut ["a", "b", "c", "d", "e", "a", "f", "g"];
        let b = &["a", "b", "c", "d", "e", "a", "f", "g"];
        // The songs should not move
        check(a, b);
    }

    #[test]
    fn songs_at_end() {
        let a = &mut ["a", "b", "c", "d", "e", "f", "g", "g"];
        let b = &["g", "b", "c", "d", "e", "f", "g", "a"];
        // The double song ("g") should be inverted with a previous song ("a")
        check(a, b);

        let a = &mut ["a", "b", "c", "d", "e", "d", "f", "g"];
        let b = &["d", "b", "c", "d", "e", "a", "f", "g"];
        // The double song ("d") should be inverted with a previous song ("a")
        check(a, b);
    }
}
