mod common;

#[test]
fn basic() {
    let name = "basic";
    let cmd = common::basic_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/basic.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn feature_sample() {
    let name = "feature_sample";
    let cmd = common::feature_sample_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/feature_sample.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn special_commands() {
    let name = "special_commands";
    let cmd = common::special_commands_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/special_commands.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn quoting() {
    let name = "quoting";
    let cmd = common::quoting_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/quoting.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn aliases() {
    let name = "aliases";
    let cmd = common::aliases_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/aliases.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn sub_subcommands() {
    let name = "sub_subcommands";
    let cmd = common::sub_subcommands_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/sub_subcommands.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}

#[test]
fn value_hint() {
    let name = "value_hint";
    let cmd = common::value_hint_command(name);
    common::assert_matches(
        snapbox::file!["snapshots/value_hint.yaml"],
        carapace_spec_clap::Spec,
        cmd,
        name,
    );
}
