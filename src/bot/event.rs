use serde_json::Value;

use crate::bot::BotInformation;
use crate::types::{ApiAndOptOneshot, ApiAndRuturn};
use std::any::Any;

/// 满足此 trait 即可在Kovi运行时中监听并处理
///
/// # Warning
///
/// 最好不要阻塞解析事件，如果目标信息需要阻塞获取，请通知用户由用户处理，而非由事件解析器阻塞
///
/// 在 Kovi 0.12.4 之后，事件是并发解析的，所以阻塞解析事件并不会阻塞其他事件的解析，虽然如此，仍然不建议阻塞解析事件
pub trait Event: Any + Send + Sync {
    /// 解析事件
    ///
    /// 传入三个东西，按需所取。
    ///  - InternalEvent 内部消息，包含OneBot消息与由框架发出去的Api消息
    ///  - 借用的bot信息，可以通过 `BotInformation` 获取 `Bot` 相关的信息，例如管理员是谁。
    ///  - 借用的api发送通道，可以通过 `api_tx.clone()` 来让事件可以发送 api
    ///
    /// 如果认为此 json 不符合事件要求，请返回 `None`。
    ///
    /// 在一个消息周期内，Kovi 运行时会缓存此事件。
    ///
    /// 不需要的信息用 `_` 忽略，例如：
    ///
    /// ```ignore
    /// 
    /// impl Event for LifecycleEvent {
    ///     fn de(
    ///         event: &InternalEvent,
    ///         _: &BotInformation,
    ///         _: &tokio::sync::mpsc::Sender<ApiAndOneshot>,
    ///     ) -> Option<Self>
    ///     where
    ///         Self: Sized,
    ///     {
    ///         let InternalEvent::OneBotEvent(json_str) = event else {
    ///             return None;
    ///         };
    ///         let event: LifecycleEvent = serde_json::from_str(json_str).ok()?;
    ///         if event.meta_event_type == "lifecycle" {
    ///             Some(event)
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Warning
    ///
    /// 最好不要阻塞解析事件，如果目标信息需要阻塞获取，请通知用户由用户处理，而非由事件解析器阻塞
    ///
    /// 在 Kovi 0.12.4 之后，事件是并发解析的，所以阻塞解析事件并不会阻塞其他事件的解析，虽然如此，仍然不建议阻塞解析事件
    ///
    /// 可以使用类似于 `MsgSendFromKoviEvent` 的实现，将所需的交给用户就行。
    ///
    /// ```ignore
    /// 
    /// pub struct MsgSendFromKoviEvent {
    ///     pub event_type: MsgSendFromKoviType,
    ///     pub send_api: SendApi,
    ///     pub res: Result<ApiReturn, ApiReturn>,
    /// }
    /// ```
    fn de(
        event: &InternalEvent,
        bot_info: &BotInformation,
        api_tx: &tokio::sync::mpsc::Sender<ApiAndOptOneshot>,
    ) -> Option<Self>
    where
        Self: Sized;
}

/// 事件
pub enum InternalEvent {
    /// 来自OneBot的事件
    OneBotEvent(Value),
    /// 来自Kovi发送给服务端并包含了返回结果
    OneBotApiEvent(ApiAndRuturn),
}
