use keyforge::{
    crypto::{build_salt, generate_key},
    encode::{encode, BUILTIN_SYMBOLS, DIGIT, LOWER, UPPER},
    sensitive::SecretVec,
};

/// Helper: run the full derivation pipeline and return the password as bytes.
fn derive_password(
    master: &[u8],
    site: &str,
    username: &str,
    length: u32,
    symbols: Option<&str>,
) -> Vec<u8> {
    let password = SecretVec::new(master.to_vec()).unwrap();
    let salt = build_salt(site, username);
    let key = generate_key(&password, &salt).unwrap();

    let mut classes = vec![LOWER, UPPER, DIGIT];
    if let Some(symbols) = symbols {
        if !symbols.is_empty() {
            classes.push(symbols.as_bytes());
        }
    }

    let generated = encode(&key, length, &classes).unwrap();
    generated.as_bytes().to_vec()
}

#[test]
fn same_input_produces_same_output() {
    let a = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        None,
    );
    let b = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        None,
    );
    assert_eq!(a, b);
}

#[test]
fn different_master_password_produces_different_output() {
    let a = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        None,
    );
    let b = derive_password(b"another-master-password!", "github.com", "alice", 16, None);
    assert_ne!(a, b);
}

#[test]
fn different_site_produces_different_output() {
    let a = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        None,
    );
    let b = derive_password(
        b"correct-horse-battery-staple",
        "gitlab.com",
        "alice",
        16,
        None,
    );
    assert_ne!(a, b);
}

#[test]
fn different_username_produces_different_output() {
    let a = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        None,
    );
    let b = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "bob",
        16,
        None,
    );
    assert_ne!(a, b);
}

#[test]
fn custom_symbols_are_used_and_every_class_is_covered() {
    let symbols = "./{}";
    let password = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        Some(symbols),
    );

    let classes = vec![LOWER, UPPER, DIGIT, symbols.as_bytes()];
    for class in classes {
        assert!(
            password.iter().any(|&c| class.contains(&c)),
            "password is missing at least one character from class {class:?}"
        );
    }

    let mut charset = Vec::new();
    for class in [LOWER, UPPER, DIGIT, symbols.as_bytes()] {
        charset.extend_from_slice(class);
    }
    assert!(password.iter().all(|c| charset.contains(c)));
}

#[test]
fn different_symbol_sets_produce_different_output() {
    let a = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        Some(BUILTIN_SYMBOLS),
    );
    let b = derive_password(
        b"correct-horse-battery-staple",
        "github.com",
        "alice",
        16,
        Some("./{}"),
    );

    assert_ne!(a, b);
}
