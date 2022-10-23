# carapace-spec-clap

[![Crates.io](https://img.shields.io/crates/v/carapace_spec_clap.svg)](https://crates.io/crates/carapace_spec_clap)

[Spec](https://github.com/rsteube/carapace-spec) generation for [clap-rs/clap](https://github.com/clap-rs/clap)

```rust
use carapace_spec_clap::Spec;
use clap::{Arg, ArgAction, Command, ValueHint};
use clap_complete::generate;
use std::io;

fn main() {
    let mut cmd = Command::new("example")
        .aliases(["alias1", "alias2"])
        .about("example command")
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .help("show help")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("optional")
                .long("optional")
                .help("optional argument")
                .require_equals(true)
                .value_hint(ValueHint::Username),
        )
        .arg(
            Arg::new("value")
                .short('v')
                .help("takes argument")
                .value_parser(["one", "two", "three"]),
        )
        .subcommand(
            Command::new("subcommand")
                .about("example subcommand")
                .arg(
                    Arg::new("command")
                        .long("command")
                        .short('c')
                        .help("execute command")
                        .value_hint(ValueHint::CommandName),
                )
                .arg(
                    Arg::new("pos1")
                        .value_parser(["four", "five", "six"])
                        .value_hint(ValueHint::DirPath),
                )
                .arg(
                    Arg::new("posAny")
                        .num_args(1..)
                        .value_hint(ValueHint::Hostname),
                ),
        );

    generate(Spec, &mut cmd, "example", &mut io::stdout());
}
```

```yaml
name: example
aliases:
- alias1
- alias2
description: example command
flags:
  -h, --help: show help
  --optional?: optional argument
  -v=: takes argument
completion:
  flag:
    optional:
    - $_os.Users
    v:
    - one
    - two
    - three
commands:
- name: subcommand
  description: example subcommand
  flags:
    -c, --command=: execute command
  completion:
    flag:
      command:
      - $_os.PathExecutables
      - $files
    positional:
    - - $directories
      - four
      - five
      - six
    positionalany:
    - $_net.Hosts
```