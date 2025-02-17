use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

use zip::write::{ExtendedFileOptions, FileOptions};
use zip::ZipArchive;
use zip_merge;
use zip_merge::ZipMerger;

struct TestFile {
    name: &'static Path,
    data: &'static [u8],
}

fn create_zip(files: &[TestFile]) -> anyhow::Result<Cursor<Vec<u8>>> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for file in files {
        let options: FileOptions<ExtendedFileOptions> = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);
        archive.start_file_from_path(file.name, options)?;
        archive.write_all(file.data)?;
    }
    Ok(archive.finish()?)
}

fn extract_zip<R: Read + Seek>(zip: R) -> anyhow::Result<HashMap<PathBuf, Vec<u8>>> {
    let mut archive = ZipArchive::new(zip)?;

    let mut files = HashMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        files.insert(file.mangled_name(), data);
    }
    Ok(files)
}

macro_rules! zip {
    ($($name:expr => $data:expr),* $(,)?) => {
        create_zip(&[
            $(
                TestFile {
                    name: ::std::path::Path::new($name),
                    data: $data,
                },
            )*
        ])
    };
}

#[test]
fn test_zip_merger() {
    let zip1 = zip! {
        "foo" => b"foo",
        "hello" => b"hello",
    }
    .unwrap();
    let zip2 = zip! {
        "hello" => b"HELLO",
        "world" => b"WORLD",
    }
    .unwrap();

    let mut merger = ZipMerger::new();
    assert!(merger.add(zip1).is_ok());
    assert!(merger.add(zip2).is_ok());

    let mut output = Cursor::new(Vec::new());
    assert!(merger.write(&mut output).is_ok());

    let merged_files = extract_zip(output);
    assert!(merged_files.is_ok());
    let merged_files = merged_files.unwrap();

    assert_eq!(merged_files.len(), 3);
    assert_eq!(merged_files[Path::new("foo")], b"foo");
    assert_eq!(merged_files[Path::new("hello")], b"HELLO");
    assert_eq!(merged_files[Path::new("world")], b"WORLD");
}

#[test]
fn test_zip_merger_with_file_selector() {
    let zip1 = zip! {
        "foo" => b"foo",
        "hello" => b"hello",
    }
    .unwrap();
    let zip2 = zip! {
        "hello" => b"HELLO",
        "world" => b"WORLD",
    }
    .unwrap();

    let mut merger = ZipMerger::new_with_selector(|files| files[0]);
    assert!(merger.add(zip1).is_ok());
    assert!(merger.add(zip2).is_ok());

    let mut output = Cursor::new(Vec::new());
    assert!(merger.write(&mut output).is_ok());

    let merged_files = extract_zip(output);
    assert!(merged_files.is_ok());
    let merged_files = merged_files.unwrap();

    assert_eq!(merged_files.len(), 3);
    assert_eq!(merged_files[Path::new("foo")], b"foo");
    assert_eq!(merged_files[Path::new("hello")], b"hello");
    assert_eq!(merged_files[Path::new("world")], b"WORLD");
}
