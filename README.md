# checkalot

This lightweight binary runs a configurable series of commands that can perform checks on a repository, such as linting or formatting. Automatic fixing can be attempted for commands that support it.

![Screenshot](screenshot.png)

## Usage

```
Usage: checkalot [OPTIONS] [REPOSITORY]

Arguments:
  [REPOSITORY]  [default: deduced from current directory]

Options:
      --fix   Tries to fix errors
  -h, --help  Print help
```

The repository should contain a `checkalot.yaml` configuration file at its root.

```yaml
checks:
  # Built-in command (self version check)
  - type: version
    version: ">=0.1.2"
  # Built-in command
  - type: git-is-clean
  # Built-in command
  - type: git-is-rebased
  # Custom command
  - type: command
    # Name, only for display
    name: group-imports
    command: cargo group-imports
    # Folder relative to the repository where to excute the command
    folder: rust
    # Optional command for --fix flag
    fix_command: cargo group-imports --fix
    # Optional version check
    version: ">=0.1.2"
    version_command: cargo group-imports --version
    # Optional output path
    output: /tmp/group-imports.txt
```

A more complete example can be found at the root of the repository.

### Fixing issues automatically

When the `--fix` command is provided the `fix_command` command of each failed check is called.

All checks are then rerun in order. This is important, as fixes from one command can invalidate another (e.g. `hakari` adding dependencies that are then falsely marked as unused by `machete`).

![Screenshot](fix.png)

### Avoiding rebuilds

If you execute `clippy` outside of `checkalot`, make sure it is run with exactly the same arguments, to avoid recompilations between invocations from different locations. This includes for example the `-D warnings` parameter.

If needed, set `CARGO_LOG=cargo::core::compiler::fingerprint=info` to understand why `clippy` thinks a file is stale.

## Installation

```
$ cargo install --git https://github.com/cpg314/checkalot --tag v0.1.1
```

This will install both `checkalot` and `cargo-checkalot`, the latter being usable as a cargo subcommand (`cargo checkalot`).

### Emojis

If emojis do not show properly, install a failback font such as [Noto Emoji](https://github.com/googlefonts/noto-emoji). For example, on Arch/Manjaro with wezterm:

```console
$ sudo pacman -S noto-fonts-emoji
$ rg font ~/.wezterm.lua
font = wezterm.font_with_fallback({"JetBrains Mono", "Noto Emoji"})
```

## See also

- https://crates.io/crates/cargo-checkmate
- The commands featured in the example:
  - https://github.com/bnjbvr/cargo-machete
  - https://github.com/EmbarkStudios/cargo-deny
  - https://crates.io/crates/cargo-hakari
  - https://prettier.io/
  - https://github.com/cpg314/cargo-group-imports

## TODOs

- Type the errors (instead of `anyhow`), so that we can distinguish stdout/stderr in commands from other errors.
