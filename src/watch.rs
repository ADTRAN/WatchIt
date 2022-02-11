use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use inotify::{EventMask, WatchMask};
use log::{debug, info};
use std::{
    collections::{HashMap, HashSet},
    fs::canonicalize,
    path::PathBuf,
};
use walkdir::WalkDir;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Watch {
    File(PathBuf),
    Directory(PathBuf),
}

#[derive(Debug)]
pub enum WatcherEvent {
    Ready,
    ChangeDetected,
    Error(Result<()>),
}

pub fn watch<WatchMaker>(
    root_directory: PathBuf,
    watch_maker: WatchMaker,
    change_channel: Sender<WatcherEvent>,
) where
    WatchMaker: FnMut() -> Result<HashSet<Watch>>,
{
    let result = wrapped_watch(root_directory, watch_maker, &change_channel);
    if result.is_err() {
        let _ignored = change_channel.send(WatcherEvent::Error(result));
    }
}

fn wrapped_watch<WatchMaker>(
    root_directory: PathBuf,
    mut watch_maker: WatchMaker,
    change_channel: &Sender<WatcherEvent>,
) -> Result<()>
where
    WatchMaker: FnMut() -> Result<HashSet<Watch>>,
{
    let mut inotifier = inotify::Inotify::init().context("Could not initialize inotify")?;
    let mut watch_descriptors = HashMap::new();
    let mut previous_allowed_watches = watch_maker()?;

    for entry in WalkDir::new(canonicalize(&root_directory).context("Could not canonicalize path")?)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
        .filter_map(|e| e.ok())
    {
        let candidate_watch =
            Watch::Directory(canonicalize(entry.path()).context("Could not canonicalize path")?);
        if previous_allowed_watches.contains(&candidate_watch) {
            let descriptor = inotifier
                .add_watch(
                    entry.path(),
                    WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY | WatchMask::MOVE,
                )
                .context(format!(
                    "Could not create inotify watch on {:?}",
                    entry.path()
                ))?;
            watch_descriptors.insert(descriptor, entry.path().to_owned());
        }
    }

    if change_channel.send(WatcherEvent::Ready).is_err() {
        return Ok(());
    }

    let mut junk_buffer = [0u8; 4096];

    loop {
        let events = inotifier
            .read_events_blocking(&mut junk_buffer)
            .context("Failed to read inotify events")?;

        let allowed_watches = watch_maker()?;
        let mut change_detected = false;

        for event in events {
            if event.mask.contains(EventMask::ISDIR | EventMask::CREATE) {
                // We must always add the watch because git won't know if we should watch it until
                // it's too late.
                let leading_path = watch_descriptors.get(&event.wd).unwrap();
                let full_path = canonicalize(leading_path.join(event.name.unwrap()))?;
                let descriptor = inotifier
                    .add_watch(
                        full_path.clone(),
                        WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY,
                    )
                    .context(format!("Could not create inotify watch on {:?}", full_path))?;
                debug!("Added watch for new directory {:?}", full_path);
                watch_descriptors.insert(descriptor, full_path);
            } else {
                if let Some(name) = event.name {
                    let leading_path = watch_descriptors.get(&event.wd).unwrap();
                    let full_path = leading_path.join(name);
                    let candidate_watch = Watch::File(full_path.clone());
                    debug!(
                        "Candidate change event for {:?} ({:?})",
                        full_path, event.mask
                    );

                    if allowed_watches.contains(&candidate_watch)
                        || previous_allowed_watches.contains(&candidate_watch)
                    {
                        info!("Change detected in {:?} ({:?})", full_path, event.mask);
                        change_detected = true;
                    }
                }
            }
        }

        if change_detected {
            if change_channel.send(WatcherEvent::ChangeDetected).is_err() {
                return Ok(());
            }
        }
        previous_allowed_watches = allowed_watches;
    }
}
