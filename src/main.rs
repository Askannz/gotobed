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

    // Ensuring that the program stops if any thread panics, so that it can be restarted
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    std::thread::spawn(telegram.get_loop());
    std::thread::spawn(http::get_loop());

    telegram.send("Starting...".into());

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
