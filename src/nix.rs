use crate::shell::VariableValue;
use anyhow::{anyhow, Context, Error};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Command};

#[derive(Serialize, Deserialize, Debug)]
pub struct BashFunctionsType(HashMap<String, String>);

impl BashFunctionsType {
    pub fn contains(&self, key: &String) -> bool {
        self.0.contains_key(key)
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut String) -> bool,
    {
        self.0.retain(f)
    }
}

impl From<HashMap<String, String>> for BashFunctionsType {
    fn from(val: HashMap<String, String>) -> BashFunctionsType {
        BashFunctionsType(val)
    }
}

impl fmt::Display for BashFunctionsType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k, v) in self.0.iter() {
            write!(f, "\n    {} = \"{}\"", k, v)?
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VariablesType(HashMap<String, VariableValue>);

impl VariablesType {
    pub fn contains(&self, key: &String) -> bool {
        self.0.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&VariableValue> {
        self.0.get(key)
    }

    pub fn add(&mut self, key: String, value: VariableValue) {
        self.0.insert(key, value);
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut VariableValue) -> bool,
    {
        self.0.retain(f)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }
}

impl<'a> IntoIterator for &'a VariablesType {
    type Item = <&'a HashMap<String, VariableValue> as IntoIterator>::Item;
    type IntoIter = <&'a HashMap<String, VariableValue> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl From<HashMap<String, VariableValue>> for VariablesType {
    fn from(val: HashMap<String, VariableValue>) -> VariablesType {
        VariablesType(val)
    }
}

impl fmt::Display for VariablesType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k, v) in self.0.iter() {
            match v {
                VariableValue::Var { value } => {
                    write!(f, "\n(Var)          {} = \"{}\"", k, value)?
                }
                VariableValue::Exported { value } => {
                    write!(f, "\n(Exported)     {} = \"{}\"", k, value)?
                }
                VariableValue::Array { value } => {
                    write!(f, "\n(Array)        {} = [ \n", k)?;

                    for array_value in value {
                        write!(f, "{}\n", array_value)?
                    }

                    write!(f, "    ]")?;
                }
                VariableValue::Associative { value } => {
                    write!(f, "\n(Associative)  {} = [ \n", k)?;

                    for map_value in value {
                        write!(f, "        {} = \"{}\"\n", map_value.0, map_value.1)?
                    }

                    write!(f, "    ]")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub bash_functions: BashFunctionsType,
    pub variables: VariablesType,
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            " bash_functions: {{ {}\n }}\n variables: {{ {}\n }}\n",
            self.bash_functions, self.variables
        )
    }
}

pub fn get_dev_env(path: Option<String>) -> Result<Env, Error> {
    let mut command = Command::new("nix");
    command.arg("print-dev-env").arg("--json");

    if let Some(path) = path.as_ref() {
        command.arg(path);
    }

    let output = command.output().context("nix print-dev-env failed.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", stderr));
    }

    let output_json = String::from_utf8_lossy(&output.stdout);

    let mut env: Env = serde_json::from_str(&output_json)?;

    // prevents gc while in the shell
    if let Some(store_path) = env.variables.get("out") {
        match store_path {
            VariableValue::Exported { value } | VariableValue::Var { value } => {
                let owned = value.to_owned();
                env.variables.add(
                    "NIX_GCROOT".to_string(),
                    VariableValue::Var { value: owned },
                )
            }
            VariableValue::Array { value: _ } | VariableValue::Associative { value: _ } => {}
        }
    }

    Ok(env)
}
