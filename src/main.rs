use anyhow::Error;
use clap::{Parser, ValueEnum};
use jtd_infer::{HintSet, Hints, Inferrer, NumType};
use serde_json::Deserializer;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::io::Read;

#[derive(Parser)]
#[command(name = "jtd-infer", version)]
struct Cli {
    /// Where to read examples from. To read from stdin, use "-"
    #[arg(name = "input", required = true, default_value = "-")]
    input: String,

    /// The default type to infer for JSON numbers.
    #[arg(name = "default-number-type", long, default_value = "uint8")]
    default_number_type: DefaultNumType,

    /// Treat a given part of the input as a discriminator "tag".
    #[arg(name = "discriminator-hint", long)]
    discriminator_hint: Vec<String>,

    /// Treat a given part of the input as an enum.
    #[arg(name = "enum-hint", long)]
    enum_hint: Vec<String>,

    /// Treat a given part of the input as a dictionary / map.
    #[arg(name = "values-hint", long)]
    values_hint: Vec<String>,
}

#[derive(Debug, Clone, ValueEnum)]
enum DefaultNumType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let reader = BufReader::new(match cli.input.as_str() {
        "-" => Box::new(stdin()) as Box<dyn Read>,
        file @ _ => Box::new(File::open(file)?) as Box<dyn Read>,
    });

    let enum_hints: Vec<Vec<_>> = cli
        .enum_hint
        .iter()
        .map(AsRef::as_ref)
        .map(parse_json_pointer)
        .collect();
    let values_hints: Vec<Vec<_>> = cli
        .values_hint
        .iter()
        .map(AsRef::as_ref)
        .map(parse_json_pointer)
        .collect();
    let discriminator_hints: Vec<Vec<_>> = cli
        .discriminator_hint
        .iter()
        .map(AsRef::as_ref)
        .map(parse_json_pointer)
        .collect();
    let default_num_type = cli.default_number_type.into();

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

impl From<DefaultNumType> for NumType {
    fn from(default_num_type: DefaultNumType) -> NumType {
        match default_num_type {
            DefaultNumType::Int8 => NumType::Int8,
            DefaultNumType::Uint8 => NumType::Uint8,
            DefaultNumType::Int16 => NumType::Int16,
            DefaultNumType::Uint16 => NumType::Uint16,
            DefaultNumType::Int32 => NumType::Int32,
            DefaultNumType::Uint32 => NumType::Uint32,
            DefaultNumType::Float32 => NumType::Float32,
            DefaultNumType::Float64 => NumType::Float64,
        }
    }
}
