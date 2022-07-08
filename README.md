# Watch It

## Installation

Install the static binary like this:

```
sudo curl -L 'https://github.com/ADTRAN/WatchIt/releases/download/v1.0.0/watchit' -o /usr/local/bin/watchit
sudo chmod +x /usr/local/bin/watchit
```

Or install with [cargo][rustup] like this:

```
cargo install --git https://github.com/ADTRAN/WatchIt/
```

## Usage

```
Hey, Watch It!
Runs a command whenever a file changes

This program uses git to determine which files should be watched. Any file that git would consider
tracking (i.e. anything not excluded by .gitignore) will be watched for changes.

The given command is run as a /bin/sh shell script. Some example invocations include:

    # Run pytest tests whenever a file changes
    watchit 'pytest -vvl test'

    # Run cargo fmt and then cargo test whenever a file changes
    watchit 'cargo fmt && cargo test'

If no command is provided, watchit will just exit after it detects a change. This can be used for
more complex scripting, and behaves similarly to inotifywait.

USAGE:
    watchit [FLAGS] [OPTIONS] [COMMAND]

FLAGS:
    -h, --help         Prints help information
    -p, --interrupt    When a change is detected, interrupt the current command and start it again
    -V, --version      Prints version information
    -v, --verbose      Output more information (e.g. for debugging problems)

OPTIONS:
    -q, --quiet-period <SECONDS>    How long to wait before starting command [default: 0.5]

ARGS:
    <COMMAND>    The command to run when a file changes. Passed to /bin/sh
```

[rustup]: https://rustup.rs/
