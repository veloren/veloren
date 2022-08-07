use std::{
    io,
    path::{Path, PathBuf},
};

/// Read `walk_tree`
#[derive(Debug)]
pub enum Walk {
    File(PathBuf),
    Dir { path: PathBuf, content: Vec<Walk> },
}

/// Utility function to build a tree of directory, recursively
///
/// At first iteration, use path to your directory as dir and root
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
