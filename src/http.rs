use log::info;
use chrono::{DateTime, TimeZone,  Duration, Datelike, Timelike};
use chrono_tz::Tz;
use plotly::common::{Mode, HoverInfo, Line};
use plotly::{Plot, Scatter, Layout, layout::{Axis, ConstrainDirection}};

use tiny_http::{Server, Response, Header, Method};
use ascii::AsciiString;
use crate::tracker::{Tracker, LOG_PATH};

pub fn get_loop() -> impl FnOnce() {

    let host = std::env::var("GOTOBED_PLOT_HOST").unwrap_or("0.0.0.0".into());
    let port = std::env::var("GOTOBED_PLOT_HOST").unwrap_or("8080".into());
    let host_port = format!("{}:{}", host, port);

    move || {

        info!("Serving plot visualization at {}", host_port);
        let server = Server::http(host_port).unwrap();

        for request in server.incoming_requests() {

            info!("Received {:?} at {:?}", request.method(), request.url());

            match *request.method() {
                Method::Get => {

                    let html = render_html().as_bytes().to_vec();

                    request.respond(
                        Response::from_data(html)
                        .with_header(Header {
                            field: "Content-Type".parse().unwrap(),
                            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap()
                        })
                    ).unwrap();
                    
                },
                _ => { request.respond(Response::empty(405)).unwrap(); }
            }

        }
    }
}

fn render_html() -> String {

    let data = std::fs::read_to_string(LOG_PATH).unwrap();
    let tracker: Tracker = serde_json::from_str(&data)
        .map_err(anyhow::Error::new)
        .expect(&format!("Cannot restore time log"));

    //
    // Plotting history

    let times_list: Vec<DateTime<Tz>> = tracker.time_log.iter()
        .map(|(t_utc, timezone)| timezone.from_utc_datetime(&t_utc.naive_utc()))
        .collect();

    let (x_ticks_values, x_ticks_labels) = get_x_ticks(&times_list);
    let (y_ticks_values, y_ticks_labels) = get_y_ticks();

    let layout = Layout::new()
        .x_axis(
            Axis::new()
                .tick_values(x_ticks_values.clone())
                .tick_text(x_ticks_labels)
        )
        .y_axis(
            Axis::new()
                .tick_values(y_ticks_values.clone())
                .tick_text(y_ticks_labels)
                .constrain_toward(ConstrainDirection::Top)
        );

    let y_coords: Vec<f64> = times_list.iter()
        .map(|t| hourminute_to_y(t.hour() as i16, t.minute() as i16))
        .collect();

    let tmin = &times_list[0];
    let x_coords: Vec<i64> = times_list.iter()
        .map(|t| get_x_coord(tmin, t))
        .collect();

    let hovers: Vec<String> = y_coords.iter()
    .map(|&y| {
        let (h, m) = y_to_hourminute(y);
        format!("{:0>2}:{:0>2}", h, m)
    })
    .collect();

    let log_trace = Scatter::new(x_coords.clone(), y_coords.clone())
        .line(Line::new().color("blue"))
        .mode(Mode::LinesMarkers)
        .hover_info(HoverInfo::Text)
        .hover_text_array(hovers)
        .show_legend(false);

    //
    // Plotting average

    let y_coords_avg: Vec<f64> = avg_filter_y(&y_coords);

    let avg_trace = Scatter::new(x_coords.clone(), y_coords_avg.clone())
        .line(Line::new().color("green"))
        .mode(Mode::Lines)
        .hover_info(HoverInfo::None)
        .show_legend(false);

    //
    // Plot target line
        
    let (target_h, target_m): (u32, u32) = (23, 0);

    let target_y_val = 24.0 - (((target_h - 12) as f64) + (target_m as f64) / 60.0);

    let xmax = x_ticks_values.iter().fold(-f64::INFINITY, |a, &b| a.max(b));

    let target_x_coords = vec![0f64, xmax];
    let target_y_coords = vec![target_y_val, target_y_val];

    let target_trace = Scatter::new(target_x_coords, target_y_coords)
        .line(Line::new().color("red"))
        .mode(Mode::Lines)
        .show_legend(false);

    //
    // Render

    let mut plot = Plot::new();
    plot.add_trace(log_trace);
    plot.add_trace(avg_trace);
    plot.add_trace(target_trace);
    plot.set_layout(layout);

    plot.to_html()

}

fn hourminute_to_y(h: i16, m: i16) -> f64 {
    let h = (h - 12).rem_euclid(24);
    let y = (h as f64) + (m as f64) / 60.0;
    let y = 24.0 - y;
    y
}

fn y_to_hourminute(y: f64) -> (i16, i16) {
    let y = 24.0 - y;
    let h = y.floor();
    let m = ((y - h) * 60.0).round() as i16;
    let h = h as i16;
    let h = (h + 12) % 24;
    (h, m)
}

fn get_x_coord(tmin: &DateTime<Tz>, t: &DateTime<Tz>) -> i64 {
    let t_offset = *t - Duration::hours(12);
    let tmin_offset = *tmin - Duration::hours(12);
    let x_coord = (t_offset.date() - tmin_offset.date()).num_days();
    x_coord
}

fn avg_filter_y(y_coords: &Vec<f64>) -> Vec<f64> {
    const N: usize = 7;
    let mut v1 = y_coords.clone();
    let mut v2 = vec![v1[0]; N - 1];
    v2.append(&mut v1);

    let v3: Vec<f64> = v2
        .windows(N)
        .map(|vals| {
            let s: f64 = vals.iter().sum();
            let n = vals.len() as f64;
            s / n
        })
        .collect();

    assert_eq!(v3.len(), y_coords.len());

    v3
}

fn get_x_ticks(times_list: &Vec<DateTime<Tz>>) -> (Vec<f64>, Vec<String>) {

    let tmin = *times_list.first().unwrap();
    let tmax = *times_list.last().unwrap();

    let n = (tmax - tmin).num_days();
    let d0 = (tmin - Duration::hours(12)).date();

    let x_ticks_values: Vec<i64> = (0..=n).collect();
    let x_ticks_labels: Vec<String> = x_ticks_values.iter()
        .map(|&x| {
            let d = d0 + Duration::days(x);
            format!("{}/{}", d.day(), d.month())
        })
        .collect();

    let x_ticks_values = x_ticks_values.iter().map(|&i| i as f64).collect();

    (x_ticks_values, x_ticks_labels)
}

fn get_y_ticks() -> (Vec<f64>, Vec<String>) {

    let mut y_ticks_values = Vec::<f64>::new();
    let mut y_ticks_labels = Vec::<String>::new();
    for i in 0..4*24 {
        let i_rev = 4 * 24 - i;
        let h = (12 + (i_rev / 4)) % 24;
        let m = 15 * (i_rev % 4);
        y_ticks_values.push((i as f64) * 0.25);
        y_ticks_labels.push(format!("{:0>2}:{:0>2}", h, m));
    }

    (y_ticks_values, y_ticks_labels)
}
