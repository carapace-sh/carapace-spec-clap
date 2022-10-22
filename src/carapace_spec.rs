use clap::{Command as ClapCommand, ValueHint::*};
use clap_complete::*;
use serde::Serialize;
use std::collections::HashMap as Map;

#[derive(Default, Serialize)]
pub struct Command<'a> {
    pub name: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<&'a str>,
    pub description: &'a str,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flags: Map<&'a str, &'a str>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub persistentflags: Map<&'a str, &'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<Completion<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command<'a>>,
}

#[derive(Default, Serialize)]
pub struct Completion<'a> {
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flag: Map<&'a str, Vec<&'a str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positional: Vec<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positionalany: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dash: Vec<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dashany: Vec<&'a str>,
}

pub struct Spec;

impl Generator for Spec {
    fn file_name(&self, name: &str) -> String {
        format!("{}.yaml", name)
    }

    fn generate(&self, cmd: &ClapCommand, _: &mut dyn std::io::Write) {
        let mut command = Command {
            name: cmd.get_name(),
            aliases: cmd.get_all_aliases().collect(),
            description: &cmd.get_about().unwrap_or_default().to_string(),
            ..Default::default()
        };

        for option in cmd.get_opts() {
            let _short = option.get_short();
            let long = option.get_long();
            let name = long.unwrap_or("ARRR TODO");
            let _description = option.get_help();
            let action = match option.get_value_hint() {
                Unknown => vec!["$files"],
                AnyPath => vec!["$files"],
                FilePath => vec!["$files"],
                DirPath => vec!["$directories"],
                ExecutablePath => vec!["$_os.PathExecutables", "$files"],
                CommandName => vec!["$(TODO)"],
                CommandString => vec!["$(TODO)"],
                CommandWithArguments => vec!["$(TODO ${C_ARGS})"],
                Username => vec!["$_os.Users"],
                Hostname => vec!["$_net.Hosts"],
                _ => vec![],
            };

            if !action.is_empty() {
                command
                    .completion
                    .get_or_insert_with(Default::default)
                    .flag
                    .insert(name, action);
            }
        }

        let serialized = serde_yaml::to_string(&command).unwrap();
        println!("{}", serialized);
    }
}
