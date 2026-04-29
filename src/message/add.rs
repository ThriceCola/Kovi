use serde::Serialize;
use serde_json::{Value, json};
use std::fmt::Display;

use super::{Message, Segment};

impl Message {
    /// 在消息加上文字
    pub fn add_text<T>(mut self, text: T) -> Self
    where
        String: From<T>,
        T: Serialize + Display,
    {
        self.push(Segment {
            kind: "text".to_string(),
            data: json!({ "text": text }),
        });
        self
    }

    /// 消息加上at
    pub fn add_at(mut self, id: &str) -> Self {
        self.0.push(Segment {
            kind: "at".to_string(),
            data: json!({ "qq": id }),
        });
        self
    }

    /// 消息加上引用
    pub fn add_reply(mut self, message_id: i32) -> Self {
        self.0.insert(
            0,
            Segment {
                kind: "reply".to_string(),
                data: json!({ "id": message_id.to_string() }),
            },
        );
        self
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    pub fn add_face(mut self, id: i64) -> Self {
        self.0.push(Segment {
            kind: "face".to_string(),
            data: json!({ "id": id.to_string() }),
        });
        self
    }

    /// 消息加上图片
    pub fn add_image(mut self, file: &str) -> Self {
        self.0.push(Segment {
            kind: "image".to_string(),
            data: json!({ "file": file }),
        });
        self
    }

    /// 消息加上 segment
    pub fn add_segment<T>(mut self, segment: T) -> Self
    where
        Value: From<T>,
        T: Serialize,
    {
        let value = Value::from(segment);
        if let Ok(segment) = serde_json::from_value(value) {
            self.0.push(segment);
        }
        self
    }
}

impl Message {
    /// 在消息加上文字
    pub fn push_text<T>(&mut self, text: T)
    where
        String: From<T>,
        T: Serialize + Display,
    {
        self.push(Segment {
            kind: "text".to_string(),
            data: json!({ "text": text }),
        });
    }

    /// 消息加上at
    pub fn push_at(&mut self, id: &str) {
        self.0.push(Segment {
            kind: "at".to_string(),
            data: json!({ "qq": id }),
        });
    }

    /// 消息加上引用
    pub fn push_reply(&mut self, message_id: i32) {
        self.0.insert(
            0,
            Segment {
                kind: "reply".to_string(),
                data: json!({ "id": message_id.to_string() }),
            },
        );
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    pub fn push_face(&mut self, id: i64) {
        self.0.push(Segment {
            kind: "face".to_string(),
            data: json!({ "id": id.to_string() }),
        });
    }

    /// 消息加上图片
    pub fn push_image(&mut self, file: &str) {
        self.0.push(Segment {
            kind: "image".to_string(),
            data: json!({ "file": file }),
        });
    }

    pub fn push(&mut self, s: Segment) {
        self.0.push(s);
    }
}
