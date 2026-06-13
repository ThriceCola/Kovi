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
        self.push_text(text);
        self
    }

    /// 消息加上at
    fn add_at(self, user_id: i64) -> Self {
        self.add_mention(user_id)
    }

    /// 消息加上全员at
    fn add_at_all(self) -> Self {
        self.add_mention_all()
    }

    /// 消息加上at
    fn add_mention(mut self, user_id: i64) -> Self {
        self.push_mention(user_id);
        self
    }

    /// 消息加上全员at
    fn add_mention_all(mut self) -> Self {
        self.push_mention_all();
        self
    }

    /// 消息加上引用
    fn add_reply(mut self, message_seq: i64) -> Self {
        self.push_reply(message_seq);
        self
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    ///
    /// 如果需要其它api内容, 请直接构建 segment
    fn add_face(mut self, face_id: String) -> Self {
        self.push_face(face_id);
        self
    }

    /// 消息加上图片
    /// 文件 URI，支持 file:// http(s):// base64:// 三种格式
    ///
    /// 如果需要其它api内容, 请直接构建 segment
    fn add_image(mut self, file_uri: &str) -> Self {
        self.push_image(file_uri);
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
    fn push_at(&mut self, id: i64) {
        self.push_mention(id);
    }

    /// 消息加上全员at
    fn push_at_all(&mut self) {
        self.push_mention_all();
    }

    /// 消息加上at
    fn push_mention(&mut self, id: i64) {
        self.push(Segment {
            kind: "mention".to_string(),
            data: json!({ "user_id": id }),
        });
    }

    /// 消息加上全员at
    fn push_mention_all(&mut self) {
        self.push(Segment {
            kind: "mention_all".to_string(),
            data: json!(null),
        });
    }

    /// 消息加上引用
    fn push_reply(&mut self, message_seq: i64) {
        self.push(Segment {
            kind: "reply".to_string(),
            data: json!({ "message_seq": message_seq }),
        });
    }

    /// 消息加上表情, 具体 id 请看服务端文档, 本框架不提供
    ///
    /// 如果需要其它api内容, 请直接构建 segment
    fn push_face(&mut self, face_id: String) {
        self.push(Segment {
            kind: "face".to_string(),
            data: json!({ "face_id": face_id, "is_large": false }),
        });
    }

    /// 消息加上图片
    ///
    /// 文件 URI，支持 file:// http(s):// base64:// 三种格式
    ///
    /// 如果需要其它api内容, 请直接构建 segment
    fn push_image(&mut self, file_url: &str) {
        self.push(Segment {
            kind: "image".to_string(),
            data: json!({ "uri": file_url, "sub_type": "normal" }),
        });
    }
}

impl MessageRegistrar for Message {
    fn push(&mut self, s: Segment) {
        self.push(s);
    }
}
