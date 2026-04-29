use clap::{
    Arg, ArgAction,
    ValueHint::{self, *},
};
use clap_complete::*;
use indexmap::IndexMap as Map;
use serde::Serialize;
use std::io::Write;

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
    pub flags: Map<String, Flag>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub persistentflags: Map<String, Flag>,
    #[serde(skip_serializing_if = "Completion::is_empty")]
    pub completion: Completion,
    #[serde(skip_serializing_if = "Documentation::is_empty")]
    pub documentation: Documentation,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Flag {
    Description(String),
    Extended {
        description: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        default: String,
    },
}

impl Flag {
    fn new(description: String, default: String) -> Self {
        if default.is_empty() {
            Self::Description(description)
        } else {
            Self::Extended {
                description,
                default,
            }
        }
    }
}

#[derive(Default, Serialize)]
pub struct Documentation {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flag: Map<String, String>,
}

impl Documentation {
    fn is_empty(&self) -> bool {
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
    fn is_empty(&self) -> bool {
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
        format!("{name}.yaml")
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn Write) {
        let mut command = command_for(cmd);
        filter_inherited_flags(&mut command, &mut Map::new());

        let serialized =
            yaml_serde::to_string(&command).expect("spec generator: YAML serialization failed");

        buf.write_all(
            b"# yaml-language-server: $schema=https://carapace.sh/schemas/command.json\n",
        )
        .expect("spec generator: failed writing schema header");

        buf.write_all(serialized.as_bytes())
            .expect("spec generator: failed writing YAML output");
    }
}

fn filter_inherited_flags(cmd: &mut Command, inherited: &mut Map<String, Flag>) {
    cmd.persistentflags
        .retain(|k, _| !inherited.contains_key(k));

    let added: Vec<_> = cmd
        .persistentflags
        .iter()
        .map(|(k, v)| {
            inherited.insert(k.clone(), v.clone());
            k.clone()
        })
        .collect();

    for child in &mut cmd.commands {
        filter_inherited_flags(child, inherited);
    }

    for k in added {
        inherited.shift_remove(&k);
    }
}

fn command_for(cmd: &clap::Command) -> Command {
    Command {
        name: cmd.get_name().to_owned(),
        aliases: cmd.get_all_aliases().map(str::to_owned).collect(),
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
            .map(command_for)
            .collect(),
    }
}

fn arg_sort_key(arg: &Arg) -> (Option<&str>, Option<char>) {
    (arg.get_long(), arg.get_short())
}

fn sorted_args(cmd: &clap::Command) -> Vec<&Arg> {
    let mut args: Vec<_> = cmd.get_arguments().collect();
    args.sort_by_key(|a| arg_sort_key(a));
    args
}

fn sorted_opts(cmd: &clap::Command) -> Vec<&Arg> {
    let mut opts: Vec<_> = cmd.get_opts().collect();
    opts.sort_by_key(|a| arg_sort_key(a));
    opts
}

fn flag_documentation_for(cmd: &clap::Command) -> Map<String, String> {
    sorted_args(cmd)
        .into_iter()
        .filter(|a| !a.is_positional())
        .filter_map(|arg| arg.get_long_help().map(|h| (arg_key(arg), h.to_string())))
        .collect()
}

fn flags_for(cmd: &clap::Command, persistent: bool) -> Map<String, Flag> {
    let mut map = Map::new();

    for arg in sorted_args(cmd)
        .into_iter()
        .filter(|a| !a.is_positional())
        .filter(|a| !a.is_hide_set())
        .filter(|a| a.is_global_set() == persistent)
    {
        let modifier = modifier_for(arg);
        let help = arg.get_help().unwrap_or_default().to_string();
        let default = default_for(arg);
        let flag = Flag::new(help, default);
        let signature = flag_signature(arg);

        map.insert(format!("{signature}{modifier}"), flag.clone());

        if let Some(aliases) = arg.get_visible_aliases() {
            for alias in aliases {
                map.insert(format!("--{alias}{modifier}"), flag.clone());
            }
        }

        if let Some(short_aliases) = arg.get_visible_short_aliases() {
            for alias in short_aliases {
                map.insert(format!("-{alias}{modifier}"), flag.clone());
            }
        }

        let mut hidden_modifier = modifier.clone();
        if !hidden_modifier.contains('&') {
            hidden_modifier.push('&');
        }

        if let Some(aliases) = arg.get_aliases() {
            for alias in aliases {
                map.insert(format!("--{alias}{hidden_modifier}"), flag.clone());
            }
        }

        if let Some(short_aliases) = arg.get_all_short_aliases() {
            for alias in short_aliases {
                let key = format!("-{alias}{modifier}");
                if !map.contains_key(&key) {
                    map.insert(format!("-{alias}{hidden_modifier}"), flag.clone());
                }
            }
        }
    }

    map
}

fn flag_signature(arg: &Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(l), Some(s)) => format!("-{s}, --{l}"),
        (Some(l), None) => format!("--{l}"),
        (None, Some(s)) => format!("-{s}"),
        _ => unreachable!("clap arg without identifier"),
    }
}

