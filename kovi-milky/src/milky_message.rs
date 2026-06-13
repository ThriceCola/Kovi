use kovi::error::MessageError;
use kovi::message::{Message, Segment as KoviSegment};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::ops::Add;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Value,
}

impl Segment {
    pub fn new(type_: &str, data: Value) -> Self {
        Segment {
            type_: type_.to_string(),
            data,
        }
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.data == other.data
    }
}

/// 消息
///
/// **不保证 data 里的 Value 格式是否正确，需要自行检查**
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MilkyMessage(Vec<Segment>);

impl From<Message> for MilkyMessage {
    fn from(v: Message) -> Self {
        let vec: Vec<Segment> = v.into_iter().map(Segment::from).collect();
        Self(vec)
    }
}
impl From<MilkyMessage> for Message {
    fn from(v: MilkyMessage) -> Self {
        let vec: Message = v
            .into_iter()
            .map(|v| KoviSegment {
                kind: v.type_,
                data: v.data,
            })
            .collect::<Vec<KoviSegment>>()
            .into();
        vec
    }
}

impl From<KoviSegment> for Segment {
    fn from(v: KoviSegment) -> Self {
        Self {
            type_: v.kind,
            data: v.data,
        }
    }
}

impl From<Vec<Segment>> for MilkyMessage {
    fn from(v: Vec<Segment>) -> Self {
        MilkyMessage(v)
    }
}

impl From<MilkyMessage> for Vec<Segment> {
    fn from(v: MilkyMessage) -> Self {
        v.0
    }
}

impl From<&str> for MilkyMessage {
    fn from(v: &str) -> Self {
        MilkyMessage(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<String> for MilkyMessage {
    fn from(v: String) -> Self {
        MilkyMessage(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<&String> for MilkyMessage {
    fn from(v: &String) -> Self {
        MilkyMessage(vec![Segment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl PartialEq for MilkyMessage {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Add for MilkyMessage {
    type Output = MilkyMessage;

    fn add(mut self, rhs: Self) -> Self::Output {
        for seg in rhs.into_iter() {
            self.push(seg);
        }
        self
    }
}

impl MilkyMessage {
    pub fn iter(&self) -> std::slice::Iter<'_, Segment> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Segment> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, s: Segment) {
        self.0.push(s);
    }
}

impl IntoIterator for MilkyMessage {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Index<usize> for MilkyMessage {
    type Output = Segment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for MilkyMessage {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl MilkyMessage {
    pub fn from_value(v: Value) -> Result<MilkyMessage, MessageError> {
        if let Some(v) = v.as_array() {
            match MilkyMessage::from_vec_segment_value(v.clone()) {
                Ok(msg) => return Ok(msg),
                Err(err) => return Err(MessageError::ParseError(err.to_string())),
            };
        }
        if let Some(v) = v.as_str() {
            return Ok(MilkyMessage::from(v));
        }

        Err(MessageError::ParseError(
            "MilkyMessage::from_value only accept array".to_string(),
        ))
    }

    pub fn from_vec_segment_value(v: Vec<Value>) -> Result<MilkyMessage, serde_json::Error> {
        let segments: Result<Vec<Segment>, serde_json::Error> = v
            .into_iter()
            .map(|value| {
                let segment: Segment = serde_json::from_value(value)?;
                Ok(segment)
            })
            .collect();

        match segments {
            Ok(segments) => Ok(MilkyMessage(segments)),
            Err(err) => Err(err),
        }
    }
}
