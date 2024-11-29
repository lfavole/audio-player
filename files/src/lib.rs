//! Functions on files.
//! They need to be here because they are used by both the macros and the main program.
use std::{
    fs::{read_dir, ReadDir},
    path::{Path, PathBuf},
};

type EBox = Box<dyn std::error::Error + Send + Sync>;

/// A recursive iterator over all the files in a directory.
///
/// Inspired from <https://stackoverflow.com/a/76820878>.
pub struct RecurseFilesIterator {
    stack: Vec<ReadDir>,
}
impl Iterator for RecurseFilesIterator {
    type Item = Result<PathBuf, EBox>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_item().transpose()
    }
}
impl RecurseFilesIterator {
    /// Create a new [`RecurseFilesIterator`] over the contents of the given `path`.
    pub fn new(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            stack: vec![read_dir(path)?],
        })
    }
    /// Private method that returns the next item as a [`Result`].
    /// This is needed to use the `?` operator properly.
    fn next_item(&mut self) -> Result<Option<PathBuf>, EBox> {
        loop {
            let entry = loop {
                if let Some(entry) = self.stack[0].next() {
                    break entry;
                }
                self.stack.remove(0);
                if self.stack.is_empty() {
                    return Ok(None);
                }
            };
            let entry = entry?;
            let meta = entry.metadata()?;

            if meta.is_dir() {
                self.stack.push(read_dir(entry.path())?);
                continue;
            } else if meta.is_file() {
                break Ok(Some(entry.path()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env::current_dir, path::PathBuf};

    use crate::RecurseFilesIterator;

    #[test]
    fn this_directory() {
        // Test the file list in this module's directory.
        // This assumes that the directory hasn't been "polluted" by `.gitignore`d files.

        fn convert(file: PathBuf) -> String {
            file.into_os_string().into_string().unwrap()
        }

        let folder = current_dir().unwrap();

        let mut files = Vec::with_capacity(2);
        for file in RecurseFilesIterator::new(&folder).unwrap() {
            let file = convert(file.unwrap());
            let file = file
                .strip_prefix(&(convert(folder.clone()) + "/"))
                .unwrap()
                .to_string();
            files.push(file);
        }

        // Make higher-level items appear first
        files.sort_by_key(|file| usize::MAX - file.matches('/').count());

        assert_eq!(files, vec!["src/lib.rs", "Cargo.toml"]);
    }
}
