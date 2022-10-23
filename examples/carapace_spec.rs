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
