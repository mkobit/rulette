use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Fixture {
    env_name: &'static str,
    owner: &'static str,
    repo: &'static str,
    sha: &'static str,
    sha256: &'static str,
}

impl Fixture {
    fn urls(&self) -> Vec<String> {
        vec![
            format!(
                "https://github.com/{}/{}/archive/{}.tar.gz",
                self.owner, self.repo, self.sha
            ),
            format!(
                "https://codeload.github.com/{}/{}/tar.gz/{}",
                self.owner, self.repo, self.sha
            ),
        ]
    }
}

fn fetch_with_retries(url: &str, github_token: Option<&String>) -> Result<Vec<u8>, anyhow::Error> {
    let mut retries = 3;
    loop {
        let mut req = ureq::get(url);
        if let Some(token) = github_token {
            req = req.header("Authorization", &format!("Bearer {}", token));
        }
        match req.call() {
            Ok(response) => {
                let mut buffer = Vec::new();
                response
                    .into_body()
                    .into_reader()
                    .read_to_end(&mut buffer)?;
                return Ok(buffer);
            }
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    return Err(anyhow::anyhow!("Request failed after retries: {}", e));
                }
                println!(
                    "cargo:warning=Request failed: {}, retrying... ({} attempts left)",
                    e, retries
                );
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

fn verify_integrity(data: &[u8], expected_sha256: &str) -> Result<(), anyhow::Error> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let actual_sha256 = hex::encode(result);

    if actual_sha256 != expected_sha256 {
        return Err(anyhow::anyhow!(
            "Integrity check failed. Expected: {}, Actual: {}",
            expected_sha256,
            actual_sha256
        ));
    }
    Ok(())
}

fn cleanup_old_fixtures(fixture: &Fixture, out_dir: &Path) {
    if !out_dir.exists() {
        return;
    }

    let current_dir_name = format!("{}-{}", fixture.repo, fixture.sha);
    let repo_prefix = format!("{}-", fixture.repo);

    if let Ok(entries) = fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();

                    if name_str.starts_with(&repo_prefix) && name_str != current_dir_name {
                        println!(
                            "cargo:warning=Cleaning up old fixture directory: {}",
                            name_str
                        );
                        let _ = fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }
}

fn download_and_extract(fixture: &Fixture, out_dir: &Path, github_token: Option<&String>) {
    cleanup_old_fixtures(fixture, out_dir);

    let extract_dir = out_dir.join(format!("{}-{}", fixture.repo, fixture.sha));
    let marker_file = extract_dir.join(".extracted");

    if extract_dir.exists() && marker_file.exists() {
        println!(
            "cargo:rustc-env={}={}",
            fixture.env_name,
            extract_dir.display()
        );
        return;
    }

    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).expect("Failed to clean up incomplete extract directory");
    }

    let mut downloaded_data = None;
    let urls = fixture.urls();

    for url in &urls {
        println!("cargo:warning=Downloading fixture from: {}", url);
        match fetch_with_retries(url, github_token) {
            Ok(data) => {
                if let Err(e) = verify_integrity(&data, fixture.sha256) {
                    println!(
                        "cargo:warning=Integrity verification failed for {}: {}",
                        url, e
                    );
                    continue;
                }
                downloaded_data = Some(data);
                break;
            }
            Err(e) => {
                println!("cargo:warning=Failed to download from {}: {}", url, e);
            }
        }
    }

    let data =
        downloaded_data.expect("Failed to download fixture from any mirror with valid integrity");

    println!(
        "cargo:warning=Extracting {} to {}",
        fixture.repo,
        extract_dir.display()
    );

    fs::create_dir_all(&extract_dir).expect("Failed to create extract directory");

    let tar = flate2::read::GzDecoder::new(&data[..]);
    let mut archive = tar::Archive::new(tar);

    for entry_result in archive.entries().expect("Failed to read archive entries") {
        let mut entry = entry_result.expect("Failed to get archive entry");
        let path = entry.path().expect("Failed to get entry path").into_owned();

        let mut components = path.components();
        if components.next().is_none() {
            continue;
        }

        let stripped_path: PathBuf = components.collect();
        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let dest_path = extract_dir.join(stripped_path);

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&dest_path).expect("Failed to create directory");
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).expect("Failed to create parent directories");
            }
            entry.unpack(&dest_path).expect("Failed to unpack file");
        }
    }

    fs::File::create(&marker_file).expect("Failed to write extraction marker file");

    println!(
        "cargo:rustc-env={}={}",
        fixture.env_name,
        extract_dir.display()
    );
}

fn main() {
    // Trivial comment to force CI re-evaluation
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=GITHUB_TOKEN");
    println!("cargo:rerun-if-env-changed=GITHUB_API_TOKEN");

    let github_token = env::var("GITHUB_TOKEN")
        .or_else(|_| env::var("GITHUB_API_TOKEN"))
        .ok();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let target_dir = Path::new(&manifest_dir).join("target").join("fixtures");
    fs::create_dir_all(&target_dir).expect("Failed to create target/fixtures directory");

    let fixtures = vec![
        Fixture {
            env_name: "FIXTURE_CLAUDE_CODE_DIR",
            owner: "anthropics",
            repo: "claude-code",
            sha: "2b53fac3b2dd381bfb29f456f43c0b3eb9b3ebff",
            sha256: "339b9bdfa4c77d7432740880e9da01701009e22af8cf7da365c4071e2df475c1",
        },
        Fixture {
            env_name: "FIXTURE_CONDUCTOR_DIR",
            owner: "gemini-cli-extensions",
            repo: "conductor",
            sha: "080a3697da33bf2bd17a868889653a3aa05b5e02",
            sha256: "c15de0f15f14a3934d44a8afe57caa606f7e3e7ce0077b0696375d6db5a11ce5",
        },
        Fixture {
            env_name: "FIXTURE_AGENCY_AGENTS_DIR",
            owner: "msitarzewski",
            repo: "agency-agents",
            sha: "783f6a72bfd7f3135700ac273c619d92821b419a",
            sha256: "29b5f837ccc8a35eb7f6f84384b00211f3c4729b7bd7f850124eaea6234b66d6",
        },
        Fixture {
            env_name: "FIXTURE_MATTPOCOCK_SKILLS_DIR",
            owner: "mattpocock",
            repo: "skills",
            sha: "90ea8eec03d4ae8f43427aaf6fe4722653561a42",
        },
    ];

    for fixture in fixtures {
        download_and_extract(&fixture, &target_dir, github_token.as_ref());
    }
}
