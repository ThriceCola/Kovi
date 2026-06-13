use kovi::bot::runtimebot::{send_api_request_with_forget, send_api_request_with_response, CanSendApi};
use kovi::bot::{ApiReturn, SendApi};
use serde_json::json;
use crate::milky_message::MilkyMessage;

/// Message APIs
pub trait MilkyMessageApi: CanSendApi {
    /// 发送私聊消息
    fn send_private_message(
        &self,
        user_id: i64,
        message: MilkyMessage,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "send_private_message",
            json!({"user_id": user_id, "message": message}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 发送群聊消息
    fn send_group_message(
        &self,
        group_id: i64,
        message: MilkyMessage,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "send_group_message",
            json!({"group_id": group_id, "message": message}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 撤回私聊消息
    fn recall_private_message(&self, user_id: i64, message_seq: i64) {
        let send_api = SendApi::new(
            "recall_private_message",
            json!({"user_id": user_id, "message_seq": message_seq}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 撤回群聊消息
    fn recall_group_message(&self, group_id: i64, message_seq: i64) {
        let send_api = SendApi::new(
            "recall_group_message",
            json!({"group_id": group_id, "message_seq": message_seq}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取消息
    fn get_message(
        &self,
        message_scene: &str,
        peer_id: i64,
        message_seq: i64,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "get_message",
            json!({"message_scene": message_scene, "peer_id": peer_id, "message_seq": message_seq}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取历史消息列表
    fn get_history_messages(
        &self,
        message_scene: &str,
        peer_id: i64,
        start_message_seq: Option<i64>,
        limit: i32,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let mut params = json!({"message_scene": message_scene, "peer_id": peer_id, "limit": limit});
        if let Some(seq) = start_message_seq {
            params["start_message_seq"] = json!(seq);
        }
        let send_api = SendApi::new("get_history_messages", params);
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取临时资源链接
    fn get_resource_temp_url(
        &self,
        resource_id: &str,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "get_resource_temp_url",
            json!({"resource_id": resource_id}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取合并转发消息内容
    fn get_forwarded_messages(
        &self,
        forward_id: &str,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "get_forwarded_messages",
            json!({"forward_id": forward_id}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 标记消息为已读
    fn mark_message_as_read(&self, message_scene: &str, peer_id: i64, message_seq: i64) {
        let send_api = SendApi::new(
            "mark_message_as_read",
            json!({"message_scene": message_scene, "peer_id": peer_id, "message_seq": message_seq}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }
}
