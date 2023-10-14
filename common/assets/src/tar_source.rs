use assets_manager::source::{DirEntry, FileContent, Source};
use hashbrown::HashMap;
use tar::EntryType;

use std::{
    fmt,
    fs::File,
    hash, io,
    os::unix::prelude::FileExt,
    path::{self, Path, PathBuf},
};

// derived from the zip source from assets_manager

#[inline]
pub(crate) fn extension_of(path: &Path) -> Option<&str> {
    match path.extension() {
        Some(ext) => ext.to_str(),
        None => Some(""),
    }
}

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

/// An entry in a archive directory.
#[derive(Debug)]
enum OwnedEntry {
    File(FileDesc),
    //    Dir(String),
}

impl OwnedEntry {
    fn as_dir_entry(&self) -> DirEntry {
        match self {
            OwnedEntry::File(FileDesc(desc0, desc1)) => DirEntry::File(desc0, desc1),
            // OwnedEntry::Dir(id) => DirEntry::Directory(id),
        }
    }
}

/// Build ids from components.
///
/// Using this allows to easily reuse buffers when building several ids in a
/// row, and thus to avoid repeated allocations.
#[derive(Default)]
struct IdBuilder {
    segments: Vec<String>,
    len: usize,
}

impl IdBuilder {
    /// Pushs a segment in the builder.
    #[inline]
    fn push(&mut self, s: &str) {
        match self.segments.get_mut(self.len) {
            Some(seg) => {
                seg.clear();
                seg.push_str(s);
            },
            None => self.segments.push(s.to_owned()),
        }
        self.len += 1;
    }

    /// Joins segments to build a id.
    #[inline]
    fn join(&self) -> String { self.segments[..self.len].join(".") }

    /// Resets the builder without freeing buffers.
    #[inline]
    fn reset(&mut self) { self.len = 0; }
}

/// Register a file of an archive in maps.
fn register_file(
    path: &Path,
    position: usize,
    length: usize,
    files: &mut HashMap<FileDesc, (usize, usize)>,
    dirs: &mut HashMap<String, Vec<OwnedEntry>>,
    id_builder: &mut IdBuilder,
) {
    id_builder.reset();

    // Parse the path and register it.
    // The closure is used as a cheap `try` block.
    let ok = (|| {
        // Fill `id_builder` from the parent's components
        let parent = path.parent()?;
        for comp in parent.components() {
            match comp {
                path::Component::Normal(s) => {
                    let segment = s.to_str()?;
                    if segment.contains('.') {
                        return None;
                    }
                    id_builder.push(segment);
                },
                _ => return None,
            }
        }

        // Build the ids of the file and its parent.
        let parent_id = id_builder.join();
        id_builder.push(path.file_stem()?.to_str()?);
        let id = id_builder.join();

        // Register the file in the maps.
        let ext = extension_of(path)?.to_owned();
        let desc = FileDesc(id, ext);
        files.insert(desc.clone(), (position, length));
        let entry = OwnedEntry::File(desc);
        dirs.entry(parent_id).or_default().push(entry);

        Some(())
    })()
    .is_some();

    if !ok {
        tracing::warn!("Unsupported path in tar archive: {path:?}");
    }
}

enum Backend {
    File(PathBuf),
    // Buffer(&'static [u8]),
}

impl Backend {
    fn read(&self, pos: usize, len: usize) -> std::io::Result<Vec<u8>> {
        match self {
            Backend::File(path) => File::open(path).and_then(|file| {
                let mut result = vec![0; len];
                file.read_at(result.as_mut_slice(), pos as u64)
                    .map(move |_bytes| result)
            }),
            // Backend::Buffer(_) => todo!(),
        }
    }
}

pub struct Tar {
    files: HashMap<FileDesc, (usize, usize)>,
    dirs: HashMap<String, Vec<OwnedEntry>>,
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
        let mut id_builder = IdBuilder::default();
        for e in contents.flatten() {
            if matches!(e.header().entry_type(), EntryType::Regular) {
                register_file(
                    e.path().map_err(io::Error::other)?.as_ref(),
                    e.raw_file_position() as usize,
                    e.size() as usize,
                    &mut files,
                    &mut dirs,
                    &mut id_builder,
                );
            }
        }
        Ok(Tar {
            files,
            dirs,
            backend: Backend::File(path.to_path_buf()),
        })
    }
}

impl Source for Tar {
    fn read(&self, id: &str, ext: &str) -> io::Result<FileContent> {
        let key: &dyn FileKey = &(id, ext);
        let id = *self
            .files
            .get(key)
            .or_else(|| {
                // also accept assets within the assets dir for now
                let with_prefix = "assets.".to_string() + id;
                let prefixed_key: &dyn FileKey = &(with_prefix.as_str(), ext);
                self.files.get(prefixed_key)
            })
            .ok_or(io::ErrorKind::NotFound)?;
        self.backend.read(id.0, id.1).map(FileContent::Buffer)
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        let dir = self.dirs.get(id).ok_or(io::ErrorKind::NotFound)?;
        dir.iter().map(OwnedEntry::as_dir_entry).for_each(f);
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
