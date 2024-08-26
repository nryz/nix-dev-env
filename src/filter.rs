use std::{env, fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Error};

use crate::{
    nix::{BashFunctionsType, Env, VariablesType},
    shell::{is_path_var, VariableValue},
};

fn variable_filter(key: &String, value: &mut VariableValue, filter: &VariablesType) -> bool {
    if let Some(f_value) = filter.get(key) {
        use VariableValue::*;
        match (f_value, value) {
            (Array { value: f_value }, Array { value }) => {
                value.retain(|v| !f_value.contains(v) && !v.is_empty());

                if f_value.is_empty() {
                    return false;
                } else {
                    return true;
                }
            }
            (Associative { value: f_value }, Associative { value }) => {
                value.retain(|k, v| !f_value.contains_key(k) && !v.is_empty());

                if f_value.is_empty() {
                    return false;
                } else {
                    return true;
                }
            }
            (Var { value: f_value }, Var { value })
            | (Exported { value: f_value }, Exported { value })
                if is_path_var(key) =>
            {
                // println!("inside match: filter: {}\n value: {}\n", f_value, value);
                let f_paths = env::split_paths(&f_value).collect::<Vec<_>>();
                let paths = env::split_paths(&value).filter(|i| !f_paths.contains(i));
                let joined_paths = env::join_paths(paths);
                if let Ok(paths) = joined_paths {
                    if let Ok(paths) = paths.into_string() {
                        *value = paths;
                    }
                }
                // println!("final path: {}", value);
                return true;
            }
            _ => return false,
        }
    }

    return true;
}

fn variable_filter_empty(_: &String, value: &mut VariableValue) -> bool {
    match value {
        VariableValue::Exported { value } => return !value.is_empty(),
        VariableValue::Var { value } => return !value.is_empty(),
        VariableValue::Array { value } => return !value.is_empty(),
        VariableValue::Associative { value } => return !value.is_empty(),
    }
}

fn function_filter(key: &String, _: &mut String, filter: &BashFunctionsType) -> bool {
    return !filter.contains(key);
}

fn function_filter_empty(_: &String, value: &mut String) -> bool {
    !value.is_empty()
}

pub fn filter_env(
    env: &mut Env,
    filter_file: Option<PathBuf>,
    filter_str: Option<String>,
) -> Result<&Env, Error> {
    if let Some(file) = filter_file {
        let reader = BufReader::new(File::open(file).context("failed to open filter file")?);
        let filter: Env =
            serde_json::from_reader(reader).context("failed to deserialie filter file")?;

        env.variables
            .retain(|k, v| variable_filter(k, v, &filter.variables));
        env.bash_functions
            .retain(|k, v| function_filter(k, v, &filter.bash_functions));
    }

    if let Some(filter_str) = filter_str {
        let filter: Env =
            serde_json::from_str(&filter_str).context("failed to deserialize filter json str")?;

        env.variables
            .retain(|k, v| variable_filter(k, v, &filter.variables));
        env.bash_functions
            .retain(|k, v| function_filter(k, v, &filter.bash_functions));
    }

    env.variables.retain(|k, v| variable_filter_empty(k, v));
    env.bash_functions
        .retain(|k, v| function_filter_empty(k, v));

    Ok(env)
}
