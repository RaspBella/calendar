use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{File, create_dir_all};
use std::io::{BufReader, BufWriter, Write};
use chrono::{Utc, DateTime, Datelike, NaiveDate, Duration, Months};
use regex::Regex;

type Name = String;
type CompID = String;

type Place = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Transit {
    Walk { from: Place, to: Place },
    Bus { from: Place, to: Place, info: String },
    Metro { from: Place, to: Place, info: String },
    Train { from: Place, to: Place, info: String },
    Plane { from: Place, to: Place }
}

type DateRange = String;
type Calendar = BTreeMap<DateRange, Vec<Event>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Event {
    Birthday(Name),
    Comp(CompID),
    Transit(Transit)
}

#[derive(Debug)]
enum DateStep {
    Days(i64),
    Months(u32),
    Years(u32)
}

#[derive(Debug)]
struct DateIter {
    current: NaiveDate,
    end: NaiveDate,
    step: DateStep,
    finished: bool
}

impl Iterator for DateIter {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished || self.current > self.end {
            return None;
        }

        let out = self.current;

        match self.step {
            DateStep::Days(0) => self.finished = true,
            DateStep::Days(d) => {
                let next = self.current.checked_add_signed(Duration::days(d));

                match next {
                    Some(d) => self.current = d,
                    None => self.finished = true
                }
            },
            DateStep::Months(m) => {
                let next = self.current.checked_add_months(Months::new(m));

                match next {
                    Some(d) => self.current = d,
                    None => self.finished = true
                }
            },
            DateStep::Years(y) => {
                let next = self.current.checked_add_months(Months::new(12 * y));

                match next {
                    Some(d) => self.current = d,
                    None => self.finished = true
                }
            }
        };

        Some(out)
    }
}

fn read_events(path: &str) -> Result<Calendar, Box<dyn Error>> {
    let file = File::open(path)?;

    let reader = BufReader::new(file);

    Ok(serde_json::from_reader(reader)?)
}

fn write_events(events: &Calendar, path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;

    let writer = BufWriter::new(file);

    Ok(serde_json::to_writer(writer, &events)?)
}

fn write_filtered<T, F>(
    data: &Calendar,
    path: &str,
    mut f: F
) -> Result<(), Box<dyn Error>>
where
    T: Serialize + 'static,
    F: FnMut(Event) -> Option<T>
{
    let file = File::create(path)?;

    let writer = BufWriter::new(file);

    let out: BTreeMap<DateRange, Vec<T>> = data
        .iter()
        .map(|(date, events)| {
            let values = events
                .iter()
                .cloned()
                .filter_map(&mut f)
                .collect::<Vec<_>>();
            (date.clone(), values)
        })
    .filter(|(_, v)| !v.is_empty())
        .collect();

    serde_json::to_writer(writer, &out)?;

    Ok(())
}

fn parse_date_range(date_range: &DateRange, now: DateTime<Utc>) -> Result<DateIter, Box<dyn Error>> {
    let re = Regex::new(
        r"^(?P<y>\d{4})(?:\+(?P<yo>\d*))?-(?P<m>\d{2})(?:\+(?P<mo>\d*))?-(?P<d>\d{2})(?:\+(?P<do>\d*))?$"
    )?;

    let caps = re
        .captures(date_range)
        .ok_or("invalid date range")?;

    let start = NaiveDate::from_ymd_opt(
        caps["y"].parse()?,
        caps["m"].parse()?,
        caps["d"].parse()?
    ).unwrap();

    let years_offset: u32 = caps.name("yo")
        .map(|m| if m.as_str().is_empty() {
            let diff = now.year() - start.year();

            if diff < 0 { 0 } else { diff as u32 }
        } else {
            m.as_str().parse::<u32>().unwrap()
        })
        .unwrap_or(0);

    let months_offset: u32 = caps.name("mo")
        .map(|m| if m.as_str().is_empty() {
            let diff = now.month() as i32 - start.month() as i32;

            if diff < 0 { 0 } else { diff as u32 }
        } else {
            m.as_str().parse::<u32>().unwrap()
        })
        .unwrap_or(0);

    let days_offset: i64 = caps.name("do")
        .map(|m| if m.as_str().is_empty() {
            let diff = now.day() as i64 - start.day() as i64;

            if diff < 0 { 0 } else { diff }
        } else {
            m.as_str().parse::<i64>().unwrap()
        })
        .unwrap_or(0);

    let mut end = start;

    if years_offset > 0 {
        end = end.checked_add_months(Months::new(12 * years_offset)).ok_or("invalid end date(y)")?;
    }
    if months_offset > 0 {
        end = end.checked_add_months(Months::new(years_offset)).ok_or("invalid end date(m)")?;
    }
    if days_offset > 0 {
        end = end.checked_add_signed(Duration::days(days_offset)).ok_or("invalid end date(d)")?;
    }

    let step = if days_offset > 0 {
        DateStep::Days(1)
    } else if months_offset > 0 {
        DateStep::Months(1)
    } else if years_offset > 0 {
        DateStep::Years(1)
    } else {
        DateStep::Days(0)
    };

    Ok(DateIter { current: start, end: end, step: step, finished: false })
}

