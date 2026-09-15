//! Opening external URLs in the user's browser.
//!
//! URLs reaching this module come from remote data — pull request listings, review
//! sources reconstructed from pasted references — so they are treated as untrusted.
//! Two rules keep that safe:
//!
//! 1. Only `http` and `https` URLs with a plausible authority are accepted, so a URL
//!    can never be mistaken for a flag by `open`/`xdg-open`, nor name a local file.
//! 2. No launcher goes through a command interpreter. A URL is normally a single argv
//!    element passed to a program that does not reparse it, which is why characters
//!    that are ordinary in a query string (`&`, `?`, `%`) need no escaping and keep
//!    working. The sole exception is the WSL PowerShell fallback, which takes a
//!    command string; there the URL is single-quoted, which makes every metacharacter
//!    literal, and rule 1 has already excluded the line breaks that could escape it.

use crate::infra::platform::{Platform, current_platform};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OpenUrlError {
    #[error("Only http and https URLs can be opened, got: {0}")]
    UnsupportedScheme(String),

    #[error("URL has no host: {0}")]
    MissingHost(String),

    #[error("URL contains characters that are not allowed: {0}")]
    IllegalCharacters(String),

    #[error("Unsupported platform for opening URLs")]
    UnsupportedPlatform,

    #[error("Failed to open URL: {0}")]
    LaunchFailed(String),
}

/// Validates `raw` as an external URL safe to hand to a platform launcher.
///
/// Returns the trimmed URL on success. Trimming is the only normalisation — nothing
/// is rewritten, so what the browser receives is what the caller supplied.
pub fn validate_external_url(raw: &str) -> Result<&str, OpenUrlError> {
    let url = raw.trim();

    let rest = strip_scheme(url).ok_or_else(|| OpenUrlError::UnsupportedScheme(url.to_string()))?;

    if url
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || c == '\u{7f}')
    {
        return Err(OpenUrlError::IllegalCharacters(url.to_string()));
    }

    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();

    if authority.is_empty() {
        return Err(OpenUrlError::MissingHost(url.to_string()));
    }

    // Userinfo makes the real host hard to see and no launcher here needs it.
    if !authority
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']' | '_'))
    {
        return Err(OpenUrlError::IllegalCharacters(url.to_string()));
    }

    Ok(url)
}

fn strip_scheme(url: &str) -> Option<&str> {
    for scheme in ["http://", "https://"] {
        if url.len() >= scheme.len() && url[..scheme.len()].eq_ignore_ascii_case(scheme) {
            return Some(&url[scheme.len()..]);
        }
    }
    None
}

/// Launchers to try, in order, for `platform`. Each is a program and its argv; none
/// is a shell, so `url` stays one argument and is never reparsed as command text.
fn launch_candidates(platform: Platform, url: &str) -> Vec<(String, Vec<String>)> {
    let direct = |program: &str| (program.to_string(), vec![url.to_string()]);

    match platform {
        Platform::MacOS => vec![direct("open")],
        Platform::Linux => vec![direct("xdg-open")],

        // wslview (from the wslu package) handles URLs properly; explorer.exe is
        // built in and reliable for URLs. Both take the URL as a bare argument. The
        // PowerShell fallback is the one launcher that does parse its argument, so
        // the URL is single-quoted with embedded quotes doubled and bound to an
        // explicit -FilePath rather than positionally.
        Platform::LinuxWsl => vec![
            direct("wslview"),
            direct("explorer.exe"),
            (
                "powershell.exe".to_string(),
                vec![
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    format!("Start-Process -FilePath '{}'", url.replace('\'', "''")),
                ],
            ),
        ],

        // FileProtocolHandler hands the URL to the registered handler directly.
        // `cmd /C start` would reparse it, letting `&` start a second command.
        Platform::Windows => vec![(
            "rundll32.exe".to_string(),
            vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
        )],

        Platform::Unknown => vec![],
    }
}

