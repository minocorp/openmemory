use openmemory_ops::PagePath;

use crate::access::Actor;
use crate::pages::error::PagesError;

pub(crate) const START_PATH: &str = "start";
const MAX_TASK_CHARACTERS: usize = 200;

pub(crate) fn can_read(actor: &Actor, path: &PagePath) -> bool {
    path.as_str() == START_PATH || actor.scope.can_read(path)
}

pub(crate) fn require_write(actor: &Actor, path: &PagePath) -> Result<(), PagesError> {
    if actor.scope.can_write(path) {
        return Ok(());
    }
    if actor.scope.can_read(path) {
        Err(PagesError::ReadOnly)
    } else {
        Err(PagesError::OutOfScope)
    }
}

pub(crate) fn require_write_visible(actor: &Actor, path: &str) -> Result<(), PagesError> {
    let path = PagePath::parse(path).map_err(PagesError::StoredPath)?;
    if !can_read(actor, &path) {
        return Err(PagesError::NotFound);
    }
    require_write(actor, &path)
}

pub(crate) fn require_version(
    row: &crate::pages::queries::pages::PageRow,
    expected: crate::pages::ids::VersionNumber,
) -> Result<(), PagesError> {
    if row.current_version == expected {
        return Ok(());
    }
    Err(PagesError::Conflict {
        current_version: row.current_version,
    })
}

pub(crate) fn require_not_start(path: &PagePath) -> Result<(), PagesError> {
    if path.as_str() == START_PATH {
        return Err(PagesError::ReservedPath);
    }
    Ok(())
}

pub(crate) fn task(task: Option<&str>) -> Result<Option<&str>, PagesError> {
    match task {
        Some(label) if !(1..=MAX_TASK_CHARACTERS).contains(&label.chars().count()) => {
            Err(PagesError::TaskLength)
        }
        Some(_) | None => Ok(task),
    }
}

#[cfg(test)]
mod tests {
    use crate::access::{Folders, Scope, UserId};

    use super::*;

    fn actor(folders: Folders, can_write: bool) -> Actor {
        Actor {
            user_id: UserId(1),
            connection_id: None,
            name: "mino".to_owned(),
            scope: Scope { folders, can_write },
        }
    }

    fn path(value: &str) -> PagePath {
        PagePath::parse(value).unwrap()
    }

    #[test]
    fn start_page_is_readable_outside_the_scope_but_not_writable() {
        let actor = actor(Folders::Only(vec![path("tax")]), true);
        assert!(can_read(&actor, &path("start")));
        assert!(!can_read(&actor, &path("clients/acme")));
        assert!(matches!(
            require_write(&actor, &path("start")),
            Err(PagesError::OutOfScope)
        ));
    }

    #[test]
    fn write_failures_say_whether_the_scope_or_the_toggle_is_the_cause() {
        let read_only = actor(Folders::All, false);
        assert!(matches!(
            require_write(&read_only, &path("a")),
            Err(PagesError::ReadOnly)
        ));
        let scoped = actor(Folders::Only(vec![path("tax")]), true);
        assert!(matches!(
            require_write(&scoped, &path("clients/a")),
            Err(PagesError::OutOfScope)
        ));
        assert!(require_write(&scoped, &path("tax/a")).is_ok());
    }

    #[test]
    fn task_labels_are_bounded() {
        assert!(task(Some("")).is_err());
        assert!(task(Some(&"a".repeat(201))).is_err());
        assert!(task(Some("tax prep")).is_ok());
        assert!(task(None).is_ok());
    }
}
