#[macro_use] extern crate serde_derive;
extern crate toml;

#[derive(Deserialize)]
pub struct CargoToml {
    pub package: Package,
}

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
}

impl CargoToml {
    pub fn from_str(input: &str) -> Result<CargoToml, toml::de::Error> {
        toml::from_str(input)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn it_works() {
        let mut cargo_file_contents = String::new();
        let mut cargo_file = File::open("Cargo.toml").unwrap();
        cargo_file.read_to_string(&mut cargo_file_contents).unwrap();

        let cargo_toml = CargoToml::from_str(&cargo_file_contents).unwrap();
        assert_eq!(cargo_toml.package.name, "cargo-toml");
    }
}
