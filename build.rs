//! This build script downloads and links a precompiled libvpx
//! static library from https://github.com/caelunshun/libvpx-binaries.

use std::{env, fs, io::Read};

use serde::Deserialize;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        const OS_SPECIFIER: &str = "macos";
    } else if #[cfg(unix)] {
        const OS_SPECIFIER: &str = "linux";
    } else if #[cfg(windows)] {
        const OS_SPECIFIER: &str = "windows";
    }
}

/// See https://docs.github.com/en/rest/reference/repos#get-the-latest-release
///
/// Most fields are omitted.
#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    id: i32,
    name: String,
}

fn get_latest_release() -> Result<Release, Box<dyn std::error::Error>> {
    Ok(
        ureq::get("https://api.github.com/repos/caelunshun/libvpx-binaries/releases/latest")
            .set("Accept", "application/vnd.github.v3+json")
            .call()?
            .into_json()?,
    )
}

fn download_asset(asset: &Asset) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    ureq::get(&format!(
        "https://api.github.com/repos/caelunshun/libvpx-binaries/releases/assets/{}",
        asset.id
    ))
    .set("Accept", "application/octet-stream")
    .call()?
    .into_reader()
    .read_to_end(&mut data)?;
    Ok(data)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let release = get_latest_release()?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == format!("libvpx-{}.a", OS_SPECIFIER))
        .unwrap_or_else(|| {
            panic!(
                "no precompiled binary available for {} - {:#?}",
                OS_SPECIFIER, release.assets
            )
        });
    let data = download_asset(asset)?;
    let path = format!("{}/libvpx.a", env::var("OUT_DIR")?);
    fs::write(&path, &data)?;

    println!("cargo:rustc-link-lib=static=vpx");
    println!("cargo:rustc-link-search={}", env::var("OUT_DIR")?);
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}
