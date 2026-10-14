use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=tests/fixtures");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixtures_dir = Path::new(&manifest_dir).join("tests").join("fixtures");

    println!(
        "cargo:rustc-env=FIXTURE_CLAUDE_CODE_DIR={}",
        fixtures_dir.join("claude-code").display()
    );
    println!(
        "cargo:rustc-env=FIXTURE_CONDUCTOR_DIR={}",
        fixtures_dir.join("conductor").display()
    );
    println!(
        "cargo:rustc-env=FIXTURE_AGENCY_AGENTS_DIR={}",
        fixtures_dir.join("agency-agents").display()
    );
    println!(
        "cargo:rustc-env=FIXTURE_MATTPOCOCK_SKILLS_DIR={}",
        fixtures_dir.join("skills").display()
    );
}
