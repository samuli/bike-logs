extern crate serde;
extern crate chrono;
extern crate colored;
extern crate structopt;

use std::{fs, io, process};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};
use serde::{de, Deserialize, Deserializer};
use colored::*;
use structopt::StructOpt;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    data: SessionData
}
#[derive(Debug, Clone, Deserialize)]
pub struct SessionData {
    #[serde(deserialize_with = "deserialize_from_str")]
    start_time: NaiveDateTime,
    #[serde(deserialize_with = "deserialize_from_str")]
    timestamp: NaiveDateTime,
    #[serde(default)]
    total_distance: f32,
    #[serde(default)]
    total_timer_time: f32,
    #[serde(default)]
    avg_speed: f32,
    #[serde(default)]
    avg_temperature: f32,
    #[serde(default)]
    total_ascent: f32,
    #[serde(default)]
    total_descent: f32
}

// You can use this deserializer for any type that implements FromStr
// and the FromStr::Err implements Display
fn deserialize_from_str<'de, S, D>(deserializer: D) -> Result<S, D::Error>
where
    S: FromStr,      // Required for S::from_str...
    S::Err: Display, // Required for .map_err(de::Error::custom)
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    S::from_str(&s).map_err(de::Error::custom)
}

fn format_minutes(mins:i64) -> String {
    let duration:Duration = chrono::Duration::seconds(mins as i64);
    let hours = duration.num_hours();
    let mins = duration.num_minutes() - (hours*60);
    format!("{hours}:{mins:02}", hours = hours, mins = mins)
}

fn print_week(
    timestamp:NaiveDateTime, week_data:Vec<String>,
    total:f32, total_minutes:i64, rides:u32
) {
    let year = timestamp.year();
    let week:u32 = timestamp.iso_week().week();
    let start:NaiveDate = NaiveDate::from_isoywd(year, week, Weekday::Mon);
    let end:NaiveDate = NaiveDate::from_isoywd(year, week, Weekday::Sun);

    println!("\n{week} ({start}.-{end}) {total:.0} km {time} {rides} rides",
             week = format!("Week {week}", week = week.to_string()).bold(),
             start = start.day(),
             end = end.format("%d.%m.%Y"),
             total = total/1000.0, rides = rides,
             time = format_minutes(total_minutes)
    );
    println!("{}", week_data.join("\n"));
}

#[derive(StructOpt, Debug, Clone)]
struct Params {
    #[structopt(long, help = "start date: YYYY-mm-dd")]
    since: Option<String>,
    #[structopt(long, help = "only output summary")]
    summary: bool,
    #[structopt(long, parse(from_os_str), help = "input data directory")]
    dir: PathBuf
}
    
fn main() -> io::Result<()> {
    let params = Params::from_args();

    let mut since:Option<NaiveDateTime> = match params.since {
        Some(datetime) => {
            match NaiveDateTime::parse_from_str(
                format!("{} 00:00:00", &datetime).as_ref(),
                "%Y-%m-%d %H:%M:%S"
            ) {
                Ok(date) => { Some(date) },
                Err(_) => { println!("Invalid date {}", datetime); process::exit(1); }
            }
        },
        None => None
    };
    let print_weekly = match params.summary {
        true => false,
        _ => true
    };
    
    let mut tot:f32 = 0.0;
    let mut tot_week:f32 = 0.0;
    let mut tot_time:i64 = 0;
    let mut tot_time_week:i64 = 0;

    let mut rides:u32 = 0;
    let mut rides_week:u32 = 0;
    
    let mut current_week:u32 = 0;
    let mut current_timestamp:Option<NaiveDateTime> = None;
    let mut week_data: Vec<String> = Vec::new();

    let files = match fs::read_dir(params.dir) {
        Ok(files) => files,
        Err(_) => {
            println!("Could not read data directory");
            process::exit(1);
        }
    };
    let mut paths: Vec<_> = files
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|dir| dir.path());

    let mut current_weekday:Option<Weekday> = None;
    let mut weekday_toggle = false;
    
    for entry in paths {
        let path = entry.path();
        let contents = fs::read_to_string(path)?;
        let data:Result<Vec<Session>, serde_json::error::Error>
            = serde_json::from_str(&contents);

        match data {
            Ok(d) => {
                let s = d.get(0).unwrap();

                let timestamp = s.data.timestamp;
                if since == None {
                    since = Some(timestamp);
                }
                if timestamp < since.unwrap() {
                    continue;
                }
                
                let week = timestamp.iso_week().week();
                if current_week != 0 {
                    if week != current_week {
                        if print_weekly {
                            print_week(current_timestamp.unwrap(), week_data, tot_week, tot_time_week, rides_week);
                        }
                        
                        rides_week = 0;
                        tot_week = 0.0;
                        tot_time_week = 0;
                        week_data = Vec::new();
                    }
                }
                current_week = week;
                current_timestamp = Some(timestamp);
                
                let total_meters = s.data.total_distance;
                tot += total_meters;
                tot_week += total_meters;

                let timer_time = s.data.total_timer_time as i64;
                let total_time:Duration = chrono::Duration::seconds(timer_time);
                let hours = total_time.num_hours();
                let mins = total_time.num_minutes() - (hours*60);
                let weekday = s.data.timestamp.weekday();

                tot_time += timer_time;
                tot_time_week += timer_time;

                let day:ColoredString = match weekday {
                    Weekday::Sat | Weekday::Sun => {
                        weekday_toggle = false;
                        weekday.to_string().yellow()
                    },
                    _ => {
                        let color = match current_weekday {
                            Some(current_day) => {
                                if weekday != current_day {
                                    weekday_toggle = !weekday_toggle;
                                }
                                if weekday_toggle == true {
                                    "green"
                                } else {
                                    "brigthGreen"
                                }
                            },
                            None => {
                                weekday_toggle = true;
                                "green"
                            }
                        };
                        weekday.to_string().color(color)
                    }
                };
                current_weekday = Some(weekday);
                
                week_data.push(format!("{day} {total: >5} km {time: >5} {avg:>4.1} km/h {asc: >4}↗ {desc: >4}↘ {temp: >4}℃",
                                       day = day,
                                       total = format!("{:.1}", total_meters/1000.0),
                                       time = format!("{hours:02}:{mins:02}", hours = hours, mins = mins),
                                       avg = s.data.avg_speed/1000.0,
                                       desc = s.data.total_descent,
                                       asc = s.data.total_ascent,
                                       temp = s.data.avg_temperature
                ));
                rides += 1;
                rides_week += 1;

            },
            Err(error) => { println!("Error parsing: {error}", error = error); }
        };
    }
    if print_weekly {
        match current_timestamp {
            Some(timestamp) => {
                print_week(timestamp, week_data, tot_week, tot_time_week, rides_week);
            },
            _ => ()
        }
    }
    if since != None {
        if print_weekly {
            println!("");
        }
        println!("{label} {date}: {tot:.1} km {time} {rides} rides",             
                 label = format!("Total since").bold(),
                 date = since.unwrap().format("%d.%m.%Y"),
                 time = format_minutes(tot_time),
                 tot = tot/1000.0, rides = rides
        );
    } else {
        println!("No data");
    }

    Ok(())
}
