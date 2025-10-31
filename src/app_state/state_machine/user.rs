use serde::{Deserialize, Serialize};

use super::super::log::{Command, ToCommand};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

impl ToCommand for CreateUserRequest {
    fn to_command(&self) -> Command {
        Command::AddUser {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}
