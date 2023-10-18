use assets_manager::source::{DirEntry, FileContent, Source};
use hashbrown::HashMap;
use tar::EntryType;

use std::{
    fmt,
    fs::File,
    hash,
    io::{self, Read, Seek, SeekFrom},
    path::{self, Path, PathBuf},
};

// Derived from the zip source in the assets_manager crate

#[derive(Clone, Hash, PartialEq, Eq)]
struct FileDesc(String, String);

/// This hack enables us to use a `(&str, &str)` as a key for an HashMap without
/// allocating a `FileDesc`
trait FileKey {
    fn id(&self) -> &str;
    fn ext(&self) -> &str;
}

impl FileKey for FileDesc {
    fn id(&self) -> &str { &self.0 }

    fn ext(&self) -> &str { &self.1 }
}

impl FileKey for (&'_ str, &'_ str) {
    fn id(&self) -> &str { self.0 }

    fn ext(&self) -> &str { self.1 }
}

impl<'a> std::borrow::Borrow<dyn FileKey + 'a> for FileDesc {
    fn borrow(&self) -> &(dyn FileKey + 'a) { self }
}

impl PartialEq for dyn FileKey + '_ {
    fn eq(&self, other: &Self) -> bool { self.id() == other.id() && self.ext() == other.ext() }
}

impl Eq for dyn FileKey + '_ {}

impl hash::Hash for dyn FileKey + '_ {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.id().hash(hasher);
        self.ext().hash(hasher);
    }
}

impl fmt::Debug for FileDesc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FileDesc")
            .field("id", &self.0)
            .field("ext", &self.1)
            .finish()
    }
}

impl FileDesc {
    fn as_dir_entry(&self) -> DirEntry { DirEntry::File(&self.0, &self.1) }
}

/// Register a file of an archive in maps, components in asset ids are separated
/// by points
fn register_file(
    path: &Path,
    position: u64,
    length: usize,
    files: &mut HashMap<FileDesc, (u64, usize)>,
    dirs: &mut HashMap<String, Vec<FileDesc>>,
) {
    // Parse the path and register it.
    let mut parent_id = String::default();
    // The closure is used as a cheap `try` block.
    let unsupported_path = (|| {
        let parent = path.parent()?;
        for comp in parent.components() {
            match comp {
                path::Component::Normal(s) => {
                    let segment = s.to_str()?;
                    // Reject paths with extensions
                    if segment.contains('.') {
                        return None;
                    }
                    if !parent_id.is_empty() {
                        parent_id.push('.');
                    }
                    parent_id.push_str(segment);
                },
                // Reject paths with non-name components
                _ => return None,
            }
        }

        let file_id = parent_id.clone() + "." + path.file_stem()?.to_str()?;
        // Register the file in the maps.
        let ext = path.extension().unwrap_or_default().to_str()?.to_owned();
        let desc = FileDesc(file_id, ext);
        files.insert(desc.clone(), (position, length));
        dirs.entry(parent_id).or_default().push(desc);

        Some(())
    })()
    .is_none();
    if unsupported_path {
        tracing::error!("Unsupported path in tar archive: {path:?}");
    }
}

// We avoid the extra dependency of sync_file introduced by Zip here by opening
// the file for each read
struct Backend(PathBuf);

impl Backend {
    fn read(&self, pos: u64, len: usize) -> std::io::Result<Vec<u8>> {
        File::open(self.0.clone()).and_then(|mut file| {
            file.seek(SeekFrom::Start(pos)).and_then(|_| {
                let mut result = vec![0; len];
                file.read_exact(result.as_mut_slice())
                    .map(move |_num_bytes| result)
            })
        })
    }
}

pub struct Tar {
    files: HashMap<FileDesc, (u64, usize)>,
    dirs: HashMap<String, Vec<FileDesc>>,
    backend: Backend,
}

impl Tar {
    /// Creates a `Tar` from a file
    pub fn from_path(path: &Path) -> io::Result<Tar> {
        let file = File::open(path)?;
        let mut tar = tar::Archive::new(file);
        let contents = tar
            .entries()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut files = HashMap::with_capacity(contents.size_hint().0);
        let mut dirs = HashMap::new();
        for entry in contents.flatten() {
            if matches!(entry.header().entry_type(), EntryType::Regular) {
                register_file(
                    entry.path().map_err(io::Error::other)?.as_ref(),
                    entry.raw_file_position(),
                    entry.size() as usize,
                    &mut files,
                    &mut dirs,
                );
            }
        }
        Ok(Tar {
            files,
            dirs,
            backend: Backend(path.to_path_buf()),
        })
    }
}

impl Source for Tar {
    fn read(&self, id: &str, ext: &str) -> io::Result<FileContent> {
        let key: &dyn FileKey = &(id, ext);
        let id = *self.files.get(key).ok_or(io::ErrorKind::NotFound)?;
        self.backend.read(id.0, id.1).map(FileContent::Buffer)
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        let dir = self.dirs.get(id).ok_or(io::ErrorKind::NotFound)?;
        dir.iter().map(FileDesc::as_dir_entry).for_each(f);
        Ok(())
    }

    fn exists(&self, entry: DirEntry) -> bool {
        match entry {
            DirEntry::File(id, ext) => self.files.contains_key(&(id, ext) as &dyn FileKey),
            DirEntry::Directory(id) => self.dirs.contains_key(id),
        }
    }
}

impl fmt::Debug for Tar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tar")
            .field("files", &self.files)
            .field("dirs", &self.dirs)
            .finish()
    }
}
