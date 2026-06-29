use chrono::{Datelike, NaiveDateTime, Timelike};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const DEFAULT_INPUT: &str = "logs/ft8op-field-day-completed.jsonl";
const DEFAULT_CONTEST_ID: &str = "ARRL-FD";

#[derive(Debug, Parser)]
#[command(name = "ft8op-fd-export")]
#[command(about = "Export ft8op Field Day completed-QSO JSONL to ADIF or Cabrillo")]
struct Cli {
    #[arg(long, default_value = DEFAULT_INPUT)]
    input: PathBuf,

    #[arg(long, value_enum)]
    format: ExportFormat,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(
        long,
        help = "Station callsign for ADIF STATION_CALLSIGN and Cabrillo CALLSIGN"
    )]
    station_call: Option<String>,

    #[arg(long, default_value = DEFAULT_CONTEST_ID)]
    contest_id: String,

    #[arg(
        long,
        help = "Drop duplicate call/band/mode records, keeping the first completed QSO"
    )]
    dedupe: bool,

    #[command(flatten)]
    cabrillo: CabrilloOptions,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ExportFormat {
    Adif,
    Cabrillo,
}

#[derive(Debug, Clone, Parser)]
struct CabrilloOptions {
    #[arg(long)]
    operators: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    address: Vec<String>,
    #[arg(long)]
    city: Option<String>,
    #[arg(long)]
    state: Option<String>,
    #[arg(long)]
    postal_code: Option<String>,
    #[arg(long)]
    country: Option<String>,
    #[arg(long)]
    club: Option<String>,
    #[arg(long)]
    email: Option<String>,
    #[arg(long)]
    location: Option<String>,
    #[arg(long)]
    claimed_score: Option<u32>,
    #[arg(long, default_value = "SINGLE-OP")]
    category_operator: String,
    #[arg(long, default_value = "NON-ASSISTED")]
    category_assisted: String,
    #[arg(long, default_value = "ALL")]
    category_band: String,
    #[arg(long, default_value = "LOW")]
    category_power: String,
    #[arg(long)]
    category_station: Option<String>,
    #[arg(long, default_value = "24-HOURS")]
    category_time: String,
    #[arg(long)]
    soapbox: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompletedQsoRecord {
    schema_version: u8,
    session_id: u64,
    completed_at: String,
    call: String,
    band: String,
    frequency_hz: Option<u64>,
    mode: String,
    sent_exchange: String,
    received_exchange: String,
    received_section: String,
    completion_confidence: String,
}

#[derive(Debug, thiserror::Error)]
enum ExportError {
    #[error("read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("line {line}: invalid JSON: {source}")]
    Json {
        line: usize,
        source: serde_json::Error,
    },
    #[error("line {line}: unsupported schema_version {version}")]
    UnsupportedSchema { line: usize, version: u8 },
    #[error("line {line}: invalid completed_at {value:?}; expected UTC YYYY-MM-DD HH:MM:SS")]
    InvalidCompletedAt { line: usize, value: String },
    #[error("Cabrillo export requires --station-call")]
    MissingStationCall,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ExportError> {
    let cli = Cli::parse();
    let contents = fs::read_to_string(&cli.input).map_err(|source| ExportError::Read {
        path: cli.input.clone(),
        source,
    })?;
    let records = read_records(&contents, cli.dedupe)?;
    let output = match cli.format {
        ExportFormat::Adif => render_adif(&records, cli.station_call.as_deref(), &cli.contest_id),
        ExportFormat::Cabrillo => render_cabrillo(&records, &cli)?,
    };

    if let Some(path) = cli.output {
        fs::write(&path, output).map_err(|source| ExportError::Write { path, source })?;
    } else {
        print!("{output}");
    }
    Ok(())
}

fn read_records(contents: &str, dedupe: bool) -> Result<Vec<CompletedQsoRecord>, ExportError> {
    let mut records = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record: CompletedQsoRecord =
            serde_json::from_str(trimmed).map_err(|source| ExportError::Json {
                line: line_number,
                source,
            })?;
        if record.schema_version != 1 {
            return Err(ExportError::UnsupportedSchema {
                line: line_number,
                version: record.schema_version,
            });
        }
        parse_completed_at(&record, line_number)?;
        records.push(record);
    }

