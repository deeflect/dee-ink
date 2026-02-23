use std::borrow::Cow;

use reqwest::Url;
use serde_json::Value;

use crate::{
    cli::{GetArgs, SearchArgs},
    models::{
        AppError, ItemResponse, OutputMode, SearchItem, SearchResponse, SummaryApi, WikiItem,
    },
};

pub fn search(args: &SearchArgs, mode: &OutputMode) -> Result<(), AppError> {
    validate_lang(&args.lang)?;

    if mode.verbose {
        eprintln!(
            "debug: searching query='{}' lang='{}' limit={}",
            args.query, args.lang, args.limit
        );
    }

    let mut url = Url::parse(&format!("https://{}.wikipedia.org/w/api.php", args.lang))
        .map_err(|_| AppError::Request)?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("action", "opensearch")
            .append_pair("search", args.query.as_str())
            .append_pair("limit", &args.limit.to_string())
            .append_pair("format", "json");
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("dee-wiki/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::Request)?;
    let value: Value = client
        .get(url)
        .send()
        .map_err(|_| AppError::Request)?
        .error_for_status()
        .map_err(|_| AppError::Request)?
        .json()
        .map_err(|_| AppError::Parse)?;

    let titles = as_array_ref(&value, 1)?;
    let descriptions = as_array_ref(&value, 2)?;
    let urls = as_array_ref(&value, 3)?;

    let mut items = Vec::with_capacity(titles.len());
    for (idx, title_val) in titles.iter().enumerate() {
        let title = to_string_or_empty(title_val);
        let description = descriptions
            .get(idx)
            .map(to_string_or_empty)
            .unwrap_or_default();
        let url = urls.get(idx).map(to_string_or_empty).unwrap_or_default();

        items.push(SearchItem {
            title,
            description,
            url,
            lang: args.lang.clone(),
        });
    }

    let response = SearchResponse {
        ok: true,
        count: items.len(),
        items,
    };

    if mode.json {
        print_json(&response).map_err(|_| AppError::Parse)?;
    } else {
        print_search_human(&response, mode.quiet);
    }

    Ok(())
}

pub fn get(args: &GetArgs, mode: &OutputMode) -> Result<(), AppError> {
    fetch_summary(args, mode, false)
}

pub fn summary(args: &GetArgs, mode: &OutputMode) -> Result<(), AppError> {
    fetch_summary(args, mode, true)
}

fn fetch_summary(args: &GetArgs, mode: &OutputMode, concise: bool) -> Result<(), AppError> {
    validate_lang(&args.lang)?;

    if mode.verbose {
        eprintln!(
            "debug: fetching title='{}' lang='{}'",
            args.title, args.lang
        );
    }

    let mut url = Url::parse(&format!("https://{}.wikipedia.org/api/rest_v1", args.lang))
        .map_err(|_| AppError::Request)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| AppError::Request)?;
        segments.extend(["page", "summary", args.title.as_str()]);
    }

    if mode.verbose {
        eprintln!("debug: request_url={url}");
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("dee-wiki/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::Request)?;

    let response = client.get(url).send().map_err(|_| AppError::Request)?;
    let status = response.status();
    if status.as_u16() == 404 {
        return Err(AppError::NotFound);
    }
    if !status.is_success() {
        return Err(AppError::Request);
    }

    let response: SummaryApi = response.json().map_err(|_| AppError::Parse)?;

    let title = response.title.unwrap_or_default();
    let mut extract = response.extract.unwrap_or_default();
    if concise {
        extract = first_sentence(&extract).into_owned();
    }

    let page_url = response
        .content_urls
        .and_then(|x| x.desktop)
        .and_then(|x| x.page)
        .unwrap_or_default();

    let thumbnail = response
        .thumbnail
        .and_then(|x| x.source)
        .unwrap_or_default();

    if title.is_empty() {
        return Err(AppError::NotFound);
    }

    let item = WikiItem {
        title,
        extract,
        url: page_url,
        thumbnail,
        lang: args.lang.clone(),
    };

    let out = ItemResponse { ok: true, item };

    if mode.json {
        print_json(&out).map_err(|_| AppError::Parse)?;
    } else {
        print_item_human(&out, mode.quiet);
    }

    Ok(())
}

fn validate_lang(lang: &str) -> Result<(), AppError> {
    let valid = !lang.is_empty() && lang.chars().all(|ch| ch.is_ascii_alphabetic() || ch == '-');
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidLanguage)
    }
}

fn as_array_ref(value: &Value, index: usize) -> Result<&Vec<Value>, AppError> {
    value
        .as_array()
        .and_then(|arr| arr.get(index))
        .and_then(Value::as_array)
        .ok_or(AppError::Parse)
}

fn to_string_or_empty(value: &Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_default()
}

fn first_sentence(input: &str) -> Cow<'_, str> {
    // Common abbreviations that end with a dot but do not end a sentence
    const ABBREVS: &[&str] = &[
        "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "St.", "Jr.", "Sr.", "U.S.", "U.K.", "e.g.", "i.e.",
        "etc.", "vs.", "approx.", "Jan.", "Feb.", "Mar.", "Apr.", "Jun.", "Jul.", "Aug.", "Sep.",
        "Oct.", "Nov.", "Dec.",
    ];

    let mut search_from = 0;
    loop {
        // Find next ". " sequence
        let Some(rel) = input[search_from..].find(". ") else {
            return Cow::Borrowed(input);
        };
        let pos = search_from + rel;

        // Check if any known abbreviation ends exactly at `pos + 1`
        let candidate = &input[..pos + 1]; // includes the dot
        let is_abbrev = ABBREVS.iter().any(|abbrev| candidate.ends_with(abbrev));

        if !is_abbrev {
            return Cow::Owned(format!("{}.", &input[..pos]));
        }

        // Skip past this ". " and keep looking
        search_from = pos + 2;
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn print_search_human(response: &SearchResponse, quiet: bool) {
    if !quiet {
        println!("Found {} results", response.count);
    }

    for item in &response.items {
        println!("{}", item.title);
        if !item.description.is_empty() {
            println!("  {}", item.description);
        }
        if !item.url.is_empty() {
            println!("  {}", item.url);
        }
    }
}

fn print_item_human(response: &ItemResponse, quiet: bool) {
    let item = &response.item;

    println!("{}", item.title);
    if !item.extract.is_empty() {
        println!("{}", item.extract);
    }
    if !quiet {
        if !item.url.is_empty() {
            println!("{}", item.url);
        }
        if !item.thumbnail.is_empty() {
            println!("thumbnail: {}", item.thumbnail);
        }
        println!("lang: {}", item.lang);
    }
}
