use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub ok: bool,
    pub count: usize,
    pub items: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GetItem {
    pub base: String,
    pub date: String,
    pub rates: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct SingleResponse<T> {
    pub ok: bool,
    pub item: T,
}

#[derive(Debug, Serialize)]
pub struct ConvertItem {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub result: f64,
    pub rate: f64,
    pub date: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub error: String,
    pub code: String,
}
