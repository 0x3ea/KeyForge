use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use url::Url;
#[derive(Parser, Debug)]
#[command(name = "keyforge")]
#[command(version)]
#[command(about = "Deterministic password generator")]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a deterministic password for <site>
    Gen(GenArgs),
    /// Print shell completion script for the given shell.
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Show current configuration and the config file path
    Config,
}

#[derive(Args, Debug)]
pub struct GenArgs {
    pub site: String,

    #[arg(short = 'u', long = "username")]
    pub username: Option<String>,

    #[arg(short = 'l', long = "length")]
    pub length: Option<u32>,

    #[arg(short = 's', long = "symbols", num_args = 0..=1, require_equals = true)]
    pub symbols: Option<Option<String>>,

    #[arg(short = 'p', long = "print")]
    pub print: bool,

    #[arg(short = 'r', long = "remember")]
    pub remember: bool,

    #[arg(long = "timeout")]
    pub timeout: Option<u32>,
}

pub fn normalize_site(raw: &str) -> Result<String, String> {
    let raw = raw.trim();

    if raw.is_empty() {
        return Err("site cannot be empty".to_string());
    }

    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };

    let url = Url::parse(&with_scheme).map_err(|e| e.to_string())?;

    let host = url
        .host_str()
        .ok_or_else(|| "site must contain a hostname".to_string())?;

    Ok(host.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_site_hostname() {
        assert_eq!(normalize_site("Github.com").unwrap(), "github.com");
        assert_eq!(
            normalize_site("https://GitHub.com/path").unwrap(),
            "github.com"
        );
        assert_eq!(normalize_site(" github.com   ").unwrap(), "github.com");
    }
    #[test]
    fn rejects_empty_site() {
        assert!(normalize_site(" ").is_err());
        assert!(normalize_site("    ").is_err());
    }

    #[test]
    fn parses_gen_with_flags() {
        let cli = Cli::try_parse_from([
            "keyforge",
            "gen",
            "github.com",
            "-u",
            "alice",
            "-l",
            "20",
            "-s",
        ])
        .unwrap();

        match cli.command {
            Commands::Gen(a) => {
                assert_eq!(a.site, "github.com");
                assert_eq!(a.username.as_deref(), Some("alice"));
                assert_eq!(a.length, Some(20));
                assert_eq!(a.symbols, Some(None));
            }
            _ => panic!("expected Gen"),
        }
    }

    #[test]
    fn symbols_defaults_to_none() {
        let cli = Cli::try_parse_from(["keyforge", "gen", "github.com"]).unwrap();

        match cli.command {
            Commands::Gen(a) => assert_eq!(a.symbols, None),
            _ => panic!("expected Gen"),
        }
    }

    #[test]
    fn parses_custom_symbols_with_equals() {
        let cli = Cli::try_parse_from(["keyforge", "gen", "github.com", "--symbols=./{}"]).unwrap();

        match cli.command {
            Commands::Gen(a) => {
                assert_eq!(a.symbols, Some(Some("./{}".to_string())));
            }
            _ => panic!("expected Gen"),
        }
    }

    #[test]
    fn short_symbols_flag_does_not_consume_print() {
        let cli = Cli::try_parse_from(["keyforge", "gen", "github.com", "-s", "-p"]).unwrap();

        match cli.command {
            Commands::Gen(a) => {
                assert_eq!(a.symbols, Some(None));
                assert!(a.print);
            }
            _ => panic!("expected Gen"),
        }
    }

    #[test]
    fn gen_without_site_errors() {
        assert!(Cli::try_parse_from(["keyforge", "gen"]).is_err());
    }

    #[test]
    fn parses_config() {
        let cli = Cli::try_parse_from(["keyforge", "config"]).unwrap();
        assert!(matches!(cli.command, Commands::Config));
    }

    #[test]
    fn parses_completion() {
        let cli = Cli::try_parse_from(["keyforge", "completion", "bash"]).unwrap();
        assert!(matches!(cli.command, Commands::Completion { .. }));
    }

    #[test]
    fn no_args_errors() {
        assert!(Cli::try_parse_from(["keyforge"]).is_err());
    }

    #[test]
    fn old_usage_rejected() {
        assert!(Cli::try_parse_from(["keyforge", "github.com"]).is_err());
    }
}
