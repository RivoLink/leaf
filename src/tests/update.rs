use crate::update::TestAsset;
use crate::*;

#[test]
fn asset_name_matches_supported_release_targets() {
    assert_eq!(
        asset_name_for_target("macos", "x86_64"),
        Some("leaf-macos-x86_64")
    );
    assert_eq!(
        asset_name_for_target("macos", "aarch64"),
        Some("leaf-macos-arm64")
    );
    assert_eq!(
        asset_name_for_target("linux", "x86_64"),
        Some("leaf-linux-x86_64")
    );
    assert_eq!(
        asset_name_for_target("linux", "aarch64"),
        Some("leaf-linux-arm64")
    );
    assert_eq!(
        asset_name_for_target("android", "aarch64"),
        Some("leaf-android-arm64")
    );
    assert_eq!(
        asset_name_for_target("windows", "x86_64"),
        Some("leaf-windows-x86_64.exe")
    );
    assert_eq!(asset_name_for_target("linux", "arm"), None);
}

#[test]
fn newer_version_comparison_accepts_optional_v_prefix() {
    assert!(is_newer_version("1.4.2", "v1.4.3").unwrap());
    assert!(!is_newer_version("1.4.2", "1.4.2").unwrap());
    assert!(!is_newer_version("1.4.2", "1.4.1").unwrap());
}

#[test]
fn expected_asset_download_url_selects_matching_asset() {
    let assets = vec![
        TestAsset {
            name: "leaf-linux-x86_64",
            download_url: "https://example.test/linux",
        },
        TestAsset {
            name: "leaf-windows-x86_64.exe",
            download_url: "https://example.test/windows",
        },
    ];

    let url = expected_asset_download_url("1.4.3", &assets, "leaf-linux-x86_64").unwrap();
    assert_eq!(url, "https://example.test/linux");
}

#[test]
fn expected_asset_download_url_errors_when_asset_is_missing() {
    let assets = vec![TestAsset {
        name: "leaf-linux-x86_64",
        download_url: "https://example.test/linux",
    }];

    let err = expected_asset_download_url("1.4.3", &assets, "leaf-macos-arm64").unwrap_err();
    assert!(err.to_string().contains("does not contain asset"));
}

#[test]
fn validate_download_size_accepts_matching_non_zero_sizes() {
    assert!(validate_download_size(Some(42), 42).is_ok());
    assert!(validate_download_size(None, 42).is_ok());
}

#[test]
fn validate_download_size_rejects_zero_or_mismatched_sizes() {
    let empty_err = validate_download_size(None, 0).unwrap_err();
    assert!(empty_err.to_string().contains("is empty"));

    let mismatch_err = validate_download_size(Some(42), 41).unwrap_err();
    assert!(mismatch_err.to_string().contains("size mismatch"));
}

#[test]
fn find_expected_checksum_extracts_matching_asset_checksum() {
    let checksums = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  leaf-linux-x86_64
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  leaf-windows-x86_64.exe
";

    let checksum = find_expected_checksum(checksums, "leaf-windows-x86_64.exe").unwrap();
    assert_eq!(
        checksum,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    );
}

#[test]
fn find_expected_checksum_rejects_missing_or_invalid_entries() {
    let missing =
        find_expected_checksum("abcd  leaf-linux-x86_64\n", "leaf-macos-arm64").unwrap_err();
    assert!(missing.to_string().contains("does not contain"));

    let invalid =
        find_expected_checksum("xyz  leaf-linux-x86_64\n", "leaf-linux-x86_64").unwrap_err();
    assert!(invalid
        .to_string()
        .contains("Invalid SHA256 checksum format"));
}

#[test]
fn validate_sha256_hex_accepts_expected_format() {
    assert!(validate_sha256_hex(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    )
    .is_ok());
}

#[test]
fn extract_tag_from_release_url_reads_last_segment() {
    let url = "https://github.com/RivoLink/leaf/releases/tag/v1.26.0";
    assert_eq!(extract_tag_from_release_url(url).unwrap(), "v1.26.0");
}

#[test]
fn extract_tag_from_release_url_rejects_missing_segment() {
    let err = extract_tag_from_release_url("https://github.com/RivoLink/leaf").unwrap_err();
    assert!(err.to_string().contains("missing /releases/tag/"));
}

#[test]
fn build_download_url_composes_release_asset_path() {
    let url = build_download_url("RivoLink/leaf", "v1.26.0", "leaf-linux-x86_64");
    assert_eq!(
        url,
        "https://github.com/RivoLink/leaf/releases/download/v1.26.0/leaf-linux-x86_64"
    );
}
