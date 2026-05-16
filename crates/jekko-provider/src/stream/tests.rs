use super::*;
use bytes::Bytes;

#[test]
fn decodes_simple_frame() {
    let mut d = SseDecoder::new();
    let frames = d.feed(&Bytes::from("event: hello\ndata: world\n\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event, "hello");
    assert_eq!(frames[0].data, "world");
}

#[test]
fn merges_multi_line_data() {
    let mut d = SseDecoder::new();
    let frames = d.feed(&Bytes::from("data: line1\ndata: line2\ndata: line3\n\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].data, "line1\nline2\nline3");
}

#[test]
fn handles_partial_chunks() {
    let mut d = SseDecoder::new();
    assert!(d.feed(&Bytes::from("event: foo\n")).is_empty());
    assert!(d.feed(&Bytes::from("data: bar")).is_empty());
    let frames = d.feed(&Bytes::from("\n\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event, "foo");
    assert_eq!(frames[0].data, "bar");
}

#[test]
fn handles_crlf_terminator() {
    let mut d = SseDecoder::new();
    let frames = d.feed(&Bytes::from("event: foo\r\ndata: bar\r\n\r\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event, "foo");
    assert_eq!(frames[0].data, "bar");
}

#[test]
fn skips_comments() {
    let mut d = SseDecoder::new();
    let frames = d.feed(&Bytes::from(": comment line\ndata: value\n\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].data, "value");
}

#[test]
fn id_and_retry_fields() {
    let mut d = SseDecoder::new();
    let frames = d.feed(&Bytes::from("id: abc\nretry: 5000\ndata: x\n\n"));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].id.as_deref(), Some("abc"));
    assert_eq!(frames[0].retry, Some(5000));
}

#[test]
fn tool_call_aggregator_roundtrip() {
    let mut agg = ToolCallAggregator::new();
    assert_eq!(
        agg.apply(&ProviderEventKind::ToolCallStart {
            id: "tc1".into(),
            name: "Read".into()
        }),
        None
    );
    assert_eq!(
        agg.apply(&ProviderEventKind::ToolCallInputDelta {
            id: "tc1".into(),
            delta: r#"{"path":"/x"}"#.into(),
        }),
        None
    );
    let done = agg.finalize("tc1").unwrap();
    assert_eq!(done.id, "tc1");
    assert_eq!(done.name, "Read");
    assert_eq!(done.input["path"], "/x");
}
