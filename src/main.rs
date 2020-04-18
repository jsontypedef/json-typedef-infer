use clap::{crate_version, App, AppSettings, Arg};
use failure::Error;
use jtd_infer::{HintSet, Hints, InferredSchema};
use serde_json::Deserializer;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::io::Read;

fn main() -> Result<(), Error> {
    let matches = App::new("jtd-infer")
    .version(crate_version!())
    .about("Infers a JSON Type Definition schema from lines of JSON")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("INPUT")
                .help("Where to read examples from. Dash (hypen) indicates stdin")
                .default_value("-"),
        )
        .arg(
            Arg::with_name("enum-hint")
                .help("Advise the inferrer that the given path points to an enum. If this hint is proven wrong, a type form will be emitted instead. This flag can be provided multiple times.")
                .multiple(true)
                .number_of_values(1)
                .long("enum-hint"),
        )
        .arg(
            Arg::with_name("values-hint")
                .help("Advise the inferrer that the given path points to a values form. If this hint is proven wrong, a properties form will be emitted instead. This flag can be provided multiple times.")
                .multiple(true)
                .number_of_values(1)
                .long("values-hint"),
        )
        .arg(
            Arg::with_name("discriminator-hint")
                .help("Advise the inferrer that the given path points to a discriminator. If this hint is proven wrong, an empty form will be emitted instead. This flag can be provided multiple times.")
                .multiple(true)
                .number_of_values(1)
                .long("discriminator-hint"),
        )
        .get_matches();

    let reader = BufReader::new(match matches.value_of("INPUT").unwrap() {
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
