use std::io::Write;

use anyhow::Context as _;
use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::{Method, Response};
use serde_json::{Map, Value};
use tokio::io::AsyncReadExt;

use crate::cli::ApiArgs;
use crate::client::Client;
use crate::context::Context;
use crate::error::CliError;

struct Call {
    method: Method,
    route: String,
    fields: Vec<(String, String)>,
    input: Option<Vec<u8>>,
    headers: Vec<(HeaderName, HeaderValue)>,
}

pub(super) async fn run(args: ApiArgs, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let call = Call::build(&args).await?;
    if args.paginate {
        return paginate(&client, &call, context).await;
    }
    let response = send(&client, &call, None).await?;
    let failed = !response.status().is_success();
    let status = response.status();
    print_response(response, args.include, context).await?;
    if failed {
        anyhow::bail!("the server answered {status}");
    }
    Ok(())
}

impl Call {
    async fn build(args: &ApiArgs) -> anyhow::Result<Self> {
        let mut fields = Vec::new();
        for field in &args.fields {
            fields.push(split_field(field)?);
        }
        for field in &args.raw_fields {
            let (name, value) = split_field(field)?;
            let value = match value.strip_prefix('@') {
                Some(file) => tokio::fs::read_to_string(file)
                    .await
                    .with_context(|| format!("read {file}"))?,
                None => value,
            };
            fields.push((name, value));
        }
        let input = match args.input.as_deref() {
            Some("-") => {
                let mut bytes = Vec::new();
                tokio::io::stdin()
                    .read_to_end(&mut bytes)
                    .await
                    .context("read stdin")?;
                Some(bytes)
            }
            Some(file) => Some(
                tokio::fs::read(file)
                    .await
                    .with_context(|| format!("read {file}"))?,
            ),
            None => None,
        };
        let method = match &args.method {
            Some(method) => Method::from_bytes(method.to_ascii_uppercase().as_bytes())
                .map_err(|_| CliError::Usage(format!("`{method}` is not an HTTP method")))?,
            None if fields.is_empty() && input.is_none() => Method::GET,
            None => Method::POST,
        };
        let headers = args
            .headers
            .iter()
            .map(|header| parse_header(header))
            .collect::<Result<_, _>>()?;
        let route = if args.path.starts_with('/') {
            args.path.clone()
        } else {
            format!("/api/{}", args.path)
        };
        Ok(Self {
            method,
            route,
            fields,
            input,
            headers,
        })
    }
}

async fn send(client: &Client, call: &Call, cursor: Option<&str>) -> anyhow::Result<Response> {
    client
        .send(|| {
            let mut request = client.builder(call.method.clone(), &call.route);
            let mut query: Vec<(&str, &str)> = Vec::new();
            if call.method == Method::GET {
                query.extend(
                    call.fields
                        .iter()
                        .map(|(name, value)| (name.as_str(), value.as_str())),
                );
            }
            if let Some(cursor) = cursor {
                query.push(("cursor", cursor));
            }
            if !query.is_empty() {
                request = request.query(&query);
            }
            if let Some(input) = &call.input {
                request = request
                    .header(CONTENT_TYPE, "application/json")
                    .body(input.clone());
            } else if call.method != Method::GET && !call.fields.is_empty() {
                request = request.json(&object(&call.fields));
            }
            for (name, value) in &call.headers {
                request = request.header(name, value);
            }
            request
        })
        .await
}

async fn paginate(client: &Client, call: &Call, context: &Context) -> anyhow::Result<()> {
    if call.method != Method::GET {
        return Err(CliError::Usage("--paginate works with GET requests".to_owned()).into());
    }
    let mut pages = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let response = send(client, call, cursor.as_deref()).await?;
        let response = crate::client::check(response).await?;
        let page: Value = response.json().await.context("read a page of results")?;
        cursor = page.get("next").and_then(Value::as_str).map(str::to_owned);
        pages.push(page);
        if cursor.is_none() {
            break;
        }
    }
    context.output.json(&pages)
}

async fn print_response(
    response: Response,
    include: bool,
    context: &Context,
) -> anyhow::Result<()> {
    let mut out = std::io::stdout().lock();
    if include {
        writeln!(out, "{:?} {}", response.version(), response.status())?;
        for (name, value) in response.headers() {
            writeln!(out, "{name}: {}", String::from_utf8_lossy(value.as_bytes()))?;
        }
        writeln!(out)?;
    }
    let body = response.bytes().await.context("read the reply")?;
    let pretty = (!context.output.is_json())
        .then(|| serde_json::from_slice::<Value>(&body).ok())
        .flatten()
        .and_then(|value| serde_json::to_string_pretty(&value).ok());
    if let Some(text) = pretty {
        writeln!(out, "{text}")?;
    } else {
        out.write_all(&body)?;
        if !body.ends_with(b"\n") && !body.is_empty() {
            writeln!(out)?;
        }
    }
    Ok(())
}

fn split_field(field: &str) -> Result<(String, String), CliError> {
    field
        .split_once('=')
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .filter(|(name, _)| !name.is_empty())
        .ok_or_else(|| CliError::Usage(format!("`{field}` is not KEY=VALUE")))
}

fn parse_header(header: &str) -> Result<(HeaderName, HeaderValue), CliError> {
    let invalid = || CliError::Usage(format!("`{header}` is not NAME: VALUE"));
    let (name, value) = header.split_once(':').ok_or_else(invalid)?;
    let name = HeaderName::from_bytes(name.trim().as_bytes()).map_err(|_| invalid())?;
    let value = HeaderValue::from_str(value.trim()).map_err(|_| invalid())?;
    Ok((name, value))
}

fn object(fields: &[(String, String)]) -> Value {
    let mut map = Map::new();
    for (name, value) in fields {
        map.insert(name.clone(), Value::String(value.clone()));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_split_on_the_first_equals_sign() {
        assert_eq!(
            split_field("a=b=c").unwrap(),
            ("a".to_owned(), "b=c".to_owned())
        );
        assert!(split_field("novalue").is_err());
        assert!(split_field("=x").is_err());
    }

    #[test]
    fn headers_need_a_name_and_a_value() {
        let (name, value) = parse_header("X-Test:  yes ").unwrap();
        assert_eq!(name.as_str(), "x-test");
        assert_eq!(value, "yes");
        assert!(parse_header("nocolon").is_err());
    }

    #[test]
    fn fields_become_a_json_object_of_strings() {
        let body = object(&[
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "x".to_owned()),
        ]);
        assert_eq!(body, serde_json::json!({ "a": "1", "b": "x" }));
    }
}
