use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::sensitive::SecretVec;

type HmacSha256 = Hmac<Sha256>;

pub const DOMAIN: &[u8] = b"keyforge-password-encode-v3";
pub const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
pub const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const DIGIT: &[u8] = b"0123456789";
pub const BUILTIN_SYMBOLS: &str = "!@#$%^&*-_=+";

const MAX_ROUNDS: u64 = 4096;
pub fn expand(
    seed: &[u8],
    length: u32,
    charset: &[u8],
    round: u64,
    block_index: u64,
) -> Result<[u8; 32], String> {
    let mut mac = HmacSha256::new_from_slice(seed).map_err(|e| e.to_string())?;

    mac.update(DOMAIN);
    mac.update(&length.to_be_bytes());
    mac.update(charset);
    mac.update(&round.to_be_bytes());
    mac.update(&block_index.to_be_bytes());

    Ok(mac.finalize().into_bytes().into())
}

pub fn encode(seed: &SecretVec, length: u32, classes: &[&[u8]]) -> Result<SecretVec, String> {
    if length < classes.len() as u32 {
        return Err(format!(
            "password length {length} is too short for {} required classes",
            classes.len()
        ));
    }

    let mut charset = Vec::new();
    for class in classes {
        charset.extend_from_slice(class);
    }

    let charset_len = charset.len();
    let max_accept = 256 - (256 % charset_len);

    for round in 0..MAX_ROUNDS {
        let mut password = Vec::with_capacity(length as usize);
        let mut block_index = 0u64;

        while password.len() < length as usize {
            let block = expand(seed.as_bytes(), length, &charset, round, block_index)?;
            block_index += 1;
            for byte in block {
                if byte < max_accept as u8 {
                    let index = (byte as usize) % charset_len;
                    password.push(charset[index]);
                }
                if password.len() == length as usize {
                    break;
                }
            }
        }

        let covered = classes
            .iter()
            .all(|class| password.iter().any(|c| class.contains(c)));

        if covered {
            return SecretVec::new(password);
        }
    }
    Err(format!(
        "could not satisfy all {} classes after {MAX_ROUNDS} rounds",
        classes.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_classes() -> Vec<&'static [u8]> {
        vec![LOWER, UPPER, DIGIT]
    }

    fn classes_with_symbols(symbols: &'static [u8]) -> Vec<&'static [u8]> {
        let mut classes = base_classes();
        classes.push(symbols);
        classes
    }

    fn assert_coverage(password: &[u8], classes: &[&[u8]]) {
        for class in classes {
            assert!(
                password.iter().any(|&c| class.contains(&c)),
                "password is missing at least one character from class {class:?}"
            );
        }
    }

    fn assert_charset_membership(password: &[u8], classes: &[&[u8]]) {
        let mut charset = Vec::new();
        for class in classes {
            charset.extend_from_slice(class);
        }
        assert!(password.iter().all(|c| charset.contains(c)));
    }

    fn test_seed(i: u8) -> SecretVec {
        SecretVec::new(vec![i; 64]).unwrap()
    }

    #[test]
    fn encode_is_deterministic() {
        let seed = test_seed(42);
        let classes = base_classes();

        let first = encode(&seed, 16, &classes).unwrap();
        let second = encode(&seed, 16, &classes).unwrap();

        assert_eq!(first.as_bytes(), second.as_bytes());
        assert_eq!(first.len(), 16);
    }

    #[test]
    fn encode_respects_length() {
        let seed = test_seed(42);
        let classes = base_classes();

        assert_eq!(encode(&seed, 8, &classes).unwrap().len(), 8);
        assert_eq!(encode(&seed, 36, &classes).unwrap().len(), 36);
    }

    #[test]
    fn shorter_password_is_not_prefix_of_longer_password() {
        let seed = test_seed(42);
        let classes = base_classes();

        let short = encode(&seed, 8, &classes).unwrap();
        let long = encode(&seed, 16, &classes).unwrap();

        assert_ne!(short.as_bytes(), &long.as_bytes()[..short.len()]);
    }

    #[test]
    fn symbols_changes_output() {
        let seed = test_seed(42);
        let without_symbols = base_classes();
        let with_symbols = classes_with_symbols(BUILTIN_SYMBOLS.as_bytes());

        let with_symbols = encode(&seed, 16, &with_symbols).unwrap();
        let without_symbols = encode(&seed, 16, &without_symbols).unwrap();

        assert_ne!(with_symbols.as_bytes(), without_symbols.as_bytes());
    }

    #[test]
    fn every_active_class_is_covered() {
        for symbols in [None, Some(BUILTIN_SYMBOLS.as_bytes())] {
            let classes = match symbols {
                Some(symbols) => classes_with_symbols(symbols),
                None => base_classes(),
            };

            for i in 0..32u8 {
                let seed = test_seed(i);
                let password = encode(&seed, 12, &classes).unwrap();
                assert_coverage(password.as_bytes(), &classes);
            }
        }
    }

    #[test]
    fn custom_symbols_stay_in_charset() {
        let classes = classes_with_symbols(&b"./{}"[..]);

        for i in 0..16u8 {
            let seed = test_seed(i);
            let password = encode(&seed, 16, &classes).unwrap();
            assert_coverage(password.as_bytes(), &classes);
            assert_charset_membership(password.as_bytes(), &classes);
        }
    }

    #[test]
    fn empty_symbol_set_never_contains_symbols() {
        let classes = base_classes();

        for i in 0..16u8 {
            let seed = test_seed(i);
            let password = encode(&seed, 16, &classes).unwrap();
            assert!(!password
                .as_bytes()
                .iter()
                .any(|c| BUILTIN_SYMBOLS.as_bytes().contains(c)));
        }
    }

    #[test]
    fn different_symbol_sets_change_output() {
        let seed = test_seed(42);
        let first = classes_with_symbols(BUILTIN_SYMBOLS.as_bytes());
        let second = classes_with_symbols(&b"./{}"[..]);

        let first = encode(&seed, 16, &first).unwrap();
        let second = encode(&seed, 16, &second).unwrap();

        assert_ne!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn length_short_than_classes_is_rejected() {
        let seed = test_seed(42);
        let classes = classes_with_symbols(BUILTIN_SYMBOLS.as_bytes());

        assert!(encode(&seed, 3, &classes).is_err());
    }

    /// Coverage rejection changes the marginal frequency of each class, so the
    /// old whole-charset chi-squared test no longer applies. Uniformity should
    /// still hold within each class: rejection sampling must not bias one
    /// character over another in the same class.
    #[test]
    fn class_internal_distribution_is_uniform_chi_squared() {
        let classes = base_classes();
        let length = 64u32;
        let seeds = 128u32;

        for &class in &classes {
            let mut counts = vec![0u64; class.len()];

            for i in 0..seeds {
                let seed = test_seed(i as u8);
                let password = encode(&seed, length, &classes).unwrap();

                for &byte in password.as_bytes() {
                    if let Some(index) = class.iter().position(|&c| c == byte) {
                        counts[index] += 1;
                    }
                }
            }

            let total: u64 = counts.iter().sum();
            let expected = total as f64 / class.len() as f64;
            let chi2: f64 = counts
                .iter()
                .map(|&observed| {
                    let diff = observed as f64 - expected;
                    diff * diff / expected
                })
                .sum();
            let threshold = 2.0 * (class.len() as f64 - 1.0);

            assert!(
                chi2 < threshold,
                "chi2 = {chi2:.2} exceeds threshold {threshold:.0}; class {class:?} is not uniform"
            );
        }
    }
}
