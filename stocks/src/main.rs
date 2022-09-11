use dotenv::dotenv;
use chrono::NaiveDateTime;

use serde::Deserialize;
use std::vec;
use std::{collections::HashMap, env};
use ureq::{Request, Response};

#[derive(thiserror::Error, Debug)]
enum AlphaVantageError {
    #[error("Failed fetching issues")]
    RequestFailed(#[from] ureq::Error),
    #[error("Failed converting response to string")]
    FailedResponseToString(#[from] std::io::Error),
    #[error("Failed parse response to string")]
    IssueDeserialisationError(#[from] serde_json::Error),
}

struct AlphaVantageRequest {
    function: String,
    symbol: String,
    interval: String,
}

impl AlphaVantageRequest {
    fn new(
        function: String,
        symbol: String,
        interval: String,
    ) -> AlphaVantageRequest {
        AlphaVantageRequest {
            function,
            symbol,
            interval,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TimeSeriesHelper {
    #[serde(flatten)]
    time_series: HashMap<String, HashMap<String, String>>,
}


#[derive(Deserialize, Debug)]
struct AlphaVantageResponse {
    #[serde(rename(deserialize = "Time Series (5min)"))]
    time_series_helper: TimeSeriesHelper,
}


struct AlphaVantageClient {
    api_key: String,
}

const BASE_URL: &str = "https://www.alphavantage.co/query";

impl AlphaVantageClient {

    fn new(key: String) -> AlphaVantageClient {
        AlphaVantageClient {
            api_key: key
        }
    }

    fn fetch(&self, av_req: AlphaVantageRequest) -> Result<AlphaVantageResponse, AlphaVantageError> {

        let req: Request = ureq::get(BASE_URL).set("Accept", "application/json")
        .query("function", &av_req.function)
        .query("symbol", &av_req.symbol)
        .query("interval", &av_req.interval)
        .query("apikey", &self.api_key);

        let res: Response = req.call()?;

        let res: AlphaVantageResponse = res.into_json()?;

        Ok(res)
    }
}

#[derive(Debug)]
struct SeriesEntry {
    date: NaiveDateTime,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
}

impl Default for SeriesEntry {
    fn default() -> SeriesEntry {
        SeriesEntry {
            date: NaiveDateTime::default(),
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0,
        }
    }
}

fn parse_response(res: AlphaVantageResponse) -> Vec<SeriesEntry> {
    let mut entities = vec![];
    for (key, value) in &res.time_series_helper.time_series {
        let mut entry: SeriesEntry = SeriesEntry::default();
        entry.date = NaiveDateTime::parse_from_str(key, "%Y-%m-%d %H:%M:%S").unwrap();
        for (key, data) in value {
            match key.as_str() {
                "1. open" => entry.open = data.parse().unwrap(),
                "2. high" => entry.high = data.parse().unwrap(),
                "3. low" => entry.low = data.parse().unwrap(),
                "4. close" => entry.close = data.parse().unwrap(),
                "5. volume" => entry.volume = data.parse().unwrap(),
                _ => {}
            }
        }
        entities.push(entry)
    }
    entities
}

fn build_request(function: &str, symbol: &str, interval: &str) -> AlphaVantageRequest {
    AlphaVantageRequest::new(
        function.to_string(),
        symbol.to_string(),
        interval.to_string(),
    )
}

fn main() {
    _ = dotenv();

    let key = env::var("API_KEY").unwrap_or_default();
    let client = AlphaVantageClient::new(key);

    let function = "TIME_SERIES_INTRADAY";
    let symbol = "TEAM";
    let interval = "5min";
    let req = build_request(function, symbol, interval);

    let res = client.fetch(req).unwrap();
    let entities = parse_response(res);

    println!("{:?}\n{}", entities, entities.len());
}
