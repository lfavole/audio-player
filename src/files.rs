use std::{fs::read_dir, path::{Path, PathBuf}};

// https://stackoverflow.com/a/76820878
pub fn recurse_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut ret = vec![];

    for entry in read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;

        if meta.is_dir() {
            let mut subdir = recurse_files(&entry.path())?;
            ret.append(&mut subdir);
        } else if meta.is_file() {
            ret.push(entry.path());
        }
    }

    Ok(ret)
}
