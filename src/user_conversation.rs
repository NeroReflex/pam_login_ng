use crate::user::User;


pub struct UserConversation {
    user: User
}

impl UserConversation {
    pub fn new(user: User) -> Self {
        Self {
            user
        }
    }
}

