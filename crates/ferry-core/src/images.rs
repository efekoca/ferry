//! Docker images are pinned by digest, so a run uses exactly the image this build was tested with.
//! `node scripts/update-image-digests.mjs` refreshes the list.

const PINNED: &[(&str, &str)] = &[
    ("composer:2", "sha256:8de2184c6de224f4689e8f80ed5f5edaf0030b6b8b7839a22a2bcb6722ac87d0"),
    ("mysql:8.0", "sha256:7dcddc01f13bab2f15cde676d44d01f61fc9f99fe7785e86196dfc07d358ae2b"),
    ("postgres:16", "sha256:f9a7b9ebf6f56201f949db745e47bc423c0f0f8294bafe991cc10bf74b029657"),
    ("php:5.6-cli", "sha256:6ce95208609dc66df163ab936c970b3b34cd901b85c747102c5999f08ade9143"),
    ("php:7.0-cli", "sha256:950fa73e35e650ee1d5fff5b053feadcd1dc8c6e83bba07d6c7fd17c44868e22"),
    ("php:7.1-cli", "sha256:e0e6ced092bbf073e4f3e48545a8e1a2592217b3346ef0cb8b18ecacf8ae1522"),
    ("php:7.2-cli", "sha256:42ffbc0798e4449bbd1e14fc4dcb87774aa1ad1900a09ef6a965bc0880aa2161"),
    ("php:7.3-cli", "sha256:bd6f2dc7a4e4537e927355c94a74c3022911aa504278f5f0fd94016c90dfaecc"),
    ("php:7.4-cli", "sha256:620a6b9f4d4feef2210026172570465e9d0c1de79766418d3affd09190a7fda5"),
    ("php:8.0-cli", "sha256:0569e384b9064c04dec55dc6e41be41b494a878dfbb6577a7d76bd50cfd5bc00"),
    ("php:8.1-cli", "sha256:76e563191d1ade120313a8736df24154d21da5155c0756f147c0b01bd19d9087"),
    ("php:8.2-cli", "sha256:604de2f0cba9e9dde16cf717c2f03b0948c99949bdf7c11cdf76326e0da7d2ea"),
    ("php:8.3-cli", "sha256:628102249d93494233b54490b785f12db9cd60f4dc89416dfca15973f2125f10"),
    ("php:8.4-cli", "sha256:d63e8b80cd4c6cfd01cfa7c910522e299cdc7ca1a2b6d6563db48df3c17e6c55"),
    ("php:8.5-cli", "sha256:50956f094e90a716d43bea28087d447555ad80510e386ab8c9c10d8c70d781a9"),
];

/// A tag outside the list stays as it is, so a PHP newer than this build can still be tried.
pub fn pinned(tag: &str) -> String {
    match PINNED.iter().find(|(name, _)| *name == tag) {
        Some((_, digest)) => format!("{tag}@{digest}"),
        None => tag.to_string(),
    }
}

/// The readable name, for reports and messages.
pub fn tag_of(image: &str) -> &str {
    image.split('@').next().unwrap_or(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_image_has_a_full_digest() {
        for (tag, digest) in PINNED {
            let hex = digest.strip_prefix("sha256:").unwrap_or_default();
            assert!(hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()), "{tag}: {digest}");
        }
    }

    #[test]
    fn php_images_are_pinned_and_keep_a_readable_name() {
        let image = pinned("php:8.4-cli");
        assert!(image.starts_with("php:8.4-cli@sha256:"), "{image}");
        assert_eq!(tag_of(&image), "php:8.4-cli");
        assert_eq!(pinned("php:9.9-cli"), "php:9.9-cli");
        assert_eq!(tag_of("ferry-php:8.4-abc"), "ferry-php:8.4-abc");
    }
}
