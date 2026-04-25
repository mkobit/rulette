use std::fs;
use std::io::{self, Read};
use std::path::Path;
use walkdir::WalkDir;

pub struct InputFile {
    pub content: String,
    pub filename: Option<String>,
}

pub fn read_inputs(paths: &[String]) -> anyhow::Result<Vec<InputFile>> {
    let mut results = Vec::new();

    for path_str in paths {
        if path_str == "-" {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            tracing::debug!("Read input from stdin");
            results.push(InputFile {
                content: buffer,
                filename: None,
            });
            continue;
        }

        let path = Path::new(path_str);
        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    let is_supported = match p.extension().and_then(|s| s.to_str()) {
                        Some("md" | "mdc" | "json" | "toml" | "yaml" | "yml") => true,
                        _ => matches!(file_name, ".cursorrules" | ".windsurfrules"),
                    };

                    if is_supported {
                        let file_content = fs::read_to_string(p)?;
                        tracing::debug!("Read file: {}", p.to_string_lossy());
                        results.push(InputFile {
                            content: file_content,
                            filename: Some(p.to_string_lossy().into_owned()),
                        });
                    }
                }
            }
        } else {
            let path_str_lower = path_str.to_lowercase();
            if path_str_lower.ends_with(".tar.gz")
                || path_str_lower.ends_with(".tgz")
                || path_str_lower.ends_with(".tar")
            {
                let file = fs::File::open(path)?;
                let mut archive: tar::Archive<Box<dyn Read>> = if path_str_lower.ends_with(".tar") {
                    tar::Archive::new(Box::new(file))
                } else {
                    tar::Archive::new(Box::new(flate2::read::GzDecoder::new(file)))
                };

                for entry_result in archive.entries()? {
                    let mut entry = entry_result?;
                    let entry_path = entry.path()?.into_owned();
                    let file_name = entry_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let is_supported = match entry_path.extension().and_then(|s| s.to_str()) {
                        Some("md" | "mdc" | "json" | "toml" | "yaml" | "yml") => true,
                        _ => matches!(file_name, ".cursorrules" | ".windsurfrules"),
                    };

                    if entry.header().entry_type().is_file() && is_supported {
                        let mut file_content = String::new();
                        entry.read_to_string(&mut file_content)?;
                        let extracted_filename =
                            format!("{}/{}", path_str, entry_path.to_string_lossy());
                        tracing::debug!("Read file from archive: {}", extracted_filename);
                        results.push(InputFile {
                            content: file_content,
                            filename: Some(extracted_filename),
                        });
                    }
                }
            } else {
                let content = fs::read_to_string(path)?;
                tracing::debug!("Read file: {}", path.to_string_lossy());
                results.push(InputFile {
                    content,
                    filename: Some(path.to_string_lossy().into_owned()),
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_read_inputs_tar_gz() {
        let temp_dir = tempdir().unwrap();
        let archive_path = temp_dir.path().join("test.tar.gz");

        let tar_gz = File::create(&archive_path).unwrap();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut builder = tar::Builder::new(enc);

        let mut header1 = tar::Header::new_gnu();
        let content1 = b"# Rule 1
content";
        header1.set_size(content1.len() as u64);
        header1.set_mode(0o644);
        header1.set_cksum();
        builder
            .append_data(&mut header1, "rule1.md", &content1[..])
            .unwrap();

        let mut header2 = tar::Header::new_gnu();
        let content2 = b"ignored content";
        header2.set_size(content2.len() as u64);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder
            .append_data(&mut header2, "ignored.txt", &content2[..])
            .unwrap();

        builder.into_inner().unwrap().finish().unwrap();

        let paths = vec![archive_path.to_string_lossy().into_owned()];
        let results = read_inputs(&paths).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "# Rule 1\ncontent");
        assert!(results[0].filename.as_ref().unwrap().contains("rule1.md"));
    }
}
