use std::fs;

use xdf::{Format, Sample, Values, XDFFile};
use xmltree::{Element, XMLNode};

// The goal here is just to have something resembling a simple app that prints stats about the file.
// This is mostly to ensure correct visibility. Checking correctness of values is done in a [different test](./read_files.rs).
// To this end I put each type from the crate into its own print function.
// This way, the type must be visible in order to be specified as a parameter.

#[test]
fn print_stats_app() {
    // read minimal.xdf
    let file_path = "tests/read/minimal.xdf";
    let bytes = fs::read(file_path).unwrap();
    let xdf_file = XDFFile::from_bytes(&bytes).unwrap();

    println!("XDF Version: {}", xdf_file.version);

    let mut header_str_bytes = Vec::new();
    xdf_file.header.write(&mut header_str_bytes).unwrap();

    let header_string = String::from_utf8(header_str_bytes).unwrap();

    println!("File header: {}", header_string);

    for stream in xdf_file.streams {
        print_stream(stream);
    }
}

fn print_stream(stream: xdf::Stream) {
    let id = stream.id;
    let channel_count = stream.channel_count;
    let nominal_srate = stream.nominal_srate.map_or("None".to_string(), |f| f.to_string());
    let format = stream.format;
    let name = stream.name.unwrap_or_default();
    let content_type = stream.content_type.unwrap_or("No content type".to_string());
    let measured_srate = stream.measured_srate.map_or("None".to_string(), |f| f.to_string());

    let header_tags = stream.header.children.iter().flat_map(XMLNode::as_element);
    let header_keys = header_tags.clone().cloned().map(|e| e.name);
    let header_values = header_tags.flat_map(Element::get_text);
    let header_content = header_keys.zip(header_values);

    let footer = stream.footer.unwrap();
    let footer_tags = footer.children.iter().flat_map(XMLNode::as_element);
    let footer_keys = footer_tags.clone().cloned().map(|e| e.name);
    let footer_values = footer_tags.flat_map(Element::get_text);
    let footer_content = footer_keys.zip(footer_values);

    let samples = stream.samples;

    println!("\n\n=== Printing stream with name \"{name}\" ===");
    println!("ID: {id}");

    println!("channel_count: {channel_count}");
    println!("nominal_srate: {nominal_srate}");
    print_format(format);
    println!("content_type: {content_type}");
    println!("measured_srate: {measured_srate}");

    println!();

    println!("Header tag names:");
    header_content.for_each(|(k, v)| println!("\t{k: >15}: {v}"));
    println!();

    println!("Footer tag names:");
    footer_content.for_each(|(k, v)| println!("\t{k: >15}: {v}"));
    println!();

    println!("\nSamples:");
    samples.iter().for_each(print_sample);
    println!("========================");
}

fn print_format(format: Format) {
    println!("Format: {format}")
}
fn print_sample(sample: &Sample) {
    let timestamp = sample.timestamp.map_or("None".to_string(), |f| f.to_string());
    println!("Timestamp: {timestamp}");

    print_values(&sample.values);
}

fn print_values(values: &Values) {
    println!("Values: {values:?}");
}
