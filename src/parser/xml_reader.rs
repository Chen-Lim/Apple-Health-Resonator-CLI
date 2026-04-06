use std::io::BufRead;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

pub enum XmlEntity {
    Record(Vec<(String, String)>),
    Workout(Vec<(String, String)>),
}

pub struct XmlStream<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

impl<R: BufRead> XmlStream<R> {
    pub fn new(source: R) -> Self {
        let mut xml_reader = Reader::from_reader(source);
        xml_reader.config_mut().trim_text(true);
        Self {
            reader: xml_reader,
            buf: Vec::with_capacity(8 * 1024),
        }
    }

    pub fn next_entity(&mut self) -> Result<Option<XmlEntity>> {
        loop {
            match self.reader.read_event_into(&mut self.buf)? {
                Event::Start(ref start) if start.name().as_ref() == b"Record" => {
                    return Ok(Some(XmlEntity::Record(attributes(start, &self.reader)?)));
                }
                Event::Empty(ref start) if start.name().as_ref() == b"Record" => {
                    return Ok(Some(XmlEntity::Record(attributes(start, &self.reader)?)));
                }
                Event::Start(ref start) if start.name().as_ref() == b"Workout" => {
                    return Ok(Some(XmlEntity::Workout(attributes(start, &self.reader)?)));
                }
                Event::Empty(ref start) if start.name().as_ref() == b"Workout" => {
                    return Ok(Some(XmlEntity::Workout(attributes(start, &self.reader)?)));
                }
                Event::Eof => return Ok(None),
                _ => {}
            }
            self.buf.clear();
        }
    }
}

fn attributes<R: BufRead>(
    start: &BytesStart<'_>,
    reader: &Reader<R>,
) -> Result<Vec<(String, String)>> {
    let mut attrs = Vec::new();
    for attr in start.attributes().with_checks(false) {
        let attr = attr?;
        let key = std::str::from_utf8(attr.key.as_ref())?.to_string();
        let value = attr
            .decode_and_unescape_value(reader.decoder())?
            .into_owned();
        attrs.push((key, value));
    }
    Ok(attrs)
}
