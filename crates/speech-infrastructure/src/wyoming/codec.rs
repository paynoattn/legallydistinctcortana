use std::io::{self, BufRead, Read, Write};

use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

const PROTOCOL_VERSION: &str = "1.5.4";
const MAX_HEADER_BYTES: u64 = 64 * 1024;
const MAX_DATA_BYTES: usize = 1024 * 1024;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

/// One Wyoming event: a JSON header line, optional JSON data, optional binary payload.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Event {
    pub(crate) kind: String,
    pub(crate) data: Map<String, Value>,
    pub(crate) payload: Vec<u8>,
}

#[derive(Debug, Error)]
pub(crate) enum CodecError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("malformed event: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event section of {0} bytes is over the size limit")]
    TooLarge(usize),
}

#[derive(Deserialize)]
struct Header {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    data: Map<String, Value>,
    #[serde(default)]
    data_length: usize,
    #[serde(default)]
    payload_length: usize,
}

pub(crate) fn write_event(writer: &mut impl Write, event: &Event) -> io::Result<()> {
    let data_bytes = serde_json::to_vec(&event.data)?;
    let payload = event.payload.as_slice();
    let mut header = Map::new();
    header.insert("type".into(), Value::from(event.kind.as_str()));
    header.insert("version".into(), Value::from(PROTOCOL_VERSION));
    header.insert("data_length".into(), Value::from(data_bytes.len()));
    if !payload.is_empty() {
        header.insert("payload_length".into(), Value::from(payload.len()));
    }
    let mut line = serde_json::to_vec(&header)?;
    line.push(b'\n');
    writer.write_all(&line)?;
    writer.write_all(&data_bytes)?;
    writer.write_all(payload)?;
    writer.flush()
}

/// The next event, or `None` at a clean end of stream.
pub(crate) fn read_event(reader: &mut impl BufRead) -> Result<Option<Event>, CodecError> {
    let mut line = String::new();
    if reader
        .by_ref()
        .take(MAX_HEADER_BYTES)
        .read_line(&mut line)?
        == 0
    {
        return Ok(None);
    }
    let header: Header = serde_json::from_str(&line)?;
    let mut data = header.data;
    if header.data_length > 0 {
        let bytes = read_section(reader, header.data_length, MAX_DATA_BYTES)?;
        data.extend(serde_json::from_slice::<Map<String, Value>>(&bytes)?);
    }
    let payload = read_section(reader, header.payload_length, MAX_PAYLOAD_BYTES)?;
    Ok(Some(Event {
        kind: header.kind,
        data,
        payload,
    }))
}

fn read_section(
    reader: &mut impl Read,
    length: usize,
    limit: usize,
) -> Result<Vec<u8>, CodecError> {
    if length > limit {
        return Err(CodecError::TooLarge(length));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;

    use super::*;

    fn data(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    fn encode(kind: &str, value: &Value, payload: &[u8]) -> Vec<u8> {
        let event = Event {
            kind: kind.into(),
            data: data(value),
            payload: payload.to_vec(),
        };
        let mut bytes = Vec::new();
        write_event(&mut bytes, &event).unwrap();
        bytes
    }

    #[test]
    fn round_trips_data_and_payload() {
        let bytes = encode("audio-chunk", &json!({"rate": 22050}), &[1, 2, 3]);
        let event = read_event(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(event.kind, "audio-chunk");
        assert_eq!(event.data, data(&json!({"rate": 22050})));
        assert_eq!(event.payload, [1, 2, 3]);
    }

    #[test]
    fn writes_the_documented_header_line() {
        let bytes = encode("synthesize", &json!({"text": "Hi"}), &[]);
        let text = String::from_utf8(bytes).unwrap();
        let (header, rest) = text.split_once('\n').unwrap();
        let header: Value = serde_json::from_str(header).unwrap();
        assert_eq!(
            header,
            json!({"type": "synthesize", "version": "1.5.4", "data_length": 13})
        );
        assert_eq!(rest, r#"{"text":"Hi"}"#);
    }

    #[test]
    fn reads_inline_data_and_merges_separate_data() {
        let mut bytes =
            br#"{"type":"audio-start","data":{"rate":16000},"data_length":14}"#.to_vec();
        bytes.extend(b"\n{\"channels\":1}");
        let event = read_event(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(event.data, data(&json!({"rate": 16000, "channels": 1})));
        assert!(event.payload.is_empty(), "no payload was announced");
    }

    #[test]
    fn end_of_stream_is_none_and_garbage_is_an_error() {
        assert!(
            read_event(&mut Cursor::new(Vec::new())).unwrap().is_none(),
            "a closed stream has no event"
        );
        let garbage = read_event(&mut Cursor::new(b"not json\n".to_vec())).unwrap_err();
        assert!(
            garbage.to_string().starts_with("malformed event:"),
            "{garbage}"
        );
    }

    #[test]
    fn accepts_a_header_line_longer_than_a_kilobyte() {
        let filler = "a".repeat(5_000);
        let line = format!(r#"{{"type":"x","data":{{"k":"{filler}"}}}}"#) + "\n";
        let event = read_event(&mut Cursor::new(line.into_bytes()))
            .unwrap()
            .unwrap();
        assert_eq!(event.data, data(&json!({"k": filler})));
    }

    #[test]
    fn accepts_a_data_section_of_exactly_the_limit() {
        let one_mebibyte = 1024 * 1024;
        let filler = "a".repeat(one_mebibyte - 8);
        let body = format!(r#"{{"k":"{filler}"}}"#);
        assert_eq!(body.len(), one_mebibyte);
        let header = format!(r#"{{"type":"x","data_length":{MAX_DATA_BYTES}}}"#) + "\n";
        let mut bytes = header.into_bytes();
        bytes.extend(body.as_bytes());
        let event = read_event(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(event.data, data(&json!({"k": filler})));
    }

    #[test]
    fn refuses_a_data_section_one_byte_over_the_limit() {
        let header = format!(r#"{{"type":"x","data_length":{}}}"#, MAX_DATA_BYTES + 1) + "\n";
        let error = read_event(&mut Cursor::new(header.into_bytes())).unwrap_err();
        assert!(matches!(error, CodecError::TooLarge(_)), "{error}");
    }

    #[test]
    fn accepts_payloads_of_two_mebibytes_and_up_to_the_limit() {
        for length in [2 * 1024 * 1024, MAX_PAYLOAD_BYTES] {
            let header = format!(r#"{{"type":"x","payload_length":{length}}}"#) + "\n";
            let mut bytes = header.into_bytes();
            bytes.resize(bytes.len() + length, 7);
            let event = read_event(&mut Cursor::new(bytes)).unwrap().unwrap();
            assert_eq!(event.payload.len(), length);
        }
    }

    #[test]
    fn refuses_oversized_sections_and_short_reads() {
        let huge = format!(
            "{{\"type\":\"x\",\"payload_length\":{}}}\n",
            MAX_PAYLOAD_BYTES + 1
        );
        let error = read_event(&mut Cursor::new(huge.into_bytes())).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "event section of {} bytes is over the size limit",
                MAX_PAYLOAD_BYTES + 1
            )
        );
        let short = b"{\"type\":\"x\",\"payload_length\":4}\n12".to_vec();
        assert!(
            matches!(read_event(&mut Cursor::new(short)), Err(CodecError::Io(_))),
            "a truncated payload is an I/O error"
        );
    }
}
