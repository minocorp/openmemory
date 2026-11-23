use std::io::IsTerminal;

use anyhow::Context;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Output {
    json: bool,
    color: bool,
    pretty: bool,
}

impl Output {
    pub(crate) fn detect(json_flag: bool) -> Self {
        let tty = std::io::stdout().is_terminal();
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        let dumb = std::env::var("TERM").is_ok_and(|term| term == "dumb");
        Self::resolve(json_flag, tty, no_color || dumb)
    }

    pub(crate) fn resolve(json_flag: bool, tty: bool, color_disabled: bool) -> Self {
        Self {
            json: json_flag || !tty,
            color: tty && !color_disabled,
            pretty: tty,
        }
    }

    pub(crate) fn is_json(self) -> bool {
        self.json
    }

    pub(crate) fn json(self, value: &impl Serialize) -> anyhow::Result<()> {
        let text = if self.pretty {
            serde_json::to_string_pretty(value)
        } else {
            serde_json::to_string(value)
        }
        .context("encode output")?;
        println!("{text}");
        Ok(())
    }

    pub(crate) fn bold(self, text: &str) -> String {
        self.paint("1", text)
    }

    pub(crate) fn dim(self, text: &str) -> String {
        self.paint("2", text)
    }

    pub(crate) fn green(self, text: &str) -> String {
        self.paint("32", text)
    }

    pub(crate) fn red(self, text: &str) -> String {
        self.paint("31", text)
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }
}

pub(crate) fn note(message: &str) {
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_chosen_by_flag_or_a_piped_stdout() {
        assert!(Output::resolve(true, true, false).is_json());
        assert!(Output::resolve(false, false, false).is_json());
        assert!(!Output::resolve(false, true, false).is_json());
    }

    #[test]
    fn color_needs_a_terminal_and_no_opt_out() {
        assert_eq!(
            Output::resolve(false, true, false).bold("x"),
            "\u{1b}[1mx\u{1b}[0m"
        );
        assert_eq!(Output::resolve(false, true, true).bold("x"), "x");
        assert_eq!(Output::resolve(false, false, false).bold("x"), "x");
    }
}
