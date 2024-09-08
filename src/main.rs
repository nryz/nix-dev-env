use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, ValueEnum};
use config::Config;
use nix::Env;
use shell::{start_shell, ShellType};
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

mod config;
mod filter;
mod nix;
mod shell;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Note: functions and arrays/assiocitive arrays are currently not implemented.
struct Cli {
    /// Path to the dev shell.
    path: Option<String>,

    /// Which shell to start.
    /// If this isn't specified, use SHELL from env.
    #[arg(short, long, value_enum, verbatim_doc_comment)]
    shell: Option<ShellType>,

    /// path to the json config file.
    /// config_file and config_str will be merged.
    #[arg(short, long, verbatim_doc_comment)]
    config_file: Option<PathBuf>,

    /// config string in json format.
    /// config_file and config_str will be merged.
    #[arg(long, verbatim_doc_comment)]
    config_str: Option<String>,

    /// path to json file of things to filter out.
    /// needs to be in the same format as nix print-dev-env --json.
    /// arrays/associative arrays and vars handled as paths will filter out
    /// only the things supplied if value isn't empty.
    /// filter_file_raw and filter_file_str will be merged.
    #[arg(long, verbatim_doc_comment)]
    filter_file_raw: Option<PathBuf>,

    /// string in json format of things to filter out.
    /// needs to be in the same format as nix print-dev-env --json.
    /// arrays/associative arrays and vars handled as paths will filter out
    /// only the things supplied if value isn't empty.
    /// filter_file_raw and filter_file_str will be merged.
    #[arg(long, verbatim_doc_comment)]
    filter_str_raw: Option<String>,

    /// Print final env, but don't start shell.
    #[arg(short, long, default_value_t = false, verbatim_doc_comment)]
    print: bool,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    let env = nix::get_dev_env(args.path)?;

    let mut config_file: Option<Config> = None;
    if let Some(file) = args.config_file {
        let reader = BufReader::new(File::open(&file).context("failed to open config file")?);
        config_file = Some(serde_json::from_reader(reader).with_context(|| {
            format!(
                "failed to deserialize config file:\n{}",
                fs::read_to_string(file.as_path()).expect("couldn't read file")
            )
        })?);
    }

    let mut config_str: Option<Config> = None;
    if let Some(config) = args.config_str {
        config_str = Some(
            serde_json::from_str(&config)
                .with_context(|| format!("failed to deserialise config json str:\n{}", config))?,
        );
    }

    let mut filter_file: Option<Env> = None;
    if let Some(file) = args.filter_file_raw {
        let reader = BufReader::new(File::open(&file).context("failed to open filter file")?);
        filter_file = Some(serde_json::from_reader(reader).with_context(|| {
            format!(
                "failed to deserialie filter file:\n{}",
                fs::read_to_string(file.as_path()).expect("couldn't read file")
            )
        })?);
    }

    let mut filter_str: Option<Env> = None;
    if let Some(filter) = args.filter_str_raw {
        filter_str = Some(
            serde_json::from_str(&filter)
                .with_context(|| format!("failed to deserialize filter json str:\n{}", filter))?,
        );
    }

    let env = filter::filter(env, filter_file, filter_str, config_file, config_str)?;

    let shell = if let Some(shell_type) = args.shell {
        shell_type
    } else {
        let shell_path = std::env::var("SHELL").context("failed to read SHELL env var")?;

        let shell = Path::new(&shell_path)
            .file_name()
            .ok_or("not a valid file name")
            .map_err(|e| anyhow!(e))?
            .to_str()
            .ok_or("failed to convert file name to str")
            .map_err(|e| anyhow!(e))?;

        ShellType::from_str(shell, true)
            .map_err(|e| anyhow!(e))
            .context("SHELL env has unknown type")?
    };

    start_shell(&env, shell, args.print).context("Failed to start the shell")?;

    Ok(())
}
