use super::{
    observe_path, observe_path_with_limits, observe_stdin, ArtifactObservation, InputOrigin,
    ObservationLimits,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tar::{Builder, EntryType, Header};

fn append_file(builder: &mut Builder<Vec<u8>>, path: &str, contents: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(contents.len() as u64);
    header.set_cksum();
    builder.append_data(&mut header, path, contents).unwrap();
}

fn replace_first_member_path(archive: &mut [u8], path: &[u8]) {
    assert!(path.len() < 100);
    archive[..100].fill(0);
    archive[..path.len()].copy_from_slice(path);
    archive[148..156].fill(b' ');
    let checksum: u32 = archive[..512].iter().map(|byte| u32::from(*byte)).sum();
    let checksum = format!("{:06o}\0 ", checksum);
    archive[148..156].copy_from_slice(checksum.as_bytes());
}

#[test]
fn observes_directory_bytes_executable_metadata_and_safe_relative_paths() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/blob.bin"), [0, 255, 1]).unwrap();
    fs::write(root.join("rules.md"), "follow the instructions").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.join("rules.md"), fs::Permissions::from_mode(0o755)).unwrap();
    }

    let observations = observe_path(root).unwrap();

    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.source_path.as_str())
            .collect::<Vec<_>>(),
        vec!["nested/blob.bin", "rules.md"]
    );
    assert_eq!(observations[0].bytes, vec![0, 255, 1]);
    assert!(observations
        .iter()
        .all(|observation| observation.origin == InputOrigin::Filesystem));
    assert!(observations
        .iter()
        .all(|observation| observation.provenance.input_label.starts_with("input_")));
    #[cfg(unix)]
    assert!(observations[1].executable);
}

#[test]
fn observes_plain_stdin_without_utf8_decoding() {
    let observations = observe_stdin(Cursor::new(vec![0, 255, 1])).unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].bytes, vec![0, 255, 1]);
    assert_eq!(observations[0].source_path.as_str(), "stdin");
    assert_eq!(observations[0].origin, InputOrigin::Stdin);
    assert_eq!(observations[0].provenance.input_label, "stdin");
}

#[test]
fn observes_gzip_tar_members_as_bytes_with_member_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("source.tar.gz");
    let mut tar = Builder::new(Vec::new());
    append_file(&mut tar, "skills/demo/SKILL.md", b"binary\0content");
    let tar = tar.into_inner().unwrap();
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut gzip, &tar).unwrap();
    fs::write(&archive_path, gzip.finish().unwrap()).unwrap();

    let observations = observe_path(&archive_path).unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].bytes, b"binary\0content");
    assert_eq!(observations[0].source_path.as_str(), "skills/demo/SKILL.md");
    assert_eq!(observations[0].origin, InputOrigin::GzipTar);
    assert_eq!(
        observations[0]
            .provenance
            .archive_member
            .as_ref()
            .unwrap()
            .as_str(),
        "skills/demo/SKILL.md"
    );
}

#[test]
fn detects_tar_input_by_magic_without_an_archive_extension() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("source.payload");
    let mut tar = Builder::new(Vec::new());
    append_file(&mut tar, "rules/AGENTS.md", b"follow the instructions");
    fs::write(&archive_path, tar.into_inner().unwrap()).unwrap();

    let observations = observe_path(&archive_path).unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].origin, InputOrigin::Tar);
    assert_eq!(observations[0].source_path.as_str(), "rules/AGENTS.md");
}

#[test]
fn rejects_symlinked_files_before_reading() {
    #[cfg(unix)]
    {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("target.md"), "contents").unwrap();
        std::os::unix::fs::symlink(root.join("target.md"), root.join("link.md")).unwrap();

        let error = observe_path(root).unwrap_err();

        assert!(error.to_string().contains("symlink"));
    }
}

