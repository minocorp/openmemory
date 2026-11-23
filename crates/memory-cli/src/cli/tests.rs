use clap::Parser;
use memory_ops::{ReadInput, RevertInput, SearchInput, WriteInput};

use super::*;

fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("memory").chain(arguments.iter().copied()))
}

fn command(arguments: &[&str]) -> Command {
    parse(arguments).unwrap().command
}

#[test]
fn read_maps_path_version_and_page() {
    let Command::Read(args) = command(&["read", "contracts/acme.pdf", "--page", "2"]) else {
        panic!("not a read command");
    };
    assert_eq!(
        ReadInput::from(args),
        ReadInput {
            path: "contracts/acme.pdf".to_owned(),
            version: None,
            page: Some(2),
        }
    );
}

#[test]
fn search_collects_repeated_conditions() {
    let Command::Search(args) = command(&[
        "search",
        "단가",
        "--where",
        "renewal<+90d",
        "--where",
        "status=active",
        "--folder",
        "customers",
        "--limit",
        "5",
    ]) else {
        panic!("not a search command");
    };
    assert_eq!(
        SearchInput::from(args),
        SearchInput {
            text: Some("단가".to_owned()),
            conditions: vec!["renewal<+90d".to_owned(), "status=active".to_owned()],
            folder: Some("customers".to_owned()),
            limit: Some(5),
            cursor: None,
        }
    );
}

#[test]
fn search_needs_no_text() {
    assert!(parse(&["search", "--where", "status=active"]).is_ok());
}

#[test]
fn write_combines_replace_and_append_with_a_source() {
    let Command::Write(args) = command(&[
        "write",
        "customers/acme",
        "--replace",
        "500,000",
        "450,000",
        "--append",
        "agreed to lower the fee",
        "--source",
        "call",
        "--date",
        "2026-03-02",
        "--task",
        "renewal",
    ]) else {
        panic!("not a write command");
    };
    assert_eq!(
        args.into_input(None, None),
        WriteInput {
            path: "customers/acme".to_owned(),
            task: Some("renewal".to_owned()),
            content: None,
            base_version: None,
            replace_old: Some("500,000".to_owned()),
            replace_new: Some("450,000".to_owned()),
            append_text: Some("agreed to lower the fee".to_owned()),
            append_source: Some("call".to_owned()),
            append_date: Some("2026-03-02".to_owned()),
        }
    );
}

#[test]
fn write_rejects_incompatible_or_incomplete_options() {
    let cases: [&[&str]; 7] = [
        &["write", "a", "--create", "x", "--append", "y"],
        &[
            "write",
            "a",
            "--create",
            "x",
            "--overwrite",
            "y",
            "--base-version",
            "1",
        ],
        &["write", "a", "--overwrite", "x"],
        &["write", "a", "--base-version", "1"],
        &["write", "a", "--source", "call"],
        &["write", "a", "--date", "2026-03-02"],
        &["write", "a", "--replace", "only-one"],
    ];
    for arguments in cases {
        assert!(parse(arguments).is_err(), "{arguments:?}");
    }
}

#[test]
fn revert_takes_a_page_version_or_a_group() {
    let Command::Revert(args) = command(&["revert", "customers/acme", "--version", "3"]) else {
        panic!("not a revert command");
    };
    assert_eq!(
        RevertInput::from(args),
        RevertInput {
            path: Some("customers/acme".to_owned()),
            version: Some(3),
            group: None,
            task: None,
        }
    );
    let Command::Revert(args) = command(&["revert", "--group", "42"]) else {
        panic!("not a revert command");
    };
    assert_eq!(RevertInput::from(args).group, Some(42));
    for arguments in [
        &["revert", "--version", "3"][..],
        &["revert", "a", "--version", "3", "--group", "4"],
        &["revert", "a", "--group", "4"],
    ] {
        assert!(parse(arguments).is_err(), "{arguments:?}");
    }
}

#[test]
fn json_is_accepted_before_and_after_the_subcommand() {
    assert!(parse(&["--json", "read", "start"]).unwrap().json);
    assert!(parse(&["read", "start", "--json"]).unwrap().json);
    assert!(!parse(&["read", "start"]).unwrap().json);
}

#[test]
fn login_and_upload_take_their_positional_arguments() {
    let Command::Login(login) = command(&["login", "https://memory.example.com", "--device"])
    else {
        panic!("not a login command");
    };
    assert_eq!(login.server, "https://memory.example.com");
    assert!(login.device);
    let Command::Upload(upload) = command(&["upload", "./acme.pdf", "contracts/acme.pdf"]) else {
        panic!("not an upload command");
    };
    assert_eq!(upload.file.to_str(), Some("./acme.pdf"));
    assert_eq!(upload.path, "contracts/acme.pdf");
    assert!(parse(&["login"]).is_err());
}

#[test]
fn every_tool_has_a_command() {
    let cases: [&[&str]; 10] = [
        &["read", "a"],
        &["search", "a"],
        &["write", "a", "--create", "x"],
        &["move", "a", "b"],
        &["delete", "a"],
        &["history"],
        &["history", "a", "--by", "mino", "--since", "2026-11-01"],
        &["revert", "--group", "1"],
        &["upload", "f", "a"],
        &["status"],
    ];
    for arguments in cases {
        assert!(parse(arguments).is_ok(), "{arguments:?}");
    }
    assert!(parse(&["logout"]).is_ok());
}
