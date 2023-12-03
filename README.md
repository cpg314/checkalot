# checkalot

This lightweight binary runs a configurable series of commands that can perform checks on a repository, such as linting or formatting. Automatic fixing can be attempted for commands that support it.

![Screenshot](screenshot.png)
![Screenshot](fix.png)

## Usage

```
Usage: checkalot [OPTIONS] [REPOSITORY]

Arguments:
  [REPOSITORY]  [default: current directory]

Options:
      --fix   Tries to fix errors
  -h, --help  Print help
```

The repository should contain a `checkalot.yaml` configuration file; an example can be found at the root of the repository.

```yaml
checks:
  - type: git-is-clean
  - type: git-is-rebased
  - type: command
    name: group-imports
    command: cargo group-imports
    folder: rust
    fix_command: cargo group-imports --fix
```

There are two built-in commands: `git-is-clean` and `git-is-rebased`.

When the `--fix` command is provided the `fix_command` command of eached failed check is called, before re-executing the check.

## Installation

```
$ cargo install --git https://github.com/cpg314/checkalot --tag v0.1.1
```

This will install both `checkalot` and `cargo-checkalot`, the latter being usable as a cargo subcommand (`cargo checkalot`).

## See also

- https://crates.io/crates/cargo-checkmate
- The commands featured in the example:
  - https://github.com/bnjbvr/cargo-machete
  - https://github.com/EmbarkStudios/cargo-deny
  - https://crates.io/crates/cargo-hakari
  - https://prettier.io/
  - https://github.com/cpg314/cargo-group-imports
