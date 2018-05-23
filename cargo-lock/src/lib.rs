#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate semver;

use regex::Regex;
use semver::Version;

lazy_static! {
    static ref VERSION_MATCH: Regex =
        Regex::new(r###"(?m)name = "([^"]+)"\n\s*version = "([^"]+)""###).unwrap();
}

pub fn find_version(package_name: &'static str, cargo_lock: &str) -> Option<Version> {
    for captures in VERSION_MATCH.captures_iter(cargo_lock) {
        let capture_package_name = captures.get(1).unwrap();
        if package_name != capture_package_name.as_str() {
            continue;
        }

        return if let Ok(version) = Version::parse(captures.get(2).unwrap().as_str()) {
            Some(version)
        } else {
            // failed to parse
            None
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cargo_lock_example() {
        let example_lock = include_str!("../data/Cargo.lock.example");

        assert_eq!(find_version("aho-corasick", example_lock), Some(Version::parse("0.6.4").unwrap()));
        assert_eq!(find_version("ansi_term", example_lock), Some(Version::parse("0.11.0").unwrap()));
        assert_eq!(find_version("arrayvec", example_lock), Some(Version::parse("0.4.7").unwrap()));
        assert_eq!(find_version("atty", example_lock), Some(Version::parse("0.2.10").unwrap()));
    }
}
