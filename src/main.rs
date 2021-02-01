use anyhow::Error;
use clap::{crate_version, load_yaml, App, AppSettings};
use jtd_infer::{HintSet, Hints, Inferrer, NumType};
use serde_json::Deserializer;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::io::Read;

fn main() -> Result<(), Error> {
    let cli_yaml = load_yaml!("cli.yaml");
    let matches = App::from(cli_yaml)
        .setting(AppSettings::ColoredHelp)
        .version(crate_version!())
        .get_matches();

    let reader = BufReader::new(match matches.value_of("input").unwrap() {
        "-" => Box::new(stdin()) as Box<dyn Read>,
        file @ _ => Box::new(File::open(file)?) as Box<dyn Read>,
    });

    let enum_hints: Vec<Vec<_>> = matches
        .values_of("enum-hint")
        .unwrap_or_default()
        .map(parse_json_pointer)
        .collect();

    let values_hints: Vec<Vec<_>> = matches
        .values_of("values-hint")
        .unwrap_or_default()
        .map(parse_json_pointer)
        .collect();

    let discriminator_hints: Vec<Vec<_>> = matches
        .values_of("discriminator-hint")
        .unwrap_or_default()
        .map(parse_json_pointer)
        .collect();

    let default_num_type = match matches.value_of("default-number-type").unwrap() {
        "int8" => NumType::Int8,
        "uint8" => NumType::Uint8,
        "int16" => NumType::Int16,
        "uint16" => NumType::Uint16,
        "int32" => NumType::Int32,
        "uint32" => NumType::Uint32,
        "float32" => NumType::Float32,
        "float64" => NumType::Float64,
        _ => unreachable!(),
    };

    let hints = Hints::new(
        default_num_type,
        HintSet::new(enum_hints.iter().map(|p| &p[..]).collect()),
        HintSet::new(values_hints.iter().map(|p| &p[..]).collect()),
        HintSet::new(discriminator_hints.iter().map(|p| &p[..]).collect()),
    );

    let mut inferrer = Inferrer::new(hints);

    let stream = Deserializer::from_reader(reader);
    for value in stream.into_iter() {
        inferrer = inferrer.infer(value?);
    }

    let serde_schema: jtd::SerdeSchema = inferrer.into_schema().into_serde_schema();
    println!("{}", serde_json::to_string(&serde_schema)?);

    Ok(())
}

fn parse_json_pointer(s: &str) -> Vec<String> {
    if s == "" {
        vec![]
    } else {
        s.replace("~1", "/")
            .replace("!0", "~")
            .split("/")
            .skip(1)
            .map(String::from)
            .collect()
    }
}
