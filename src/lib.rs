use std::collections::BTreeMap;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;

use zip::write::{ExtendedFileOptions, FileOptions};
use zip::{CompressionMethod, ZipArchive, ZipWriter};

type Result<T> = anyhow::Result<T>;

#[derive(Debug, Copy, Clone)]
struct Index {
    archive_idx: usize,
    file_idx: usize,
}

fn default_file_selector(indices: &[Index]) -> Index {
    assert!(!indices.is_empty());

    indices
        .iter()
        .max_by_key(|&index| index.archive_idx)
        .unwrap()
        .clone()
}

pub struct ZipMerger<R> {
    archives: Vec<ZipArchive<R>>,
    files: BTreeMap<PathBuf, Vec<Index>>,

    file_selector: fn(&[Index]) -> Index,
}

impl<R: Read + Seek> ZipMerger<R> {
    pub fn new() -> Self {
        ZipMerger {
            archives: Vec::new(),
            files: BTreeMap::new(),
            file_selector: default_file_selector,
        }
    }

    pub fn add(&mut self, input: R) -> Result<()> {
        let archive_idx = self.archives.len();

        let mut archive = ZipArchive::new(input)?;
        for file_idx in 0..archive.len() {
            let file = archive.by_index(file_idx)?;
            if let Some(path) = file.enclosed_name() {
                self.files.entry(path).or_insert_with(Vec::new).push(Index {
                    archive_idx,
                    file_idx,
                });
            }
        }

        self.archives.push(archive);
        Ok(())
    }

    pub fn write<W: Write + Seek>(&mut self, dest: W) -> Result<()> {
        let mut writer = ZipWriter::new(dest);

        let opt: FileOptions<ExtendedFileOptions> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for (key, value) in self.files.iter() {
            writer.start_file_from_path(key, opt.clone())?;

            let index = (self.file_selector)(value);
            let archive = &mut self.archives[index.archive_idx];
            let mut file = archive.by_index(index.file_idx)?;

            io::copy(&mut file, &mut writer)?;
        }
        writer.finish()?;

        Ok(())
    }
}
