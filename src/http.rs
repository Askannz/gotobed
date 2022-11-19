use log::info;
use chrono::{DateTime, TimeZone,  Duration, Datelike, Timelike};
use chrono_tz::Tz;
use plotly::common::{Mode, HoverInfo, Line};
use plotly::{Plot, Scatter, Layout, layout::{Axis, ConstrainDirection}};

use tiny_http::{Server, Response, Header, Method};
use ascii::AsciiString;
use crate::tracker::{Tracker, LOG_PATH};

struct DataPoint {
    x_coord: i64,
    y_coord: f64,
    hover: String
}

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

    let times_list: Vec<DateTime<Tz>> = tracker.time_log.iter()
        .map(|(t_utc, timezone)| timezone.from_utc_datetime(&t_utc.naive_utc()))
        .collect();

    let (x_ticks_values, x_ticks_labels) = get_x_ticks(&times_list);
    let (y_ticks_values, y_ticks_labels) = get_y_ticks();
    let xmax = x_ticks_values.iter().fold(-f64::INFINITY, |a, &b| a.max(b));

    let layout = Layout::new()
        .x_axis(
            Axis::new()
                .tick_values(x_ticks_values)
                .tick_text(x_ticks_labels)
        )
        .y_axis(
            Axis::new()
                .tick_values(y_ticks_values)
                .tick_text(y_ticks_labels)
                .constrain_toward(ConstrainDirection::Top)
        );

    let datapoints: Vec<DataPoint> = times_list.iter()
        .map(|t| get_datapoint(&times_list[0], t)).collect();

    let x_coords: Vec<i64> = datapoints.iter().map(|dp| dp.x_coord).collect();
    let y_coords: Vec<f64> = datapoints.iter().map(|dp| dp.y_coord).collect();
    let hovers: Vec<String> = datapoints.iter().map(|dp| dp.hover.clone()).collect();

    let log_trace = Scatter::new(x_coords, y_coords)
        .mode(Mode::LinesMarkers)
        .hover_info(HoverInfo::Text)
        .hover_text_array(hovers)
        .show_legend(false);
        
    let (target_h, target_m): (u32, u32) = (23, 0);

    let target_y_val = 24.0 - (((target_h - 12) as f64) + (target_m as f64) / 60.0);

    let target_x_coords = vec![0f64, xmax];
    let target_y_coords = vec![target_y_val, target_y_val];

    let target_trace = Scatter::new(target_x_coords, target_y_coords)
        .line(Line::new().color("red"))
        .mode(Mode::Lines)
        .show_legend(false);

    let mut plot = Plot::new();
    plot.add_trace(log_trace);
    plot.add_trace(target_trace);
    plot.set_layout(layout);

    plot.to_html()

}

fn get_datapoint(tmin: &DateTime<Tz>, t: &DateTime<Tz>) -> DataPoint {

    let t_offset = *t - Duration::hours(12);
    let tmin_offset = *tmin - Duration::hours(12);
    let d_offset = t_offset.date();

    let x_coord = (t_offset.date() - tmin_offset.date()).num_days();
    let y_coord = get_y_coord(t_offset.hour(), t_offset.minute());
    let x_label = format!("{}/{}", d_offset.day(), d_offset.month());
    let y_label = format!("{:0>2}:{:0>2}", t.hour(), t.minute());

    let hover = format!("{} {}", x_label, y_label);

    DataPoint {
        x_coord,
        y_coord,
        hover
    }
}


fn get_y_coord(h: u32, m: u32) -> f64 {
    24.0 - ((h as f64) + (m as f64) / 60.0)
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
