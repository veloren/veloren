use std::{env::args, path::Path, vec::Vec};
use veloren_i18n_check::analysis;

fn main() {
    let cli: Vec<String> = args().collect();

    // Generate paths
    let curr_dir = std::env::current_dir().unwrap();
    let root = curr_dir.parent().unwrap().parent().unwrap();
    let asset_path = Path::new("assets/voxygen/i18n/");
    for (i, arg) in cli.iter().enumerate() {
        match arg.as_str() {
            "--all" => analysis::test_all_localizations(root, asset_path),
            "--verify" => analysis::verify_all_localizations(root, asset_path),
            "--lang" => {
                let code = cli[i + 1].clone();
                analysis::test_specific_localization(code, root, asset_path);
            },
            _ => continue,
        }
    }
}
