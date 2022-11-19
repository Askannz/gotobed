use std::vec::Vec;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, offset, TimeZone};

type Instant = DateTime<offset::Utc>;
type LogEntry = (Instant, chrono_tz::Tz);

#[derive(Serialize, Deserialize)]
pub struct Tracker {
    pub time_log: Vec<LogEntry>
}

const TZ: &'static str = "Australia/Melbourne";
pub const LOG_PATH: &'static str = "time_log.json";


impl Tracker {

    pub fn new() -> Self {
        Tracker { time_log: Vec::new() }
    }

    pub fn restore() -> anyhow::Result<Self> {

        let path = PathBuf::from(LOG_PATH);

        let data = std::fs::read_to_string(path)?;
        let logger: Tracker = serde_json::from_str(&data)
            .map_err(anyhow::Error::new)
            .expect(&format!("Cannot restore time log"));

        Ok(logger)
    }

    fn save(&self) {

        let path = PathBuf::from(LOG_PATH);

        || -> anyhow::Result<()> {
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(path, data)?;
            Ok(())
        }()
        .expect(&format!("Cannot save time log"));
    }

    pub fn log(&mut self) -> DateTime<chrono_tz::Tz> {

        let tz = TZ.parse().unwrap();
        let utc_now = offset::Utc::now();

        log::info!("Logging {}, {}", utc_now, tz);
        self.time_log.push((utc_now, tz));
        self.save();

        let local_dt = tz.from_utc_datetime(&utc_now.naive_utc());

        return local_dt
    }
}
