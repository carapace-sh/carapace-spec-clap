use carapace_spec_clap::Spec;
use clap::{Arg, Command, ValueHint};
use clap_complete::generate;
use std::io;

fn main() {
    let mut cmd = Command::new("myapp")
        .aliases(["alias1", "alias2"])
        .about("some description")
        .arg(
            Arg::new("value")
                .long("value")
                .short('v')
                .help("takes value")
                .value_hint(ValueHint::ExecutablePath),
        )
        .subcommand(Command::new("test").subcommand(Command::new("config")))
        .subcommand(Command::new("hello"));

    generate(Spec, &mut cmd, "myapp", &mut io::stdout());
}
