extern crate dot_vox;
use std::fs;

#[test]
fn validate_vox_files() {
    let paths = fs::read_dir("./vox").unwrap();
    
    for path in paths {
        let path_string = path.unwrap().path().into_os_string().into_string().unwrap();
        let vox = dot_vox::load(&path_string);
        assert_eq!(true, vox.is_ok(), "Failed to validate file '{:?}'", path_string);
    }
}
