use crate::watch::Watch;

use anyhow::{Context, Result};
use log::debug;
use std::{
    collections::HashSet,
    fs::{canonicalize, metadata},
    io::BufRead,
    process::Command,
};

pub fn discover_watches() -> Result<HashSet<Watch>> {
    let mut watches = HashSet::new();

    metadata(".git").context("This is not a git repository")?;

    debug!("Checking cached files with git ls-files");
    let output = Command::new("git")
        .args(["ls-files", "-c"])
        .output()
        .context("Failed to run git ls-files")?;
    for line in output.stdout.lines() {
        let line = line.context("Error decoding git output")?;
        if metadata(line.as_str()).is_err() {
            debug!("Ignoring file {} since it cannot be accessed", line);
            continue;
        }
        let path = canonicalize(line.as_str())
            .context(format!("Could not canonicalize path {:?}", line))?;
        watches.insert(Watch::Directory(
            path.parent()
                .context("Could not get directory parent")?
                .to_owned(),
        ));
        watches.insert(Watch::File(path));
    }

    debug!("Checking untracked files with git status");
    let output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .context("Failed to run git status")?;
    for line in output.stdout.lines() {
        let line = line.context("Could not decode git output")?;
        if line.starts_with("?? ") {
            let trimmed_line = &line[3..];
            if metadata(trimmed_line).is_err() {
                debug!("Ignoring file {} since it cannot be accessed", trimmed_line);
                continue;
            }
            let path = canonicalize(trimmed_line)
                .context(format!("Could not canonicalize path {}", trimmed_line))?;
            watches.insert(Watch::Directory(
                path.parent()
                    .context("Could not get parent directory")?
                    .to_owned(),
            ));
            watches.insert(Watch::File(path));
        }
    }

    Ok(watches)
}
