use std::{collections::HashMap, env, fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    nix::{BashFunctionsType, Env, VariablesType},
    shell::VariableValue,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct FinalEnv {
    pub paths: HashMap<String, String>,
    pub variables: HashMap<String, String>,
}

fn variable_filter(
    key: &String,
    value: &mut VariableValue,
    filter: &VariablesType,
    path_var_names: &Vec<String>,
) -> bool {
    if let Some(f_value) = filter.get(key) {
        use VariableValue::*;
        match (f_value, value) {
            (Array { value: _ }, Array { value: _ })
            | (Associative { value: _ }, Associative { value: _ }) => {
                return false;
            }
            (Var { value: f_value }, Var { value })
            | (Exported { value: f_value }, Exported { value })
                if path_var_names.contains(key) =>
            {
                let f_paths = env::split_paths(&f_value).collect::<Vec<_>>();
                let paths = env::split_paths(&value).filter(|i| !f_paths.contains(i));
                let joined_paths = env::join_paths(paths);
                if let Ok(paths) = joined_paths {
                    if let Ok(paths) = paths.into_string() {
                        *value = paths;
                    }
                }
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

pub fn filter(
    env: Env,
    filter_file_path: Option<PathBuf>,
    filter_str: Option<String>,
    config_file_path: Option<PathBuf>,
    config_str: Option<String>,
) -> Result<FinalEnv, Error> {
    let mut res: FinalEnv = FinalEnv {
        paths: HashMap::new(),
        variables: HashMap::new(),
    };

    let mut path_var_names: Vec<String> = vec!["PATH".to_string(), "XDG_DATA_DIRS".to_string()];
    let mut configs: [Option<Config>; 2] = [None, None];
    if let Some(file) = config_file_path {
        let reader = BufReader::new(File::open(file).context("failed to open config file")?);
        configs[0] = serde_json::from_reader(reader).context("failed to deserialie config file")?;
    }

    if let Some(config_str) = config_str {
        configs[1] =
            serde_json::from_str(&config_str).context("failed to deserialize config json str")?;
    }

    for c in &configs {
        if let Some(config) = c {
            path_var_names.extend_from_slice(config.path_vars.as_slice());
        }
    }

    let env = filter_raw(env, filter_file_path, filter_str, &path_var_names)?;

    for c in configs {
        if let Some(config) = c {
            filter_config(&env, config, &path_var_names, &mut res);
        }
    }

    Ok(res)
}

fn filter_config(env: &Env, config: Config, path_var_names: &Vec<String>, out_env: &mut FinalEnv) {
    for (k, v) in &env.variables {
        if config.variables.contains(k) {
            continue;
        }

        match v {
            VariableValue::Exported { value } | VariableValue::Var { value } => {
                if path_var_names.contains(k) {
                    // TODO: filter path
                    out_env.paths.insert(k.to_string(), value.to_string());
                } else {
                    out_env.variables.insert(k.to_string(), value.to_string());
                }
            }
            _ => {}
        }
    }
}

fn filter_raw(
    mut env: Env,
    filter_file_path: Option<PathBuf>,
    filter_str: Option<String>,
    path_var_names: &Vec<String>,
) -> Result<Env, Error> {
    if let Some(file) = filter_file_path {
        let reader = BufReader::new(File::open(file).context("failed to open filter file")?);
        let filter: Env =
            serde_json::from_reader(reader).context("failed to deserialie filter file")?;

        env.variables
            .retain(|k, v| variable_filter(k, v, &filter.variables, &path_var_names));
        env.bash_functions
            .retain(|k, v| function_filter(k, v, &filter.bash_functions));
    }

    if let Some(filter_str) = filter_str {
        let filter: Env =
            serde_json::from_str(&filter_str).context("failed to deserialize filter json str")?;

        env.variables
            .retain(|k, v| variable_filter(k, v, &filter.variables, &path_var_names));
        env.bash_functions
            .retain(|k, v| function_filter(k, v, &filter.bash_functions));
    }

    env.variables.retain(|k, v| variable_filter_empty(k, v));
    env.bash_functions
        .retain(|k, v| function_filter_empty(k, v));

    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_raw_filter_str() {
        let filter_str = r#"
            {
                "bashFunctions": { "func1": "body1" },
                "variables": { 
                    "var1": { "type": "exported", "value": "value1"},
                    "var2": { "type": "var", "value": "value2"},
                    "var3": { "type": "array", "value": ["1", "2", "3"]},
                    "var4": { "type": "associative", "value": {"1": "v1", "2": "v2"}}
                }
            }
        "#;
        let result: Result<Env, _> = serde_json::from_str(&filter_str);

        assert!(
            result.is_ok(),
            "failed to deserialize filter: {}
            {:#}
            ",
            result.unwrap_err(),
            filter_str
        );
    }

    #[test]
    fn test_filter_raw() {
        let env_str = r#"
            {
                "bashFunctions": {
                    "func1": "body1",
                    "func2": "body2",
                    "func3": "body3"
                },
                "variables": { 
                    "var1": { "type": "exported", "value": "value1"},
                    "var2": { "type": "var", "value": "value2"},
                    "var3": { "type": "array", "value": ["1", "2", "3"]},
                    "var4": { "type": "array", "value": ["4", "5", "6"]},
                    "var5": { "type": "associative", "value": {"1": "v1", "2": "v2"}},
                    "var6": { "type": "associative", "value": {"3": "v3", "4": "v4"}}
                }
            }
        "#;

        let filter_str = r#"
            {
                "bashFunctions": { "func2": "" },
                "variables": { 
                    "var2": { "type": "var", "value": ""},
                    "var3": { "type": "array", "value": ["2"]},
                    "var4": { "type": "array", "value": []},
                    "var5": { "type": "associative", "value": {"2": ""}},
                    "var6": { "type": "associative", "value": {}}
                }
            }
        "#;

        let env: Result<Env, _> = serde_json::from_str(&env_str);

        assert!(
            env.is_ok(),
            "failed to deserialize env: {}
                    {:#}
                    ",
            env.unwrap_err(),
            env_str
        );

        let empty_vec = Vec::new();
        let env = filter_raw(env.unwrap(), None, Some(filter_str.to_string()), &empty_vec);

        assert!(env.is_ok(), "filter_env failed: {:#}", env.unwrap_err());

        let env = env.unwrap();

        for (k, v) in &env.bash_functions {
            match k.as_str() {
                "func1" => {
                    assert_eq!(v, "body1");
                }
                "func2" => {
                    panic!("func2 should have been filtered")
                }
                "func3" => {
                    assert_eq!(v, "body3");
                }
                _ => {
                    panic!("unknown (k, v): {}{}", k, v)
                }
            }
        }

        for (k, v) in &env.variables {
            match k.as_str() {
                "var1" => {
                    if let VariableValue::Exported { value } = v {
                        assert_eq!(value, "value1");
                    } else {
                        panic!("expected var1 to be exported");
                    }
                }
                "var3" => {
                    if let VariableValue::Array { value } = v {
                        assert_eq!(value.as_slice(), ["1", "3"]);
                    } else {
                        panic!("expected var1 to be array");
                    }
                }
                "var5" => {
                    if let VariableValue::Associative { value } = v {
                        assert_eq!(value.get_key_value("1").unwrap().1, "v1");
                    } else {
                        panic!("expected var1 to be associative");
                    }
                }
                "var2" | "var4" | "var6" => {
                    panic!("{} should have been filtered", k);
                }
                _ => {
                    panic!("unknown (k, v): {}{:?}", k, v)
                }
            }
        }
    }
}
