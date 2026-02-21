use crate::models::{ConvertItem, GetItem};
use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

const PRIMARY_BASE: &str = "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1";
const FALLBACK_BASE: &str = "https://latest.currency-api.pages.dev/v1";

fn base_urls() -> [String; 2] {
    if let Ok(url) = std::env::var("RATES_TEST_BASE_URL") {
        [url.clone(), url]
    } else {
        [PRIMARY_BASE.to_string(), FALLBACK_BASE.to_string()]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RatesError {
    #[error("Currency not found: {0}")]
    CurrencyNotFound(String),
    #[error("Target currency not found: {0}")]
    TargetCurrencyNotFound(String),
    #[error("Request failed")]
    RequestFailed,
    #[error("Invalid API response")]
    InvalidResponse,
    #[error("Amount must be finite")]
    InvalidAmount,
    #[error("Invalid currency code (must be 3 uppercase letters): {0}")]
    InvalidCurrencyCode(String),
}

impl RatesError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CurrencyNotFound(_) => "NOT_FOUND",
            Self::TargetCurrencyNotFound(_) => "NOT_FOUND",
            Self::RequestFailed => "REQUEST_FAILED",
            Self::InvalidResponse => "BAD_RESPONSE",
            Self::InvalidAmount => "INVALID_ARGUMENT",
            Self::InvalidCurrencyCode(_) => "INVALID_ARGUMENT",
        }
    }
}

#[derive(Debug, Deserialize)]
struct BaseRatesResponse {
    date: String,
    #[serde(flatten)]
    rates_by_base: HashMap<String, HashMap<String, f64>>,
}

pub fn validate_currency_code(code: &str) -> Result<(), RatesError> {
    let upper = code.trim().to_uppercase();
    if upper.len() == 3 && upper.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(RatesError::InvalidCurrencyCode(code.to_string()))
    }
}

pub fn get_rates(from: &str, to: Option<&str>, verbose: bool) -> Result<GetItem, RatesError> {
    validate_currency_code(from)?;
    if let Some(t) = to {
        validate_currency_code(t)?;
    }
    let from = normalize_currency(from);
    let from_api = from.to_lowercase();
    let payload: BaseRatesResponse =
        fetch_json_with_fallback(&format!("currencies/{from_api}.json"), verbose)
            .map_err(|_| RatesError::RequestFailed)?;

    let rates = payload
        .rates_by_base
        .get(&from_api)
        .cloned()
        .ok_or_else(|| RatesError::CurrencyNotFound(from.clone()))?;

    let date = normalize_date_iso8601(&payload.date).map_err(|_| RatesError::InvalidResponse)?;

    if let Some(target) = to {
        let target = normalize_currency(target);
        let target_api = target.to_lowercase();
        let rate = rates
            .get(&target_api)
            .copied()
            .ok_or_else(|| RatesError::TargetCurrencyNotFound(target.clone()))?;
        let mut filtered = HashMap::new();
        filtered.insert(target, rate);

        Ok(GetItem {
            base: from,
            date,
            rates: filtered,
        })
    } else {
        let upper_rates = rates
            .into_iter()
            .map(|(code, rate)| (code.to_uppercase(), rate))
            .collect();
        Ok(GetItem {
            base: from,
            date,
            rates: upper_rates,
        })
    }
}

pub fn convert(
    amount: f64,
    from: &str,
    to: &str,
    verbose: bool,
) -> Result<ConvertItem, RatesError> {
    if !amount.is_finite() {
        return Err(RatesError::InvalidAmount);
    }
    validate_currency_code(from)?;
    validate_currency_code(to)?;

    let from = normalize_currency(from);
    let to = normalize_currency(to);

    let payload = get_rates(&from, Some(&to), verbose)?;
    let rate = payload
        .rates
        .get(&to)
        .copied()
        .ok_or_else(|| RatesError::TargetCurrencyNotFound(to.clone()))?;

    Ok(ConvertItem {
        from,
        to,
        amount,
        result: amount * rate,
        rate,
        date: payload.date,
    })
}

pub fn list_currencies(verbose: bool) -> Result<Vec<String>, RatesError> {
    let payload: HashMap<String, String> = fetch_json_with_fallback("currencies.json", verbose)
        .map_err(|_| RatesError::RequestFailed)?;

    let mut items: Vec<String> = payload.keys().map(|k| k.to_uppercase()).collect();
    items.sort();
    items.dedup();
    Ok(items)
}

fn fetch_json_with_fallback<T: for<'de> Deserialize<'de>>(path: &str, verbose: bool) -> Result<T> {
    let client = Client::builder().build()?;
    let bases = base_urls();

    for base in &bases {
        let url = format!("{base}/{path}");
        if verbose {
            eprintln!("debug: fetching {url}");
        }
        match client.get(&url).send() {
            Ok(resp) if resp.status().is_success() => {
                let parsed = resp.json::<T>()?;
                return Ok(parsed);
            }
            Ok(resp) => {
                if verbose {
                    eprintln!("debug: non-success {} from {url}", resp.status());
                }
            }
            Err(err) => {
                if verbose {
                    eprintln!("debug: request error from {url}: {err}");
                }
            }
        }
    }

    Err(anyhow::anyhow!("all providers failed"))
}

fn normalize_currency(code: &str) -> String {
    code.trim().to_uppercase()
}

fn normalize_date_iso8601(input: &str) -> Result<String> {
    let parsed = NaiveDate::parse_from_str(input, "%Y-%m-%d")?;
    let dt = Utc
        .from_local_datetime(
            &parsed
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow::anyhow!("invalid date"))?,
        )
        .single()
        .ok_or_else(|| anyhow::anyhow!("invalid datetime"))?;
    Ok(dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
