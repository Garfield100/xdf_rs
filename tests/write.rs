use xdf::writer::{HasMetadataAndDesc, HasTimestamps, StreamInfo, XDFBuilder};

#[test]
fn write_simple() {
    let builder = XDFBuilder::new();
    let mut buffer = Vec::new();
    let mut writer = builder
        .build(&mut buffer)
        .expect("failed to create XDFWriter from builder");

    let marker_stream_info = StreamInfo::new(1, None);
    let marker_stream_builder = writer.add_stream::<&str, HasTimestamps>(marker_stream_info);

    let marker_stream = marker_stream_builder
        .name("Cue")
        .content_type("Markers")
        .add_metadata_key("hostname", "Test")
        .start_stream()
        .expect("could not create marker stream from its builder");

    todo!("Write some markers to the stream and check the output")
}
