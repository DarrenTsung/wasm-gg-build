#[macro_use] extern crate structopt;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
extern crate cargo_toml;
extern crate env_logger;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{PathBuf, Path};
use std::process::{Command, exit};
use std::str;

use failure::Error;
use log::LevelFilter;
use structopt::StructOpt;

use cargo_toml::CargoToml;

#[derive(Debug, StructOpt)]
#[structopt(name = "wrg-build", about = "Tool for building wasm-rgame projects.")]
struct Opt {
    /// Data path for bundled files
    #[structopt(parse(from_os_str), long = "data-path")]
    data_path: Option<PathBuf>,
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
    let Opt { data_path, .. } = Opt::from_args();

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

    // Verify that the bundled data exists, the user must include this in their
    // project in order for the script to include it with the built wasm files
    let data_path = data_path.unwrap_or_else(|| Path::new("./wasm-rgame-js").to_path_buf());
    if !data_path.exists() {
        return Err(format_err!(
            "wasm-rgame data path specified: `{:?}` does not exist\nPlease make sure you've included wasm-rgame bundled data properly, see README for more information.",
            data_path,
        ));
    }

    if !data_path.is_dir() {
        return Err(format_err!("wasm-rgame data path specified: `{:?}` is not a directory", data_path));
    }

    info!("Building the project, this may take some time.. ");
    // Execute the build before cleaning the target directory
    execute_command(
        "cargo",
        "+nightly build --target wasm32-unknown-unknown",
        "Build project targeting wasm32-unknown-unknown"
    )?;
    info!("done!\n");

    // Cleanup and create the wasm-rgame target directory
    // The bundled data specified with the data_path will be added to this clean directory.
    let target_dir = format!("target/wasm-rgame/{}", project_name);
    let target_dir_path = Path::new(&target_dir);
    if target_dir_path.exists() {
        fs::remove_dir_all(target_dir_path)
            .map_err(|err| format_err!("Failed removing existing wasm-rgame target directory, error: {}", err))?;
    }

    fs::create_dir_all(target_dir_path)
        .map_err(|err| format_err!("Failed creating wasm-rgame target directory, error: {}", err))?;

    // Copy over bundled data to target directory
    for entry_path in fs::read_dir(data_path)
        .map_err(|err| format_err!("Failed to read bundled data path, error: {}", err))?
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
                .map_err(|err| format_err!("Failed to copy over bundled data (from: {:?}, to: {:?}), error: {}", entry_path.path(), target_entry_path, err))?;

            // Now that the copied file exists, we need to configure it for the project
            let new_file_contents = {
                let mut target_entry_file = File::open(target_entry_path.clone())
                    .map_err(|err| format_err!("Failed to open newly created copy of bundled data, error: {}", err))?;

                let mut file_contents = String::new();
                target_entry_file.read_to_string(&mut file_contents)
                    .map_err(|err| format_err!("Failed to read newly created copy of bundled data for: {:?}, error: {}", target_entry_path, err))?;

                file_contents.replace("$PROJECT_NAME", &built_project_name)
            };

            let mut target_entry_file = File::create(target_entry_path)
                .map_err(|err| format_err!("Failed to re-create the newly created copy of bundled data, error: {}", err))?;
            target_entry_file.write(new_file_contents.as_bytes())
                .map_err(|err| format_err!("Failed to write the new file contents to the bundled data copy, error: {}", err))?;
        }
    }

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