fn arg_key(arg: &Arg) -> String {
    arg.get_long()
        .map(str::to_owned)
        .or_else(|| arg.get_short().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn positionalany_completion_for(cmd: &clap::Command) -> Vec<String> {
    let mut pos: Vec<_> = cmd.get_positionals().collect();
    pos.sort_by_key(|a| a.get_index());

    pos.last()
        .filter(|p| p.get_num_args().unwrap_or_default().max_values() == usize::MAX)
        .map(|p| {
            action_for(p.get_value_hint())
                .into_iter()
                .chain(values_for(p))
                .collect()
        })
        .unwrap_or_default()
}

fn positional_completion_for(cmd: &clap::Command) -> Vec<Vec<String>> {
    let mut pos: Vec<_> = cmd.get_positionals().collect();
    pos.sort_by_key(|a| a.get_index());

    pos.into_iter()
        .filter(|p| p.get_num_args().unwrap_or_default().max_values() != usize::MAX)
        .map(|p| {
            action_for(p.get_value_hint())
                .into_iter()
                .chain(values_for(p))
                .collect()
        })
        .filter(|v: &Vec<_>| !v.is_empty())
        .collect()
}

fn flag_completions_for(cmd: &clap::Command) -> Map<String, Vec<String>> {
    let mut map = Map::new();

    for opt in sorted_opts(cmd).into_iter().filter(|o| !o.is_hide_set()) {
        let name = arg_key(opt);

        let actions: Vec<_> = action_for(opt.get_value_hint())
            .into_iter()
            .chain(values_for(opt))
            .collect();

        if actions.is_empty() {
            continue;
        }

        map.insert(name.clone(), actions.clone());

        if let Some(aliases) = opt.get_all_aliases() {
            for alias in aliases {
                map.insert(alias.to_string(), actions.clone());
            }
        }

        if let Some(short_aliases) = opt.get_all_short_aliases() {
            for alias in short_aliases {
                map.insert(alias.to_string(), actions.clone());
            }
        }
    }

    map
}

fn values_for(arg: &Arg) -> Vec<String> {
    generator::utils::possible_values(arg)
        .into_iter()
        .flatten()
        .map(|v| {
            v.get_help()
                .map(|h| format!("{}\t{}", v.get_name(), h))
                .unwrap_or_else(|| v.get_name().to_owned())
        })
        .collect()
}

fn default_for(arg: &Arg) -> String {
    if !matches!(arg.get_action(), ArgAction::Set | ArgAction::Append) {
        return String::new();
    }

    arg.get_default_values()
        .iter()
        .map(|v| v.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn modifier_for(arg: &Arg) -> String {
    let mut m = String::new();

    if arg.get_action().takes_values() {
        if arg.is_hide_set() {
            m.push('&');
        }

        if arg.is_required_set() {
            m.push('!');
        }

        if arg.is_require_equals_set() {
            m.push('?');
        } else {
            m.push('=');
        }
    }

    if matches!(arg.get_action(), ArgAction::Append | ArgAction::Count) {
        m.push('*');
    }

    m
}

fn action_for(hint: ValueHint) -> Vec<String> {
    let actions = match hint {
        AnyPath | FilePath | ExecutablePath => vec!["$files"],
        DirPath => vec!["$directories"],
        CommandName | CommandString => vec!["$executables", "$files"],
        Username => vec!["$carapace.os.Users"],
        Hostname => vec!["$carapace.net.Hosts"],
        _ => vec![],
    };

    actions.into_iter().map(String::from).collect()
}
