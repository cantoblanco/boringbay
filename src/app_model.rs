use std::{collections::HashMap, sync::Arc, time};

use crate::{boring_face::BoringFace, membership_model::Membership, DbPool};

use tokio::sync::RwLock;
use tracing::info;

pub type DynContext = Arc<Context>;

pub struct Context {
    pub badge: BoringFace,
    pub badge_reverse: BoringFace,
    pub render_cache: RwLock<HashMap<usize, String>>,
    pub render_reverse_cache: RwLock<HashMap<usize, String>>,
    pub db_pool: DbPool,
    pub members: RwLock<Vec<Membership>>,
    pub page_view: RwLock<HashMap<i32, i64>>,
    pub referrer: RwLock<HashMap<i32, i64>>,
    pub last_sorted_at: RwLock<time::SystemTime>,
    pub geoip_db: ipdb::Reader,

    pub id2index: RwLock<HashMap<i32, usize>>,
    pub domain2id: RwLock<HashMap<String, i32>>,
}

impl Context {
    pub async fn boring_vistor(&self, domain: &str, ip: &str) -> i64 {
        let doamin2id = self.domain2id.read().await;
        if let Some(id) = doamin2id.get(domain) {
            let mut referrer = self.referrer.write().await;
            let dist_r = match referrer.get(id) {
                Some(r) => r + 1,
                None => 1,
            };
            referrer.insert(id.clone(), dist_r);
            drop(referrer);
            let mut pv = self.page_view.write().await;
            let dist_pv = match pv.get(id) {
                Some(r) => r + 1,
                None => 1,
            };
            pv.insert(id.clone(), dist_pv);
            drop(pv);
            let mut tend = (dist_r * 5 + dist_pv) / 20;
            if tend > 10 {
                tend = 10;
            } else if tend < 1 {
                tend = 1
            }

            // TODO 广播访客信息
            info!("ip {}", ip);

            return tend;
        }
        0
    }
}
