use memory_ops::PagePath;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::guard::START_PATH;
use crate::pages::read;
use crate::state::AppState;

const FALLBACK: &str = "This notebook has no `start` page yet. Interview the owner before other work: ask which subjects they track, which folders and field names they want, and which date format. Then call `write` with `create` on path `start`: terse facts and rules (field names, date format) and a map of folders, with no explanatory prose and nothing sensitive.";

pub(super) async fn start_page(state: &AppState, actor: &Actor) -> String {
    let Ok(path) = PagePath::parse(START_PATH) else {
        return FALLBACK.to_owned();
    };
    match read::read(state.pool(), actor, &path, None).await {
        Ok(page) => page.content,
        Err(PagesError::NotFound) => FALLBACK.to_owned(),
        Err(error) => {
            tracing::error!(%error, "read start page for server instructions");
            FALLBACK.to_owned()
        }
    }
}