#[test]
fn rejects_non_regular_and_duplicate_archive_members() {
    for entry_type in [EntryType::Symlink, EntryType::Link] {
        let mut linked = Builder::new(Vec::new());
        let mut link_header = Header::new_gnu();
        link_header.set_entry_type(entry_type);
        link_header.set_size(0);
        link_header.set_link_name("target").unwrap();
        link_header.set_cksum();
        linked
            .append_data(&mut link_header, "link.md", std::io::empty())
            .unwrap();
        let linked = linked.into_inner().unwrap();

        let error = super::observe_tar(Cursor::new(linked), "archive").unwrap_err();
        assert!(error.to_string().contains("non-regular"));
    }

    let mut duplicate = Builder::new(Vec::new());
    append_file(&mut duplicate, "same.md", b"first");
    append_file(&mut duplicate, "same.md", b"second");
    let duplicate = duplicate.into_inner().unwrap();

    let error = super::observe_tar(Cursor::new(duplicate), "archive").unwrap_err();
    assert!(error.to_string().contains("duplicate"));
}

#[test]
fn rejects_unsafe_archive_member_paths() {
    let mut archive = Builder::new(Vec::new());
    append_file(&mut archive, "inside.md", b"escape");
    let mut archive = archive.into_inner().unwrap();
    replace_first_member_path(&mut archive, b"../outside.md");

    let error = super::observe_tar(Cursor::new(archive), "archive").unwrap_err();

    assert!(error.to_string().contains("safe relative"));
}

#[test]
fn rejects_unsafe_pax_and_gnu_path_overrides() {
    let mut pax = Builder::new(Vec::new());
    pax.append_pax_extensions([("path", b"../outside.md" as &[u8])])
        .unwrap();
    append_file(&mut pax, "fallback.md", b"escape");
    let error = super::observe_tar(Cursor::new(pax.into_inner().unwrap()), "archive").unwrap_err();
    assert!(error.to_string().contains("safe relative"));

    let safe_long_path = "a".repeat(110);
    let unsafe_long_path = format!("../{}", "b".repeat(107));
    let mut gnu = Builder::new(Vec::new());
    append_file(&mut gnu, &safe_long_path, b"escape");
    let mut gnu = gnu.into_inner().unwrap();
    let path_offset = gnu
        .windows(safe_long_path.len())
        .position(|window| window == safe_long_path.as_bytes())
        .unwrap();
    gnu[path_offset..path_offset + unsafe_long_path.len()]
        .copy_from_slice(unsafe_long_path.as_bytes());

    let error = super::observe_tar(Cursor::new(gnu), "archive").unwrap_err();
    assert!(error.to_string().contains("safe relative"));
}

#[test]
fn enforces_each_observation_budget_during_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("one.md"), b"12").unwrap();
    fs::write(root.join("two.md"), b"34").unwrap();

    let too_many = ObservationLimits {
        max_observations: 1,
        max_resource_bytes: 8,
        max_total_bytes: 8,
    };
    assert!(observe_path_with_limits(root, too_many)
        .unwrap_err()
        .to_string()
        .contains("observation limit"));

    let too_large = ObservationLimits {
        max_observations: 8,
        max_resource_bytes: 1,
        max_total_bytes: 8,
    };
    assert!(observe_path_with_limits(root, too_large)
        .unwrap_err()
        .to_string()
        .contains("resource byte limit"));

    let too_much_total = ObservationLimits {
        max_observations: 8,
        max_resource_bytes: 8,
        max_total_bytes: 3,
    };
    assert!(observe_path_with_limits(root, too_much_total)
        .unwrap_err()
        .to_string()
        .contains("total byte limit"));
}

#[test]
fn rejects_unsafe_source_paths() {
    let observation = ArtifactObservation::new(
        vec![],
        "../escape.md",
        false,
        InputOrigin::Filesystem,
        "input",
        None,
    );

    assert!(observation.is_err());
}

#[test]
fn normalizes_safe_relative_input_labels_without_retaining_host_paths() {
    assert_eq!(
        super::input_label(Path::new("./rules/./AGENTS.md")).unwrap(),
        "rules/AGENTS.md"
    );
    assert!(super::input_label(Path::new("."))
        .unwrap()
        .starts_with("input_"));
    assert!(super::input_label(Path::new("../escape")).is_err());
}
