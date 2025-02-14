use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
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

    let mut max = indices[0];
    for index in indices {
        if index.archive_idx > max.archive_idx {
            max = *index;
        }
    }
    max
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
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if let Some(path) = file.enclosed_name() {
                self.files.entry(path).or_insert(Vec::new()).push(Index {
                    archive_idx,
                    file_idx: i,
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
            let index = if value.len() == 1 {
                value[0]
            } else {
                (self.file_selector)(value.as_slice())
            };

            let archive = &mut self.archives[index.archive_idx];
            let mut file = archive.by_index(index.file_idx)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            writer.write_all(&buf)?;
        }
        writer.finish()?;

        Ok(())
    }
}
