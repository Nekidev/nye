use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use colored::{Color, Colorize};
use http::Method;
use jiff::{Unit, Zoned};

pub async fn handle(request: Request, next: Next) -> Response {
    let uri = request.uri().clone();
    let method = request.method().clone();

    let response = next.run(request).await;
    let status = response.status().as_u16();

    println!(
        "{} {} {:<7} {}",
        colored_timestamp(),
        colored_status(status),
        colored_method(method),
        uri
    );

    response
}

fn colored_timestamp() -> String {
    format!(
        "{:<24}",
        Zoned::now()
            .round(Unit::Millisecond)
            .unwrap()
            .timestamp()
            .to_string()
            .dimmed()
    )
}

fn colored_status(status: u16) -> String {
    let color = match status {
        0..100 => Color::White,
        100..200 => Color::Yellow,
        200..300 => Color::Green,
        300..400 => Color::Blue,
        400..500 => Color::BrightMagenta,
        500.. => Color::Red,
    };

    status.to_string().color(color).to_string()
}

fn colored_method(method: Method) -> String {
    let color = match method {
        Method::GET => Color::Green,
        Method::POST => Color::Blue,
        Method::HEAD => Color::Cyan,
        Method::PUT => Color::Yellow,
        Method::DELETE => Color::Red,
        Method::TRACE => Color::Black,
        Method::OPTIONS => Color::Magenta,
        Method::PATCH => Color::BrightYellow,
        _ => Color::White,
    };

    format!("{method:<7}").color(color).to_string()
}
