// #![allow(dead_code)]

use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, ValueEnum};
use shell::{start_shell, ShellType};
use std::path::{Path, PathBuf};

mod filter;
mod nix;
mod shell;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the dev shell
    path: Option<String>,

    /// Which shell to start
    /// If this isn't specified, use SHELL from env
    #[arg(short, long, value_enum)]
    shell: Option<ShellType>,

    /// path to json file of things to filter out
    /// needs to be in the same format as nix print-dev-env --json
    #[arg(long)]
    filter_file_raw: Option<PathBuf>,

    /// string in json format of things to filter out
    /// needs to be in the same format as nix print-dev-env --json
    #[arg(long)]
    filter_str_raw: Option<String>,

    // Print final env, but don't start shell
    #[arg(short, long, default_value_t = false)]
    print: bool,
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();

    let mut env = nix::get_dev_env(args.path)?;
    let env = filter::filter_env(&mut env, args.filter_file_raw, args.filter_str_raw)?;

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
