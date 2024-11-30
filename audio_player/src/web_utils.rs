//! Utility functions to work with webpages.
use rayon::prelude::*;
use ureq::Agent;
use url::Url;

use crate::song::EBox;

/// An iterator over the links in a webpage.
struct LinksIterator<'a> {
    content: &'a str,
}

impl<'a> LinksIterator<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }
    fn eat(&mut self, bytes: usize) -> &'a str {
        let (ret, new_content) = &self.content.split_at(bytes);
        self.content = new_content;
        ret
    }
    /// Checks if the asked position is preceded by a comment start.
    /// If this is the case, remove the current comment
    /// (or the whole document if the comment doesn't end).
    #[must_use = "this edits the buffer by removing comments; your code must act differently according to the return value (or wait until false is returned)"]
    fn in_comment(&mut self, position: usize) -> bool {
        // If there is a comment before the current position...
        if let Some(start) = self.content[..position].find("<!--") {
            // Remove it (and everything that's before -- we don't need it)
            self.eat(start + 4); // length of the comment start

            // If it has an end...
            if let Some(end) = self.content.find("-->") {
                // Remove it completely
                self.eat(end + 3); // length of the comment end
            } else {
                // Otherwise remove all the other text
                self.content = "";
            }
            // Signal that there is a comment
            return true;
        }
        // Signal that there is no comment
        false
    }
}

impl<'a> Iterator for LinksIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // If we find the pattern...
            if let Some(index) = self.content.find("<a href=") {
                // Check if we are in a comment. If it's true, restart
                if self.in_comment(index) {
                    continue;
                }
                // Add (don't set) because the index is relative to the new substring
                self.eat(index + 8); // length of the link start
                                     // Find the end character and optionally advance the last index
                let end_chars = match self.content.get(..1) {
                    // Safety: self.eat(1) will never panic
                    // because self.content.get(..1) contains a character
                    Some("\"" | "'") => vec![self.eat(1).chars().next().unwrap()],
                    Some(_) => vec!['>', ' '],
                    None => break,
                };
                // If there is a closing quote (or a space, or a tag end)...
                if let Some(end_index) = self.content.find(&end_chars[..]) {
                    // Save the link
                    return Some(self.eat(end_index));
                }
            }
            break;
        }

        self.content = "";
        None
    }
}

/// Returns the list of the files and folders available at the given `url`.
fn get_files_and_folders(agent: &Agent, url: &Url) -> Result<(Vec<Url>, Vec<Url>), EBox> {
    let mut files = vec![];
    let mut folders = vec![];

    // Get the directory listing page
    let body = agent.request_url("GET", url).call()?.into_string()?;

    for link in LinksIterator::new(&body[..]) {
        // Get the target URL
        let target_url = url.join(link)?;
        // Save the URL to the files/folders list
        // Don't add parent/current folders
        if url
            .make_relative(&target_url)
            .is_some_and(|x| !x.is_empty() && !x.starts_with(".."))
        {
            if target_url.path().ends_with('/') {
                folders.push(target_url);
            } else {
                files.push(target_url);
            }
        }
    }

    Ok((files, folders))
}

/// Recursively pings the given `url` and its subdirectories and returns the list of the available files.
///
/// # Errors
/// Fails:
/// * if an URL cannot be fetched
/// * if a response cannot be decoded
/// * if a link cannot be resolved
pub fn get_files(agent: &Agent, url: &Url) -> Result<Vec<Url>, EBox> {
    let mut files: Vec<Url> = vec![];
    let mut folders: Vec<Url> = vec![url.clone()];

    while !folders.is_empty() {
        let results = folders
            .par_iter()
            .map(|url| get_files_and_folders(agent, url))
            .collect::<Vec<_>>();
        folders.clear();

        for result in results {
            let (mut new_files, mut new_folders) = result?;
            files.append(&mut new_files);
            folders.append(&mut new_folders);
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::LinksIterator;

    #[test]
    fn links() {
        let html = r#"
        <a href="/a/b/c">...</a>
        <a href="/d/e/f">...</a>
        "#;
        let links: Vec<_> = LinksIterator::new(html).collect();
        assert_eq!(links, vec!["/a/b/c", "/d/e/f"]);
    }

    #[test]
    fn links_in_comments() {
        let html = r#"
        <a href="/a/b/c">...</a>
        <!-- <a href="/d/e/f">...</a> -->
        <a href="/g/h/i">...</a>
        "#;
        let links: Vec<_> = LinksIterator::new(html).collect();
        assert_eq!(links, vec!["/a/b/c", "/g/h/i"]);
    }

    #[test]
    fn links_in_malformed_comment() {
        let html = r#"
        <a href="/a/b/c">...</a>
        <!-- <a href="/d/e/f">...</a>
        <a href="/g/h/i">...</a>
        "#;
        let links: Vec<_> = LinksIterator::new(html).collect();
        assert_eq!(links, vec!["/a/b/c"]);
    }

    #[test]
    fn minified() {
        let html = r"
        <a href=/a/b/c>...</a>
        <a href=/d/e/f rel=noreferrer>...</a>
        ";
        let links: Vec<_> = LinksIterator::new(html).collect();
        assert_eq!(links, vec!["/a/b/c", "/d/e/f"]);
    }
}
