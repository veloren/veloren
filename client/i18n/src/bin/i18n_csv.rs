use common_assets::find_root;
use std::{fs, io::Write, path::Path};
use veloren_client_i18n::{
    analysis::{Language, ReferenceLanguage},
    list_localizations, REFERENCE_LANG,
};

fn main() {
    let root = find_root().unwrap();
    let output = root.join("translation_analysis.csv");
    let mut f = fs::File::create(output).expect("couldn't write csv file");

    writeln!(
        f,
        "country_code,file_name,translation_key,status,git_commit"
    )
    .unwrap();

    let i18n_directory = root.join("assets/voxygen/i18n");
    let reference = ReferenceLanguage::at(&i18n_directory.join(REFERENCE_LANG));

    let list = list_localizations();
    let file = |filename| {
        let file = Path::new(&filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<err>");

        file.to_string()
    };
    for meta in list {
        let code = meta.language_identifier;
        let lang = Language {
            code: code.clone(),
            path: i18n_directory.join(code.clone()),
        };
        let stats = reference.compare_with(&lang);
        for key in stats.up_to_date {
            let code = &code;
            let filename = file(key.file);
            let key = &key.key;
            writeln!(f, "{code},{filename},{key},UpToDate,None").unwrap();
        }
        for key in stats.not_found {
            let code = &code;
            let filename = file(key.file);
            let key = &key.key;
            writeln!(f, "{code},{filename},{key},NotFound,None").unwrap();
        }
        for key in stats.unused {
            let code = &code;
            let filename = file(key.file);
            let key = &key.key;
            writeln!(f, "{code},{filename},{key},Unused,None").unwrap();
        }
    }
}
