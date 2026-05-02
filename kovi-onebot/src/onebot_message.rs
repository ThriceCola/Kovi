use ahash::HashMap;
use kovi::error::MessageError;
use kovi::message::{Message, Segment};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::ops::Add;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotSegment {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: Value,
}

impl OneBotSegment {
    pub fn new(type_: &str, data: Value) -> Self {
        OneBotSegment {
            type_: type_.to_string(),
            data,
        }
    }
}

impl PartialEq for OneBotSegment {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.data == other.data
    }
}

/// 消息
///
/// **不保证 data 里的 Value 格式是否正确，需要自行检查**
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OneBotMessage(Vec<OneBotSegment>);

impl From<Message> for OneBotMessage {
    fn from(v: Message) -> Self {
        let vec: Vec<OneBotSegment> = v.into_iter().map(OneBotSegment::from).collect();
        Self(vec)
    }
}
impl From<OneBotMessage> for Message {
    fn from(v: OneBotMessage) -> Self {
        let vec: Message = v
            .into_iter()
            .map(|v| Segment {
                kind: v.type_,
                data: v.data,
            })
            .collect::<Vec<Segment>>()
            .into();
        vec
    }
}

impl From<Segment> for OneBotSegment {
    fn from(v: Segment) -> Self {
        Self {
            type_: v.kind,
            data: v.data,
        }
    }
}

impl From<Vec<OneBotSegment>> for OneBotMessage {
    fn from(v: Vec<OneBotSegment>) -> Self {
        OneBotMessage(v)
    }
}

impl From<OneBotMessage> for Vec<OneBotSegment> {
    fn from(v: OneBotMessage) -> Self {
        v.0
    }
}

