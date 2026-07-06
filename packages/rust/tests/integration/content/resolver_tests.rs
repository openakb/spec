use std::fs;

use openakb_validate::{LocalFileResolver, Resolver};
use tempfile::TempDir;

fn fixture_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("root.md"), "hello\n").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("leaf.md"), "leaf\n").unwrap();
    dir
}

#[tokio::test]
async fn test_resolves_root_file() {
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    assert_eq!(resolver.fetch("root.md").await.unwrap(), b"hello\n");
}

#[tokio::test]
async fn test_strips_fragment() {
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    assert_eq!(resolver.fetch("root.md#anchor").await.unwrap(), b"hello\n");
}

#[tokio::test]
async fn test_resolves_sub_file() {
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    assert_eq!(resolver.fetch("sub/leaf.md").await.unwrap(), b"leaf\n");
}

#[tokio::test]
async fn test_rejects_escapes() {
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());
    let rejected = [
        "../root.md",
        "/etc/hosts",
        "sub/../../root.md",
        "root.md?x=1",
        "root.md;v=1",
        r"sub\leaf.md",
        "https://example.com/root.md",
        "//example.com/root.md",
    ];

    for uri in rejected {
        let error = resolver.fetch(uri).await.unwrap_err();
        assert!(
            error.reason.contains("outside local base"),
            "{uri}: {}",
            error.reason
        );
    }
}

#[tokio::test]
async fn test_missing_errors() {
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    assert!(resolver.fetch("missing.md").await.is_err());
}

#[tokio::test]
async fn test_directory_read_errors() {
    // `sub` resolves inside the base and canonicalizes, so screening admits it, but
    // reading a directory as bytes fails at the filesystem layer, not the guard.
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    let error = resolver.fetch("sub").await.unwrap_err();

    assert!(
        !error.reason.contains("outside local base"),
        "{}",
        error.reason
    );
}

#[tokio::test]
async fn test_colon_after_slash_not_scheme() {
    // A colon after a path segment (`a/b:c`) and a leading colon (`:x`) are not URI
    // schemes, so screening admits them; the read then fails on the missing file.
    let dir = fixture_dir();
    let resolver = LocalFileResolver::new(dir.path());

    for uri in ["sub/leaf:x.md", ":missing.md"] {
        let error = resolver.fetch(uri).await.unwrap_err();
        assert!(
            !error.reason.contains("outside local base"),
            "{uri}: {}",
            error.reason
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let dir = fixture_dir();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("outside.md"), "outside\n").unwrap();
    symlink(
        outside.path().join("outside.md"),
        dir.path().join("escape.md"),
    )
    .unwrap();
    let resolver = LocalFileResolver::new(dir.path());

    let error = resolver.fetch("escape.md").await.unwrap_err();

    assert!(error.reason.contains("outside local base"));
}
