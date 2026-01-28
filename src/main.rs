use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;

type Name = String;
type CompID = String;
type Transit = ();
type DateRange = String;
type Calendar = BTreeMap<DateRange, Vec<Event>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Event {
    Birthday(Name),
    Comp(CompID),
    Transit(Transit)
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
    T: Serialize,
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

fn main() -> Result<(), Box<dyn Error>> {
    let events: Calendar = read_events("docs/events.json")?;

    println!("{:#?}", events);

    write_events(&events, "docs/events.json")?;

    write_filtered(&events, "docs/birthdays.json", |e| {
        if let Event::Birthday(name) = e {
            Some(name)
        } else {
            None
        }
    })?;

    write_filtered(&events, "docs/comps.json", |e| {
        if let Event::Comp(id) = e {
            Some(id)
        } else {
            None
        }
    })?;

    write_filtered(&events, "docs/trans.json", |e| {
        if let Event::Transit(transit) = e {
            Some(transit)
        } else {
            None
        }
    })?;

    Ok(())
}