impl From<&str> for OneBotMessage {
    fn from(v: &str) -> Self {
        OneBotMessage(vec![OneBotSegment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<String> for OneBotMessage {
    fn from(v: String) -> Self {
        OneBotMessage(vec![OneBotSegment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<&String> for OneBotMessage {
    fn from(v: &String) -> Self {
        OneBotMessage(vec![OneBotSegment {
            type_: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl PartialEq for OneBotMessage {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Add for OneBotMessage {
    type Output = OneBotMessage;

    fn add(mut self, rhs: Self) -> Self::Output {
        for seg in rhs.into_iter() {
            self.push(seg);
        }
        self
    }
}

impl OneBotMessage {
    pub fn iter(&self) -> std::slice::Iter<'_, OneBotSegment> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, OneBotSegment> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, s: OneBotSegment) {
        self.0.push(s);
    }
}

impl IntoIterator for OneBotMessage {
    type Item = OneBotSegment;
    type IntoIter = std::vec::IntoIter<OneBotSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Index<usize> for OneBotMessage {
    type Output = OneBotSegment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for OneBotMessage {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl OneBotMessage {
    pub fn from_value(v: Value) -> Result<OneBotMessage, MessageError> {
        if let Some(v) = v.as_array() {
            match OneBotMessage::from_vec_segment_value(v.clone()) {
                Ok(msg) => return Ok(msg),
                Err(err) => return Err(MessageError::ParseError(err.to_string())),
            };
        }
        if let Some(v) = v.as_str() {
            return Ok(OneBotMessage::from(v));
        }

        Err(MessageError::ParseError(
            "Message::from_value only accept array".to_string(),
        ))
    }

    pub fn from_vec_segment_value(v: Vec<Value>) -> Result<OneBotMessage, serde_json::Error> {
        let segments: Result<Vec<OneBotSegment>, serde_json::Error> = v
            .into_iter()
            .map(|value| {
                let segment: OneBotSegment = serde_json::from_value(value)?;
                Ok(segment)
            })
            .collect();

        match segments {
            Ok(segments) => Ok(OneBotMessage(segments)),
            Err(err) => Err(err),
        }
    }
}

pub(crate) fn cq_to_arr_inner(message: &str) -> Vec<serde_json::Value> {
    let cqstr = message.chars().collect::<Vec<char>>();
    let mut text = "".to_owned();
    let mut type_ = "".to_owned();
    let mut val = "".to_owned();
    let mut key = "".to_owned();
    let mut jsonarr: Vec<serde_json::Value> = vec![];
    let mut cqcode: HashMap<String, serde_json::Value> = ahash::HashMap::default();
    let mut stat = 0; //0:text 1:cqcode_type 2:cqcode_key 3:cqcode_val
    let mut i = 0usize;
    while i < cqstr.len() {
        let cur_ch = cqstr[i];
        if stat == 0 {
            if cur_ch == '[' {
                if i + 4 <= cqstr.len() {
                    let t = &cqstr[i..i + 4];
                    if t.starts_with(&['[', 'C', 'Q', ':']) {
                        if !text.is_empty() {
                            let mut node: HashMap<String, serde_json::Value> =
                                ahash::HashMap::default();
                            node.insert("type".to_string(), serde_json::json!("text"));
                            node.insert("data".to_string(), serde_json::json!({"text": text}));
                            jsonarr.push(serde_json::json!(node));
                            text.clear();
                        }
                        stat = 1;
                        i += 3;
                    } else {
                        text.push(cqstr[i]);
                    }
                } else {
                    text.push(cqstr[i]);
                }
            } else if cur_ch == '&' {
                if i + 5 <= cqstr.len() {
                    let t = &cqstr[i..i + 5];
                    if t.starts_with(&['&', '#', '9', '1', ';']) {
                        text.push('[');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '9', '3', ';']) {
                        text.push(']');
                        i += 4;
                    } else if t.starts_with(&['&', 'a', 'm', 'p', ';']) {
                        text.push('&');
                        i += 4;
                    } else {
                        text.push(cqstr[i]);
                    }
                } else {
                    text.push(cqstr[i]);
                }
            } else {
                text.push(cqstr[i]);
            }
        } else if stat == 1 {
            if cur_ch == ',' {
                stat = 2;
            } else if cur_ch == '&' {
                if i + 5 <= cqstr.len() {
                    let t = &cqstr[i..i + 5];
                    if t.starts_with(&['&', '#', '9', '1', ';']) {
                        type_.push('[');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '9', '3', ';']) {
                        type_.push(']');
                        i += 4;
                    } else if t.starts_with(&['&', 'a', 'm', 'p', ';']) {
                        type_.push('&');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '4', '4', ';']) {
                        type_.push(',');
                        i += 4;
                    } else {
                        type_.push(cqstr[i]);
                    }
                } else {
                    type_.push(cqstr[i]);
                }
            } else {
                type_.push(cqstr[i]);
            }
        } else if stat == 2 {
            if cur_ch == '=' {
                stat = 3;
            } else if cur_ch == '&' {
                if i + 5 <= cqstr.len() {
                    let t = &cqstr[i..i + 5];
                    if t.starts_with(&['&', '#', '9', '1', ';']) {
                        key.push('[');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '9', '3', ';']) {
                        key.push(']');
                        i += 4;
                    } else if t.starts_with(&['&', 'a', 'm', 'p', ';']) {
                        key.push('&');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '4', '4', ';']) {
                        key.push(',');
                        i += 4;
                    } else {
                        key.push(cqstr[i]);
                    }
                } else {
                    key.push(cqstr[i]);
                }
            } else {
                key.push(cqstr[i]);
            }
        } else if stat == 3 {
            if cur_ch == ']' {
                let mut node: HashMap<String, serde_json::Value> = ahash::HashMap::default();
                cqcode.insert(key.clone(), serde_json::json!(val));
                node.insert("type".to_string(), serde_json::json!(type_));
                node.insert("data".to_string(), serde_json::json!(cqcode));
                jsonarr.push(serde_json::json!(node));
                key.clear();
                val.clear();
                text.clear();
                type_.clear();
                cqcode.clear();
                stat = 0;
            } else if cur_ch == ',' {
                cqcode.insert(key.clone(), serde_json::json!(val));
                key.clear();
                val.clear();
                stat = 2;
            } else if cur_ch == '&' {
                if i + 5 <= cqstr.len() {
                    let t = &cqstr[i..i + 5];
                    if t.starts_with(&['&', '#', '9', '1', ';']) {
                        val.push('[');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '9', '3', ';']) {
                        val.push(']');
                        i += 4;
                    } else if t.starts_with(&['&', 'a', 'm', 'p', ';']) {
                        val.push('&');
                        i += 4;
                    } else if t.starts_with(&['&', '#', '4', '4', ';']) {
                        val.push(',');
                        i += 4;
                    } else {
                        val.push(cqstr[i]);
                    }
                } else {
                    val.push(cqstr[i]);
                }
            } else {
                val.push(cqstr[i]);
            }
        }
        i += 1;
    }
    if !text.is_empty() {
        let mut node: HashMap<String, serde_json::Value> = ahash::HashMap::default();
        node.insert("type".to_string(), serde_json::json!("text"));
        node.insert("data".to_string(), serde_json::json!({"text": text}));
        jsonarr.push(serde_json::json!(node));
    }
    jsonarr
}
