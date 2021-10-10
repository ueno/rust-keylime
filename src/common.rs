// SPDX-License-Identifier: Apache-2.0
// Copyright 2021 Keylime Authors

use crate::error::{Error, Result};
use ini::Ini;
use log::*;
use std::env;
use std::path::{Path, PathBuf};

/*
 * Constants and static variables
 */
pub const API_VERSION: &str = "v1.0";
pub const STUB_VTPM: bool = false;
pub const STUB_IMA: bool = true;
pub const TPM_DATA_PCR: usize = 16;
pub const IMA_PCR: usize = 10;
pub static DEFAULT_CONFIG: &str = "/etc/keylime.conf";
pub static RSA_PUBLICKEY_EXPORTABLE: &str = "rsa placeholder";
pub static TPM_TOOLS_PATH: &str = "/usr/local/bin/";
pub static IMA_ML: &str =
    "/sys/kernel/security/ima/ascii_runtime_measurements";
pub static KEY: &str = "secret";
pub static WORK_DIR: &str = "/tmp";
// Note: The revocation certificate name is generated inside the Python tenant and the
// certificate(s) can be generated by running the tenant with the --cert flag. For more
// information, check the README: https://github.com/keylime/keylime/#using-keylime-ca
pub static REV_CERT: &str = "RevocationNotifier-cert.crt";

// Secure mount of tpmfs (False is generally used for development environments)
#[cfg(not(feature = "testing"))]
pub static MOUNT_SECURE: bool = true;

#[cfg(feature = "testing")]
pub static MOUNT_SECURE: bool = false;

pub const AGENT_UUID_LEN: usize = 36;
pub const AUTH_TAG_LEN: usize = 96;
pub const KEY_LEN: usize = 32;
pub const AES_BLOCK_SIZE: usize = 16;

// symmetric keys as bytes
pub type KeyBytes = [u8; KEY_LEN];

// a vector holding keys
pub type KeySet = Vec<SymmKey>;

// a key of len KEY_LEN
#[derive(Debug, Clone, Copy)]
pub struct SymmKey {
    pub bytes: KeyBytes,
}

impl Default for SymmKey {
    fn default() -> Self {
        SymmKey {
            bytes: [0u8; KEY_LEN],
        }
    }
}

impl SymmKey {
    pub fn is_empty(&self) -> bool {
        self.bytes == [0u8; KEY_LEN]
    }

    pub fn from_vec(v: Vec<u8>) -> Self {
        let mut b = [0u8; KEY_LEN];
        b.copy_from_slice(&v[..]);
        SymmKey { bytes: b }
    }
}

/*
 * Return: Returns the configuration file provided in the environment variable
 * KEYLIME_CONFIG or defaults to /etc/keylime.conf
 *
 * Example call:
 * let config = config_file_get();
 */
pub(crate) fn config_file_get() -> String {
    match env::var("KEYLIME_CONFIG") {
        Ok(cfg) => {
            // The variable length must be larger than 0 to accept
            if !cfg.is_empty() {
                cfg
            } else {
                String::from(DEFAULT_CONFIG)
            }
        }
        _ => String::from(DEFAULT_CONFIG),
    }
}

/// Returns revocation ip from keylime.conf if env var not present
pub(crate) fn revocation_ip_get() -> Result<String> {
    config_get_env("general", "receive_revocation_ip", "REVOCATION_IP")
}

/// Returns revocation port from keylime.conf if env var not present
pub(crate) fn revocation_port_get() -> Result<String> {
    config_get_env("general", "receive_revocation_port", "REVOCATION_PORT")
}

/// Returns cloud agent IP from keylime.conf if env var not present
pub(crate) fn cloudagent_ip_get() -> Result<String> {
    config_get_env("cloud_agent", "cloudagent_ip", "CLOUDAGENT_IP")
}

/// Returns cloud agent port from keylime.conf if env var not present
pub(crate) fn cloudagent_port_get() -> Result<String> {
    config_get_env("cloud_agent", "cloudagent_port", "CLOUDAGENT_PORT")
}

/// Returns registrar IP from keylime.conf if env var not present
pub(crate) fn registrar_ip_get() -> Result<String> {
    config_get_env("cloud_agent", "registrar_ip", "REGISTRAR_IP")
}

/// Returns registrar port from keylime.conf if env var not present
pub(crate) fn registrar_port_get() -> Result<String> {
    config_get_env("cloud_agent", "registrar_port", "REGISTRAR_PORT")
}

