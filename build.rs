use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Defines an external test fixture dependency.
struct Fixture {
    /// Environment variable name to export the path as (e.g., FIXTURE_CLAUDE_CODE_DIR)
    env_name: &'static str,
    /// GitHub repository owner (e.g., "anthropics")
    owner: &'static str,
    /// GitHub repository name (e.g., "claude-code")
    repo: &'static str,
    /// Specific commit SHA to pin down
    sha: &'static str,
}

impl Fixture {
    fn primary_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/archive/{}.tar.gz",
            self.owner, self.repo, self.sha
        )
    }

    fn fallback_url(&self) -> String {
        // As a fallback, we can try the codeload URL format
        format!(
            "https://codeload.github.com/{}/{}/tar.gz/{}",
            self.owner, self.repo, self.sha
        )
    }
}

fn download_and_extract(fixture: &Fixture, out_dir: &Path) {
    let extract_dir = out_dir.join(format!("{}-{}", fixture.repo, fixture.sha));

    // Idempotency: skip if we've already extracted this specific SHA
    if extract_dir.exists() {
        println!(
            "cargo:rustc-env={}={}",
            fixture.env_name,
            extract_dir.display()
        );
        return;
    }

    let tarball_path = out_dir.join(format!("{}-{}.tar.gz", fixture.repo, fixture.sha));

    // Only download if tarball isn't already there (e.g., partial previous run)
    if !tarball_path.exists() {
        let primary = fixture.primary_url();
        let fallback = fixture.fallback_url();

        println!("cargo:warning=Downloading fixture: {}", primary);
        let status = Command::new("curl")
            .args(["-sL", "-o", tarball_path.to_str().unwrap(), &primary])
            .status()
            .expect("Failed to execute curl");

        if !status.success() {
            println!(
                "cargo:warning=Primary download failed, trying fallback: {}",
                fallback
            );
            let status = Command::new("curl")
                .args(["-sL", "-o", tarball_path.to_str().unwrap(), &fallback])
                .status()
                .expect("Failed to execute curl on fallback url");
            assert!(
                status.success(),
                "Failed to download fixture {}",
                fixture.repo
            );
        }
    }

    // Extract the tarball
    println!("cargo:warning=Extracting {}", tarball_path.display());

    // tar --strip-components=1 will remove the top-level repo-sha directory from the tarball
    // so we extract directly into our deterministic extract_dir.
    fs::create_dir_all(&extract_dir).expect("Failed to create extract directory");
    let status = Command::new("tar")
        .args([
            "xzf",
            tarball_path.to_str().unwrap(),
            "-C",
            extract_dir.to_str().unwrap(),
            "--strip-components=1",
        ])
        .status()
        .expect("Failed to execute tar");

    assert!(
        status.success(),
        "Failed to extract fixture {}",
        fixture.repo
    );

    // Export the path for tests
    println!(
        "cargo:rustc-env={}={}",
        fixture.env_name,
        extract_dir.display()
    );
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    let fixtures = vec![
        Fixture {
            env_name: "FIXTURE_CLAUDE_CODE_DIR",
            owner: "anthropics",
            repo: "claude-code",
            sha: "2b53fac3b2dd381bfb29f456f43c0b3eb9b3ebff", // The sha we just saw in submodule status
        },
        Fixture {
            env_name: "FIXTURE_CONDUCTOR_DIR",
            owner: "gemini-cli-extensions",
            repo: "conductor",
            sha: "080a3697da33bf2bd17a868889653a3aa05b5e02",
        },
        Fixture {
            env_name: "FIXTURE_AGENCY_AGENTS_DIR",
            owner: "msitarzewski",
            repo: "agency-agents",
            sha: "783f6a72bfd7f3135700ac273c619d92821b419a",
        },
    ];

    for fixture in fixtures {
        download_and_extract(&fixture, out_path);
    }
}
