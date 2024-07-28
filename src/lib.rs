pub mod config;
pub mod mains;
use config::*;
pub mod checks;
pub mod errors;

use std::path::Path;

use anyhow::Context;
use sha2::Digest;

mod toolchain {
    use super::*;
    pub const ENVVAR: &str = "RUSTUP_TOOLCHAIN";
    pub struct Toolchain(pub String);
    impl Toolchain {
        pub fn from_str(data: &str) -> anyhow::Result<Self> {
            let data: toml::Table = toml::from_str(data)?;
            data.get("toolchain")
                .and_then(|d| d.get("channel"))
                .and_then(|d| d.as_str())
                .context("Failed to find channel in rust-toolchain.toml")
                .map(String::from)
                .map(Self)
        }
        /// Parse from a rust-toolchain.toml
        pub fn from_file(filename: &Path) -> anyhow::Result<Self> {
            let data = std::fs::read_to_string(filename)?;
            Self::from_str(&data)
        }
    }
    #[test]
    fn parse_toolchain() {
        assert_eq!(
            Toolchain::from_str(
                "
[toolchain]
channel = \"1.75.0\"
"
            )
            .unwrap()
            .0,
            "1.75.0"
        );
    }
}

pub fn sha256(mut r: impl std::io::Read) -> anyhow::Result<String> {
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut r, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}
