use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path_vars: Vec<String>,
    pub paths: HashMap<String, Vec<String>>,
    pub variables: Vec<String>,
}
