use anyhow::Result;
use vergen::{
    AddCustomEntries, BuildBuilder, CargoBuilder, CargoRerunIfChanged, CargoWarning, DefaultConfig,
    Emitter, RustcBuilder,
};

use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Default)]
struct FrontendEmitter;

impl AddCustomEntries<&str, &str> for FrontendEmitter {
    fn add_calculated_entries(
        &self,
        _idempotent: bool,
        _cargo_rustc_env_map: &mut BTreeMap<&str, &str>,
        cargo_rerun_if_changed: &mut CargoRerunIfChanged,
        _cargo_warning: &mut CargoWarning,
    ) -> Result<()> {
        cargo_rerun_if_changed.extend(
            ["src", "package.json", "tsconfig.json", "vite.config.mjs"]
                .into_iter()
                .map(|s| "../frontend/".to_string() + s),
        );
        cargo_rerun_if_changed.push("../package.json".to_string());
        Ok(())
    }

    fn add_default_entries(
        &self,
        _config: &DefaultConfig,
        _cargo_rustc_env_map: &mut BTreeMap<&str, &str>,
        _cargo_rerun_if_changed: &mut CargoRerunIfChanged,
        _cargo_warning: &mut CargoWarning,
    ) -> Result<()> {
        Ok(())
    }
}

fn run(mut cmd: Command) -> anyhow::Result<()> {
    Ok(cmd
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .map(|_| ())?)
}

const fn yarn_cmd() -> &'static str {
    if cfg!(windows) {
        "yarn.cmd"
    } else {
        "yarn"
    }
}

fn build_frontend() -> anyhow::Result<()> {
    let m = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = m.parent().unwrap();
    println!("Running yarn install...");
    let mut cmd = Command::new(yarn_cmd());
    cmd.args(["install"]);
    cmd.current_dir(project.join("frontend"));
    run(cmd)?;

    println!("Running yarn build...");
    let mut cmd = Command::new(yarn_cmd());
    cmd.args(["build"]);
    cmd.current_dir(project.join("frontend"));
    run(cmd)?;

    // copy to assets\artifacts\book.mjs
    let src = project.join("frontend/dist/book.mjs");
    let dst = project.join("assets/artifacts/book.mjs");
    std::fs::copy(src, dst)?;

    // copy typst ts renderer wasm module
    let src =
        project.join("node_modules/@myriaddreamin/typst-ts-renderer/pkg/typst_ts_renderer_bg.wasm");
    let dst = project.join("assets/artifacts/typst_ts_renderer_bg.wasm");
    std::fs::copy(src, dst)?;

    Ok(())
}

fn main() -> Result<()> {
    let build = BuildBuilder::default()
        .build_timestamp(!cfg!(debug_assertions))
        .build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let rustc = RustcBuilder::default()
        .commit_hash(true)
        .semver(true)
        .host_triple(true)
        .channel(true)
        .llvm_version(true)
        .build()?;

    let emitter = &mut Emitter::default();
    emitter
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .add_custom_instructions(&FrontendEmitter)?;

    #[cfg(not(debug_assertions))]
    {
        let gitcl = vergen_gitcl::GitclBuilder::default()
            .sha(false)
            .describe(true, true, None)
            .build()?;
        emitter.add_instructions(&gitcl)?;
    }

    // Emit the instructions
    emitter.emit()?;

    // Build frontend
    build_frontend()?;
    Ok(())
}
