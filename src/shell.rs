use crate::filter::FinalEnv;
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::io::stdout;
use std::io::Write;
use std::{os::unix::process::CommandExt, process::Command};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VariableValue {
    Exported { value: String },
    Var { value: String },
    Array { value: Vec<String> },
    Associative { value: HashMap<String, String> },
}

pub fn combine_path(a: String, b: &str, split: &str) -> String {
    if a.is_empty() {
        a + b
    } else {
        a + split + b
    }
}

pub fn start_shell(env: &FinalEnv, shell: &String, only_print: bool) -> Result<(), Error> {
    let mut command = Command::new(shell);

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
        let stdout = stdout();
        let mut stdout = stdout.lock();
        write!(stdout, "env:\n{}", env)?;

        Ok(())
    } else {
        println!("starting shell: {}", shell);
        Err(command.exec().into())
    }
}
