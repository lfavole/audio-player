//! Functions on files.
//! They need to be here because they are used by both the macros and the main program.
use std::{
    error,
    fs::{read_dir, ReadDir},
    io,
    path::{Path, PathBuf},
};

type EBox = Box<dyn error::Error + Send + Sync>;

/// A recursive iterator over all the files in a directory.
///
/// Inspired from <https://stackoverflow.com/a/76820878>.
#[must_use]
pub struct RecurseFilesIterator {
    /// A list of folder iterators to check.
    stack: Vec<ReadDir>,
}
impl Iterator for RecurseFilesIterator {
    type Item = Result<PathBuf, EBox>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_item().transpose()
    }
}
impl RecurseFilesIterator {
    /// Creates a new [`RecurseFilesIterator`] over the contents of the given `path`.
    ///
    /// # Errors
    /// Fails if the `path` cannot be found.
    pub fn new(path: &Path) -> io::Result<Self> {
        Ok(Self {
            stack: vec![read_dir(path)?],
        })
    }
    /// Private method that returns the next item as a [`Result`].
    /// This is needed to use the `?` operator properly.
    ///
    /// # Errors
    /// Fails if an entry or its metadata cannot be determined.
    fn next_item(&mut self) -> Result<Option<PathBuf>, EBox> {
        loop {
            // Get the current element on the stack
            let entry = loop {
                // If a folder on the stack has an item, return it
                if let Some(entry) = self.stack[0].next() {
                    break entry;
                }
                // Otherwise drop the folder (because we have finished to use it)
                self.stack.remove(0);
                // If we have used all the folders on the stack, stop here
                if self.stack.is_empty() {
                    return Ok(None);
                }
                // Restart
            };
            let entry = entry?;
            let meta = entry.metadata()?;

            // Add directories to the stack and restart
            if meta.is_dir() {
                self.stack.push(read_dir(entry.path())?);
                continue;
            }
            // Return the file
            if meta.is_file() {
                break Ok(Some(entry.path()));
            }
            // For other file types (e.g. symlinks), skip and restart
        }
    }
}

#[cfg(test)]
#[expect(clippy::missing_panics_doc)]
mod tests {
    use std::{
        env::current_dir,
        path::{PathBuf, MAIN_SEPARATOR_STR},
    };

    use crate::RecurseFilesIterator;

    /// Tests the file list in this module's directory.
    ///
    /// This assumes that the directory hasn't been "polluted" by `.gitignore`d files.
    #[test]
    fn this_directory() {
        /// Converts a [`PathBuf`] to a [`String`].
        ///
        /// # Panics
        /// Panics if the path is not UTF-8.
        fn convert(file: PathBuf) -> String {
            file.into_os_string()
                .into_string()
                .expect("path should be UTF-8")
        }

        let folder = current_dir().unwrap();

        let mut files = Vec::with_capacity(2);
        for file in RecurseFilesIterator::new(&folder).unwrap() {
            let file = convert(file.unwrap());
            // Remove the current directory (because it will always vary)
            let file = file
                .strip_prefix(&(convert(folder.clone()) + MAIN_SEPARATOR_STR))
                .unwrap()
                .to_string();
            files.push(file);
        }

        // Make higher-level items appear first
        files.sort_by_key(|file| usize::MAX - file.matches(MAIN_SEPARATOR_STR).count());

        assert_eq!(
            files,
            vec![
                "src".to_owned() + MAIN_SEPARATOR_STR + "lib.rs",
                "Cargo.toml".to_owned()
            ]
        );
    }
}
