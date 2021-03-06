#[macro_use] extern crate structopt;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
extern crate cargo_lock;
extern crate cargo_toml;
extern crate env_logger;
extern crate flate2;
extern crate futures;
extern crate hubcaps;
extern crate reqwest;
extern crate semver;
extern crate tar;
extern crate tempfile;
extern crate tokio_core;

use std::env;
use std::fs::{self, File, DirBuilder};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::str;

use flate2::read::GzDecoder;
use hubcaps::Github;
use log::LevelFilter;
use semver::Version;
use structopt::StructOpt;
use tempfile::TempDir;
use tokio_core::reactor::Core;

use cargo_toml::CargoToml;

mod build;
mod init;

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug, StructOpt)]
#[structopt(name = "wargo", about = "Tool used with wasm-rgame projects.")]
enum Opt {
    /// Build the current project, packing the output wasm file with all
    /// the additional Javascript / HTML.
    #[structopt(name = "build")]
    Build {
        /// Use a local path for the js files, defaults to downloading the latest
        /// matching release.
        #[structopt(long = "js-path", parse(from_os_str))]
        js_path: Option<PathBuf>,
    },
    /// Initialize the current directory as a wasm-rgame project.
    #[structopt(name = "init")]
    Init {
        /// Set the resulting package name, defaults to the directory name.
        #[structopt(long = "name")]
        name: Option<String>,
    },
    /// Create a new cargo package at <path> and initialize it.
    #[structopt(name = "new")]
    New {
        /// Set the resulting package name, defaults to the directory name.
        #[structopt(long = "name")]
        name: Option<String>,

        /// The path to create the new cargo package at.
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
}

fn main() {
    env_logger::Builder::new()
        .format(|buf, record| write!(buf, "{}", record.args()))
        .filter_level(LevelFilter::Info)
        .init();

    if let Err(err) = main_ty() {
        error!("{}", err);
        exit(1);
    }
}

fn main_ty() -> Result<()> {
    match Opt::from_args() {
        Opt::Build { js_path } => {
            build::build_project(build::BuildProjectConfig {
                js_path,
            })
        },
        Opt::Init { name } => {
            init::initialize_entrypoint(name)
        },
        Opt::New { path, name } => {
            DirBuilder::new()
                .create(path.clone())
                .map_err(|err| format_err!("Could not create directory at path: {:?}, error: {}", path, err))?;

            env::set_current_dir(path.clone())
                .map_err(|err| format_err!("Could not move into newly created path: {:?}, error: {}", path, err))?;

            init::initialize_entrypoint(name)
        },
    }
}

fn project_name() -> Result<String> {
    let mut cargo_file = File::open("Cargo.toml")
        .map_err(|err| format_err!("Cannot find Cargo.toml in project directory, error: {}", err))?;

    let mut cargo_contents = String::new();
    let _ = cargo_file.read_to_string(&mut cargo_contents)
        .map_err(|err| format_err!("Cannot read Cargo.toml contents, error: {}", err))?;

    let cargo_toml = CargoToml::from_str(&cargo_contents)
        .map_err(|err| format_err!("Cannot parse Cargo.toml, error: {}", err))?;

    Ok(cargo_toml.package.name.to_owned())
}

fn built_project_name(project_name: &str) -> String {
    project_name.replace("-", "_")
}

fn wasm_rgame_version() -> Result<Version> {
    let cargo_lock_contents = fs::read_to_string("Cargo.lock")
        .map_err(|err| format_err!("Cannot find / read Cargo.lock in project directory, error: {}", err))?;

    if let Some(version) = cargo_lock::find_version("wasm-rgame", &cargo_lock_contents) {
        Ok(version)
    } else {
        Err(format_err!("Cannot find wasm-rgame package in the Cargo.lock file!"))
    }
}

/// Executes the command with process::Command, mapping both the error of
/// executing the command and the status code + output to a Failure::Error
fn execute_command(command: &str, args: &str, context: &str) -> Result<()> {
    let output = Command::new(command)
        .args(args.split_whitespace())
        .output()
        .map_err(|err| format_err!("Failed to execute, context: `{}`, error: {}\nFull command: `{} {}`", context, err, command, args))?;

    if !output.status.success() {
        return Err(format_err!(
            "Command failed, context: `{}`\n\n\nStdout:\n{}\n\n\nStderr:\n{}\n\n\nFull command: `{} {}`",
            context,
            str::from_utf8(&output.stdout).unwrap(),
            str::from_utf8(&output.stderr).unwrap(),
            command,
            args,
        ));
    }

    Ok(())
}
