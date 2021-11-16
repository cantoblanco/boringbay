use std::{collections::HashMap, sync::Arc};

use crate::boring_face::BoringFace;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use tokio::sync::RwLock;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub type DynContext = Arc<Context>;

pub struct Context {
    pub badge: BoringFace,
    pub badge_reverse: BoringFace,
    pub render_cache: RwLock<HashMap<usize, String>>,
}
