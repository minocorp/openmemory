use openmemory_api::SearchResults;
use reqwest::Method;

use crate::cli::SearchArgs;
use crate::context::Context;
use crate::output::note;
use crate::render;

pub(super) async fn run(args: &SearchArgs, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let mut query = vec![("q", args.text.as_str())];
    query.extend(
        args.conditions
            .iter()
            .map(|condition| ("where", condition.as_str())),
    );
    if let Some(folder) = &args.folder {
        query.push(("folder", folder.as_str()));
    }
    let results: SearchResults = client
        .json(|| client.builder(Method::GET, "/api/search").query(&query))
        .await?;
    if context.output.is_json() {
        return context.output.json(&results);
    }
    print!("{}", render::search(&results, context.output));
    if let Some(no_hits) = &results.no_hits {
        note(&format!("No matches for `{}`", no_hits.query));
    }
    if results.next.is_some() {
        note(
            "More results exist: page through them with `openmemory api search --paginate -f q=...`",
        );
    }
    Ok(())
}
