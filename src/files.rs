use crate::song::EBox;
use std::{
    fs::{read_dir, ReadDir},
    path::{Path, PathBuf},
};

// Inspired from https://stackoverflow.com/a/76820878
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
    pub fn new(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            stack: vec![read_dir(path)?],
        })
    }
    fn next_item(&mut self) -> Result<Option<PathBuf>, EBox> {
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
            Ok(None)
        } else if meta.is_file() {
            Ok(Some(entry.path()))
        } else {
            Ok(None)
        }
    }
}
