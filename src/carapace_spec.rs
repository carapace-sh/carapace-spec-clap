use clap::{
    Arg, ArgAction,
    ValueHint::{self, *},
};
use clap_complete::*;
use indexmap::IndexMap as Map;
use serde::Serialize;

#[derive(Default, Serialize)]
pub struct Command {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flags: Map<String, String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub persistentflags: Map<String, String>,
    #[serde(skip_serializing_if = "Completion::is_empty")]
    pub completion: Completion,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command>,
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
            && self.positionalany.is_empty()
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
        let command = command_for(cmd);
        let serialized = serde_yaml::to_string(&command).unwrap();
        buf.write_all(serialized.as_bytes())
            .expect("Failed to write to generated file");
    }
}

fn command_for(cmd: &clap::Command) -> Command {
    Command {
        name: cmd.get_name().to_owned(),
        aliases: cmd.get_all_aliases().map(String::from).collect(),
        description: cmd.get_about().unwrap_or_default().to_string(),
        flags: flags_for(cmd, false),
        persistentflags: flags_for(cmd, true),
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
        ..Default::default()
    }
}

fn flags_for(cmd: &clap::Command, persistent: bool) -> Map<String, String> {
    let mut m = Map::new();

    let mut arguments = cmd
        .get_arguments()
        .filter(|o| !o.is_positional())
        .filter(|o| !o.is_hide_set())
        .filter(|o| o.is_global_set() == persistent)
        .map(|x| x.to_owned())
        .collect::<Vec<Arg>>();
    arguments.sort_by_key(|o| {
        o.get_long()
            .unwrap_or(&o.get_short().unwrap_or_default().to_string())
            .to_owned()
    });

    for arg in arguments {
        let signature = if let Some(long) = arg.get_long() {
            if let Some(short) = arg.get_short() {
                format!("-{}, --{}", short, long)
            } else {
                format!("--{}", long)
            }
        } else {
            format!("-{}", arg.get_short().unwrap())
        };
        m.insert(
            format!("{}{}", signature, modifier_for(&arg)),
            arg.get_help().unwrap_or_default().to_string(),
        );
    }
    m
}

fn positionalany_completion_for(cmd: &clap::Command) -> Vec<String> {
    let mut positionals = cmd.get_positionals().collect::<Vec<&Arg>>();
    positionals.sort_by_key(|a| a.get_index());
    if let Some(last) = positionals.last() {
        if last.get_num_args().unwrap_or_default().max_values() == usize::MAX {
            // TODO different way to detect unboundend?
            return action_for(last.get_value_hint())
                .into_iter()
                .chain(values_for(last))
                .collect::<Vec<String>>();
        }
    }
    vec![]
}

fn positional_completion_for(cmd: &clap::Command) -> Vec<Vec<String>> {
    let mut positionals = cmd.get_positionals().collect::<Vec<&Arg>>();
    positionals.sort_by_key(|a| a.get_index());
    positionals
        .into_iter()
        .filter(|p| p.get_num_args().unwrap_or_default().max_values() != usize::MAX) // filter last vararg pos (added to positionalany)
        .map(|p| {
            action_for(p.get_value_hint())
                .into_iter()
                .chain(values_for(p))
                .collect::<Vec<String>>()
        })
        .collect()
}

fn flag_completions_for(cmd: &clap::Command) -> Map<String, Vec<String>> {
    let mut m = Map::new();
    let mut options = cmd
        .get_opts()
        .filter(|o| !o.is_positional())
        .filter(|o| !o.is_hide_set())
        .map(|x| x.to_owned())
        .collect::<Vec<Arg>>();
    options.sort_by_key(|o| {
        o.get_long()
            .unwrap_or(&o.get_short().unwrap_or_default().to_string())
            .to_owned()
    });

    for option in options {
        let name = option
            .get_long()
            .unwrap_or(&option.get_short().unwrap_or_default().to_string())
            .to_owned();
        let action = action_for(option.get_value_hint())
            .into_iter()
            .chain(values_for(&option))
            .collect::<Vec<String>>();

        if !action.is_empty() {
            m.insert(name, action);
        }
    }
    m
}

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

fn modifier_for(option: &Arg) -> String {
    let mut modifier = vec![];

    if option.get_action().takes_values() {
        if option.is_require_equals_set() {
            modifier.push("?");
        } else {
            modifier.push("=");
        }
    }

    if let ArgAction::Append | ArgAction::Count = option.get_action() {
        modifier.push("*");
    }

    modifier.join("")
}

fn action_for<'a>(hint: ValueHint) -> Vec<String> {
    match hint {
        // TODO actions for command are wrong
        Unknown => vec![],
        Other => vec![],
        AnyPath => vec!["$files"],
        FilePath => vec!["$files"],
        DirPath => vec!["$directories"],
        ExecutablePath => vec!["$files"],
        CommandName => vec!["$_os.PathExecutables", "$files"],
        CommandString => vec!["$_os.PathExecutables", "$files"],
        CommandWithArguments => vec!["TODO"], // TODO
        Username => vec!["$_os.Users"],
        Hostname => vec!["$_net.Hosts"],
        Url => vec![],
        EmailAddress => vec![],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}
