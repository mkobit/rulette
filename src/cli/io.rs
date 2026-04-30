use flate2::read::GzDecoder;
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use tar::Archive;
use walkdir::WalkDir;

pub struct InputFile {
    pub content: String,
    pub filename: Option<String>,
}

fn is_supported_extension(path: &Path) -> bool {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if matches!(
        filename.as_str(),
        "package.json"
            | "package-lock.json"
            | "cargo.lock"
            | "cargo.toml"
            | "rulette.toml"
            | "plugin.json"
    ) {
        return false;
    }

    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(ext, "md" | "mdc" | "json" | "toml" | "yaml" | "yml")
    } else {
        false
    }
}

fn read_archive<R: Read>(reader: R, archive_name: Option<&str>) -> anyhow::Result<Vec<InputFile>> {
    let mut results = Vec::new();
    let mut archive = Archive::new(reader);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.into_owned();

        if is_supported_extension(&path) {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;

            let filename = if let Some(base) = archive_name {
                format!("{}:{}", base, path.display())
            } else {
                path.display().to_string()
            };

            results.push(InputFile {
                content,
                filename: Some(filename),
            });
        }
    }
    Ok(results)
}

pub fn read_inputs(paths: &[String]) -> anyhow::Result<Vec<InputFile>> {
    let mut results = Vec::new();

    for path_str in paths {
        if path_str == "-" {
            let mut buffer = Vec::new();
            io::stdin().read_to_end(&mut buffer)?;

            // Detect gzip magic bytes (1f 8b)
            if buffer.len() > 2 && buffer[0] == 0x1f && buffer[1] == 0x8b {
                tracing::debug!("Detected GZIP archive from stdin");
                let decoder = GzDecoder::new(Cursor::new(buffer));
                results.extend(read_archive(decoder, Some("stdin"))?);
            } else if buffer.len() > 262 && &buffer[257..262] == b"ustar" {
                // Specific check for USTAR magic bytes (offset 257)
                tracing::debug!("Detected USTAR archive from stdin");
                results.extend(read_archive(Cursor::new(buffer), Some("stdin"))?);
            } else {
                tracing::debug!("Read input from stdin as plain text");
                let content = String::from_utf8(buffer)?;
                results.push(InputFile {
                    content,
                    filename: None,
                });
            }
            continue;
        }

        let path = Path::new(path_str);
        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() && is_supported_extension(p) {
                    let content = fs::read_to_string(p)?;
                    tracing::debug!("Read file: {}", p.to_string_lossy());
                    results.push(InputFile {
                        content,
                        filename: Some(p.to_string_lossy().into_owned()),
                    });
                }
            }
        } else {
            let filename = path.to_string_lossy().into_owned();
            if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
                tracing::debug!("Reading GZIP archive: {}", filename);
                let file = fs::File::open(path)?;
                let decoder = GzDecoder::new(file);
                results.extend(read_archive(decoder, Some(&filename))?);
            } else if filename.ends_with(".tar") {
                tracing::debug!("Reading TAR archive: {}", filename);
                let file = fs::File::open(path)?;
                results.extend(read_archive(file, Some(&filename))?);
            } else {
                let content = fs::read_to_string(path)?;
                tracing::debug!("Read file: {}", filename);
                results.push(InputFile {
                    content,
                    filename: Some(filename),
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs;
    use tar::Builder;

    #[test]
    fn test_read_inputs_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path();

        let file1_path = dir_path.join("file1.md");
        fs::write(&file1_path, "content 1").unwrap();

        let file2_path = dir_path.join("file2.json");
        fs::write(&file2_path, "content 2").unwrap();

        let subdir_path = dir_path.join("subdir");
        fs::create_dir(&subdir_path).unwrap();
        let file3_path = subdir_path.join("file3.mdc");
        fs::write(&file3_path, "content 3").unwrap();

        let ignore_path = dir_path.join("ignore.txt");
        fs::write(&ignore_path, "ignored content").unwrap();

        let paths = vec![dir_path.to_string_lossy().into_owned()];
        let results = read_inputs(&paths).unwrap();

        assert_eq!(results.len(), 3);
        let mut contents: Vec<String> = results.into_iter().map(|f| f.content).collect();
        contents.sort();

        assert_eq!(contents, vec!["content 1", "content 2", "content 3"]);
    }

    #[test]
    fn test_read_inputs_tar_gz() {
        let temp_dir = tempfile::tempdir().unwrap();
        let tar_gz_path = temp_dir.path().join("test.tar.gz");

        {
            let file = fs::File::create(&tar_gz_path).unwrap();
            let enc = GzEncoder::new(file, Compression::default());
            let mut tar = Builder::new(enc);

            let mut header = tar::Header::new_gnu();
            let content = b"archived content";
            header.set_size(content.len() as u64);
            header.set_cksum();
            tar.append_data(&mut header, "inside.md", &content[..])
                .unwrap();
            tar.finish().unwrap();
        }

        let paths = vec![tar_gz_path.to_string_lossy().into_owned()];
        let results = read_inputs(&paths).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "archived content");
        assert!(results[0].filename.as_ref().unwrap().contains("inside.md"));
    }
}