/// Opens `url` in the user's browser after validating it.
pub fn open_external_url(url: &str) -> Result<(), OpenUrlError> {
    let url = validate_external_url(url)?;
    let platform = current_platform();

    let candidates = launch_candidates(platform, url);
    if candidates.is_empty() {
        return Err(OpenUrlError::UnsupportedPlatform);
    }

    let mut last_error = None;
    for (program, args) in candidates {
        match std::process::Command::new(&program).args(&args).spawn() {
            Ok(_) => return Ok(()),
            Err(e) => last_error = Some(format!("{}: {}", program, e)),
        }
    }

    let detail = last_error.unwrap_or_else(|| "no launcher available".to_string());
    Err(OpenUrlError::LaunchFailed(match platform {
        Platform::LinuxWsl => format!(
            "{}. Install the wslu package (sudo apt install wslu) for better support, \
             or ensure Windows interop is enabled.",
            detail
        ),
        _ => detail,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_http_and_https() {
        assert_eq!(
            validate_external_url("https://github.com/o/r/pull/1"),
            Ok("https://github.com/o/r/pull/1")
        );
        assert_eq!(
            validate_external_url("http://example.test"),
            Ok("http://example.test")
        );
    }

    #[test]
    fn accepts_uppercase_scheme() {
        assert_eq!(
            validate_external_url("HTTPS://example.test/x"),
            Ok("HTTPS://example.test/x")
        );
    }

    #[test]
    fn accepts_ports_ipv6_and_percent_encoding() {
        assert!(validate_external_url("https://example.test:8443/x").is_ok());
        assert!(validate_external_url("http://[::1]:3000/x").is_ok());
        assert!(validate_external_url("https://example.test/a%20b").is_ok());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            validate_external_url("  https://example.test/x\n"),
            Ok("https://example.test/x")
        );
    }

    #[test]
    fn rejects_non_web_schemes() {
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<script>",
            "vbscript:x",
            "example.test/x",
            "//example.test/x",
        ] {
            assert!(
                matches!(
                    validate_external_url(url),
                    Err(OpenUrlError::UnsupportedScheme(_))
                ),
                "expected {url} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_missing_host() {
        assert!(matches!(
            validate_external_url("https:///etc/passwd"),
            Err(OpenUrlError::MissingHost(_))
        ));
    }

    #[test]
    fn rejects_control_characters_and_internal_whitespace() {
        for url in [
            "https://example.test/x\nStart-Process calc",
            "https://example.test/x\r\nHost: evil",
            "https://example.test/a b",
            "https://exa mple.test/",
            "https://example.test/x\0",
        ] {
            assert!(
                matches!(
                    validate_external_url(url),
                    Err(OpenUrlError::IllegalCharacters(_))
                ),
                "expected {url:?} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_userinfo_and_metacharacters_in_the_authority() {
        for url in [
            "https://user@evil.test/",
            "https://example.test&whoami/",
            "https://example.test|whoami/",
            "https://exa`mple.test/",
            "https://example.test;whoami/",
        ] {
            assert!(
                matches!(
                    validate_external_url(url),
                    Err(OpenUrlError::IllegalCharacters(_))
                ),
                "expected {url} to be rejected"
            );
        }
    }

    /// The reported injection: `cmd /C start` would treat `&whoami` as a second
    /// command. The URL is legitimate, so it must still open — the fix is that no
    /// launcher reparses it, not that the character is banned.
    #[test]
    fn query_string_ampersand_opens_without_becoming_a_second_command() {
        let url = "https://example.test/?x=1&whoami";
        assert_eq!(validate_external_url(url), Ok(url));

        for platform in [
            Platform::Windows,
            Platform::MacOS,
            Platform::Linux,
            Platform::LinuxWsl,
        ] {
            let candidates = launch_candidates(platform, url);
            assert!(!candidates.is_empty(), "{platform:?} has no launcher");

            for (program, args) in candidates {
                assert!(
                    program != "cmd" && program != "cmd.exe" && program != "sh",
                    "{program} reparses its arguments"
                );

                let whole_argument = args.iter().any(|a| a == url);
                let quoted_in_command = args
                    .iter()
                    .any(|a| a.contains(&format!("'{}'", url.replace('\'', "''"))));

                assert!(
                    whole_argument || quoted_in_command,
                    "{program} must receive the URL intact, got {args:?}"
                );
            }
        }
    }

    #[test]
    fn wsl_powershell_fallback_quotes_the_url() {
        let candidates = launch_candidates(Platform::LinuxWsl, "https://example.test/?x=1&whoami");
        let (program, args) = candidates.last().expect("fallback candidate");

        assert_eq!(program, "powershell.exe");
        assert_eq!(
            args,
            &vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Start-Process -FilePath 'https://example.test/?x=1&whoami'".to_string(),
            ]
        );
    }

    #[test]
    fn wsl_powershell_fallback_doubles_embedded_quotes() {
        let candidates =
            launch_candidates(Platform::LinuxWsl, "https://example.test/?q='+whoami+'");
        let command = candidates.last().unwrap().1.last().unwrap();

        assert_eq!(
            command,
            "Start-Process -FilePath 'https://example.test/?q=''+whoami+'''"
        );
    }

    #[test]
    fn windows_uses_the_protocol_handler_rather_than_a_shell() {
        let url = "https://example.test/?x=1&y=2";
        assert_eq!(
            launch_candidates(Platform::Windows, url),
            vec![(
                "rundll32.exe".to_string(),
                vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()]
            )]
        );
    }

    #[test]
    fn unknown_platform_has_no_launcher() {
        assert!(launch_candidates(Platform::Unknown, "https://example.test").is_empty());
    }
}
