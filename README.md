# wasm-rgame-tools
Repository for all the tools used for wasm-rgame:
* wargo 
  * The main tool used for wasm-rgame projects. Analogous to `cargo`.
  * Subcommands:
    * `warg init` - Runs `cargo init` and adds the entrypoint to the wasm-rgame application to the `lib.rs` file.
    * `warg build` - Builds the project, runs wasm-bindgen, and bundles HTML/javascript to create the full application.
