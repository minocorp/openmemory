use clap::Parser;
use clap::error::ErrorKind;

use super::*;

fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("memory").chain(arguments.iter().copied()))
}

#[test]
fn every_documented_command_parses() {
    for arguments in [
        &["auth", "login"][..],
        &["auth", "login", "--device", "--host", "https://m.example"],
        &["auth", "status"],
        &["auth", "logout"],
        &["page", "list"],
        &["page", "list", "clients"],
        &["page", "view", "clients/acme"],
        &["page", "create", "clients/acme", "-"],
        &["page", "create", "clients/acme", "acme.md"],
        &["page", "edit", "clients/acme"],
        &[
            "page",
            "append",
            "clients/acme",
            "called",
            "--source",
            "phone",
            "--date",
            "2026-11-24",
        ],
        &["page", "move", "a", "b"],
        &["page", "delete", "a"],
        &["file", "list", "scans"],
        &["file", "upload", "./a.pdf", "scans/a.pdf"],
        &["file", "download", "scans/a.pdf", "-o", "-"],
        &["file", "view", "scans/a.pdf", "--page", "2"],
        &[
            "search",
            "fee",
            "--where",
            "status=active",
            "--where",
            "renewal>today",
            "--folder",
            "clients",
        ],
        &["history"],
        &["history", "clients/acme"],
        &["revert", "clients/acme", "3"],
        &["changes", "--seen"],
        &[
            "api",
            "/api/tree",
            "-X",
            "GET",
            "-f",
            "a=b",
            "-F",
            "c=@d",
            "-H",
            "X: y",
            "-i",
            "--paginate",
        ],
        &["api", "pages", "--input", "-"],
        &["completion", "zsh"],
        &["--json", "--no-input", "page", "view", "a"],
    ] {
        assert!(parse(arguments).is_ok(), "{arguments:?}");
    }
}

#[test]
fn global_flags_are_read_anywhere() {
    let cli = parse(&["page", "view", "a", "--json", "--host", "https://m.example"]).unwrap();
    assert!(cli.json);
    assert_eq!(cli.host.as_deref(), Some("https://m.example"));
    assert!(!cli.no_input);
}

#[test]
fn usage_mistakes_are_rejected() {
    for arguments in [
        &[][..],
        &["page"],
        &["page", "view"],
        &["revert", "a", "three"],
        &["file", "upload", "only-one"],
        &["completion", "tcsh"],
        &["login"],
    ] {
        let error = parse(arguments).unwrap_err();
        assert_ne!(error.kind(), ErrorKind::DisplayHelp, "{arguments:?}");
        assert_eq!(error.exit_code(), 2, "{arguments:?}");
    }
}

#[test]
fn search_collects_repeated_conditions() {
    let cli = parse(&["search", "fee", "--where", "a=1", "--where", "b=2"]).unwrap();
    let Command::Search(args) = cli.command else {
        unreachable!("parsed a search command");
    };
    assert_eq!(args.conditions, ["a=1", "b=2"]);
}
