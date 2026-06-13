use serde::Serialize;
use serde_json::{Value, json};
use std::fmt::Display;

use kovi::{Message, Segment};

pub trait MessageRegistrar: Sized {
    fn push(&mut self, s: Segment);

    /// 在消息加上文字
    fn add_text<T>(mut self, text: T) -> Self
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
    fn add_at(mut self, id: &str) -> Self {
        self.push(Segment {
            kind: "at".to_string(),
            data: json!({ "qq": id }),
        });
        self
    }

    /// 消息加上at
    fn add_mention(mut self, id: &str) -> Self {
        self.push(Segment {
            kind: "at".to_string(),
            data: json!({ "qq": id }),
        });
        self
    }

    /// 消息加上引用
    fn add_reply(mut self, message_id: i32) -> Self {
        self.push(Segment {
            kind: "reply".to_string(),
            data: json!({ "id": message_id.to_string() }),
        });
        self
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    fn add_face(mut self, id: i64) -> Self {
        self.push(Segment {
            kind: "face".to_string(),
            data: json!({ "id": id.to_string() }),
        });
        self
    }

    /// 消息加上图片
    fn add_image(mut self, file: &str) -> Self {
        self.push(Segment {
            kind: "image".to_string(),
            data: json!({ "file": file }),
        });
        self
    }

    /// 消息加上 segment
    fn add_segment<T>(mut self, segment: T) -> Self
    where
        Value: From<T>,
        T: Serialize,
    {
        let value = Value::from(segment);
        if let Ok(segment) = serde_json::from_value(value) {
            self.push(segment);
        }
        self
    }

    /// 在消息加上文字
    fn push_text<T>(&mut self, text: T)
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
    fn push_at(&mut self, id: &str) {
        self.push(Segment {
            kind: "at".to_string(),
            data: json!({ "qq": id }),
        });
    }

    /// 消息加上引用
    fn push_reply(&mut self, message_id: i32) {
        self.push(Segment {
            kind: "reply".to_string(),
            data: json!({ "id": message_id.to_string() }),
        });
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    fn push_face(&mut self, id: i64) {
        self.push(Segment {
            kind: "face".to_string(),
            data: json!({ "id": id.to_string() }),
        });
    }

    /// 消息加上图片
    fn push_image(&mut self, file: &str) {
        self.push(Segment {
            kind: "image".to_string(),
            data: json!({ "file": file }),
        });
    }
}

impl MessageRegistrar for Message {
    fn push(&mut self, s: Segment) {
        self.push(s);
    }
}
