use clap::{
    Arg, ArgAction,
    ValueHint::{self, *},
};
use clap_complete::*;
use indexmap::IndexMap as Map;
use serde::Serialize;

/// Returns true if a value equals its default representation.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

#[derive(Default, Serialize)]
pub struct Command {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub description: String,
    #[serde(skip_serializing_if = "is_default")]
    pub hidden: bool,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flags: Map<String, String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub persistentflags: Map<String, String>,
    #[serde(skip_serializing_if = "Completion::is_empty")]
    pub completion: Completion,
    #[serde(skip_serializing_if = "Documentation::is_empty")]
    pub documentation: Documentation,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command>,
}

#[derive(Default, Serialize)]
pub struct Documentation {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flag: Map<String, String>,
}

impl Documentation {
    pub fn is_empty(&self) -> bool {
        self.command.is_empty() && self.flag.is_empty()
    }
}

#[derive(Default, Serialize)]
pub struct Completion {
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flag: Map<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positional: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positionalany: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dash: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dashany: Vec<String>,
}

impl Completion {
    pub fn is_empty(&self) -> bool {
        self.flag.is_empty()
            && self.positional.is_empty()
            && self.positionalany.is_empty()
            && self.dash.is_empty()
            && self.dashany.is_empty()
    }
}

pub struct Spec;

impl Generator for Spec {
    fn file_name(&self, name: &str) -> String {
        format!("{}.yaml", name)
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        let mut command = command_for(cmd);
        filter_inherited_flags(&mut command, Map::new());
        let serialized =
            yaml_serde::to_string(&command).expect("Failed to serialize command to YAML");

        buf.write_all(
            b"# yaml-language-server: $schema=https://carapace.sh/schemas/command.json\n",
        )
        .expect("Failed to write schema header to generated file");

        buf.write_all(serialized.as_bytes())
            .expect("Failed to write YAML content to generated file");
    }
}

/// Recursively filters out persistent flags that were inherited from parent commands.
fn filter_inherited_flags(command: &mut Command, inherited_flags: Map<String, String>) {
    command
        .persistentflags
        .retain(|k, _| !inherited_flags.contains_key(k));

    let mut merged = inherited_flags.clone();
    merged.extend(command.persistentflags.clone());

    for child in &mut command.commands {
        filter_inherited_flags(child, merged.clone());
    }
}

/// Converts a clap Command into our Command structure.
fn command_for(cmd: &clap::Command) -> Command {
    Command {
        name: cmd.get_name().to_owned(),
        aliases: cmd.get_all_aliases().map(String::from).collect(),
        description: cmd.get_about().unwrap_or_default().to_string(),
        hidden: cmd.is_hide_set(),
        flags: flags_for(cmd, false),
        persistentflags: flags_for(cmd, true),
        documentation: Documentation {
            command: cmd.get_long_about().unwrap_or_default().to_string(),
            flag: flag_documentation_for(cmd),
        },
        completion: Completion {
            flag: flag_completions_for(cmd),
            positional: positional_completion_for(cmd),
            positionalany: positionalany_completion_for(cmd),
            ..Default::default()
        },
        commands: cmd
            .get_subcommands()
            .filter(|c| !c.is_hide_set())
            .map(|c| command_for(c))
            .collect(),
        ..Default::default()
    }
}

/// Builds a sort key from an argument's long or short name.
fn arg_sort_key(arg: &Arg) -> String {
    arg.get_long()
        .map(|s| s.to_string())
        .or_else(|| arg.get_short().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Extracts documentation for all non-positional arguments.
fn flag_documentation_for(cmd: &clap::Command) -> Map<String, String> {
    let mut m = Map::new();

    let mut arguments: Vec<Arg> = cmd
        .get_arguments()
        .filter(|o| !o.is_positional())
        .cloned()
        .collect();

    arguments.sort_by(|a, b| arg_sort_key(a).cmp(&arg_sort_key(b)));

    for arg in arguments {
        if let Some(long_help) = arg.get_long_help() {
            let key = arg_sort_key(&arg);
            m.insert(key, long_help.to_string());
        }
    }
    m
}

/// Extracts flags (local or persistent) from a command, excluding hidden and positional arguments.
fn flags_for(cmd: &clap::Command, persistent: bool) -> Map<String, String> {
    let mut m = Map::new();

    let mut arguments: Vec<Arg> = cmd
        .get_arguments()
        .filter(|o| !o.is_positional())
        .filter(|o| !o.is_hide_set())
        .filter(|o| o.is_global_set() == persistent)
        .cloned()
        .collect();

    arguments.sort_by(|a, b| arg_sort_key(a).cmp(&arg_sort_key(b)));

    for arg in arguments {
        let modifier = modifier_for(&arg);
        let signature = build_flag_signature(&arg);
        let help = arg.get_help().unwrap_or_default().to_string();

        m.insert(format!("{}{}", signature, modifier), help.clone());

        // Add visible aliases
        if let Some(aliases) = arg.get_visible_aliases() {
            for alias in aliases {
                m.insert(format!("--{}{}", alias, modifier), help.clone());
            }
        }

        // Add visible short aliases
        if let Some(short_aliases) = arg.get_visible_short_aliases() {
            for alias in short_aliases {
                m.insert(format!("-{}{}", alias, modifier), help.clone());
            }
        }

        // Add hidden aliases with & modifier
        let mut hidden_modifier = modifier.clone();
        if !hidden_modifier.contains('&') {
            hidden_modifier.push('&');
        }

        if let Some(aliases) = arg.get_aliases() {
            for alias in aliases {
                m.insert(format!("--{}{}", alias, hidden_modifier), help.clone());
            }
        }

        if let Some(short_aliases) = arg.get_all_short_aliases() {
            for alias in short_aliases {
                let key = format!("-{}{}", alias, modifier);
                if !m.contains_key(key.as_str()) {
                    let key = format!("-{}{}", alias, hidden_modifier);
                    m.insert(key, help.clone());
                }
            }
        }
    }
    m
}

/// Builds the flag signature (e.g., "-h, --help" or "--verbose").
fn build_flag_signature(arg: &Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), Some(short)) => format!("-{}, --{}", short, long),
        (Some(long), None) => format!("--{}", long),
        (None, Some(short)) => format!("-{}", short),
        (None, None) => String::new(), // Shouldn't happen for valid args
    }
}