/// Returns the contact ip for the agent if set
pub(crate) fn cloudagent_contact_ip_get() -> Option<String> {
    match config_get_env(
        "cloud_agent",
        "agent_contact_ip",
        "KEYLIME_AGENT_CONTACT_IP",
    ) {
        Ok(ip) => Some(ip),
        Err(_) => None, // Ignore errors because this option might not be set
    }
}

/// Returns the contact ip for the agent if set
pub(crate) fn cloudagent_contact_port_get() -> Result<Option<u32>> {
    match config_get_env(
        "cloud_agent",
        "agent_contact_port",
        "KEYLIME_AGENT_CONTACT_PORT",
    ) {
        Ok(port_str) => match port_str.parse::<u32>() {
            Ok(port) => Ok(Some(port)),
            _ => Err(Error::Configuration(format!(
                "Parse {} to a port number.",
                port_str
            ))),
        },
        _ => Ok(None), // Ignore errors because this option might not be set
    }
}

/*
 * Input: [section] and key
 * Return: Returns the matched key
 *
 * Example call:
 * let port = common::config_get("general","cloudagent_port");
 */
pub(crate) fn config_get(section: &str, key: &str) -> Result<String> {
    let conf_name = config_file_get();
    let conf = Ini::load_from_file(&conf_name)?;
    let section = match conf.section(Some(section.to_owned())) {
        Some(section) => section,
        None =>
        // TODO: Make Error::Configuration an alternative with data instead of string
        {
            return Err(Error::Configuration(format!(
                "Cannot find section called {} in file {}",
                section, conf_name
            )))
        }
    };
    let value = match section.get(key) {
        Some(value) => value,
        None =>
        // TODO: Make Error::Configuration an alternative with data instead of string
        {
            return Err(Error::Configuration(format!(
                "Cannot find key {} in fine {}",
                key, conf_name
            )))
        }
    };

    Ok(value.to_string())
}

/*
 * Input: [section] and key and environment variable
 * Return: Returns the matched key
 *
 * Example call:
 * let port = common::config_get_env("general","cloudagent_port", "CLOUDAGENT_PORT");
 */
pub(crate) fn config_get_env(
    section: &str,
    key: &str,
    env: &str,
) -> Result<String> {
    match env::var(env) {
        Ok(ip) => {
            // The variable length must be larger than 0 to accept
            if !ip.is_empty() {
                Ok(ip)
            } else {
                config_get(section, key)
            }
        }
        _ => config_get(section, key),
    }
}

/*
 * Input: path directory to be changed owner to root
 * Return: Result contains execution result
 *         - directory name for successful execution
 *         - -1 code for failure execution.
 *
 * If privilege requirement is met, change the owner of the path to root
 * This function is unsafely using libc. Result is returned indicating
 * execution result.
 */
pub(crate) fn chownroot(path: String) -> Result<String> {
    unsafe {
        // check privilege
        if libc::geteuid() != 0 {
            error!("Privilege level unable to change ownership to root for file: {}", path);
            return Err(Error::Permission);
        }

        // change directory owner to root
        if libc::chown(path.as_bytes().as_ptr() as *const i8, 0, 0) != 0 {
            error!("Failed to change file {} owner.", path);
            return Err(Error::Permission);
        }

        info!("Changed file {} owner to root.", path);
        Ok(path)
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "testing")] {
        pub(crate) fn ima_ml_path_get() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("test-data")
                .join("ima")
                .join("ascii_runtime_measurements")
        }
    } else {
        pub(crate) fn ima_ml_path_get() -> PathBuf {
            Path::new(IMA_ML).to_path_buf()
        }
    }
}

// Unit Testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_get_parameters_exist() {
        //let result = config_get("keylime.conf", "general", "cloudagent_port");
        //assert_eq!(result, "9002");
    }

    #[test]
    fn test_config_file_get() {
        let conf_orig = option_env!("KEYLIME_CONFIG").or(Some("")).unwrap(); //#[allow_ci]

        // Test with no environment variable
        env::set_var("KEYLIME_CONFIG", "");
        assert_eq!(config_file_get(), String::from("/etc/keylime.conf"));

        // Test with an environment variable
        env::set_var("KEYLIME_CONFIG", "/tmp/testing.conf");
        assert_eq!(config_file_get(), String::from("/tmp/testing.conf"));
        // Reset environment
        env::set_var("KEYLIME_CONFIG", conf_orig);
    }
}