fn format_birthday(name: &String, age: i32) -> String {
    match age {
        0 => format!("<h1>{name} has been born</h1>"),
        11..13 => format!("<h1>Happy {age}th Birthday {name}</h1>"),
        _ => match age % 10 {
            1 => format!("<h1>Happy {age}st Birthday {name}</h1>"),
            2 => format!("<h1>Happy {age}nd Birthday {name}</h1>"),
            3 => format!("<h1>Happy {age}rd Birthday {name}</h1>"),
            _ => format!("<h1>Happy {age}th Birthday {name}</h1>")
        }
    }
}

fn format_comp(id: &CompID) -> String {
    format!("<h1><a href=\"https://www.worldcubeassociation.org/competitions/{}\">{0}</a></h1>", id)
}

fn format_transit(transit: &Transit) -> String {
    "bleh :p".to_string()
}

fn main() -> Result<(), Box<dyn Error>> {
    let calendar: Calendar = read_events("docs/events.json")?;

    let now: DateTime<Utc> = Utc::now();

    let mut index = String::new();

    for (date, events) in &calendar {
        let mut range = parse_date_range(&date, now)?;
        let start = range.current;

        for virtual_date in &mut range {
            let y = format!("{:04}", virtual_date.year());
            let m = format!("{:02}", virtual_date.month());
            let d = format!("{:02}", virtual_date.day());

            let ymd = format!("{y}/{m}/{d}");

            index.push_str(
                &format!("<h1><a href=\"{}\">{0}</a></h1>", ymd)
            );

            create_dir_all(
                format!("docs/{ymd}")
            )?;

            let divs = events
                .iter()
                .map(|e|
                    match e {
                        Event::Birthday(name) => format_birthday(name, virtual_date.year() - start.year()),
                        Event::Comp(id) => format_comp(id),
                        Event::Transit(transit) => format_transit(transit)
                    }
                )
                .collect::<Vec<_>>()
                .join("");

            let index = format!(
                include_str!("index.html"),
                format!("{ymd} - calendar"),
                divs
            );

            let file = File::create(
                format!("docs/{ymd}/index.html")
            )?;

            let mut writer = BufWriter::new(file);

            write!(writer, "{}", index)?;
        }
    }

    let index = format!(
        include_str!("index.html"),
        format!("calendar"),
        index
    );

    let file = File::create("docs/index.html")?;

    let mut writer = BufWriter::new(file);

    write!(writer, "{}", index)?;

    write_events(&calendar, "docs/events.json")?;

    write_filtered(&calendar, "docs/birthdays.json", |e| {
        if let Event::Birthday(name) = e {
            Some(name)
        } else {
            None
        }
    })?;

    write_filtered(&calendar, "docs/comps.json", |e| {
        if let Event::Comp(id) = e {
            Some(id)
        } else {
            None
        }
    })?;

    write_filtered(&calendar, "docs/trans.json", |e| {
        if let Event::Transit(transit) = e {
            Some(transit)
        } else {
            None
        }
    })?;

    Ok(())
}
