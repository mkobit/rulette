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
                    if let Some("md" | "mdc" | "json" | "toml" | "yaml" | "yml") =
                        p.extension().and_then(|s| s.to_str())
                    {
                        let content = fs::read_to_string(p)?;
                        results.push(InputFile {
                            content,
                            filename: Some(p.to_string_lossy().into_owned()),
                        });
                    }
                }
            }
        } else {
            let content = fs::read_to_string(path)?;
            results.push(InputFile {
                content,
                filename: Some(path.to_string_lossy().into_owned()),
            });
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
