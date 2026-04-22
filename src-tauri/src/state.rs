use std::collections::HashMap;

use crate::models::Config;
use crate::session::Session;

pub struct WorkmanState {
    pub config: Config,
    pub sessions: HashMap<String, Session>,
}
