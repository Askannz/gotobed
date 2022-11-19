mod telegram;
mod tracker;
mod http;

use crossbeam_channel::unbounded;
use tracker::Tracker;
use telegram::Telegram;

fn main() {

    env_logger::Builder::new()
        .filter_module("gotobed", log::LevelFilter::Info)
        .init();
    
    let (sender, receiver) = unbounded();

    let mut telegram = Telegram::new(&sender);
    let mut tracker = Tracker::restore().unwrap_or(Tracker::new());

    std::thread::spawn(telegram.get_loop());
    std::thread::spawn(http::get_loop());

    loop {

        let msg = receiver.recv().unwrap();

        let resp: String = match msg.as_str() {
            "log" | "/log" => {
                let local_t = tracker.log();
                format!("{}", local_t.format("%d/%m/%Y %H:%M"))
            },
            _ => "Unknown command".into()
        };

        telegram.send(&resp);
    }
}

pub fn format_error(err: anyhow::Error) -> String {
    err.chain()
        .map(|err| format!("{}", err))
        .collect::<Vec<String>>()
        .join(": ")
}
