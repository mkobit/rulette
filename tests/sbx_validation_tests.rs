use rulette::sbx::{check_kit_spec, check_sbx_env, REQUIRED_PORTS};
use std::path::Path;

#[test]
fn test_docker_sandbox_environment_validation() {
    let kit_spec = Path::new(".sbx/kit/spec.yaml");
    let env_file = Path::new(".sbx/.sbxenv.yaml");
    let agy_file = Path::new(".sbx/.sbxenv.agy.yaml");

    let kit = check_kit_spec(kit_spec).expect(".sbx/kit/spec.yaml must be valid");
    assert_eq!(kit.name(), "rulette-toolchain");

    let env = check_sbx_env(env_file, REQUIRED_PORTS).expect(".sbx/.sbxenv.yaml must be valid");
    assert_eq!(env.agent, "codex");

    let agy = check_sbx_env(agy_file, REQUIRED_PORTS).expect(".sbx/.sbxenv.agy.yaml must be valid");
    assert_eq!(agy.agent, "agy");
}
