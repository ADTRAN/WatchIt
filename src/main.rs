use anyhow::{Context, Result};
use clap::{crate_version, value_t, App, Arg};
use log::LevelFilter;
use std::time::Duration;

mod git;
mod run;
mod watch;

const DESCRIPTION: &str = r"Runs a command whenever a file changes

This program uses git to determine which files should be watched. Any file that git would consider
tracking (i.e. anything not excluded by .gitignore) will be watched for changes.

The given command is run as a /bin/sh shell script. Some example invocations include:

    # Run pytest tests whenever a file changes
    watchit 'pytest -vvl test'

    # Run cargo fmt and then cargo test whenever a file changes
    watchit 'cargo fmt && cargo test'

If no command is provided, watchit will just exit after it detects a change. This can be used for
more complex scripting, and behaves similarly to inotifywait.
";

fn main() -> Result<()> {
    let cli_matches = App::new("Hey, Watch It!")
        .version(crate_version!())
        .about(DESCRIPTION)
        .arg(
            Arg::with_name("COMMAND")
                .help("The command to run when a file changes. Passed to /bin/sh")
                .required(false)
                .index(1),
        )
        .arg(
            Arg::with_name("quiet-period")
                .help("How long to wait before starting command")
                .short("q")
                .long("quiet-period")
                .takes_value(true)
                .value_name("SECONDS")
                .default_value("0.5"),
        )
        .arg(
            Arg::with_name("verbose")
                .help("Output more information (e.g. for debugging problems)")
                .short("v")
                .long("verbose")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("interrupt")
                .help("When a change is detected, interrupt the current command and start it again")
                .short("p")
                .long("interrupt")
                .takes_value(false),
        )
        .get_matches();

    let mut log_builder = pretty_env_logger::formatted_builder();
    let log_level = if !cli_matches.is_present("verbose") {
        LevelFilter::Info
    } else {
        LevelFilter::Debug
    };
    log_builder.filter_level(log_level);
    log_builder.init();

    let (tx, rx) = crossbeam_channel::unbounded();

    std::thread::spawn(|| {
        watch::watch(".".into(), git::discover_watches, tx);
    });

    let quiet_period = Duration::from_secs_f32(
        value_t!(cli_matches, "quiet-period", f32).context("Invalid quiet-period")?,
    );
    run::run_on_change(
        cli_matches.value_of("COMMAND"),
        cli_matches.is_present("interrupt"),
        rx,
        quiet_period,
    )?;

    Ok(())
}
