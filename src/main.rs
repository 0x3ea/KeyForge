use clap_complete::Shell;
use keyforge::{
    cli::{self, Commands, GenArgs},
    clipboard, completions,
    config::{self, config_path, get_config, render_config_summary, SiteConfig},
    crypto::{build_salt, generate_key},
    encode::{encode, BUILTIN_SYMBOLS, DIGIT, LOWER, UPPER},
    term,
};

use clap::{CommandFactory, Parser};

struct ResolvedOptions {
    site: String,
    username: String,
    length: u32,
    symbols: String,
    timeout: u32,
    print: bool,
    remember: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = cli::Cli::parse();

    match args.command {
        Commands::Gen(a) => run_gen(a),
        Commands::Completion { shell } => run_completion(shell),
        Commands::Config => run_config(),
    }
}

fn run_gen(args: GenArgs) -> Result<(), String> {
    let mut cfg = config::get_config()?;

    let site = cli::normalize_site(&args.site)?;
    let site_cfg = cfg.sites.get(&site);

    let username = args
        .username
        .or_else(|| site_cfg.and_then(|s| s.user_name.clone()))
        .unwrap_or_else(|| cfg.default_user_name.clone())
        .to_lowercase();

    let length = args
        .length
        .or_else(|| site_cfg.and_then(|s| s.length))
        .unwrap_or(cfg.default_length);

    let symbols = match args.symbols {
        Some(Some(s)) => s,
        Some(None) => BUILTIN_SYMBOLS.to_string(),
        None => site_cfg
            .and_then(|s| s.symbols.clone())
            .unwrap_or_else(|| cfg.default_symbols.clone()),
    };

    let options = ResolvedOptions {
        site,
        username,
        length,
        symbols,
        timeout: args.timeout.unwrap_or(cfg.default_timeout),
        print: args.print || cfg.default_print,
        remember: args.remember || cfg.default_remember,
    };
    valid_options(&options)?;

    let password = term::get_master_password(site_cfg.is_none())?;
    let salt = build_salt(&options.site, &options.username);
    let key = generate_key(&password, &salt)?;
    let mut classes = vec![LOWER, UPPER, DIGIT];
    if !options.symbols.is_empty() {
        classes.push(options.symbols.as_bytes());
    }
    let generated = encode(&key, options.length, &classes)?;

    if options.remember {
        let should_save = match cfg.sites.get(&options.site) {
            //when username in json doesn't match input
            Some(existing) if existing.user_name.as_deref() != Some(options.username.as_str()) => {
                let prompt = format!(
                    "{} is remembered as {}; overwrite with {}? [y/N] ",
                    options.site,
                    existing.user_name.as_deref().unwrap_or("(none)"),
                    options.username,
                );
                term::confirm(&prompt)?
            }
            // new site or change length/symbols
            _ => true,
        };

        if should_save {
            cfg.sites.insert(
                options.site.clone(),
                SiteConfig {
                    user_name: Some(options.username.clone()),
                    length: Some(options.length),
                    symbols: Some(options.symbols),
                },
            );
            config::set_config(&cfg)?;
        }
    }

    if options.print {
        let password_text = std::str::from_utf8(generated.as_bytes())
            .map_err(|e| format!("generated password is not valid UTF-8: {e}"))?;

        println!("{password_text}");
    } else {
        clipboard::write_to_clipboard(&generated, options.timeout)?;
    }

    Ok(())
}

fn run_completion(shell: Shell) -> Result<(), String> {
    let mut cmd = cli::Cli::command();
    let output = completions::generate_completion(shell, &mut cmd)?;
    print!("{output}");
    Ok(())
}

fn run_config() -> Result<(), String> {
    let cfg = get_config()?;
    let path = config_path()?;
    print!("{}", render_config_summary(&cfg, &path));
    Ok(())
}

fn valid_options(options: &ResolvedOptions) -> Result<(), String> {
    if options.length < 12 {
        return Err("password length must be at least 12".to_string());
    }

    if options.length > 128 {
        return Err("password length must be at most 128".to_string());
    }

    if options.timeout > 3600 {
        return Err("clipboard timeout must be at most 3600".to_string());
    }

    Ok(())
}
