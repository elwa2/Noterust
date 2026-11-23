//! Vendored `webview2-com-sys` build script.
//!
//! Differences from upstream, both needed for the GNU (MinGW) toolchain:
//! * the loader is always linked **statically**, so the shipped exe needs no
//!   `WebView2Loader.dll` next to it;
//! * `WebView2LoaderStatic.lib` is also exposed as `libWebView2LoaderStatic.a`,
//!   because rustc resolves `#[link(kind = "static")]` on windows-gnu by that
//!   file name.
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=x64/WebView2LoaderStatic.lib");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH")?.as_str() {
        "x86_64" => "x64",
        other => unimplemented!("`{other}` is not supported by this vendored WebView2 build"),
    };

    let lib_dir = out_dir.join(arch);
    std::fs::create_dir_all(&lib_dir)?;

    let static_lib = manifest_dir.join(arch).join("WebView2LoaderStatic.lib");
    if !static_lib.is_file() {
        return Err(Error::Missing(static_lib));
    }
    std::fs::copy(&static_lib, lib_dir.join("WebView2LoaderStatic.lib"))?;
    std::fs::copy(&static_lib, lib_dir.join("libWebView2LoaderStatic.a"))?;

    if let Some(path) = lib_dir.to_str() {
        println!("cargo:rustc-link-search=native={path}");
    }
    println!("cargo:rustc-link-lib=advapi32");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Var(#[from] std::env::VarError),
    #[error("vendored WebView2 static loader library is missing: {0}")]
    Missing(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
