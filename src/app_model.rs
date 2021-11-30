use std::{collections::HashMap, sync::Arc, time};

use crate::{boring_face::BoringFace, membership_model::Membership, DbPool};

use tokio::sync::RwLock;

pub type DynContext = Arc<Context>;

pub struct Context {
    pub badge: BoringFace,
    pub badge_reverse: BoringFace,
    pub render_cache: RwLock<HashMap<usize, String>>,
    pub render_reverse_cache: RwLock<HashMap<usize, String>>,
    pub db_pool: DbPool,
    pub members: RwLock<Vec<Membership>>,
    pub page_view: RwLock<HashMap<i64, i64>>,
    pub referrer: RwLock<HashMap<i64, i64>>,
    pub domain2id: RwLock<HashMap<String, i64>>,
    pub last_sorted_at: RwLock<time::SystemTime>,
}
