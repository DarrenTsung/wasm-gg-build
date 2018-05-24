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

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path};
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

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug, StructOpt)]
#[structopt(name = "wargo", about = "Tool used with wasm-rgame projects.")]
enum Opt {
    #[structopt(name = "build")]
    Build {

    },
    #[structopt(name = "init")]
    Init {

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
        Opt::Build { .. } => {
            build::build_project()
        },
        Opt::Init { .. } => {
            // do nothing for now
            Ok(())
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
