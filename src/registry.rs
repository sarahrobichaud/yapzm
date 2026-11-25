//! The version registry: game build metadata and the steamcmd package the installer trusts.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use serde::Deserialize;

/// How long the registry fetch may take.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// The registry's remote source.
const URL: &str = "https://raw.githubusercontent.com/sarahrobichaud/yapzm/main/registry.json";

/// The registry file bundled with the binary, the fallback when the remote fetch fails.
const BUNDLED: &str = include_str!("../registry.json");

/// A game build's depot coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub version: String,
    pub manifest: u64,
    pub depot: Option<u64>,
    pub buildid: Option<u64>,
}

/// The steamcmd distribution the installer should trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamcmdPackage {
    pub url: String,
    pub sha256: String,
}

/// The parsed registry: game builds and the vouched steamcmd package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    pub entries: Vec<Entry>,
    pub steamcmd: Option<SteamcmdPackage>,
}

/// Loads the registry from its remote source, falling back to the bundled copy.
pub fn load() -> Result<Registry> {
    let url = cache_bust(URL);
    match fetch(&url) {
        Ok(registry) => Ok(registry),
        Err(error) => {
            eprintln!(
                "warning: could not load the registry from {url} ({error:#}); \
                 using the bundled copy"
            );
            parse(BUNDLED).context("parsing the bundled registry")
        }
    }
}

/// Loads a registry file from disk.
pub fn load_file(path: &Path) -> Result<Registry> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Appends a cache-busting query parameter so CDNs cannot serve a stale copy.
fn cache_bust(url: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}t={}", unix_now() / 60)
}

/// The current time in seconds since the epoch, zero when the clock is unset.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// Fetches and parses the registry from a remote URL.
fn fetch(url: &str) -> Result<Registry> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .build()
        .new_agent();
    let mut response = agent
        .get(url)
        // The CDN caches per Accept-Encoding variant and ignores query-string
        // cache busting, so the gzip variant can serve stale content.
        .header("accept-encoding", "identity")
        .call()
        .with_context(|| format!("fetching {url}"))?;
    let text = response
        .body_mut()
        .read_to_string()
        .with_context(|| format!("reading the response from {url}"))?;
    parse(&text).with_context(|| format!("parsing the registry from {url}"))
}

/// Parses registry JSON, skipping build entries without a version.
fn parse(text: &str) -> Result<Registry> {
    let file: RegistryFile = serde_json::from_str(text).context("invalid registry JSON")?;
    Ok(Registry {
        entries: file
            .builds
            .into_iter()
            .filter(|build| !build.version.is_empty())
            .map(|build| Entry {
                version: build.version,
                manifest: build.manifest,
                depot: build.depot,
                buildid: build.buildid,
            })
            .collect(),
        steamcmd: file.steamcmd.map(|package| SteamcmdPackage {
            url: package.url,
            sha256: package.sha256,
        }),
    })
}

#[derive(Deserialize)]
struct RegistryFile {
    #[serde(default)]
    builds: Vec<FileEntry>,
    #[serde(default)]
    steamcmd: Option<FileSteamcmd>,
}

#[derive(Deserialize)]
struct FileEntry {
    version: String,
    manifest: u64,
    #[serde(default)]
    depot: Option<u64>,
    #[serde(default)]
    buildid: Option<u64>,
}

#[derive(Deserialize)]
struct FileSteamcmd {
    url: String,
    sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_registry_file() {
        let text = r#"{
            "builds": [
                { "version": "42.22.2", "manifest": 2378094061158458897 },
                { "version": "", "manifest": 1 },
                { "version": "42.21.0", "manifest": 7544133343391543313, "depot": 108604, "buildid": 25485521 }
            ],
            "steamcmd": {
                "url": "https://client-update.steamstatic.com/installer/steamcmd.zip",
                "sha256": "7669b170dee42db8ee2273775ed7dfb2d173bdba1b849f70d2c7b379290bce13"
            }
        }"#;

        let registry = parse(text).unwrap();

        assert_eq!(
            registry.entries,
            vec![
                Entry {
                    version: "42.22.2".to_owned(),
                    manifest: 2378094061158458897,
                    depot: None,
                    buildid: None,
                },
                Entry {
                    version: "42.21.0".to_owned(),
                    manifest: 7544133343391543313,
                    depot: Some(108604),
                    buildid: Some(25485521),
                },
            ],
            "entries without a version are skipped"
        );
        assert_eq!(
            registry.steamcmd,
            Some(SteamcmdPackage {
                url: "https://client-update.steamstatic.com/installer/steamcmd.zip".to_owned(),
                sha256: "7669b170dee42db8ee2273775ed7dfb2d173bdba1b849f70d2c7b379290bce13"
                    .to_owned(),
            })
        );
    }

    #[test]
    fn the_steamcmd_section_is_optional() {
        let registry =
            parse(r#"{ "builds": [ { "version": "42.22.2", "manifest": 1 } ] }"#).unwrap();

        assert_eq!(registry.steamcmd, None);
        assert_eq!(registry.entries.len(), 1);
    }

    #[test]
    fn rejects_malformed_registry_files() {
        assert!(parse("not json").is_err(), "non-JSON text must be refused");
        assert!(
            parse(r#"{ "builds": [{ "version": "1.0" }] }"#).is_err(),
            "a build without its manifest must be refused"
        );
    }

    #[test]
    fn the_bundled_copy_parses() {
        let registry = parse(BUNDLED).unwrap();

        assert!(!registry.entries.is_empty());
        assert!(registry.steamcmd.is_some());
    }

    #[test]
    fn cache_busts_the_url() {
        let busted = cache_bust("https://example.com/registry.json");
        assert!(busted.starts_with("https://example.com/registry.json?t="));

        let busted = cache_bust("https://example.com/registry.json?x=1");
        assert!(busted.contains("&t="), "an existing query is extended");
    }
}
