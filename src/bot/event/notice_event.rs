use crate::{
    bot::{
        BotInformation,
        event::InternalEvent,
        plugin_builder::event::{Event, PostType},
    },
    error::EventBuildError,
    types::ApiAndOneshot,
};
use serde_json::{Value, value::Index};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct NoticeEvent {
    /// 事件发生的时间戳
    pub time: i64,
    /// 收到事件的机器人 登陆号
    pub self_id: i64,
    /// 上报类型
    pub post_type: PostType,
    /// 通知类型
    pub notice_type: String,

    /// 原始的onebot消息，已处理成json格式
    pub original_json: Value,
}
impl Event for NoticeEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        _: &mpsc::Sender<ApiAndOneshot>,
    ) -> Option<Self> {
        let InternalEvent::OneBotEvent(json_str) = event else {
            return None;
        };
        Self::new(json_str).ok()
    }
}

impl NoticeEvent {
    pub(crate) fn new(msg: &str) -> Result<NoticeEvent, EventBuildError> {
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
        let notice_type = temp
            .get("notice_type")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or(EventBuildError::ParseError("notice_type".to_string()))?;
        Ok(NoticeEvent {
            time,
            self_id,
            post_type,
            notice_type,
            original_json: temp,
        })
    }
}

impl NoticeEvent {
    /// 直接从原始的 Json Value 获取某值
    ///
    /// # example
    ///
    /// ```ignore
    /// use kovi::PluginBuilder;
    ///
    /// PluginBuilder::on_notice(|event| async move {
    ///     let time = event.get("time").and_then(|v| v.as_i64()).unwrap();
    ///
    ///     assert_eq!(time, event.time);
    /// });
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        self.original_json.get(index)
    }
}

impl<I> std::ops::Index<I> for NoticeEvent
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Self::Output {
        &self.original_json[index]
    }
}
