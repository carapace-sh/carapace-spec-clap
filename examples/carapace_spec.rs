use carapace_spec_clap::Spec;
use clap::{Arg, ArgAction, Command, ValueHint};
use clap_complete::generate;
use std::io;

fn main() {
    let mut cmd = Command::new("myapp")
        .aliases(["alias1", "alias2"])
        .about("some description")
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("optional")
                .long("optional")
                .require_equals(true)
                .value_hint(ValueHint::Username),
        )
        .arg(
            Arg::new("value")
                .short('v')
                .help("takes value")
                .value_hint(ValueHint::ExecutablePath),
        )
        .subcommand(Command::new("test").subcommand(Command::new("config")))
        .subcommand(Command::new("hello"));

    generate(Spec, &mut cmd, "myapp", &mut io::stdout());
}