/// Extracts completion suggestions for variadic positional arguments.
fn positionalany_completion_for(cmd: &clap::Command) -> Vec<String> {
    let mut positionals: Vec<&Arg> = cmd.get_positionals().collect();
    positionals.sort_by_key(|a| a.get_index());

    if let Some(last) = positionals.last() {
        if last.get_num_args().unwrap_or_default().max_values() == usize::MAX {
            return action_for(last.get_value_hint())
                .into_iter()
                .chain(values_for(last))
                .collect();
        }
    }
    vec![]
}

/// Extracts completion suggestions for fixed positional arguments.
fn positional_completion_for(cmd: &clap::Command) -> Vec<Vec<String>> {
    let mut positionals: Vec<&Arg> = cmd.get_positionals().collect();
    positionals.sort_by_key(|a| a.get_index());

    positionals
        .into_iter()
        .filter(|p| p.get_num_args().unwrap_or_default().max_values() != usize::MAX)
        .map(|p| {
            action_for(p.get_value_hint())
                .into_iter()
                .chain(values_for(p))
                .collect::<Vec<String>>()
        })
        .filter(|actions| !actions.is_empty())
        .collect()
}

/// Extracts completion suggestions for flag values.
fn flag_completions_for(cmd: &clap::Command) -> Map<String, Vec<String>> {
    let mut m = Map::new();
    let mut options: Vec<Arg> = cmd
        .get_opts()
        .filter(|o| !o.is_positional())
        .filter(|o| !o.is_hide_set())
        .cloned()
        .collect();

    options.sort_by(|a, b| arg_sort_key(a).cmp(&arg_sort_key(b)));

    for option in options {
        let name = arg_sort_key(&option);

        let action: Vec<String> = action_for(option.get_value_hint())
            .into_iter()
            .chain(values_for(&option))
            .collect();

        if !action.is_empty() {
            m.insert(name.clone(), action.clone());

            if let Some(aliases) = option.get_all_aliases() {
                for alias in aliases {
                    m.insert(alias.to_string(), action.clone());
                }
            }

            if let Some(short_aliases) = option.get_all_short_aliases() {
                for alias in short_aliases {
                    m.insert(alias.to_string(), action.clone());
                }
            }
        }
    }
    m
}

/// Extracts possible values for an argument with optional descriptions.
fn values_for(option: &Arg) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(values) = generator::utils::possible_values(option) {
        for value in values {
            if let Some(description) = value.get_help() {
                v.push(format!("{}\t{}", value.get_name(), description));
            } else {
                v.push(value.get_name().to_owned());
            }
        }
    }
    v
}

/// Builds a modifier string indicating argument properties (e.g., "!=", "*=").
fn modifier_for(option: &Arg) -> String {
    let mut modifier = String::new();

    if option.get_action().takes_values() {
        if option.is_hide_set() {
            modifier.push('&');
        }

        if option.is_required_set() {
            modifier.push('!');
        }

        if option.is_require_equals_set() {
            modifier.push('?');
        } else {
            modifier.push('=');
        }
    }

    if matches!(option.get_action(), ArgAction::Append | ArgAction::Count) {
        modifier.push('*');
    }

    modifier
}

/// Maps clap ValueHint to carapace action strings.
fn action_for(hint: ValueHint) -> Vec<String> {
    let actions = match hint {
        AnyPath => vec!["$files"],
        FilePath => vec!["$files"],
        DirPath => vec!["$directories"],
        ExecutablePath => vec!["$files"],
        CommandName => vec!["$executables", "$files"],
        CommandString => vec!["$executables", "$files"],
        Username => vec!["$carapace.os.Users"],
        Hostname => vec!["$carapace.net.Hosts"],
        Unknown | Other | Url | EmailAddress | CommandWithArguments => vec![],
        _ => vec![],
    };
    actions.into_iter().map(String::from).collect()
}
