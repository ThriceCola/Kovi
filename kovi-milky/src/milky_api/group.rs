use kovi::bot::runtimebot::{
    CanSendApi, send_api_request_with_forget, send_api_request_with_response,
};
use kovi::bot::{ApiReturn, SendApi};
use serde_json::json;

/// Group APIs
pub trait MilkyGroupApi: CanSendApi {
    /// 设置群名称
    fn set_group_name(&self, group_id: i64, new_group_name: &str) {
        let send_api = SendApi::new(
            "set_group_name",
            json!({"group_id": group_id, "new_group_name": new_group_name}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群头像
    fn set_group_avatar(&self, group_id: i64, image_uri: &str) {
        let send_api = SendApi::new(
            "set_group_avatar",
            json!({"group_id": group_id, "image_uri": image_uri}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群名片
    fn set_group_member_card(&self, group_id: i64, user_id: i64, card: &str) {
        let send_api = SendApi::new(
            "set_group_member_card",
            json!({"group_id": group_id, "user_id": user_id, "card": card}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群成员专属头衔
    fn set_group_member_special_title(&self, group_id: i64, user_id: i64, special_title: &str) {
        let send_api = SendApi::new(
            "set_group_member_special_title",
            json!({"group_id": group_id, "user_id": user_id, "special_title": special_title}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群管理员
    fn set_group_member_admin(&self, group_id: i64, user_id: i64, is_set: bool) {
        let send_api = SendApi::new(
            "set_group_member_admin",
            json!({"group_id": group_id, "user_id": user_id, "is_set": is_set}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群成员禁言
    fn set_group_member_mute(&self, group_id: i64, user_id: i64, duration: i32) {
        let send_api = SendApi::new(
            "set_group_member_mute",
            json!({"group_id": group_id, "user_id": user_id, "duration": duration}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 设置群全员禁言
    fn set_group_whole_mute(&self, group_id: i64, is_mute: bool) {
        let send_api = SendApi::new(
            "set_group_whole_mute",
            json!({"group_id": group_id, "is_mute": is_mute}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 踢出群成员
    fn kick_group_member(&self, group_id: i64, user_id: i64, reject_add_request: bool) {
        let send_api = SendApi::new(
            "kick_group_member",
            json!({"group_id": group_id, "user_id": user_id, "reject_add_request": reject_add_request}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取群公告列表
    fn get_group_announcements(
        &self,
        group_id: i64,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new("get_group_announcements", json!({"group_id": group_id}));
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 发送群公告
    fn send_group_announcement(&self, group_id: i64, content: &str, image_uri: Option<&str>) {
        let mut params = json!({"group_id": group_id, "content": content});
        if let Some(uri) = image_uri {
            params["image_uri"] = json!(uri);
        }
        let send_api = SendApi::new("send_group_announcement", params);
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 删除群公告
    fn delete_group_announcement(&self, group_id: i64, announcement_id: &str) {
        let send_api = SendApi::new(
            "delete_group_announcement",
            json!({"group_id": group_id, "announcement_id": announcement_id}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取群精华消息列表
    fn get_group_essence_messages(
        &self,
        group_id: i64,
        page_index: i32,
        page_size: i32,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let send_api = SendApi::new(
            "get_group_essence_messages",
            json!({"group_id": group_id, "page_index": page_index, "page_size": page_size}),
        );
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 设置群精华消息
    fn set_group_essence_message(&self, group_id: i64, message_seq: i64, is_set: bool) {
        let send_api = SendApi::new(
            "set_group_essence_message",
            json!({"group_id": group_id, "message_seq": message_seq, "is_set": is_set}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 退出群
    fn quit_group(&self, group_id: i64) {
        let send_api = SendApi::new("quit_group", json!({"group_id": group_id}));
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 发送群消息表情回应
    fn send_group_message_reaction(
        &self,
        group_id: i64,
        message_seq: i64,
        reaction: &str,
        reaction_type: &str,
        is_add: bool,
    ) {
        let send_api = SendApi::new(
            "send_group_message_reaction",
            json!({"group_id": group_id, "message_seq": message_seq, "reaction": reaction, "reaction_type": reaction_type, "is_add": is_add}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 发送群戳一戳
    fn send_group_nudge(&self, group_id: i64, user_id: i64) {
        let send_api = SendApi::new(
            "send_group_nudge",
            json!({"group_id": group_id, "user_id": user_id}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 获取群通知列表
    fn get_group_notifications(
        &self,
        start_notification_seq: Option<i64>,
        is_filtered: bool,
        limit: i32,
    ) -> impl std::future::Future<Output = Result<ApiReturn, ApiReturn>> {
        let mut params = json!({"is_filtered": is_filtered, "limit": limit});
        if let Some(seq) = start_notification_seq {
            params["start_notification_seq"] = json!(seq);
        }
        let send_api = SendApi::new("get_group_notifications", params);
        send_api_request_with_response(self.__get_api_tx(), send_api)
    }

    /// 同意入群/邀请他人入群请求
    fn accept_group_request(
        &self,
        notification_seq: i64,
        notification_type: &str,
        group_id: i64,
        is_filtered: bool,
    ) {
        let send_api = SendApi::new(
            "accept_group_request",
            json!({"notification_seq": notification_seq, "notification_type": notification_type, "group_id": group_id, "is_filtered": is_filtered}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 拒绝入群/邀请他人入群请求
    fn reject_group_request(
        &self,
        notification_seq: i64,
        notification_type: &str,
        group_id: i64,
        is_filtered: bool,
        reason: Option<&str>,
    ) {
        let mut params = json!({"notification_seq": notification_seq, "notification_type": notification_type, "group_id": group_id, "is_filtered": is_filtered});
        if let Some(r) = reason {
            params["reason"] = json!(r);
        }
        let send_api = SendApi::new("reject_group_request", params);
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 同意他人邀请自身入群
    fn accept_group_invitation(&self, group_id: i64, invitation_seq: i64) {
        let send_api = SendApi::new(
            "accept_group_invitation",
            json!({"group_id": group_id, "invitation_seq": invitation_seq}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }

    /// 拒绝他人邀请自身入群
    fn reject_group_invitation(&self, group_id: i64, invitation_seq: i64) {
        let send_api = SendApi::new(
            "reject_group_invitation",
            json!({"group_id": group_id, "invitation_seq": invitation_seq}),
        );
        send_api_request_with_forget(self.__get_api_tx(), send_api);
    }
}
