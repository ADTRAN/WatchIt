use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use log::{error, info};
use nix::{sys::signal, unistd::Pid};
use std::{process::Command, thread::sleep, time::Duration};

use crate::watch::WatcherEvent;

pub fn run_on_change(
    maybe_command: Option<&str>,
    interrupt_on_changes: bool,
    change_channel: Receiver<WatcherEvent>,
    quiet_period: Duration,
) -> Result<()> {
    let mut change_event = change_channel.recv();
    while let Ok(mut event) = change_event {
        let mut changed_during_run = true;
        while changed_during_run {
            changed_during_run = false;
            match event {
                WatcherEvent::Ready => {
                    info!("Initial watches established");
                    if let Some(command) = maybe_command {
                        flush(quiet_period, &change_channel)?;
                        changed_during_run =
                            process_change_cycle(command, interrupt_on_changes, &change_channel)?;
                    }

                    // If changes happen during the initial run, the event is different
                    if changed_during_run {
                        event = WatcherEvent::ChangeDetected
                    }
                }
                WatcherEvent::ChangeDetected => {
                    if let Some(command) = maybe_command {
                        flush(quiet_period, &change_channel)?;
                        changed_during_run =
                            process_change_cycle(command, interrupt_on_changes, &change_channel)?;
                    } else {
                        info!("Changes were detected. Exiting, since no command was specificed");
                        return Ok(());
                    }
                }
                WatcherEvent::Error(err) => {
                    return err;
                }
            }
        }

        change_event = change_channel.recv();
    }

    Ok(())
}

fn flush(quiet_period: Duration, change_channel: &Receiver<WatcherEvent>) -> Result<()> {
    sleep(quiet_period);
    // Flush any available events
    while let Ok(event) = change_channel.try_recv() {
        if let WatcherEvent::Error(err) = event {
            return err;
        }
    }
    Ok(())
}

fn process_change_cycle(
    command: &str,
    interrupt_on_changes: bool,
    change_channel: &Receiver<WatcherEvent>,
) -> Result<bool> {
    info!("Running {}", command);
    let mut child = Command::new("sh")
        .args(["-c", command])
        .spawn()
        .context("Could not spawn shell")?;
    let child_pid = child.id();
    let (child_tx, child_rx) = crossbeam_channel::unbounded();
    let mut status = None;
    let mut changed_during_run = false;

    std::thread::spawn(move || {
        let _ignored = child_tx.send(child.wait());
    });

    while status.is_none() {
        crossbeam_channel::select! {
            recv(child_rx) -> message => {
                status = Some(message.context("Child thread communication error")??);
            }
            recv(change_channel) -> message => {
                if let WatcherEvent::Error(err) = message? {
                    return err.map(|_| false);
                }
                changed_during_run = true;
                if interrupt_on_changes {
                    info!("Interrupting command so it can be restarted");
                    signal::kill(Pid::from_raw(child_pid as i32), signal::Signal::SIGINT)?;
                }
            }
        }
    }

    let code = status
        .unwrap()
        .code()
        .map(|c| c.to_string())
        .unwrap_or("Signal".into());
    let message = format!("Command finished with status {}", code);
    if status.unwrap().success() {
        info!("{}", message)
    } else {
        error!("{}", message)
    };

    Ok(changed_during_run)
}
