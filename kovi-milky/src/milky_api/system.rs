use kovi::bot::runtimebot::{send_api_request_with_forget, send_api_request_with_response, CanSendApi};
use kovi::bot::{ApiReturn, SendApi};
use serde_json::json;

/// System APIs
pub trait MilkySystemApi: CanSendApi {
    /// 获取登录信息
    fn get_login_info(&self) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_login_info", json!({}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取协议端信息
    fn get_impl_info(&self) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_impl_info", json!({}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取用户个人信息
    fn get_user_profile(&self, user_id: i64) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_user_profile", json!({"user_id": user_id}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取好友列表
    fn get_friend_list(&self, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_friend_list", json!({"no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取好友信息
    fn get_friend_info(&self, user_id: i64, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_friend_info", json!({"user_id": user_id, "no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取群列表
    fn get_group_list(&self, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_group_list", json!({"no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取群信息
    fn get_group_info(&self, group_id: i64, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_group_info", json!({"group_id": group_id, "no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取群成员列表
    fn get_group_member_list(&self, group_id: i64, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_group_member_list", json!({"group_id": group_id, "no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取群成员信息
    fn get_group_member_info(&self, group_id: i64, user_id: i64, no_cache: bool) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_group_member_info", json!({"group_id": group_id, "user_id": user_id, "no_cache": no_cache}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取置顶的好友和群列表
    fn get_peer_pins(&self) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_peer_pins", json!({}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 设置好友或群的置顶状态
    fn set_peer_pin(&self, message_scene: &str, peer_id: i64, is_pinned: bool) {
        let send_api = SendApi::new("set_peer_pin", json!({"message_scene": message_scene, "peer_id": peer_id, "is_pinned": is_pinned}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置 QQ 账号头像
    fn set_avatar(&self, uri: &str) {
        let send_api = SendApi::new("set_avatar", json!({"uri": uri}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置 QQ 账号昵称
    fn set_nickname(&self, new_nickname: &str) {
        let send_api = SendApi::new("set_nickname", json!({"new_nickname": new_nickname}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置 QQ 账号个性签名
    fn set_bio(&self, new_bio: &str) {
        let send_api = SendApi::new("set_bio", json!({"new_bio": new_bio}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取自定义表情 URL 列表
    fn get_custom_face_url_list(&self) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_custom_face_url_list", json!({}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取 Cookies
    fn get_cookies(&self, domain: &str) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_cookies", json!({"domain": domain}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 获取 CSRF Token
    fn get_csrf_token(&self) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_csrf_token", json!({}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }
}
