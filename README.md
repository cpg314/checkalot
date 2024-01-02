# checkalot

This lightweight binary runs a configurable series of commands that can perform checks on a repository, such as linting or formatting. Automatic fixing can be attempted for commands that support it.

![Screenshot](screenshot.png)

## Usage

```
Usage: checkalot [OPTIONS] [REPOSITORY]

Arguments:
  [REPOSITORY]  Repository root. If not provided, deduced from the current directory

Options:
      --skip <SKIP>      Skip these checks
      --only <ONLY>      Only perform these checks
      --config <CONFIG>  Configuration path relative to repository root [default: checkalot.yaml]
      --fix              Tries to fix errors
  -h, --help             Print help
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

### Bundling dependencies

It might be burdensome for users to have to manually install and keep up-to-date all commands in a `checkalot` pipeline.

The `checkalot-bundle` tool takes a YAML configuration of the form

```yaml
- name: cargo-group-imports
  version: 0.1.3
  url: https://github.com/cpg314/cargo-group-imports/releases/download/v0.1.3/cargo-group-imports-0.1.3-x86_64-unknown-linux-gnu.tar.gz
  license: licenses.html
  files:
    - cargo-group-imports
- ...
```

and produces a `.tar.gz` bundle that can be served by HTTPs and referenced by the `checkalot.yaml` configuration:

```yaml
bundle:
  url: https://.../bundle-v1.tar.gz
  checksum: 056e77264767271fe3b267f63296576366fb7115a7125df11953c41c20a46756
```

The contents will be extracted to `~/.cache/checkalot/{bundle}`, which is added to the `PATH` during the execution of commands.

Note: Currently, the bundle is uniquely identified by its URL. Therefore, the URL must be changed (e.g. with a version number) for the contents to be redownloaded.

### Avoiding rebuilds

If you execute `clippy` outside of `checkalot`, make sure it is run with exactly the same arguments, to avoid recompilations between invocations from different locations. This includes for example the `-D warnings` parameter.

If needed, set `CARGO_LOG=cargo::core::compiler::fingerprint=info` to understand why `clippy` thinks a file is stale.

## Installation

```
$ cargo install --git https://github.com/cpg314/checkalot --tag v0.1.2
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
