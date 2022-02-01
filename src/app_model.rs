use std::fs;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use crate::statistics_model::Statistics;
use crate::{boring_face::BoringFace, DbPool};

use crate::membership_model::Membership;
use anyhow::anyhow;
use chrono::{NaiveDateTime, NaiveTime, Utc};
use headers::HeaderMap;
use tokio::sync::RwLock;
use tracing::info;

pub type DynContext = Arc<Context>;

pub enum VistorType {
    Referrer,
    Badge,
}

pub struct Context {
    pub badge: BoringFace,
    pub favicon: BoringFace,
    pub icon: BoringFace,
    pub badge_render_cache: RwLock<HashMap<usize, String>>,
    pub favicon_render_cache: RwLock<HashMap<usize, String>>,
    pub icon_render_cache: RwLock<HashMap<usize, String>>,

    pub db_pool: DbPool,
    pub page_view: RwLock<HashMap<i64, i64>>,
    pub referrer: RwLock<HashMap<i64, i64>>,

    pub domain2id: HashMap<String, i64>,
    pub id2member: HashMap<i64, Membership>,
}

impl Context {
    pub async fn default(db_pool: DbPool) -> Context {
        let statistics = Statistics::today(db_pool.get().unwrap()).unwrap_or_default();

        let mut page_view: HashMap<i64, i64> = HashMap::new();
        let mut referrer: HashMap<i64, i64> = HashMap::new();

        statistics.iter().for_each(|s| {
            page_view.insert(s.membership_id, s.page_view);
            referrer.insert(s.membership_id, s.referrer);
        });

        let membership: HashMap<i64, Membership> =
            serde_json::from_str(&fs::read_to_string("./resources/membership.json").unwrap())
                .unwrap();
        let mut domain2id: HashMap<String, i64> = HashMap::new();
        membership.iter().for_each(|(k, v)| {
            domain2id.insert(v.domain.clone(), k.clone());
        });

        Context {
            badge: BoringFace::new("#d0273e".to_string(), "#f5acb9".to_string(), true),
            favicon: BoringFace::new("#f5acb9".to_string(), "#d0273e".to_string(), false),
            icon: BoringFace::new("#d0273e".to_string(), "#f5acb9".to_string(), false),
            badge_render_cache: RwLock::new(HashMap::new()),
            favicon_render_cache: RwLock::new(HashMap::new()),
            icon_render_cache: RwLock::new(HashMap::new()),
            db_pool,

            page_view: RwLock::new(page_view),
            referrer: RwLock::new(referrer),

            domain2id: domain2id,
            id2member: membership,
        }
    }

    // 每五分钟存一次，发现隔天刷新
    pub async fn save_per_5_minutes(&self) {
        let mut page_view_cache: HashMap<i64, i64> = HashMap::new();
        let mut referrer_cache: HashMap<i64, i64> = HashMap::new();
        let mut changed_list: Vec<i64> = Vec::new();
        let _today =
            NaiveDateTime::new(Utc::now().date().naive_utc(), NaiveTime::from_hms(0, 0, 0));
        let id_list = Vec::from_iter(self.id2member.keys());
        loop {
            tokio::time::sleep(Duration::from_secs(60 * 5)).await;
            changed_list.clear();
            // 对比是否有数据更新
            let mut page_view_write = self.page_view.write().await;
            let mut referrer_write = self.referrer.write().await;
            id_list.iter().for_each(|id| {
                let pv = page_view_cache.get(id).unwrap_or(&0).clone();
                let new_pv = page_view_write.get(id).unwrap_or(&0).clone();
                if pv.ne(&new_pv) {
                    page_view_cache.insert(id.clone().clone(), new_pv);
                    changed_list.push(id.clone().clone());
                }
                let referrer = referrer_cache.get(id).unwrap_or(&0).clone();
                let new_referrer = referrer_write.get(id).unwrap_or(&0).clone();
                if referrer.ne(&new_referrer) {
                    referrer_cache.insert(id.clone().clone(), new_referrer);
                    if !changed_list.contains(id) {
                        changed_list.push(id.clone().clone());
                    }
                }
            });
            // 更新到数据库
            changed_list.iter().for_each(|id| {
                Statistics::insert_or_update(
                    self.db_pool.get().unwrap(),
                    &Statistics {
                        created_at: _today,
                        updated_at: NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0),
                        membership_id: id.clone(),
                        page_view: page_view_cache.get(id).unwrap().clone(),
                        referrer: referrer_cache.get(id).unwrap().clone(),
                        id: 0,
                    },
                )
                .unwrap();
            });
            // 如果是跨天重置数据
            let new_day =
                NaiveDateTime::new(Utc::now().date().naive_utc(), NaiveTime::from_hms(0, 0, 0));
            if new_day.ne(&_today) {
                page_view_write.clear();
                referrer_write.clear();
                page_view_cache.clear();
                referrer_cache.clear();
            }
            drop(page_view_write);
            drop(referrer_write);
        }
    }

    pub async fn boring_vistor(
        &self,
        v_type: VistorType,
        domain: &str,
        headers: &HeaderMap,
    ) -> Result<i64, anyhow::Error> {
        if let Some(id) = self.domain2id.get(domain) {
            let mut referrer = self.referrer.write().await;
            let mut dist_r = referrer.get(id).or(Some(&0)).unwrap().clone();
            if matches!(v_type, VistorType::Referrer) {
                dist_r = dist_r + 1;
                referrer.insert(id.clone(), dist_r);
            }
            drop(referrer);
            let mut pv = self.page_view.write().await;
            let mut dist_pv = pv.get(id).or(Some(&0)).unwrap().clone();
            if matches!(v_type, VistorType::Badge) {
                dist_pv = dist_pv + 1;
                pv.insert(id.clone(), dist_pv);
            }
            drop(pv);
            let mut tend = (dist_r * 5 + dist_pv) / 20;
            if tend > 10 {
                tend = 10;
            } else if tend < 1 {
                tend = 1
            }

            // TODO 广播访客信息
            let ip =
                String::from_utf8(headers.get("CF-Connecting-IP").unwrap().as_bytes().to_vec())
                    .unwrap();
            info!("ip {}", ip);

            let country =
                String::from_utf8(headers.get("CF-IPCountry").unwrap().as_bytes().to_vec())
                    .unwrap();
            info!("country {}", country);

            return Ok(tend);
        }
        return Err(anyhow!("not a member"));
    }
}
