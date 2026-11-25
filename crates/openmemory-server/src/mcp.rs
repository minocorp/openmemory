macro_rules! start_rule {
    () => {
        "Rules: read the page `start` first.\nField names are English; dates are ISO (YYYY-MM-DD)."
    };
}

mod actor;
mod endpoint;
mod error;
mod instructions;
mod operations;
mod reply;
mod server;
mod tools;

pub(crate) use endpoint::router;
