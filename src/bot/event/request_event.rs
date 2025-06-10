use super::EventBuildError;
use crate::{
    bot::{
        BotInformation,
        plugin_builder::event::{Event, PostType},
    },
    types::ApiAndOneshot,
};
use serde_json::{Value, value::Index};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct RequestEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: PostType,
    /// 请求类型
    pub request_type: String,

    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,
}
impl Event for RequestEvent {
    fn de(json_str: &str, _: &BotInformation, _: &mpsc::Sender<ApiAndOneshot>) -> Option<Self> {
        Self::new(json_str).ok()
    }
}

impl RequestEvent {
    pub(crate) fn new(msg: &str) -> Result<RequestEvent, EventBuildError> {
        let temp: Value =
            serde_json::from_str(msg).map_err(|e| EventBuildError::ParseError(e.to_string()))?;
        let time = temp
            .get("time")
            .and_then(Value::as_i64)
            .ok_or(EventBuildError::ParseError("time".to_string()))?;
        let self_id = temp
            .get("self_id")
            .and_then(Value::as_i64)
            .ok_or(EventBuildError::ParseError("self_id".to_string()))?;
        let post_type = temp
            .get("post_type")
            .and_then(|v| serde_json::from_value::<PostType>(v.clone()).ok())
            .ok_or(EventBuildError::ParseError("Invalid post_type".to_string()))?;
        let request_type = temp
            .get("request_type")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or(EventBuildError::ParseError("request_type".to_string()))?;
        Ok(RequestEvent {
            time,
            self_id,
            post_type,
            request_type,
            original_json: temp,
        })
    }
}

impl RequestEvent {
    /// 直接从原始的 Json Value 获取某值
    ///
    /// # example
    ///
    /// ```rust
    /// use kovi::PluginBuilder;
    ///
    /// PluginBuilder::on_request(|event| async move {
    ///     let time = event.get("time").and_then(|v| v.as_i64()).unwrap();
    ///
    ///     assert_eq!(time, event.time);
    /// });
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        self.original_json.get(index)
    }
}

impl<I> std::ops::Index<I> for RequestEvent
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Self::Output {
        &self.original_json[index]
    }
}
