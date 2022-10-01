use clap::{Arg, Command};
use common_assets::find_root;
use veloren_client_i18n::{
    analysis::{Language, ReferenceLanguage},
    REFERENCE_LANG,
};

fn main() {
    let args = Command::new("i18n-check")
        .about("Tool to check your Veloren localisation for correctness and missing keys")
        .arg(
            Arg::new("CODE")
                .required(true)
                .help("Run diagnostic for specific language code (de_DE, for example)"),
        )
        .get_matches();

    let root = find_root().unwrap();
    let i18n_directory = root.join("assets/voxygen/i18n");
    let reference = ReferenceLanguage::at(&i18n_directory.join(REFERENCE_LANG));

    let code = args.value_of("CODE").expect("arg is required");
    let lang = Language {
        code: code.to_owned(),
        path: root.join(i18n_directory.join(code)),
    };
    let stats = reference.compare_with(&lang);
    println!("\t[Not found]: {}", stats.not_found.len());
    for key in stats.not_found {
        let key = &key.key;
        println!("{key}");
    }

    println!("\n\t[Unused]: {}", stats.unused.len());
    for key in stats.unused {
        let key = &key.key;
        println!("{key}")
    }
}
