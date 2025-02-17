use std::collections::BTreeMap;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;

use zip::write::{ExtendedFileOptions, FileOptionExtension, FileOptions};
use zip::{CompressionMethod, ZipArchive, ZipWriter};

type Result<T> = anyhow::Result<T>;

#[derive(Debug, Copy, Clone)]
pub struct Index {
    pub archive_index: usize,
    pub file_index: usize,
}

fn default_file_selector(indices: &[Index]) -> Index {
    assert!(!indices.is_empty());

    indices
        .iter()
        .max_by_key(|&index| index.archive_index)
        .unwrap()
        .clone()
}

pub struct ZipMerger<R, F> {
    archives: Vec<ZipArchive<R>>,
    files: BTreeMap<PathBuf, Vec<Index>>,

    file_selector: F,
}

impl<R> ZipMerger<R, fn(&[Index]) -> Index>
where
    R: Read + Seek,
{
    pub fn new() -> Self {
        ZipMerger {
            archives: Vec::new(),
            files: BTreeMap::new(),
            file_selector: default_file_selector,
        }
    }
}

impl<R, F> ZipMerger<R, F>
where
    R: Read + Seek,
    F: Fn(&[Index]) -> Index,
{
    pub fn new_with_selector(file_selector: F) -> Self {
        ZipMerger {
            archives: Vec::new(),
            files: BTreeMap::new(),
            file_selector,
        }
    }

    pub fn add(&mut self, input: R) -> Result<()> {
        let archive_idx = self.archives.len();

        log::info!("adding archive#{}", archive_idx);
        let mut archive = ZipArchive::new(input)?;
        for file_idx in 0..archive.len() {
            let file = archive.by_index(file_idx)?;
            if let Some(path) = file.enclosed_name() {
                log::debug!(
                    "adding file: {:?} from archive#{} file#{}",
                    path,
                    archive_idx,
                    file_idx
                );
                self.files.entry(path).or_insert_with(Vec::new).push(Index {
                    archive_index: archive_idx,
                    file_index: file_idx,
                });
            } else {
                log::warn!("skipping file: {:?}, invalid name", file.name());
            }
        }

        self.archives.push(archive);
        Ok(())
    }

    pub fn write<W: Write + Seek>(&mut self, dest: W) -> Result<()> {
        let default_opt: FileOptions<ExtendedFileOptions> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);
        self.write_with_option(dest, default_opt)
    }

    pub fn write_with_option<W: Write + Seek>(
        &mut self,
        dest: W,
        option: FileOptions<impl FileOptionExtension + Clone>,
    ) -> Result<()> {
        let mut writer = ZipWriter::new(dest);

        log::info!("writing to the output archive");
        for (key, value) in self.files.iter() {
            writer.start_file_from_path(key, option.clone())?;

            let index = (self.file_selector)(value);
            let archive = &mut self.archives[index.archive_index];
            let mut file = archive.by_index(index.file_index)?;

            log::debug!(
                "copying file: {:?} from archive#{} file#{}",
                key,
                index.archive_index,
                index.file_index
            );
            io::copy(&mut file, &mut writer)?;
        }
        writer.finish()?;

        Ok(())
    }
}
