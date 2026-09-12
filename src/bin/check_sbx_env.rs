use rulette::sbx::{
    check_kit_spec, check_kit_with_sbx_if_available, check_sbx_env, check_toolchain_parity,
    REQUIRED_PORTS,
};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let repo_root = Path::new(".");
    let kit_spec = Path::new(".sbx/kit/spec.yaml");
    let env_files = [
        Path::new(".sbx/sbxenv.yaml"),
        Path::new(".sbx/sbxenv.agy.yaml"),
    ];
    let kit_dir = Path::new(".sbx/kit");

    let mut failed = false;

    if let Err(err) = check_toolchain_parity(repo_root) {
        eprintln!("Error: {:#}", err);
        failed = true;
    }

    if let Err(err) = check_kit_spec(kit_spec) {
        eprintln!("Error: {:#}", err);
        failed = true;
    }

    for env_file in env_files {
        if let Err(err) = check_sbx_env(env_file, REQUIRED_PORTS) {
            eprintln!("Error: {:#}", err);
            failed = true;
        }
    }

    if failed {
        return ExitCode::FAILURE;
    }

    if let Err(err) = check_kit_with_sbx_if_available(kit_dir) {
        eprintln!("Error: {:#}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
