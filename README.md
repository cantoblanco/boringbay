# 无聊湾 · BoringBay

> 无聊人的中继站。发现独立博客，也让独立博客彼此发现。

[线上站点](https://boringbay.com) · [加入无聊湾](https://boringbay.com/join-us) · [排行榜](https://boringbay.com/rank)

无聊湾是一个基于成员互链和匿名访问统计的独立博客联盟。它保留原有的成员列表、UV/RV、等级、排行榜、实时动态与 Badge，同时加入每日航线、公平随机探索、本地护照、漂流瓶文章和分享卡等轻量玩法。

V2 采用渐进增强：关闭功能开关时仍提供原有体验；开启后也不要求注册账号，成员已有的链接和 Badge 无需修改。

## 功能

### 原有能力继续兼容

- 成员站点展示、UV（独立访客）、RV（链入访客）与等级
- 经典总排行、加入流程和实时湾内动态
- `/api/badge/:domain`、`/api/icon/:domain`、`/api/favicon/:domain`
- `/api/ws` WebSocket 接口及现有成员数据、历史统计

### V2 探索体验

- **今日航线**：每天稳定生成同一条五站路线，兼顾主题差异与曝光公平
- **随便逛逛**：优先探索尚未访问、近期曝光较少的站点
- **无聊护照**：访问、收藏、邮戳与成就保存在浏览器本地，无需登录
- **最新漂流瓶**：成员自愿提供 RSS/Atom，只展示标题、短摘要、时间和原文链接
- **可分享路线**：固定日期链接、Open Graph 信息和 SVG 分享卡
- **三类榜单**：30 天活跃榜、7 天上升榜、保留历史意义的经典总榜
- **状态观察**：按最后活动和连续健康检测分为活跃、安静、观察、可移除候选；永不自动删除成员

## 技术栈

- Rust 2021
- Axum + Askama
- Diesel + SQLite
- 服务端渲染，少量原生 ES Modules
- 加法数据库迁移；启动时自动执行待处理迁移

项目默认监听 `0.0.0.0:3000`。

## 本地运行

### 1. 准备环境

- Rust stable（项目已使用 Rust 1.98.1 验证）
- SQLite 由 `libsqlite3-sys` bundled feature 提供，无需单独安装开发库
- Node.js 仅用于浏览器本地护照测试

### 2. 配置并启动

```bash
export SYSTEM_DOMAIN=localhost:3000
export DATABASE_URL=/tmp/boringbay.db
export BORINGBAY_V2_ENABLED=true
export TRUSTED_PROXY_MODE=disabled

cargo run --release
```

浏览器打开 <http://localhost:3000>。首次启动会创建数据库并自动执行全部迁移。

### 环境变量

| 变量 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `SYSTEM_DOMAIN` | 是 | 无 | 对外域名；本地可用 `localhost:3000` |
| `DATABASE_URL` | 是 | 无 | SQLite 文件路径 |
| `BORINGBAY_V2_ENABLED` | 否 | `false` | `true/yes/on/1` 开启 V2，`false/no/off/0` 关闭 |
| `TRUSTED_PROXY_MODE` | 否 | `disabled` | 可选 `disabled` 或 `cloudflare` |

只有服务确实位于 Cloudflare 后方时才应设置 `TRUSTED_PROXY_MODE=cloudflare`。直连部署必须保持 `disabled`，避免信任客户端伪造的 Cloudflare 请求头。

## 加入无聊湾

1. Fork 本仓库。
2. 在 `resources/membership.json` 中使用未占用的数字 ID 添加站点。
3. 将经典 Badge 或 V2 Badge 放到你的网站。
4. 提交 Pull Request。

成员示例：

```json
{
  "123": {
    "domain": "example.com",
    "icon": "https://example.com/icon.png",
    "name": "示例博客",
    "description": "一句话介绍你的站点",
    "github_username": "owner/repository",
    "tags": ["技术", "生活"],
    "feed_url": "https://example.com/feed.xml"
  }
}
```

- `domain` 不要包含协议或路径。
- `tags` 与 `feed_url` 均为可选字段，不填写不会影响成员资格。
- `feed_url` 只接受公开可访问的 HTTPS RSS/Atom 地址；正文不会被镜像，点击始终回到原博客。
- 暂时隐藏站点可使用 `"hidden": true`。

### Badge

经典 Badge（继续兼容）：

```html
<a title="无聊湾 🥱 The Boring Bay" href="https://boringbay.com">
  <img height="18" src="https://boringbay.com/api/badge/example.com" alt="无聊湾">
</a>
```

V2 Badge（可选）：

```html
<a title="无聊湾 🥱 The Boring Bay" href="https://boringbay.com">
  <img height="36" src="https://boringbay.com/api/badge-v2/example.com" alt="无聊湾 V2">
</a>
```

仅展示图标、不记录访问：

```text
https://boringbay.com/api/icon/example.com
https://boringbay.com/api/favicon/example.com
```

## 排行与成员状态

- **30 天活跃榜**：默认榜，按近期 UV/RV 展示真实互动。
- **7 天上升榜**：比较最近 7 天与此前 7 天，让小站和新站也有曝光机会。
- **经典总榜**：保留现有历史累计数据。
- **状态观察**：最后活动同时考虑最后受访和最后链入；30 天安静、60 天且连续检测异常进入观察、90 天且持续异常才成为人工复核候选。

探索、护照、分享事件与正式 UV/RV 分开存储，不会因为完成航线或分享链接而刷榜。任何状态都不会触发自动删除。

## Feed、安全与隐私

- Feed 仅接受成员主动配置的 HTTPS 地址。
- DNS 解析、目标 IP 与每次重定向都会重新校验，拒绝私网、回环、链路本地等地址。
- 最多跟随 2 次重定向；单次响应上限 1 MiB；只接受 XML/RSS/Atom 类型。
- 拒绝 DTD/外部实体，摘要转为纯文本并统一转义。
- Feed、健康检查失败只降级对应模块，不影响首页、排行或 Badge。
- 不记录或通过 WebSocket 广播完整 IP；访客去重使用进程内短期 HMAC 标识，实时动态最多展示国家/地区。

## 测试

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
node --test tests/passport.test.mjs
node --check resources/static/app.js
node --check resources/static/discovery.js
node --check resources/static/passport.js
cargo build --release --locked
```

测试覆盖旧 API 兼容、数据库迁移、三类排行、30/60/90 天状态边界、确定性每日航线、公平随机、Feed 解析与安全清洗、V1/V2 页面和本地护照。

## 部署与升级

生产升级建议：

1. 备份 `DATABASE_URL` 指向的 SQLite 文件。
2. 使用同一份 `migrations/`、`resources/`、`templates/` 与新二进制启动。
3. 先设置 `BORINGBAY_V2_ENABLED=false` 完成旧页面和 API 冒烟测试。
4. 再启用 V2，检查首页、三类排行、今日航线、Badge、Feed 和 WebSocket。
5. 保留上一版二进制与数据库备份，以便快速回滚。

SQLite 在建池前由单连接切换到 WAL；池连接启用外键和 5 秒 busy timeout，降低统计、Feed 与健康任务并发时的锁竞争。

`Dockerfile` 面向 CI 生成的 `artifact/$TARGETPLATFORM/naive`，并把 `migrations/`、`resources/`、`templates/` 复制到 `/webapp`。运行容器时应持久化 `/webapp/data`，并设置上述环境变量。

## 主要路由

| 路由 | 用途 |
| --- | --- |
| `/` | 首页和成员列表 |
| `/rank` | 排行榜和成员状态 |
| `/route/:date` | 指定日期的五站航线 |
| `/join-us` | 加入说明与 Badge 示例 |
| `/api/discovery/today` | 今日航线 JSON |
| `/api/share/route/:date.svg` | 航线 SVG 分享卡 |
| `/api/events` | 白名单匿名产品事件 |
| `/api/ws` | 实时湾内动态 |

## 项目结构

```text
src/                  Rust 应用、排行、探索、Feed、健康与分享逻辑
templates/            Askama 服务端模板
resources/static/     CSS 与原生 JavaScript
resources/membership.json
migrations/           Diesel SQLite 迁移
tests/                兼容、迁移、页面、Feed 与护照测试
docs/superpowers/     V2 设计、风险登记与实施计划
```

详细方案：

- [V2 设计与风险登记](docs/superpowers/specs/2026-09-16-boringbay-v2-design.md)
- [V2 实施计划](docs/superpowers/plans/2026-09-16-boringbay-v2-implementation.md)

## 贡献者

<!--GAMFC_DELIMITER--><a href="https://github.com/naiba" title="naiba"><img src="https://avatars.githubusercontent.com/u/29243953?v=4" width="66;" alt="naiba"/></a>
<a href="https://github.com/cantoblanco" title="Kris"><img src="https://avatars.githubusercontent.com/u/116849421?v=4" width="66;" alt="Kris"/></a>
<a href="https://github.com/SinzMise" title="王九弦SZ·Ninty"><img src="https://avatars.githubusercontent.com/u/120767492?v=4" width="66;" alt="王九弦SZ·Ninty"/></a>
<a href="https://github.com/wulintang" title="懋和道人"><img src="https://avatars.githubusercontent.com/u/17123583?v=4" width="66;" alt="懋和道人"/></a>
<a href="https://github.com/fenychn0206" title="Frederick Chen"><img src="https://avatars.githubusercontent.com/u/120368045?v=4" width="66;" alt="Frederick Chen"/></a>
<a href="https://github.com/diaozhenda" title="JB20CM"><img src="https://avatars.githubusercontent.com/u/99374145?v=4" width="66;" alt="JB20CM"/></a>
<a href="https://github.com/ImMello223" title="Mello"><img src="https://avatars.githubusercontent.com/u/72654383?v=4" width="66;" alt="Mello"/></a>
<a href="https://github.com/MCTGyt" title="MCTGyt"><img src="https://avatars.githubusercontent.com/u/75358424?v=4" width="66;" alt="MCTGyt"/></a>
<a href="https://github.com/WoLeo-Z" title="WoLeo-Z"><img src="https://avatars.githubusercontent.com/u/45914900?v=4" width="66;" alt="WoLeo-Z"/></a>
<a href="https://github.com/ruying-suixing" title="如形"><img src="https://avatars.githubusercontent.com/u/248474335?v=4" width="66;" alt="如形"/></a>
<a href="https://github.com/kokona-shiki" title="Kokona_Shiki"><img src="https://avatars.githubusercontent.com/u/119119641?v=4" width="66;" alt="Kokona_Shiki"/></a>
<a href="https://github.com/chocoboqjj" title="AdeleNaumann"><img src="https://avatars.githubusercontent.com/u/31750441?v=4" width="66;" alt="AdeleNaumann"/></a>
<a href="https://github.com/jipa233" title="jipa233"><img src="https://avatars.githubusercontent.com/u/36941617?v=4" width="66;" alt="jipa233"/></a>
<a href="https://github.com/qmxs" title="gitmba"><img src="https://avatars.githubusercontent.com/u/49803761?v=4" width="66;" alt="gitmba"/></a>
<a href="https://github.com/mcenahle" title="mcenahle"><img src="https://avatars.githubusercontent.com/u/85427807?v=4" width="66;" alt="mcenahle"/></a>
<a href="https://github.com/KyomuDesu" title="Kyomu"><img src="https://avatars.githubusercontent.com/u/86147570?v=4" width="66;" alt="Kyomu"/></a>
<a href="https://github.com/hjh-cn" title="ShSuperyun"><img src="https://avatars.githubusercontent.com/u/71384238?v=4" width="66;" alt="ShSuperyun"/></a>
<a href="https://github.com/xiongbao" title="熊宝"><img src="https://avatars.githubusercontent.com/u/4247191?v=4" width="66;" alt="熊宝"/></a>
<a href="https://github.com/mfeng057" title="枫韵"><img src="https://avatars.githubusercontent.com/u/81139357?v=4" width="66;" alt="枫韵"/></a>
<a href="https://github.com/yzl3014" title="圆周率3014"><img src="https://avatars.githubusercontent.com/u/79385954?v=4" width="66;" alt="圆周率3014"/></a>
<a href="https://github.com/yunzeok" title="yunze"><img src="https://avatars.githubusercontent.com/u/97683172?v=4" width="66;" alt="yunze"/></a>
<a href="https://github.com/cxw521" title="taoziwei"><img src="https://avatars.githubusercontent.com/u/34087208?v=4" width="66;" alt="taoziwei"/></a>
<a href="https://github.com/Catwb" title="qiyao"><img src="https://avatars.githubusercontent.com/u/111560834?v=4" width="66;" alt="qiyao"/></a>
<a href="https://github.com/guangzhoueven" title="guangzhoueven"><img src="https://avatars.githubusercontent.com/u/193498313?v=4" width="66;" alt="guangzhoueven"/></a>
<a href="https://github.com/everfu" title="伍拾柒"><img src="https://avatars.githubusercontent.com/u/74389842?v=4" width="66;" alt="伍拾柒"/></a>
<a href="https://github.com/CasearF" title="Caesar"><img src="https://avatars.githubusercontent.com/u/75901800?v=4" width="66;" alt="Caesar"/></a>
<a href="https://github.com/acanyo" title="Handsome"><img src="https://avatars.githubusercontent.com/u/76821797?v=4" width="66;" alt="Handsome"/></a>
<a href="https://github.com/maolog" title="Mauzhu"><img src="https://avatars.githubusercontent.com/u/17608225?v=4" width="66;" alt="Mauzhu"/></a>
<a href="https://github.com/qindarkstone" title="Qindarkstone"><img src="https://avatars.githubusercontent.com/u/81075299?v=4" width="66;" alt="Qindarkstone"/></a>
<a href="https://github.com/SEYYl" title="SEYYl"><img src="https://avatars.githubusercontent.com/u/76978511?v=4" width="66;" alt="SEYYl"/></a>
<a href="https://github.com/SaboZhang" title="SaboZhang"><img src="https://avatars.githubusercontent.com/u/34998007?v=4" width="66;" alt="SaboZhang"/></a>
<a href="https://github.com/exef-star" title="The hanta"><img src="https://avatars.githubusercontent.com/u/150582140?v=4" width="66;" alt="The hanta"/></a>
<a href="https://github.com/rt265" title="Watermelonabc"><img src="https://avatars.githubusercontent.com/u/59759428?v=4" width="66;" alt="Watermelonabc"/></a>
<a href="https://github.com/xuanxuan666niupi666" title="XCATX"><img src="https://avatars.githubusercontent.com/u/86157698?v=4" width="66;" alt="XCATX"/></a>
<a href="https://github.com/freedom2599" title="freedom2599"><img src="https://avatars.githubusercontent.com/u/148047066?v=4" width="66;" alt="freedom2599"/></a>
<a href="https://github.com/gogei-cn" title="gogei2010"><img src="https://avatars.githubusercontent.com/u/128907557?v=4" width="66;" alt="gogei2010"/></a>
<a href="https://github.com/longchunxin" title="longchunxin"><img src="https://avatars.githubusercontent.com/u/169962016?v=4" width="66;" alt="longchunxin"/></a>
<a href="https://github.com/lylelove" title="lyle"><img src="https://avatars.githubusercontent.com/u/61548984?v=4" width="66;" alt="lyle"/></a>
<a href="https://github.com/miniwater" title="miniwater"><img src="https://avatars.githubusercontent.com/u/14000053?v=4" width="66;" alt="miniwater"/></a>
<a href="https://github.com/nieshilin" title="nieshilin"><img src="https://avatars.githubusercontent.com/u/116864248?v=4" width="66;" alt="nieshilin"/></a>
<a href="https://github.com/sdfghjwmbkdouble" title="sdfghjwmbkdouble"><img src="https://avatars.githubusercontent.com/u/153349302?v=4" width="66;" alt="sdfghjwmbkdouble"/></a>
<a href="https://github.com/sefvcf" title="sefvcf"><img src="https://avatars.githubusercontent.com/u/134713676?v=4" width="66;" alt="sefvcf"/></a>
<a href="https://github.com/spiritLHLS" title="spiritlhl"><img src="https://avatars.githubusercontent.com/u/103393591?v=4" width="66;" alt="spiritlhl"/></a>
<a href="https://github.com/tech-fever" title="tech-fever"><img src="https://avatars.githubusercontent.com/u/105153585?v=4" width="66;" alt="tech-fever"/></a>
<a href="https://github.com/tianhukj" title="tianhukj"><img src="https://avatars.githubusercontent.com/u/166341634?v=4" width="66;" alt="tianhukj"/></a>
<a href="https://github.com/lololowe" title="lololololowe"><img src="https://avatars.githubusercontent.com/u/72289066?v=4" width="66;" alt="lololololowe"/></a>
<a href="https://github.com/furlingdu" title="澪度"><img src="https://avatars.githubusercontent.com/u/117048039?v=4" width="66;" alt="澪度"/></a>
<a href="https://github.com/lcrworld-jtl" title="lcrworld"><img src="https://avatars.githubusercontent.com/u/281657584?v=4" width="66;" alt="lcrworld"/></a>
<a href="https://github.com/kufx" title="kufx"><img src="https://avatars.githubusercontent.com/u/144138627?v=4" width="66;" alt="kufx"/></a>
<a href="https://github.com/it985" title="it985"><img src="https://avatars.githubusercontent.com/u/62421120?v=4" width="66;" alt="it985"/></a>
<a href="https://github.com/imshadow" title="imshadow"><img src="https://avatars.githubusercontent.com/u/23455967?v=4" width="66;" alt="imshadow"/></a>
<a href="https://github.com/hhhkkk520" title="Kris"><img src="https://avatars.githubusercontent.com/u/52115472?v=4" width="66;" alt="Kris"/></a>
<a href="https://github.com/gankudadiz" title="gankudadiz"><img src="https://avatars.githubusercontent.com/u/102597939?v=4" width="66;" alt="gankudadiz"/></a>
<a href="https://github.com/dreamerhe114514" title="dreamerhe114514"><img src="https://avatars.githubusercontent.com/u/156502065?v=4" width="66;" alt="dreamerhe114514"/></a>
<a href="https://github.com/citihu" title="citihu"><img src="https://avatars.githubusercontent.com/u/225527544?v=4" width="66;" alt="citihu"/></a>
<a href="https://github.com/dysf888" title="黑歌"><img src="https://avatars.githubusercontent.com/u/47450409?v=4" width="66;" alt="黑歌"/></a>
<a href="https://github.com/xiaoranawa" title="霄染"><img src="https://avatars.githubusercontent.com/u/112391218?v=4" width="66;" alt="霄染"/></a>
<a href="https://github.com/zaxigia" title="langpa"><img src="https://avatars.githubusercontent.com/u/63903027?v=4" width="66;" alt="langpa"/></a>
<a href="https://github.com/xiangyugongzuoliu" title="翔宇工作流"><img src="https://avatars.githubusercontent.com/u/175918432?v=4" width="66;" alt="翔宇工作流"/></a>
<a href="https://github.com/ysicing" title="缘生"><img src="https://avatars.githubusercontent.com/u/8605565?v=4" width="66;" alt="缘生"/></a>
<a href="https://github.com/xingwangzhe" title="王兴家"><img src="https://avatars.githubusercontent.com/u/162127610?v=4" width="66;" alt="王兴家"/></a>
<a href="https://github.com/xiowo" title="MortalCat"><img src="https://avatars.githubusercontent.com/u/87068069?v=4" width="66;" alt="MortalCat"/></a>
<a href="https://github.com/Elegy17" title="星然♚"><img src="https://avatars.githubusercontent.com/u/24751111?v=4" width="66;" alt="星然♚"/></a>
<a href="https://github.com/wumingblog" title="wumingblog"><img src="https://avatars.githubusercontent.com/u/176279568?v=4" width="66;" alt="wumingblog"/></a>
<a href="https://github.com/kobaridev" title="江晚正愁余ฅ"><img src="https://avatars.githubusercontent.com/u/192551955?v=4" width="66;" alt="江晚正愁余ฅ"/></a>
<a href="https://github.com/mcxiaochenn" title="辰渊尘 ChenDusk"><img src="https://avatars.githubusercontent.com/u/130777336?v=4" width="66;" alt="辰渊尘 ChenDusk"/></a>
<a href="https://github.com/xiaolaji404" title="小垃圾"><img src="https://avatars.githubusercontent.com/u/107843497?v=4" width="66;" alt="小垃圾"/></a>
<a href="https://github.com/Furry-yebai" title="夜白"><img src="https://avatars.githubusercontent.com/u/127961310?v=4" width="66;" alt="夜白"/></a>
<a href="https://github.com/yinzhuoqun" title="yinzhuoqun"><img src="https://avatars.githubusercontent.com/u/12694828?v=4" width="66;" alt="yinzhuoqun"/></a>
<a href="https://github.com/xqk" title="Qiankun Xia"><img src="https://avatars.githubusercontent.com/u/3123993?v=4" width="66;" alt="Qiankun Xia"/></a>
<a href="https://github.com/vxincode" title="xinye"><img src="https://avatars.githubusercontent.com/u/66963380?v=4" width="66;" alt="xinye"/></a>
<a href="https://github.com/xiaoheiyo" title="嘿哟"><img src="https://avatars.githubusercontent.com/u/26519690?v=4" width="66;" alt="嘿哟"/></a>
<a href="https://github.com/xmoieo" title="xMoieo"><img src="https://avatars.githubusercontent.com/u/249294085?v=4" width="66;" alt="xMoieo"/></a>
<a href="https://github.com/wzwzx" title="wzwzx"><img src="https://avatars.githubusercontent.com/u/69845256?v=4" width="66;" alt="wzwzx"/></a>
<a href="https://github.com/tosspi" title="Masone"><img src="https://avatars.githubusercontent.com/u/91527286?v=4" width="66;" alt="Masone"/></a>
<a href="https://github.com/bbb-lsy07" title="bbb-lsy07"><img src="https://avatars.githubusercontent.com/u/183173376?v=4" width="66;" alt="bbb-lsy07"/></a>
<a href="https://github.com/Lafcadia" title="Percival Zheng"><img src="https://avatars.githubusercontent.com/u/147896059?v=4" width="66;" alt="Percival Zheng"/></a>
<a href="https://github.com/Lbb886" title="LBB"><img src="https://avatars.githubusercontent.com/u/107664455?v=4" width="66;" alt="LBB"/></a>
<a href="https://github.com/zhufacai" title="Joker"><img src="https://avatars.githubusercontent.com/u/14821269?v=4" width="66;" alt="Joker"/></a>
<a href="https://github.com/Jochen233" title="Jochen233"><img src="https://avatars.githubusercontent.com/u/89528624?v=4" width="66;" alt="Jochen233"/></a>
<a href="https://github.com/yjh2643408123" title="JianHao Yang"><img src="https://avatars.githubusercontent.com/u/43641046?v=4" width="66;" alt="JianHao Yang"/></a>
<a href="https://github.com/PyXMR2025" title="Jackie"><img src="https://avatars.githubusercontent.com/u/211526759?v=4" width="66;" alt="Jackie"/></a>
<a href="https://github.com/HowieHz" title="Howie Xie"><img src="https://avatars.githubusercontent.com/u/94725606?v=4" width="66;" alt="Howie Xie"/></a>
<a href="https://github.com/HongShi211" title="HongShi211"><img src="https://avatars.githubusercontent.com/u/190166268?v=4" width="66;" alt="HongShi211"/></a>
<a href="https://github.com/listener-He" title="Honesty"><img src="https://avatars.githubusercontent.com/u/39252579?v=4" width="66;" alt="Honesty"/></a>
<a href="https://github.com/TGU-HansJack" title="HansJack"><img src="https://avatars.githubusercontent.com/u/157383592?v=4" width="66;" alt="HansJack"/></a>
<a href="https://github.com/PearsSauce" title="Gil Schneider"><img src="https://avatars.githubusercontent.com/u/56643217?v=4" width="66;" alt="Gil Schneider"/></a>
<a href="https://github.com/Echo846" title="Echo846"><img src="https://avatars.githubusercontent.com/u/238907462?v=4" width="66;" alt="Echo846"/></a>
<a href="https://github.com/Yikoutian1" title="Calyee"><img src="https://avatars.githubusercontent.com/u/90994826?v=4" width="66;" alt="Calyee"/></a>
<a href="https://github.com/Calvert97" title="Calvert Lee"><img src="https://avatars.githubusercontent.com/u/53511886?v=4" width="66;" alt="Calvert Lee"/></a>
<a href="https://github.com/BlueLanM" title="BlueLanM"><img src="https://avatars.githubusercontent.com/u/100191779?v=4" width="66;" alt="BlueLanM"/></a>
<a href="https://github.com/PlayWithAndyJin" title="AndyJin"><img src="https://avatars.githubusercontent.com/u/219800600?v=4" width="66;" alt="AndyJin"/></a>
<a href="https://github.com/linhaii" title="Amitabha"><img src="https://avatars.githubusercontent.com/u/32946306?v=4" width="66;" alt="Amitabha"/></a>
<a href="https://github.com/9527DHX" title="9527DHX"><img src="https://avatars.githubusercontent.com/u/31348749?v=4" width="66;" alt="9527DHX"/></a>
<a href="https://github.com/2022471674" title="2022471674/28.7"><img src="https://avatars.githubusercontent.com/u/177599986?v=4" width="66;" alt="2022471674/28.7"/></a>
<a href="https://github.com/awaae001" title="awaae"><img src="https://avatars.githubusercontent.com/u/108462724?v=4" width="66;" alt="awaae"/></a>
<a href="https://github.com/zlemoni" title="Zlemoni"><img src="https://avatars.githubusercontent.com/u/36426590?v=4" width="66;" alt="Zlemoni"/></a>
<a href="https://github.com/ZhouyiStudio" title="𝖅𝖍𝖔𝖚𝖞𝖎"><img src="https://avatars.githubusercontent.com/u/160443385?v=4" width="66;" alt="𝖅𝖍𝖔𝖚𝖞𝖎"/></a>
<a href="https://github.com/ZebinGao" title="Zebin"><img src="https://avatars.githubusercontent.com/u/46494321?v=4" width="66;" alt="Zebin"/></a>
<a href="https://github.com/akaTiger" title="Yicheng Ding"><img src="https://avatars.githubusercontent.com/u/74623731?v=4" width="66;" alt="Yicheng Ding"/></a>
<a href="https://github.com/Veitzn1" title="Veitzn"><img src="https://avatars.githubusercontent.com/u/158764398?v=4" width="66;" alt="Veitzn"/></a>
<a href="https://github.com/zhyzy" title="Txiao Meng"><img src="https://avatars.githubusercontent.com/u/204500493?v=4" width="66;" alt="Txiao Meng"/></a>
<a href="https://github.com/huang233893" title="Supermini233"><img src="https://avatars.githubusercontent.com/u/69844441?v=4" width="66;" alt="Supermini233"/></a>
<a href="https://github.com/Styunlen" title="Styunlen"><img src="https://avatars.githubusercontent.com/u/30810222?v=4" width="66;" alt="Styunlen"/></a>
<a href="https://github.com/Senc-QWA" title="Senc-QWA"><img src="https://avatars.githubusercontent.com/u/134200418?v=4" width="66;" alt="Senc-QWA"/></a>
<a href="https://github.com/suuseer" title="SeerSu"><img src="https://avatars.githubusercontent.com/u/129711970?v=4" width="66;" alt="SeerSu"/></a>
<a href="https://github.com/zheep1209" title="zheep"><img src="https://avatars.githubusercontent.com/u/148862646?v=4" width="66;" alt="zheep"/></a>
<a href="https://github.com/kuang2714" title="Redcha"><img src="https://avatars.githubusercontent.com/u/149299632?v=4" width="66;" alt="Redcha"/></a>
<a href="https://github.com/rabbitxuanxuan" title="Rabbit_xuan"><img src="https://avatars.githubusercontent.com/u/112363084?v=4" width="66;" alt="Rabbit_xuan"/></a>
<a href="https://github.com/Pstarchen" title="StarChen"><img src="https://avatars.githubusercontent.com/u/102441220?v=4" width="66;" alt="StarChen"/></a>
<a href="https://github.com/Peter267" title="Peter267"><img src="https://avatars.githubusercontent.com/u/175904095?v=4" width="66;" alt="Peter267"/></a>
<a href="https://github.com/xiangleovo" title="OvO"><img src="https://avatars.githubusercontent.com/u/95113433?v=4" width="66;" alt="OvO"/></a>
<a href="https://github.com/lucki-cn" title="L"><img src="https://avatars.githubusercontent.com/u/23611464?v=4" width="66;" alt="L"/></a>
<a href="https://github.com/LixdHappy" title="LixdHappy"><img src="https://avatars.githubusercontent.com/u/54619525?v=4" width="66;" alt="LixdHappy"/></a><!--GAMFC_DELIMITER_END-->

## 致谢

项目最初名为 NAiVe。无聊湾图标修改自熊大的 [The Boring Studio](https://the.boring.studio)。
