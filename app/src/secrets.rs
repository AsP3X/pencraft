//! Where API keys are kept: the operating system's keychain, never the settings file.
//!
//! macOS Keychain, Windows Credential Manager or the Linux Secret Service, through the `keyring` crate. The
//! settings file holds only a key's name in the keychain (`CustomProvider::key_ref`). Keys travel through the
//! app as [`SecretString`], which can't be printed or saved by accident.
//!
//! A key is read back only by the chat and by the maker of pictures ([`crate::imagery`]), when they call the
//! provider the key belongs to.

use craft_ai_core::SecretString;

/// The name the keys are filed under in the keychain (macOS shows it in Keychain Access).
const SERVICE: &str = "Pencraft";

/// A place that keeps secrets by name.
pub trait Vault {
    /// Keeps `key` under `name`, replacing what was there.
    ///
    /// # Errors
    /// A sentence for the user when the key couldn't be stored; it never contains the key.
    fn put(&self, name: &str, key: &SecretString) -> Result<(), String>;

    /// The key kept under `name`.
    ///
    /// # Errors
    /// A sentence for the user when there is none or the keychain couldn't be reached.
    fn get(&self, name: &str) -> Result<SecretString, String>;

    /// Removes what is kept under `name`; nothing kept there is fine.
    ///
    /// # Errors
    /// A sentence for the user when the keychain couldn't be reached.
    fn forget(&self, name: &str) -> Result<(), String>;
}

/// The operating system's keychain.
pub struct Keychain;

impl Vault for Keychain {
    fn put(&self, name: &str, key: &SecretString) -> Result<(), String> {
        // Human: the library's errors describe the keychain, not the key, so they are safe to show.
        keyring::Entry::new(SERVICE, name).and_then(|entry| entry.set_password(key.expose())).map_err(|e| e.to_string())
    }

    fn get(&self, name: &str) -> Result<SecretString, String> {
        keyring::Entry::new(SERVICE, name).and_then(|entry| entry.get_password()).map(SecretString::new).map_err(|e| e.to_string())
    }

    fn forget(&self, name: &str) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, name).and_then(|entry| entry.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// A new name for a key in the keychain.
// Human: the time in nanoseconds is different for every provider a person adds by hand.
#[must_use]
pub fn new_name() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    format!("provider-{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_plain_and_start_alike() {
        let name = new_name();
        assert!(name.starts_with("provider-"));
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    /// Writes, reads and removes a key in the real keychain. Run by hand: `cargo test -- --ignored keychain`.
    #[test]
    #[ignore = "touches the real keychain of the person running it"]
    fn the_real_keychain_keeps_and_forgets_a_key() {
        let name = format!("test-{}", new_name());
        Keychain.put(&name, &SecretString::new("not-a-real-key")).unwrap();
        assert_eq!(keyring::Entry::new(SERVICE, &name).unwrap().get_password().unwrap(), "not-a-real-key");
        Keychain.put(&name, &SecretString::new("replaced")).unwrap();
        assert_eq!(keyring::Entry::new(SERVICE, &name).unwrap().get_password().unwrap(), "replaced");
        Keychain.forget(&name).unwrap();
        assert!(matches!(keyring::Entry::new(SERVICE, &name).unwrap().get_password(), Err(keyring::Error::NoEntry)));
        // Forgetting what isn't there is fine.
        Keychain.forget(&name).unwrap();
    }
}
