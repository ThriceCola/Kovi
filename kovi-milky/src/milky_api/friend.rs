use kovi::bot::runtimebot::{send_api_request_with_forget, send_api_request_with_response, CanSendApi};
use kovi::bot::{ApiReturn, SendApi};
use serde_json::json;

/// Friend APIs
pub trait MilkyFriendApi: CanSendApi {
    /// 发送好友戳一戳
    fn send_friend_nudge(&self, user_id: i64, is_self: bool) {
        let send_api = SendApi::new(
            "send_friend_nudge",
            json!({"user_id": user_id, "is_self": is_self}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 发送名片点赞
    fn send_profile_like(&self, user_id: i64, count: i32) {
        let send_api = SendApi::new(
            "send_profile_like",
            json!({"user_id": user_id, "count": count}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 删除好友
    fn delete_friend(&self, user_id: i64) {
        let send_api = SendApi::new("delete_friend", json!({"user_id": user_id}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取好友请求列表
    fn get_friend_requests(
        &self,
        limit: i32,
        is_filtered: bool,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "get_friend_requests",
            json!({"limit": limit, "is_filtered": is_filtered}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 同意好友请求
    fn accept_friend_request(&self, initiator_uid: &str, is_filtered: bool) {
        let send_api = SendApi::new(
            "accept_friend_request",
            json!({"initiator_uid": initiator_uid, "is_filtered": is_filtered}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 拒绝好友请求
    fn reject_friend_request(&self, initiator_uid: &str, is_filtered: bool, reason: Option<&str>) {
        let mut params = json!({"initiator_uid": initiator_uid, "is_filtered": is_filtered});
        if let Some(r) = reason {
            params["reason"] = json!(r);
        }
        let send_api = SendApi::new("reject_friend_request", params);
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }
}
