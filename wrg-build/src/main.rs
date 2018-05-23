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

use failure::Error;
use flate2::read::GzDecoder;
use hubcaps::Github;
use log::LevelFilter;
use semver::Version;
use structopt::StructOpt;
use tempfile::TempDir;
use tokio_core::reactor::Core;

use cargo_toml::CargoToml;

mod choose_version;
use self::choose_version::choose_version_by_key;

#[derive(Debug, StructOpt)]
#[structopt(name = "wrg-build", about = "Tool for building wasm-rgame projects.")]
struct Opt {
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

fn main_ty() -> Result<(), Error> {
    let Opt { .. } = Opt::from_args();

    let project_name = {
        let mut cargo_file = File::open("Cargo.toml")
            .map_err(|err| format_err!("Cannot find Cargo.toml in project directory, error: {}", err))?;

        let mut cargo_contents = String::new();
        let _ = cargo_file.read_to_string(&mut cargo_contents)
            .map_err(|err| format_err!("Cannot read Cargo.toml contents, error: {}", err))?;

        let cargo_toml = CargoToml::from_str(&cargo_contents)
            .map_err(|err| format_err!("Cannot parse Cargo.toml, error: {}", err))?;
        cargo_toml.package.name.to_owned()
    };
    let built_project_name = project_name.replace("-", "_");

    let wasm_rgame_version = {
        let cargo_lock_contents = fs::read_to_string("Cargo.lock")
            .map_err(|err| format_err!("Cannot find / read Cargo.lock in project directory, error: {}", err))?;

        if let Some(version) = cargo_lock::find_version("wasm-rgame", &cargo_lock_contents) {
            version
        } else {
            return Err(format_err!("Cannot find wasm-rgame package in the Cargo.lock file!"));
        }
    };
    info!("The current project is using wasm-rgame version: `{}`.\n", wasm_rgame_version);

    // Download the release of wasm-rgame-js that corresponds to the version of
    // wasm-rgame that the project is using
    let mut core = Core::new().unwrap();
    let github = Github::new("wrg-build-agent".to_string(), None, &core.handle());
    let repo_releases = github.repo("DarrenTsung", "wasm-rgame-js").releases();
    let releases = core.run(repo_releases.list()).unwrap();
    if releases.is_empty() {
        return Err(format_err!("Found no releases for wasm-rgame-js!"));
    }

    let chosen_release = choose_version_by_key(wasm_rgame_version, releases, |r| {
        // Tags look like: "v0.1.0", need to become "0.1.0"
        let version_str = r.tag_name.split("v").nth(1).unwrap();
        Version::parse(version_str).ok()
    });

    if chosen_release.is_none() {
        return Err(format_err!("Found no valid releases for wasm-rgame version!"));
    }

    let chosen_release = chosen_release.unwrap();
    info!("Found valid release version `{}` for wasm-rgame-js!\n", chosen_release.tag_name);

    let res = reqwest::get(chosen_release.tarball_url.as_str())
        .map_err(|err| format_err!("Could not download release tarball, error: {}", err))?;

    let tmp_dir = TempDir::new()
        .map_err(|err| format_err!("Could not create a temporary directory, error: {}", err))?;

    let decoded_res = GzDecoder::new(res);
    let mut archive = tar::Archive::new(decoded_res);
    archive.unpack(tmp_dir.path())
        .map_err(|err| format_err!("Could not unpack archive into the temporary directory, error: {}", err))?;

    let unpacked_dir_path = {
        // Because it dumped the contents into some directory inside the temporary directory
        // there should only be one entry (which is the unpacked_dir_path)
        fs::read_dir(tmp_dir.path())
            .map_err(|err| format_err!("Failed to read temporary directory, error: {}", err))?
            .nth(0).expect("one entry exists").expect("can read entry").path()
    };

    info!("Building the project, this may take some time.. ");
    // Execute the build before cleaning the target directory
    execute_command(
        "cargo",
        "build --target wasm32-unknown-unknown",
        "Build project targeting wasm32-unknown-unknown"
    )?;
    info!("done!\n");

    // Cleanup and create the wasm-rgame target directory
    // The unpacked data specified with the data_path will be added to this clean directory.
    let target_dir = format!("target/wasm-rgame/{}", project_name);
    let target_dir_path = Path::new(&target_dir);
    if target_dir_path.exists() {
        fs::remove_dir_all(target_dir_path)
            .map_err(|err| format_err!("Failed removing existing wasm-rgame target directory, error: {}", err))?;
    }

    fs::create_dir_all(target_dir_path)
        .map_err(|err| format_err!("Failed creating wasm-rgame target directory, error: {}", err))?;

    // Copy over unpacked data to target directory
    for entry_path in fs::read_dir(unpacked_dir_path)
        .map_err(|err| format_err!("Failed to read unpacked data dir path, error: {}", err))?
    {
        if let Ok(entry_path) = entry_path {
            let file_name = entry_path.file_name();

            if let Ok(file_name) = file_name.clone().into_string() {
                // ignore hidden files
                if file_name.starts_with(".") {
                    continue;
                }
            } else {
                warn!("Failed to parse file_name into string: {:?}, skipping!", file_name);
                continue;
            }

            let target_entry_path = target_dir_path.join(file_name);

            fs::copy(entry_path.path(), target_entry_path.clone())
                .map_err(|err| format_err!("Failed to copy over unpacked data (from: {:?}, to: {:?}), error: {}", entry_path.path(), target_entry_path, err))?;

            // Now that the copied file exists, we need to configure it for the project
            let new_file_contents = {
                let mut target_entry_file = File::open(target_entry_path.clone())
                    .map_err(|err| format_err!("Failed to open newly created copy of unpacked data, error: {}", err))?;

                let mut file_contents = String::new();
                target_entry_file.read_to_string(&mut file_contents)
                    .map_err(|err| format_err!("Failed to read newly created copy of unpacked data for: {:?}, error: {}", target_entry_path, err))?;

                file_contents.replace("$PROJECT_NAME", &built_project_name)
            };

            let mut target_entry_file = File::create(target_entry_path)
                .map_err(|err| format_err!("Failed to re-create the newly created copy of unpacked data, error: {}", err))?;
            target_entry_file.write(new_file_contents.as_bytes())
                .map_err(|err| format_err!("Failed to write the new file contents to the unpacked data copy, error: {}", err))?;
        }
    }

    tmp_dir.close()
        .map_err(|err| format_err!("Failed to cleanup temporary directory holding unpacked release, error: {}", err))?;

    info!("Running wasm-bindgen, this may take some time.. ");
    let wasm_output_path = format!("target/wasm32-unknown-unknown/debug/{}.wasm", built_project_name);
    execute_command(
        "wasm-bindgen",
        &format!("{} --no-modules --no-modules-global {} --no-typescript --out-dir {}", wasm_output_path, built_project_name, target_dir),
        &format!("Run wasm-bindgen, directing output to wasm-rgame `{}` folder", target_dir),
    )?;
    info!("done!\n");

    let target_index_path = target_dir_path.join("index.html");
    info!("Finished building project: {} successfully. View the deployed project at {:?}.\n", project_name, target_index_path.as_os_str());
    Ok(())
}

/// Executes the command with process::Command, mapping both the error of
/// executing the command and the status code + output to a Failure::Error
fn execute_command(command: &str, args: &str, context: &str) -> Result<(), Error> {
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
