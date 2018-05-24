use semver::Version;

/// Pick the correct version given some main version. If no version is found
/// then the latest version is assumed to work.
///
/// For example: if the main version is 0.3.1, and the versions are
/// [0.2.0, 0.3.0] then 0.3.0 will be chosen because it was the most up-to-date.
///
/// ```rust
/// ```
pub fn choose_version_by_key<T>(
    main_version: Version,
    items: Vec<T>,
    key_fn: impl Fn(&T) -> Option<Version>,
) -> Option<T>
{
    assert!(!items.is_empty());

    let mut filtered_items = items.into_iter()
        .map(|i| {
            let version = key_fn(&i);
            (i, version)
        })
        .filter(move |(_i, version)| {
            if let Some(version) = version {
                version <= &main_version
            } else {
                false
            }
        })
        .map(|(i, version)| (i, version.expect("invalid versions are filtered out")))
        .collect::<Vec<_>>();

    if filtered_items.is_empty() {
        return None;
    }

    // Sort from greatest version to least
    filtered_items.sort_unstable_by(|(_i, version), (_other_i, other_version)| other_version.cmp(version));

    Some(filtered_items.remove(0).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_version_by_key_chooses_latest() {
        let main_version = Version::parse("0.3.1").unwrap();
        let items = vec!["0.2.0", "0.3.0"];

        let chosen = choose_version_by_key(main_version, items, |s| Version::parse(s).ok());
        assert_eq!(chosen, Some("0.3.0"));
    }

    #[test]
    fn choose_version_by_key_picks_matching_if_possible() {
        let main_version = Version::parse("0.3.1").unwrap();
        let items = vec!["0.2.0", "0.3.0", "0.3.1", "0.5.2"];

        let chosen = choose_version_by_key(main_version, items, |s| Version::parse(s).ok());
        assert_eq!(chosen, Some("0.3.1"));
    }

    #[test]
    fn choose_version_by_key_with_no_matching() {
        let main_version = Version::parse("0.1.1").unwrap();
        let items = vec!["0.2.0", "0.3.0", "0.3.1", "0.5.2"];

        let chosen = choose_version_by_key(main_version, items, |s| Version::parse(s).ok());
        assert_eq!(chosen, None);
    }
}
