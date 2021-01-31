use clap::{crate_version, load_yaml, App, AppSettings};
use failure::Error;
use jtd_infer::{HintSet, Hints, InferredSchema};
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

    let hints = Hints::new(
        HintSet::new(enum_hints.iter().map(|p| &p[..]).collect()),
        HintSet::new(values_hints.iter().map(|p| &p[..]).collect()),
        HintSet::new(discriminator_hints.iter().map(|p| &p[..]).collect()),
    );

    let mut inference = InferredSchema::Unknown;

    let stream = Deserializer::from_reader(reader);
    for value in stream.into_iter() {
        inference = inference.infer(value?, &hints);
    }

    let serde_schema: jtd::SerdeSchema = inference.into_schema().into();
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
