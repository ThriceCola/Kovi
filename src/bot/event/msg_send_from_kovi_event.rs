use crate::ApiReturn;
use crate::bot::BotInformation;
use crate::bot::SendApi;
use crate::bot::event::InternalEvent;
use crate::bot::plugin_builder::event::Event;
use crate::bot::runtimebot::CanSendApi;
use crate::types::ApiAndOneshot;
use tokio::sync::mpsc;

/// 此事件会监听以下消息发送
///
/// "send_msg" => MsgSendFromKoviType::SendMsg
/// "send_private_msg" => MsgSendFromKoviType::SendPrivateMsg
/// "send_group_msg" => MsgSendFromKoviType::SendGroupMsg
/// "send_forward_msg" => MsgSendFromKoviType::SendForwardMsg
/// "send_private_forward_msg" => MsgSendFromKoviType::SendPrivateForwardMsg
/// "send_group_forward_msg" => MsgSendFromKoviType::SendGroupForwardMsg
#[derive(Debug, Clone)]
pub struct MsgSendFromKoviEvent {
    /// 事件类型
    pub event_type: MsgSendFromKoviType,
    /// 发送消息的API内容
    pub send_api: SendApi,
    /// 发送消息的API响应结果
    pub res: Result<ApiReturn, ApiReturn>,

    /// 不推荐的消息发送方式
    pub api_tx: mpsc::Sender<ApiAndOneshot>,
}
#[derive(Debug, Copy, Clone)]
pub enum MsgSendFromKoviType {
    SendMsg,
    SendPrivateMsg,
    SendGroupMsg,
    SendForwardMsg,
    SendPrivateForwardMsg,
    SendGroupForwardMsg,
}

impl TryFrom<String> for MsgSendFromKoviType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "send_msg" => Ok(MsgSendFromKoviType::SendMsg),
            "send_private_msg" => Ok(MsgSendFromKoviType::SendPrivateMsg),
            "send_group_msg" => Ok(MsgSendFromKoviType::SendGroupMsg),
            "send_forward_msg" => Ok(MsgSendFromKoviType::SendForwardMsg),
            "send_private_forward_msg" => Ok(MsgSendFromKoviType::SendPrivateForwardMsg),
            "send_group_forward_msg" => Ok(MsgSendFromKoviType::SendGroupForwardMsg),
            _ => Err(format!("Invalid MsgSendFromKoviType: {}", value)),
        }
    }
}
impl TryFrom<&str> for MsgSendFromKoviType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "send_msg" => Ok(MsgSendFromKoviType::SendMsg),
            "send_private_msg" => Ok(MsgSendFromKoviType::SendPrivateMsg),
            "send_group_msg" => Ok(MsgSendFromKoviType::SendGroupMsg),
            "send_forward_msg" => Ok(MsgSendFromKoviType::SendForwardMsg),
            "send_private_forward_msg" => Ok(MsgSendFromKoviType::SendPrivateForwardMsg),
            "send_group_forward_msg" => Ok(MsgSendFromKoviType::SendGroupForwardMsg),
            _ => Err(format!("Invalid MsgSendFromKoviType: {}", value)),
        }
    }
}
impl TryFrom<&String> for MsgSendFromKoviType {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "send_msg" => Ok(MsgSendFromKoviType::SendMsg),
            "send_private_msg" => Ok(MsgSendFromKoviType::SendPrivateMsg),
            "send_group_msg" => Ok(MsgSendFromKoviType::SendGroupMsg),
            "send_forward_msg" => Ok(MsgSendFromKoviType::SendForwardMsg),
            "send_private_forward_msg" => Ok(MsgSendFromKoviType::SendPrivateForwardMsg),
            "send_group_forward_msg" => Ok(MsgSendFromKoviType::SendGroupForwardMsg),
            _ => Err(format!("Invalid MsgSendFromKoviType: {}", value)),
        }
    }
}

impl Event for MsgSendFromKoviEvent {
    fn de(
        event: &InternalEvent,
        _: &BotInformation,
        api_tx: &mpsc::Sender<ApiAndOneshot>,
    ) -> Option<Self> {
        let InternalEvent::OneBotApiEvent((send_api, res)) = event else {
            return None;
        };

        let Ok(event_type) = MsgSendFromKoviType::try_from(&send_api.action) else {
            return None;
        };

        Some(Self::new(
            event_type,
            send_api.clone(),
            res.clone(),
            api_tx.clone(),
        ))
    }
}

impl MsgSendFromKoviEvent {
    fn new(
        event_type: MsgSendFromKoviType,
        send_api: SendApi,
        res: Result<ApiReturn, ApiReturn>,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) -> MsgSendFromKoviEvent {
        MsgSendFromKoviEvent {
            event_type,
            send_api,
            res,
            api_tx,
        }
    }
}

impl CanSendApi for MsgSendFromKoviEvent {
    fn __get_api_tx(&self) -> &tokio::sync::mpsc::Sender<crate::types::ApiAndOneshot> {
        &self.api_tx
    }
}
