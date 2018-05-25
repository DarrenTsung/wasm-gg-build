use super::*;

use std::fs::OpenOptions;

const LIB_RS_TEMPLATE_TEXT : &'static str = include_str!("lib.rs.template");
const BOOTSTRAP_RS_TEMPLATE_TEXT : &'static str = include_str!("bootstrap.rs.template");
const SIMPLE_BOX_RS_TEMPLATE_TEXT : &'static str = include_str!("simple_box.rs.template");
const CARGO_TOML_APPEND_TEXT : &'static str = include_str!("cargo_toml.append");

pub fn initialize_entrypoint(name: Option<String>) -> Result<()> {
    info!("Initializing the project.. ");
    let name_arg = if let Some(name) = name {
        format!("--name {}", name)
    } else {
        String::new()
    };

    if let Err(_err) = execute_command(
        "cargo",
        &format!("init --lib {}", name_arg),
        "Initialize project with `cargo init --lib`"
    ) {
        return Err(format_err!("Failed to initialize project with `cargo init`, does the project already exist?\n\
                                You can reference the lib.rs file of `wrg-snake` to manually add the entrypoint:\n\
                                https://github.com/DarrenTsung/wrg-snake/blob/master/src/lib.rs"));
    }
    info!("done!\n");

    let project_name = project_name()?;
    let built_project_name = built_project_name(&project_name);

    info!("Adding in bootstrap files.. ");
    {
        let mut lib_rs = File::create("src/lib.rs")
            .map_err(|err| format_err!("Failed to open src/lib.rs with `File::create()`, error: {}", err))?;
        let lib_rs_text = LIB_RS_TEMPLATE_TEXT.replace("$PROJECT_NAME", &built_project_name);

        lib_rs.write(lib_rs_text.as_bytes())
            .map_err(|err| format_err!("Failed to write template into src/lib.rs, error: {}", err))?;
    }

    {
        let mut cargo_toml = OpenOptions::new()
            .append(true)
            .open("Cargo.toml")
            .map_err(|err| format_err!("Failed to open Cargo.toml to add dependencies, error: {}", err))?;

        cargo_toml.write(CARGO_TOML_APPEND_TEXT.as_bytes())
            .map_err(|err| format_err!("Failed to write dependencies into Cargo.toml, error: {}", err))?;
    }

    {
        let mut bootstrap_rs = File::create("src/bootstrap.rs")
            .map_err(|err| format_err!("Failed to open src/bootstrap.rs with `File::create()`, error: {}", err))?;

        bootstrap_rs.write(BOOTSTRAP_RS_TEMPLATE_TEXT.as_bytes())
            .map_err(|err| format_err!("Failed to write template into src/bootstrap.rs, error: {}", err))?;
    }

    {
        let mut simple_box_rs = File::create("src/simple_box.rs")
            .map_err(|err| format_err!("Failed to open src/simple_box.rs with `File::create()`, error: {}", err))?;

        simple_box_rs.write(SIMPLE_BOX_RS_TEMPLATE_TEXT.as_bytes())
            .map_err(|err| format_err!("Failed to write template into src/simple_box.rs, error: {}", err))?;
    }
    info!("done!\n");

    info!("Finished initializing project: {} successfully. Run `wargo build` next to get started!\n", project_name);
    Ok(())
}
