//! Docker images are pinned by digest, so a run uses exactly the image this build was tested with.
//! `node scripts/update-image-digests.mjs` refreshes the list.

const PINNED: &[(&str, &str)] = &[
    ("composer:2", "sha256:4ed71c38c6d788eb4e4e3747b5332f5b40d5b5a6fcadd5062a1a70986a118e0a"),
    ("mysql:8.0", "sha256:7dcddc01f13bab2f15cde676d44d01f61fc9f99fe7785e86196dfc07d358ae2b"),
    ("postgres:16", "sha256:5db6dac2ef5dcff512f4f00a009c3c332b9f2f28f24277175bf3e4a3d02ef83a"),
    ("php:5.6-cli", "sha256:6ce95208609dc66df163ab936c970b3b34cd901b85c747102c5999f08ade9143"),
    ("php:7.0-cli", "sha256:950fa73e35e650ee1d5fff5b053feadcd1dc8c6e83bba07d6c7fd17c44868e22"),
    ("php:7.1-cli", "sha256:e0e6ced092bbf073e4f3e48545a8e1a2592217b3346ef0cb8b18ecacf8ae1522"),
    ("php:7.2-cli", "sha256:42ffbc0798e4449bbd1e14fc4dcb87774aa1ad1900a09ef6a965bc0880aa2161"),
    ("php:7.3-cli", "sha256:bd6f2dc7a4e4537e927355c94a74c3022911aa504278f5f0fd94016c90dfaecc"),
    ("php:7.4-cli", "sha256:620a6b9f4d4feef2210026172570465e9d0c1de79766418d3affd09190a7fda5"),
    ("php:8.0-cli", "sha256:0569e384b9064c04dec55dc6e41be41b494a878dfbb6577a7d76bd50cfd5bc00"),
    ("php:8.1-cli", "sha256:76e563191d1ade120313a8736df24154d21da5155c0756f147c0b01bd19d9087"),
    ("php:8.2-cli", "sha256:a76ea7d3dd8f65eefff85e9c883f4d146808e0075c30ae6bccbb2a3753d373b8"),
    ("php:8.3-cli", "sha256:600bce7c7acf524756e5aac989f4a763aed709960d7326970bb670f46efee53a"),
    ("php:8.4-cli", "sha256:7e7413ed31b8c42b31c939cc83b3758b1cd618f7bf3803f5a0c39f98d4adcbee"),
    ("php:8.5-cli", "sha256:9236fd373d3bcda0659cf94838a744bd79791419292958963f8c6b34a0071554"),
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
