use clap::{App, Arg};
use std::path::Path;
use veloren_i18n::{analysis, verification};

fn main() {
    let matches = App::new("i18n-check")
        .version("0.1.0")
        .author("juliancoffee <lightdarkdaughter@gmail.com>")
        .about("Test veloren localizations")
        .arg(
            Arg::with_name("CODE")
                .required(false)
                .help("Run diagnostic for specific language code (de_DE as example)"),
        )
        .arg(
            Arg::with_name("verify")
                .long("verify")
                .help("verify all localizations"),
        )
        .arg(
            Arg::with_name("test")
                .long("test")
                .help("test all localizations"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("print additional information"),
        )
        .get_matches();

    // Generate paths
    let root = common_assets::find_root().expect("Failed to find root of repository");
    let asset_path = Path::new("assets/voxygen/i18n/");

    if let Some(code) = matches.value_of("CODE") {
        analysis::test_specific_localization(
            code,
            &root,
            &asset_path,
            matches.is_present("verbose"),
        );
    }
    if matches.is_present("test") {
        analysis::test_all_localizations(&root, &asset_path, matches.is_present("verbose"));
    }
    if matches.is_present("verify") {
        verification::verify_all_localizations(&root, &asset_path);
    }
}
