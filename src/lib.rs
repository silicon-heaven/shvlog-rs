use std::collections::HashMap;
use time::format_description;
use ansi_term::Color;

// use chrono::{NaiveDateTime, TimeZone};
use flexi_logger::{DeferredNow, FlexiLoggerError, Level, Logger, LoggerHandle, Record};
use flexi_logger::filter::{LogLineFilter, LogLineWriter};

pub struct LogConfig {
    module_levels: HashMap<String, log::Level>,
    target_levels: HashMap<String, log::Level>
}
impl LogConfig {
    pub fn new(module_tresholds: &[String], target_tresholds: &[String]) -> LogConfig {
        let mut lv = LogConfig {
            module_levels: LogConfig::parse_level_strings(module_tresholds),
            target_levels: LogConfig::parse_level_strings(target_tresholds),
        };
        if lv.module_levels.is_empty() {
            lv.module_levels.insert("".into(), Level::Info);
        }
        lv
    }
    fn parse_level_strings(level_strings: &[String]) -> HashMap<String, log::Level> {
        let mut levels = HashMap::new();
        for tresholds in level_strings {
            for level_str in tresholds.split(',') {
                if level_str.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = level_str.split(':').collect();
                let (target, level_abbr) = if parts.len() == 1 {
                    (parts[0], "T")
                } else if parts.len() == 2 {
                    (parts[0], parts[1])
                } else {
                    panic!("Cannot happen");
                };
                let level = match level_abbr {
                    "T" => log::Level::Trace,
                    "D" => log::Level::Debug,
                    "I" => log::Level::Info,
                    "W" => log::Level::Warn,
                    "E" => log::Level::Error,
                    _ => log::Level::Info,
                };
                levels.insert(target.into(), level);
            }
        }
        levels
    }
    fn levels_to_string(levels: &HashMap<String, log::Level>) -> String {
        levels.iter()
            .map(|(target, level)| format!("{}:{}", target, level))
            .fold(String::new(), |acc, s| if acc.is_empty() { s } else { acc + "," + &s })
    }
    pub fn verbosity_string(&self) -> String {
        let mut ret: String = "".into();
        if !self.module_levels.is_empty() {
            ret = format!("-d {}", LogConfig::levels_to_string(&self.module_levels));
        }
        if !self.target_levels.is_empty() {
            if !ret.is_empty() {
                ret = ret + " ";
            }
            ret = ret + &format!("-v {}", LogConfig::levels_to_string(&self.target_levels));
        }
        ret
    }
}
impl LogLineFilter for LogConfig {
    fn write(&self, now: &mut DeferredNow, record: &log::Record, log_line_writer: &dyn LogLineWriter) -> std::io::Result<()> {
        let mut verbosity_level = Level::Info;
        let module = record.module_path().unwrap_or("");
        let target = record.target();
        let is_target_set = module != target;
        //println!("level: {}, module: {}, target: {}, target_set: {}, message: '{}'", record.level(), module, target, is_target_set, record.args());
        if is_target_set {
            for (key, level) in &self.target_levels {
                if let Some(_) = target.find(key) {
                    //println!("target found: {} with level: {}", key, level);
                    verbosity_level = *level;
                    break;
                }
            }
        } else {
            for (key, level) in &self.module_levels  {
                //println!("checking module '{}' contains: '{}'", module, key);
                if let Some(_) = module.find(key) {
                    //println!("module found: {} with level: {}", key, level);
                    verbosity_level = *level;
                    break;
                }
            }
        }
        //println!("comparing to level: {}", verbosity_level);
        if record.level() <= verbosity_level {
            log_line_writer.write(now, record)?;
        }
        Ok(())
    }
}

const TS_S: &str = "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]\
                    [offset_hour sign:mandatory]";//:[offset_minute]";
lazy_static::lazy_static! {
    static ref TS: Vec<format_description::FormatItem<'static>>
        = format_description::parse(TS_S).unwrap(/*ok*/);
}

fn log_format(w: &mut dyn std::io::Write, now: &mut DeferredNow, record: &Record) -> Result<(), std::io::Error> {
    // let sec = (now.now().unix_timestamp_nanos() / 1000_000_000) as i64;
    // let nano = (now.now().unix_timestamp_nanos() % 1000_000_000) as u32;
    // let ndt = NaiveDateTime::from_timestamp(sec, nano);
    // let dt = chrono::Local.from_utc_datetime(&ndt);
    let args = match record.level() {
        Level::Error => Color::Red.paint(format!("|E|{}", record.args())),
        Level::Warn => Color::Purple.paint(format!("|W|{}", record.args())),
        Level::Info => Color::Cyan.paint(format!("|I|{}", record.args())),
        Level::Debug => Color::Yellow.paint(format!("|D|{}", record.args())),
        Level::Trace => Color::White.dimmed().paint(format!("|T|{}", record.args())),
    };
    let target = if record.module_path().unwrap_or("") == record.target() { "".to_string() } else { format!("({})", record.target()) };
    write!(
        w,
        "{}{}{}{}",
        //dt.format("%Y-%m-%dT%H:%M:%S.%3f%z"),
        Color::Green.paint(
            now.now()
                .format(&TS)
                .unwrap_or_else(|_| "Timestamping failed".to_string())
        ),
        Color::Yellow.paint(format!("[{}:{}]", record.module_path().unwrap_or("<unnamed>"), record.line().unwrap_or(0))),
        Color::White.bold().paint(target),
        args,
    )
}

pub fn init(config: LogConfig) -> Result<LoggerHandle, FlexiLoggerError> {
    let handle = Logger::try_with_str("debug")?
        .filter(Box::new(config))
        .format(log_format)
        .set_palette("b1;3;2;4;6".into())
        .start()?;
    Ok(handle)
}
