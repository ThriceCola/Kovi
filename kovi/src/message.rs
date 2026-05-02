use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::ops::Add;

use crate::error::MessageError;

pub mod add;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub kind: String,
    pub data: Value,
}

impl Segment {
    pub fn new(type_: &str, data: Value) -> Self {
        Segment {
            kind: type_.to_string(),
            data,
        }
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.data == other.data
    }
}

/// 消息
///
/// **不保证 data 里的 Value 格式是否正确，需要自行检查**
///
/// # Examples
/// ```
/// use kovi::bot::message::Message;
/// use serde_json::json;
///
/// let msg: Message = Message::from("Hi");
/// let msg: Message = Message::from_value(json!(
///     [
///         {
///             "type":"text",
///             "data":{
///                 "text":"Some msg"
///             }
///         }
///     ]
/// )).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message(Vec<Segment>);

impl From<Vec<Segment>> for Message {
    fn from(v: Vec<Segment>) -> Self {
        Message(v)
    }
}

impl From<Message> for Vec<Segment> {
    fn from(v: Message) -> Self {
        v.0
    }
}

impl From<&str> for Message {
    fn from(v: &str) -> Self {
        Message(vec![Segment {
            kind: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<String> for Message {
    fn from(v: String) -> Self {
        Message(vec![Segment {
            kind: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl From<&String> for Message {
    fn from(v: &String) -> Self {
        Message(vec![Segment {
            kind: "text".to_string(),
            data: json!({
                "text":v,
            }),
        }])
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Add for Message {
    type Output = Message;

    fn add(mut self, rhs: Self) -> Self::Output {
        for seg in rhs.into_iter() {
            self.push(seg);
        }
        self
    }
}

impl Message {
    pub fn iter(&self) -> std::slice::Iter<'_, Segment> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Segment> {
        self.0.iter_mut()
    }
}

impl IntoIterator for Message {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Index<usize> for Message {
    type Output = Segment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Message {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Message {
    pub fn from_value(v: Value) -> Result<Message, MessageError> {
        if let Some(v) = v.as_array() {
            match Message::from_vec_segment_value(v.clone()) {
                Ok(msg) => return Ok(msg),
                Err(err) => return Err(MessageError::ParseError(err.to_string())),
            };
        }
        if let Some(v) = v.as_str() {
            return Ok(Message::from(v));
        }

        Err(MessageError::ParseError(
            "Message::from_value only accept array".to_string(),
        ))
    }

    pub fn from_vec_segment_value(v: Vec<Value>) -> Result<Message, serde_json::Error> {
        let segments: Result<Vec<Segment>, serde_json::Error> = v
            .into_iter()
            .map(|value| {
                let segment: Segment = serde_json::from_value(value)?;
                Ok(segment)
            })
            .collect();

        match segments {
            Ok(segments) => Ok(Message(segments)),
            Err(err) => Err(err),
        }
    }

    /// Message 解析成人类可读字符串, 会将里面的 segment 转换成 `[type]` 字符串，如： image segment 会转换成 `[image]` 字符串。不要靠此函数做判断，可能不同版本会改变内容。
    pub fn to_human_string(&self) -> String {
        let mut result = String::new();

        for item in self.iter() {
            match item.kind.as_str() {
                "text" => {
                    if let Some(text_data) = item.data.get("text")
                        && let Some(text_str) = text_data.as_str()
                    {
                        result.push_str(text_str);
                    }
                }
                _ => {
                    result.push_str(&format!("[{}]", item.kind));
                }
            }
        }
        result
    }

    pub fn get_from_index(&self, index: usize) -> Option<&Segment> {
        self.0.get(index)
    }

    pub fn get_mut_from_index(&mut self, index: usize) -> Option<&mut Segment> {
        self.0.get_mut(index)
    }
}

impl Message {
    /// 返回空的 Message
    pub fn new() -> Message {
        Default::default()
    }

    /// 检查 Message 是否包含任意一项 segment 。返回 bool。
    ///
    /// # Examples
    /// ```
    /// use kovi::bot::message::Message;
    /// use serde_json::json;
    ///
    /// let msg1: Message = Message::from("Hi");
    /// let msg2: Message = Message::from_value(json!(
    ///     [
    ///         {
    ///             "type":"text",
    ///             "data":{
    ///                 "text":"Some msg"
    ///             }
    ///         }
    ///     ]
    /// )).unwrap();
    ///
    /// assert!(msg1.contains("text"));
    /// assert!(msg2.contains("text"));
    pub fn contains(&self, s: &str) -> bool {
        self.iter().any(|item| item.kind == s)
    }

    /// 获取 Message 任意一种 segment 。返回 `Vec<Value>`，有多少项，就会返回多少项。
    ///
    /// # Examples
    /// ```
    /// use kovi::bot::message::Segment;
    /// use kovi::bot::message::Message;
    /// use serde_json::{json, Value};
    ///
    /// let msg: Message = Message::from_value(json!(
    ///     [
    ///         {
    ///             "type":"text",
    ///             "data":{
    ///                 "text":"Some msg"
    ///             }
    ///         },
    ///         {
    ///             "type":"face",
    ///             "data":{
    ///                 "id":"0"
    ///             }
    ///         },
    ///     ]
    /// )).unwrap();
    ///
    /// let text_value: Segment = Segment::new("text", json!({"text": "Some msg"}));
    /// let face_value: Segment = Segment::new("face", json!({"id": "0"}));
    /// assert_eq!(msg.get("text")[0], text_value);
    /// assert_eq!(msg.get("face")[0], face_value);
    pub fn get(&self, s: &str) -> Vec<Segment> {
        self.iter().filter(|item| item.kind == s).cloned().collect()
    }
}

#[test]
fn check_msg() {
    let msg: Message = Message::from_value(json!(
        [
            {
                "type":"text",
                "data":{
                    "text":"Some msg"
                }
            },
            {
                "type":"face",
                "data":{
                    "id":"0"
                }
            },
        ]
    ))
    .unwrap();
    let text_value: Segment = serde_json::from_value(json!({
        "type":"text",
        "data":{
            "text":"Some msg"
        }
    }))
    .unwrap();
    let face_value: Segment = serde_json::from_value(json!({
        "type":"face",
        "data":{
            "id":"0"
        }
    }))
    .unwrap();
    assert_eq!(msg.get("text")[0], text_value);
    assert_eq!(msg.get("face")[0], face_value);

    let msg1: Message = Message::from("Hi");
    let msg2: Message = Message::from_value(json!(
        [
            {
                "type":"text",
                "data":{
                    "text":"Some msg"
                }
            }
        ]
    ))
    .unwrap();
    assert!(msg1.contains("text"));
    assert!(msg2.contains("text"));
}