    records.sort_by_key(|record| {
        (
            parse_completed_at(record, 0).expect("validated completed_at"),
            record.session_id,
        )
    });

    if dedupe {
        let mut seen = BTreeSet::new();
        records.retain(|record| {
            seen.insert((
                record.call.to_uppercase(),
                record.band.to_uppercase(),
                record.mode.to_uppercase(),
            ))
        });
    }

    Ok(records)
}

fn render_adif(
    records: &[CompletedQsoRecord],
    station_call: Option<&str>,
    contest_id: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Generated by ft8op-fd-export\n");
    out.push_str("<ADIF_VER:5>3.1.4\n");
    out.push_str("<PROGRAMID:5>ft8op\n");
    out.push_str("<EOH>\n");

    for record in records {
        let completed_at = parse_completed_at(record, 0).expect("validated completed_at");
        write_adif_field(&mut out, "CALL", &record.call);
        if let Some(station_call) = station_call {
            write_adif_field(&mut out, "STATION_CALLSIGN", station_call);
        }
        write_adif_field(&mut out, "QSO_DATE", &adif_date(completed_at));
        write_adif_field(&mut out, "TIME_ON", &adif_time(completed_at));
        write_adif_field(&mut out, "BAND", &record.band);
        if let Some(freq_mhz) = frequency_mhz(record) {
            write_adif_field(&mut out, "FREQ", &freq_mhz);
        }
        let (mode, submode) = adif_mode_fields(&record.mode);
        write_adif_field(&mut out, "MODE", &mode);
        if let Some(submode) = submode {
            write_adif_field(&mut out, "SUBMODE", &submode);
        }
        write_adif_field(&mut out, "CONTEST_ID", contest_id);
        write_adif_field(&mut out, "STX_STRING", &record.sent_exchange);
        write_adif_field(&mut out, "SRX_STRING", &record.received_exchange);
        if !record.received_section.trim().is_empty() {
            write_adif_field(&mut out, "ARRL_SECT", &record.received_section);
        }
        write_adif_field(
            &mut out,
            "APP_FT8OP_COMPLETION_CONFIDENCE",
            &record.completion_confidence,
        );
        out.push_str("<EOR>\n");
    }

    out
}

fn render_cabrillo(records: &[CompletedQsoRecord], cli: &Cli) -> Result<String, ExportError> {
    let station_call = cli
        .station_call
        .as_deref()
        .ok_or(ExportError::MissingStationCall)?;
    let sent_exchange = records
        .iter()
        .find_map(|record| non_empty(&record.sent_exchange))
        .unwrap_or_default();
    let location = cli
        .cabrillo
        .location
        .as_deref()
        .or_else(|| sent_exchange.split_whitespace().nth(1))
        .unwrap_or("");
    let category_station = cli
        .cabrillo
        .category_station
        .as_deref()
        .or_else(|| sent_exchange.split_whitespace().next())
        .unwrap_or("");

    let mut out = String::new();
    out.push_str("START-OF-LOG: 3.0\n");
    writeln!(out, "CALLSIGN: {}", station_call.to_uppercase()).expect("write string");
    writeln!(out, "CONTEST: {}", cli.contest_id).expect("write string");
    write_header(
        &mut out,
        "CATEGORY-OPERATOR",
        &cli.cabrillo.category_operator,
    );
    write_header(
        &mut out,
        "CATEGORY-ASSISTED",
        &cli.cabrillo.category_assisted,
    );
    write_header(&mut out, "CATEGORY-BAND", &cli.cabrillo.category_band);
    write_header(&mut out, "CATEGORY-MODE", "DIGI");
    write_header(&mut out, "CATEGORY-POWER", &cli.cabrillo.category_power);
    write_header(&mut out, "CATEGORY-STATION", category_station);
    write_header(&mut out, "CATEGORY-TIME", &cli.cabrillo.category_time);
    write_header(&mut out, "LOCATION", location);
    if let Some(score) = cli.cabrillo.claimed_score {
        writeln!(out, "CLAIMED-SCORE: {score}").expect("write string");
    }
    write_optional_header(&mut out, "OPERATORS", cli.cabrillo.operators.as_deref());
    write_optional_header(&mut out, "CLUB", cli.cabrillo.club.as_deref());
    write_optional_header(&mut out, "NAME", cli.cabrillo.name.as_deref());
    for address in &cli.cabrillo.address {
        write_header(&mut out, "ADDRESS", address);
    }
    write_optional_header(&mut out, "ADDRESS-CITY", cli.cabrillo.city.as_deref());
    write_optional_header(
        &mut out,
        "ADDRESS-STATE-PROVINCE",
        cli.cabrillo.state.as_deref(),
    );
    write_optional_header(
        &mut out,
        "ADDRESS-POSTALCODE",
        cli.cabrillo.postal_code.as_deref(),
    );
    write_optional_header(&mut out, "ADDRESS-COUNTRY", cli.cabrillo.country.as_deref());
    write_optional_header(&mut out, "EMAIL", cli.cabrillo.email.as_deref());
    for soapbox in &cli.cabrillo.soapbox {
        write_header(&mut out, "SOAPBOX", soapbox);
    }

    for record in records {
        let completed_at = parse_completed_at(record, 0).expect("validated completed_at");
        let freq = cabrillo_frequency(record);
        let mode = cabrillo_mode(&record.mode);
        writeln!(
            out,
            "QSO: {:>5} {:<2} {} {} {:<13} {:<8} {:<4} {:<13} {:<8} {:<4}",
            freq,
            mode,
            cabrillo_date(completed_at),
            cabrillo_time(completed_at),
            station_call.to_uppercase(),
            exchange_class(&record.sent_exchange),
            exchange_section(&record.sent_exchange),
            record.call.to_uppercase(),
            exchange_class(&record.received_exchange),
            exchange_section(&record.received_exchange),
        )
        .expect("write string");
    }
    out.push_str("END-OF-LOG:\n");
    Ok(out)
}

fn parse_completed_at(
    record: &CompletedQsoRecord,
    line: usize,
) -> Result<NaiveDateTime, ExportError> {
    NaiveDateTime::parse_from_str(&record.completed_at, "%Y-%m-%d %H:%M:%S").map_err(|_| {
        ExportError::InvalidCompletedAt {
            line,
            value: record.completed_at.clone(),
        }
    })
}

fn write_adif_field(out: &mut String, name: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    write!(out, "<{name}:{}>{value}", value.len()).expect("write string");
}

fn write_header(out: &mut String, name: &str, value: &str) {
    let value = value.trim();
    if !value.is_empty() {
        writeln!(out, "{name}: {value}").expect("write string");
    }
}

fn write_optional_header(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        write_header(out, name, value);
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn adif_date(time: NaiveDateTime) -> String {
    format!("{:04}{:02}{:02}", time.year(), time.month(), time.day())
}

fn adif_time(time: NaiveDateTime) -> String {
    format!("{:02}{:02}{:02}", time.hour(), time.minute(), time.second())
}

fn cabrillo_date(time: NaiveDateTime) -> String {
    format!("{:04}-{:02}-{:02}", time.year(), time.month(), time.day())
}

fn cabrillo_time(time: NaiveDateTime) -> String {
    format!("{:02}{:02}", time.hour(), time.minute())
}

fn frequency_mhz(record: &CompletedQsoRecord) -> Option<String> {
    record
        .frequency_hz
        .map(|hz| trim_float(format!("{:.6}", hz as f64 / 1_000_000.0)))
}

fn cabrillo_frequency(record: &CompletedQsoRecord) -> String {
    if let Some(hz) = record.frequency_hz {
        return ((hz + 500) / 1000).to_string();
    }
    match record.band.trim().to_lowercase().as_str() {
        "160m" => "1800",
        "80m" => "3500",
        "40m" => "7000",
        "20m" => "14000",
        "15m" => "21000",
        "10m" => "28000",
        "6m" => "50",
        "2m" => "144",
        "1.25m" | "125cm" => "222",
        "70cm" => "432",
        _ => "0",
    }
    .to_string()
}

fn trim_float(mut value: String) -> String {
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}

fn adif_mode_fields(mode: &str) -> (String, Option<String>) {
    let mode = mode.trim().to_uppercase();
    match mode.trim().to_uppercase().as_str() {
        "FT8" | "FT4" | "FT2" => ("MFSK".to_string(), Some(mode)),
        _ => (mode, None),
    }
}

fn cabrillo_mode(mode: &str) -> &'static str {
    match mode.trim().to_uppercase().as_str() {
        "CW" => "CW",
        "SSB" | "USB" | "LSB" | "AM" | "FM" => "PH",
        _ => "DG",
    }
}

fn exchange_class(exchange: &str) -> String {
    exchange
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim()
        .to_uppercase()
}

fn exchange_section(exchange: &str) -> String {
    exchange
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"schema_version":1,"session_id":2,"started_at":"2026-06-28 11:22:13","completed_at":"2026-06-28 11:23:43","call":"N7UVH","band":"40m","frequency_hz":7074000,"mode":"FT8","exchange_mode":"field_day","field_day_mode_active":true,"field_day_only":true,"sent_exchange":"1A SB","sent_transmitter_count":1,"received_exchange":"1E ID","received_transmitter_count":1,"received_class":"E","received_section":"ID","contest_exchange_received":true,"completion_confidence":"confirmed","exit_reason":"send_73_once_complete","last_rx_event":"to_us_reply_rr73"}
{"schema_version":1,"session_id":1,"started_at":"2026-06-28 11:20:00","completed_at":"2026-06-28 11:21:00","call":"K1ABC","band":"20m","frequency_hz":14074000,"mode":"FT8","exchange_mode":"field_day","field_day_mode_active":true,"field_day_only":true,"sent_exchange":"1A SB","sent_transmitter_count":1,"received_exchange":"2A WWA","received_transmitter_count":2,"received_class":"A","received_section":"WWA","contest_exchange_received":true,"completion_confidence":"confirmed","exit_reason":"send_73_once_complete","last_rx_event":"to_us_reply_rr73"}"#;

