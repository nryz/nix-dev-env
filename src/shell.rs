use crate::filter::FinalEnv;
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::{os::unix::process::CommandExt, process::Command};
use strum_macros::AsRefStr;

#[derive(clap::ValueEnum, Clone, Debug, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum ShellType {
    Bash,
    Zsh,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VariableValue {
    Exported { value: String },
    Var { value: String },
    Array { value: Vec<String> },
    Associative { value: HashMap<String, String> },
}

fn combine_path(a: String, b: &str, split: &str) -> String {
    a + split + b
}

pub fn start_shell(env: &FinalEnv, shell: ShellType, only_print: bool) -> Result<(), Error> {
    let mut command = Command::new(shell.as_ref());

    for (k, v) in &env.variables {
        command.env(k, v);
    }

    for (k, v) in &env.paths {
        if let Ok(env_var) = env::var(k) {
            command.env(k, combine_path(env_var, v, ":"));
        } else {
            command.env(k, v);
        }
    }

    if only_print {
        print!("env:\n{:?}", env);

        Ok(())
    } else {
        Err(command.exec().into())
    }
}
