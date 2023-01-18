extern crate configparser;

use configparser::ini::Ini;

pub fn get_config(path: &str) -> Ini {
    let mut conf = Ini::new();

    conf.load(path).expect(
        &format!("Failed to load config from path: {}", path)
    );

    return conf;
}