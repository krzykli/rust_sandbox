use chrono::NaiveDateTime;
use dotenv::dotenv;

use serde::Deserialize;
use std::vec;
use std::{collections::HashMap, env};
use ureq::{Request, Response};

use nannou::prelude::*;

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
    fn new(function: String, symbol: String, interval: String) -> AlphaVantageRequest {
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
    #[serde(rename(deserialize = "Time Series (60min)"))]
    time_series_helper: TimeSeriesHelper,
}

struct AlphaVantageClient {
    api_key: String,
}

const BASE_URL: &str = "https://www.alphavantage.co/query";

impl AlphaVantageClient {
    fn new(key: String) -> AlphaVantageClient {
        AlphaVantageClient { api_key: key }
    }

    fn fetch(
        &self,
        av_req: AlphaVantageRequest,
    ) -> Result<AlphaVantageResponse, AlphaVantageError> {
        let req: Request = ureq::get(BASE_URL)
            .set("Accept", "application/json")
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
    open: f32,
    high: f32,
    low: f32,
    close: f32,
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
    entities.sort_by(|a, b| a.date.cmp(&b.date));

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
    nannou::app(model).update(update).simple_window(view).run();
}

struct Model {
    symbol: String,
    entries: Vec<SeriesEntry>,
}

fn model(_app: &App) -> Model {
    _ = dotenv();

    let key = env::var("API_KEY").unwrap_or_default();
    let client = AlphaVantageClient::new(key);

    let function = "TIME_SERIES_INTRADAY";
    let symbol = "TEAM";
    let interval = "60min";
    let req = build_request(function, symbol, interval);

    let res = client.fetch(req).unwrap();
    let entries = parse_response(res);

    Model {
        symbol: symbol.to_string(),
        entries,
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {}

fn view(app: &App, _model: &Model, frame: Frame) {
    let win_rect = app.window_rect();

    app.main_window().set_title("stocks");
    // get canvas to draw on
    let draw = app.draw();

    // set background to blue
    draw.background().color(DARKBLUE);

    draw.text(&_model.symbol.to_string())
        .x(win_rect.x() / 2.0)
        .y(300.0)
        .font_size(70);

    let bound_min = -200.0;
    let bound_max = 200.0;

    draw.line()
        .start(pt2(0.0, bound_min - 10.0))
        .end(pt2(0.0, bound_max + 10.0))
        .weight(2.0)
        .color(DARKMAGENTA);

    let mut current_price = 0.0;
    let mut date: NaiveDateTime = NaiveDateTime::default();
    let close_values: Vec<f32> = _model.entries.iter().map(|a| a.close).collect();
    let min_close = close_values.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max_close = close_values
        .iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let _average_close: f32 = close_values.iter().sum::<f32>() / close_values.len() as f32;

    let legend_x = -win_rect.w() / 2.0 + 50.0;

    draw.line()
        .start(pt2(legend_x + 10.0, bound_min))
        .end(pt2(legend_x - 10.0, bound_min))
        .weight(2.0)
        .color(DARKGRAY);
    draw.text(&min_close.to_string())
        .x(legend_x)
        .y(bound_min - 20.0)
        .font_size(10);

    draw.line()
        .start(pt2(legend_x + 10.0, bound_max))
        .end(pt2(legend_x - 10.0, bound_max))
        .weight(2.0)
        .color(DARKGRAY);
    draw.text(&max_close.to_string())
        .x(legend_x)
        .y(bound_max + 20.0)
        .font_size(10);

    let speed = 70.0;
    let spacing = 15.0;
    let offset = -app.time * speed;

    let points = _model.entries.iter().enumerate().map(|(i, entry)| {
        let x = i as f32 * spacing + offset;

        let mut color = STEELBLUE;
        if x < 0.0 {
            current_price = entry.close;
            date = entry.date;
            color = DARKMAGENTA;
        }

        let y = {
            ((entry.close - min_close) / (max_close - min_close)) * (bound_max - bound_min)
                + bound_min
        };

        (pt2(x, y), color)
    });

    draw.polyline().weight(3.0).points_colored(points);

    draw.text(&current_price.to_string())
        .x(win_rect.x() / 2.0)
        .y(-300.0)
        .font_size(30);

    draw.text(&date.to_string())
        .x(win_rect.x() / 2.0)
        .y(-350.0)
        .font_size(20);

    draw.to_frame(app, &frame).unwrap();
}