    #[test]
    fn adif_export_contains_field_day_records() {
        let records = read_records(SAMPLE, false).expect("records");
        let adif = render_adif(&records, Some("AA6FD"), DEFAULT_CONTEST_ID);

        assert!(adif.contains("<CALL:5>K1ABC"));
        assert!(adif.contains("<STATION_CALLSIGN:5>AA6FD"));
        assert!(adif.contains("<QSO_DATE:8>20260628<TIME_ON:6>112100"));
        assert!(adif.contains("<FREQ:6>14.074"));
        assert!(adif.contains("<MODE:4>MFSK<SUBMODE:3>FT8"));
        assert!(adif.contains("<SRX_STRING:6>2A WWA<ARRL_SECT:3>WWA"));
    }

    #[test]
    fn cabrillo_export_contains_field_day_qso_lines() {
        let records = read_records(SAMPLE, false).expect("records");
        let cli = Cli {
            input: PathBuf::from(DEFAULT_INPUT),
            format: ExportFormat::Cabrillo,
            output: None,
            station_call: Some("aa6fd".to_string()),
            contest_id: DEFAULT_CONTEST_ID.to_string(),
            dedupe: false,
            cabrillo: CabrilloOptions {
                operators: Some("AA6FD".to_string()),
                name: None,
                address: Vec::new(),
                city: None,
                state: None,
                postal_code: None,
                country: None,
                club: None,
                email: None,
                location: None,
                claimed_score: None,
                category_operator: "SINGLE-OP".to_string(),
                category_assisted: "NON-ASSISTED".to_string(),
                category_band: "ALL".to_string(),
                category_power: "LOW".to_string(),
                category_station: None,
                category_time: "24-HOURS".to_string(),
                soapbox: Vec::new(),
            },
        };

        let cabrillo = render_cabrillo(&records, &cli).expect("cabrillo");
        assert!(cabrillo.contains("CALLSIGN: AA6FD"));
        assert!(cabrillo.contains("CATEGORY-STATION: 1A"));
        assert!(cabrillo.contains("LOCATION: SB"));
        assert!(cabrillo.contains(
            "QSO: 14074 DG 2026-06-28 1121 AA6FD         1A       SB   K1ABC         2A       WWA"
        ));
        assert!(cabrillo.ends_with("END-OF-LOG:\n"));
    }

    #[test]
    fn dedupe_keeps_first_completed_qso() {
        let duplicate = format!("{SAMPLE}\n{}", SAMPLE.lines().next().unwrap());
        let records = read_records(&duplicate, true).expect("records");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].call, "K1ABC");
        assert_eq!(records[1].call, "N7UVH");
    }
}
