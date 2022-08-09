use std::{
    io,
    path::{Path, PathBuf},
};

/// Represent tree of directory, result of [generate_tree].
///
/// Note that paths are always relative to root it was generated from.
#[derive(Debug, Clone)]
pub enum Walk {
    /// Represents file node, path is relative to directory root Walk was
    /// generated from.
    File(PathBuf),
    /// Represents directory subtree, path is relative to directory root Walk
    /// was generated from.
    Dir { path: PathBuf, content: Vec<Walk> },
}

impl Walk {
    /// Utility function to build a tree of directory, recursively
    ///
    /// Path needs to be absolute.
    pub fn generate(root: &Path) -> io::Result<Walk> {
        let trees = walk_tree(root, root);
        Ok(Walk::Dir {
            path: Path::new("").to_owned(),
            content: trees?,
        })
    }

    // TODO: implement iterator?
    pub fn for_each_file<F>(&self, root: &Path, f: &mut F)
    where
        F: FnMut(&Path),
    {
        match self {
            Self::File(filepath) => {
                let path = root.join(filepath);
                f(&path);
            },
            Self::Dir {
                path: _,
                content: files,
            } => {
                for path in files {
                    path.for_each_file(root, f);
                }
            },
        }
    }
}

/// Helper function to [Walk::generate()], prefer using it instead.
pub fn walk_tree(dir: &Path, root: &Path) -> io::Result<Vec<Walk>> {
    let mut buff = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            buff.push(Walk::Dir {
                path: path
                    .strip_prefix(root)
                    .expect("strip can't fail, this path is created from root")
                    .to_owned(),
                content: walk_tree(&path, root)?,
            });
        } else {
            let filename = path
                .strip_prefix(root)
                .expect("strip can't fail, this file is created from root")
                .to_owned();
            buff.push(Walk::File(filename));
        }
    }

    Ok(buff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trie() {
        let root = crate::find_root().unwrap();
        let assets = Path::new(&root).join("assets/");
        Walk::generate(&assets).unwrap();
    }
}
