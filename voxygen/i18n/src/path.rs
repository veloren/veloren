use std::path::{Path, PathBuf};

pub(crate) const LANG_MANIFEST_FILE: &str = "_manifest";
pub(crate) const LANG_EXTENSION: &str = "ron";

#[derive(Clone)]
pub struct BasePath {
    ///repo part, git main folder
    root_path: PathBuf,
    ///relative path to i18n path which contains, currently
    /// 'assets/voxygen/i18n'
    relative_i18n_root_path: PathBuf,
    ///i18n_root_folder
    cache: PathBuf,
}

impl BasePath {
    pub fn new(root_path: &Path) -> Self {
        let relative_i18n_root_path = Path::new("assets/voxygen/i18n").to_path_buf();
        let cache = root_path.join(&relative_i18n_root_path);
        assert!(
            cache.is_dir(),
            "i18n_root_path folder doesn't exist, something is wrong!"
        );
        Self {
            root_path: root_path.to_path_buf(),
            relative_i18n_root_path,
            cache,
        }
    }

    pub fn root_path(&self) -> &Path { &self.root_path }

    pub fn relative_i18n_root_path(&self) -> &Path { &self.relative_i18n_root_path }

    /// absolute path to `relative_i18n_root_path`
    pub fn i18n_root_path(&self) -> &Path { &self.cache }

    pub fn i18n_path(&self, language_identifier: &str) -> LangPath {
        LangPath::new(self, language_identifier)
    }

    /// List localization directories
    pub fn i18n_directories(&self) -> Vec<LangPath> {
        std::fs::read_dir(&self.cache)
            .unwrap()
            .map(|res| res.unwrap())
            .filter(|e| e.file_type().unwrap().is_dir())
            .map(|e| LangPath::new(self, e.file_name().to_str().unwrap()))
            .collect()
    }
}

impl core::fmt::Debug for BasePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &self.cache)
    }
}

#[derive(Clone)]
pub struct LangPath {
    base: BasePath,
    ///  `en`, `de_DE`, `fr_FR`, etc..
    language_identifier: String,
    /// i18n_path
    cache: PathBuf,
}

impl LangPath {
    fn new(base: &BasePath, language_identifier: &str) -> Self {
        let cache = base.i18n_root_path().join(language_identifier);
        if !cache.is_dir() {
            panic!("language folder '{}' doesn't exist", language_identifier);
        }
        Self {
            base: base.clone(),
            language_identifier: language_identifier.to_owned(),
            cache,
        }
    }

    pub fn base(&self) -> &BasePath { &self.base }

    pub fn language_identifier(&self) -> &str { &self.language_identifier }

    ///absolute path to `i18n_root_path` + `language_identifier`
    pub fn i18n_path(&self) -> &Path { &self.cache }

    /// fragment or manifest file, based on a path
    pub fn sub_path(&self, sub_path: &Path) -> PathBuf { self.cache.join(sub_path) }

    /// fragment or manifest file, based on a string without extension
    pub fn file(&self, name_without_extension: &str) -> PathBuf {
        self.cache
            .join(format!("{}.{}", name_without_extension, LANG_EXTENSION))
    }

    /// return all fragments sub_pathes
    pub(crate) fn fragments(&self) -> Result<Vec</* sub_path */ PathBuf>, std::io::Error> {
        let mut result = vec![];
        recursive_fragments_paths_in_language(self, Path::new(""), &mut result)?;
        Ok(result)
    }
}

//unwraps cant fail as they are in same Path
fn recursive_fragments_paths_in_language(
    lpath: &LangPath,
    subfolder: &Path,
    result: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    let search_dir = lpath.sub_path(subfolder);
    for fragment_file in search_dir.read_dir()?.flatten() {
        let file_type = fragment_file.file_type()?;
        let full_path = fragment_file.path();
        let relative_path = full_path.strip_prefix(lpath.i18n_path()).unwrap();
        if file_type.is_dir() {
            recursive_fragments_paths_in_language(lpath, relative_path, result)?;
        } else if file_type.is_file()
            && relative_path != Path::new(&format!("{}.{}", LANG_MANIFEST_FILE, LANG_EXTENSION))
        {
            result.push(relative_path.to_path_buf());
        }
    }
    Ok(())
}

impl core::fmt::Debug for LangPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?}",
            self.base
                .relative_i18n_root_path
                .join(&self.language_identifier)
        )
    }
}
