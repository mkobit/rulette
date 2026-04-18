use std::env;
use std::fs;
use std::path::Path;

struct Fixture {
    env_name: &'static str,
    owner: &'static str,
    repo: &'static str,
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
        format!(
            "https://codeload.github.com/{}/{}/tar.gz/{}",
            self.owner, self.repo, self.sha
        )
    }
}

fn download_and_extract(fixture: &Fixture, out_dir: &Path) {
    let extract_dir = out_dir.join(format!("{}-{}", fixture.repo, fixture.sha));

    if extract_dir.exists() {
        println!(
            "cargo:rustc-env={}={}",
            fixture.env_name,
            extract_dir.display()
        );
        return;
    }

    let primary = fixture.primary_url();
    let fallback = fixture.fallback_url();

    println!("cargo:warning=Downloading fixture: {}", primary);

    let response = ureq::get(&primary)
        .call()
        .or_else(|_| {
            println!(
                "cargo:warning=Primary download failed, trying fallback: {}",
                fallback
            );
            ureq::get(&fallback).call()
        })
        .expect("Failed to download fixture");

    println!(
        "cargo:warning=Extracting {} directly to memory",
        fixture.repo
    );

    fs::create_dir_all(&extract_dir).expect("Failed to create extract directory");

    let tar = flate2::read::GzDecoder::new(response.into_body().into_reader());
    let mut archive = tar::Archive::new(tar);

    // tar --strip-components=1 equivalent using tar crate directly
    for file in archive.entries().expect("Failed to read archive entries") {
        let mut file = file.expect("Failed to get archive entry");
        let path = file.path().expect("Failed to get entry path").into_owned();

        let mut components = path.components();
        // Skip first component (the top-level directory)
        if components.next().is_none() {
            continue;
        }

        let stripped_path: std::path::PathBuf = components.collect();
        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let dest_path = extract_dir.join(stripped_path);

        if file.header().entry_type().is_dir() {
            fs::create_dir_all(&dest_path).expect("Failed to create directory");
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).expect("Failed to create parent directories");
            }
            file.unpack(&dest_path).expect("Failed to unpack file");
        }
    }

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
            sha: "2b53fac3b2dd381bfb29f456f43c0b3eb9b3ebff",
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
