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

    let re_log = regex::Regex::new(r"^/?log$").unwrap();
    let re_target_print = regex::Regex::new(r"^/?target$").unwrap();
    let re_target_set = regex::Regex::new(r"^/?target ([0-9]{1,2}):([0-9]{1,2})$").unwrap();

    loop {

        let msg = receiver.recv().unwrap();
        let msg = msg.as_str();
     
        let resp = {
            if re_log.is_match(msg) {
                let local_t = tracker.log();
                format!("{}", local_t.format("%d/%m/%Y %H:%M"))
            }
            else if re_target_print.is_match(msg) {
                let (t_h, t_m) = tracker.current_target;
                format!("Current target: {t_h:02}:{t_m:02}")
            }
            else if let Some(caps) = re_target_set.captures(msg) {
                let t_h: u8 = caps.get(1).unwrap().as_str().parse().unwrap();
                let t_m: u8 = caps.get(2).unwrap().as_str().parse().unwrap();
                if t_h > 23 || t_m > 59 {
                    "Invalid target time".into()
                } else {
                    tracker.set_target((t_h, t_m));
                    format!("Set target to {t_h:02}:{t_m:02}")
                }
            }
            else {
                "Invalid command".into()
            }
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
