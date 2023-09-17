use std::vec::Vec;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, offset, TimeZone};

const DEFAULT_TARGET: (u8, u8) = (11, 20);

type Instant = DateTime<offset::Utc>;

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub bedtime: Instant,
    pub timezone: chrono_tz::Tz,
    pub target: (u8, u8)
}

#[derive(Serialize, Deserialize)]
pub struct Tracker {
    pub current_target: (u8, u8),
    pub time_log: Vec<LogEntry>
}

const TZ: &'static str = "Australia/Melbourne";
pub const LOG_PATH: &'static str = "time_log.json";


impl Tracker {

    pub fn new() -> Self {
        Tracker { 
            current_target: DEFAULT_TARGET,
            time_log: Vec::new()
        }
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

    pub fn set_target(&mut self, target: (u8, u8)) {
        self.current_target = target;
        self.save();
    }

    pub fn log(&mut self) -> DateTime<chrono_tz::Tz> {

        let tz = TZ.parse().unwrap();
        let utc_now = offset::Utc::now();

        log::info!("Logging {}, {}", utc_now, tz);
        self.time_log.push(LogEntry { 
            bedtime: utc_now,
            timezone: tz,
            target: self.current_target
        });
        self.save();

        let local_dt = tz.from_utc_datetime(&utc_now.naive_utc());

        return local_dt
    }
}
