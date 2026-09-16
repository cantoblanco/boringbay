# BoringBay 公开流量分析与访问修复设计

**状态：** 已批准，由维护者授权自主完成设计、实现、测试并创建 PR  
**日期：** 2026-09-16  
**基线：** `origin/main` 加本地修复提交 `9f7e022`

## 1. 目标

在不要求成员安装新脚本、不保存访客级历史的前提下，把无聊湾现有可观察链路扩展为公开、轻量、可解释的流量分析功能。同时合并并保留以下修复：

- 排行榜兼容历史 `latest_referrer_at = NULL` 数据；
- 首页只展示当天 UV 或 RV 大于 0 的成员；
- 恢复实时访问弹窗；
- 在实时弹窗中展示服务端生成的脱敏 IP 与国家；
- `boringbay.com` 默认延续 Cloudflare 可信代理统计链路。

任何人都可以访问全站分析和任意成员的分析页。分析只描述无聊湾能够可靠观察到的流量，不宣称代表成员博客的全部访问量。

## 2. 统计口径

第一版只记录三类事件：

1. `badge_view`：成员页面加载无聊湾 Badge 时，经现有四小时去重后增加的 UV；
2. `inbound_referral`：访客从成员域名进入无聊湾时，经现有四小时去重后增加的 RV；
3. `outbound_click`：访客从无聊湾点击成员博客、漂流瓶、每日航线或随机探索产生的导出点击。

UV、RV 沿用项目现有语义和去重窗口，避免新旧页面出现两套数字。导出点击不计入 UV/RV 或排行榜。分析页必须显示口径说明和数据起始日期。

“来源”分为两种，不混为一谈：

- 外部来源域名：仅用于可验证的成员链入事件；
- 无聊湾内部入口：`home`、`rank`、`route`、`random`、`feed`、`share`。

不统计设备、浏览器、成员页面路径、完整 Referer URL、访客轨迹或成员博客自身的全部 PV。

## 3. 页面设计

### 3.1 全站总览 `/analytics`

- 今日、7 天、30 天的 UV、RV、导出点击；
- 与前一等长周期相比的变化；
- 7/30/90 天趋势；
- 最近 90 天的小时分布；
- 热门成员与增长成员；
- 国家分布、链入来源域名、无聊湾内部导出入口；
- 最后更新时间、统计口径和隐私规则。

### 3.2 成员分析 `/analytics/{domain}`

- 成员名称、域名、博客入口；
- 今日、7 天、30 天 UV、RV、导出点击与环比；
- 90 天趋势和小时分布；
- 国家、链入来源、导出入口；
- 从成员卡和排行榜返回的稳定公开链接；
- 无数据时显示数据起始日和空状态，不生成虚假趋势。

### 3.3 表现层

继续使用 Askama 服务端渲染和原生 CSS。图表使用语义化 HTML、CSS 柱状图和小型内联 SVG，不引入前端图表框架。页面在 JavaScript 禁用时仍完整可读。移动端图表允许横向滚动，颜色同时配合标签和数值，不能只靠颜色表达。

## 4. 聚合存储架构

新增两个表：

```sql
traffic_hourly(
  bucket_start TIMESTAMP,
  member_id BIGINT,
  event_kind TEXT,
  dimension_kind TEXT,
  dimension_value TEXT,
  count BIGINT,
  PRIMARY KEY(bucket_start, member_id, event_kind, dimension_kind, dimension_value)
)

traffic_daily(
  bucket_date DATE,
  member_id BIGINT,
  event_kind TEXT,
  dimension_kind TEXT,
  dimension_value TEXT,
  count BIGINT,
  PRIMARY KEY(bucket_date, member_id, event_kind, dimension_kind, dimension_value)
)
```

每个计数事件在同一短事务中写入小时表和日表：

- 必写一条 `dimension_kind = total`；
- 有可信国家时写一条 `country`；
- 有可信链入域名时写一条 `referrer_domain`；
- 有内部入口时写一条 `channel`。

维度分别聚合，不存储国家与来源的交叉组合，因此无法从数据库重建单个访客。`member_id = 0` 只用于没有成员目标的全站产品事件；成员分析只查询正数成员 ID。

所有计数使用 SQLite `INSERT ... ON CONFLICT DO UPDATE` 原子累加。维度值进入数据库前必须经过枚举校验或域名规范化；拒绝自由文本、完整 URL、控制字符和超长值。

## 5. 数据流

### 5.1 Badge 与链入

`Context::boring_visitor` 继续负责现有去重和内存计数。只有事件真正使 UV/RV 增加时，才调用聚合服务写入 `badge_view` 或 `inbound_referral`，避免刷新和实时弹窗重复计数。

### 5.2 导出点击

现有 `/api/events` 保持兼容。`EventInput` 增加可选、受限枚举 `channel`。服务端从请求头提取可信国家，但不接收客户端 IP。只有 `member_outbound` 和 `feed_outbound` 映射为 `outbound_click`；其他产品事件仍只进入 `product_events`。

前端为首页、排行榜、路线、随机探索、漂流瓶和分享页提供明确渠道值。缺少渠道的旧客户端仍可记录为 `unknown`，不会返回错误。

### 5.3 查询

新增独立 `analytics` 模块，负责：写入聚合、范围查询、前一周期比较、小样本合并和 90 天清理。路由只组合 ViewModel，不直接拼 SQL。查询只从 `dimension_kind = total` 计算总量，防止把多个维度重复相加。

## 6. 保留、回填与迁移

- 小时表只保留最近 90 天；日表长期保留；
- 日表在事件发生时同步累加，因此删除小时记录不需要再次汇总；
- 每日后台任务删除早于 90 天的小时记录，失败时下次重试；
- 迁移从现有 `statistics` 回填历史日级 `badge_view` 和 `inbound_referral` 总量；
- 迁移从 `product_events` 回填能明确映射的成员导出日总量；
- 历史数据没有国家、来源和小时维度，页面必须标注“详细维度从本版本启用后开始”；
- 回填采用唯一键与幂等 SQL，不修改或重算原表。

## 7. 隐私与实时弹窗

完整 IP 只在当前请求内短暂用于 HMAC 去重和掩码生成，不写日志、数据库或历史聚合。

服务端先把 IP 解析为 `IpAddr`，再生成：

- IPv4：保留首尾段，例如 `203.****.9`；
- IPv6：保留前两段和后两段，中间替换为 `****`。

WebSocket 只发送脱敏 IP、国家、成员和事件类型。浏览器永远收不到完整 IP。实时弹窗显示国家与脱敏 IP，十秒后消失；“湾内动态”仍保留。

公开分析的小样本保护在查询层执行：国家、来源域名和渠道的单项计数小于 3 时不单独展示，统一合并为“其他”。UV、RV、导出点击总量保持精确。接口和模板共用同一隐私函数，不能由前端自行隐藏。

## 8. 错误处理与性能

- 聚合写入失败不能影响 Badge、Icon、首页或跳转响应；记录不含 IP 的结构化警告；
- 所有写入是短事务，沿用 WAL 与 5 秒 busy timeout；
- 分析页查询失败返回 500 通用页面，不回显 SQL/反序列化错误；
- 未知成员返回 404；非法范围回退到 30 天；
- 为时间、成员、事件和维度查询建立覆盖索引；
- SQLite 数据规模达到持续写锁或页面查询超时后再评估 PostgreSQL，本期不迁移数据库。

## 9. 功能开关与兼容性

- 分析路由随现有 `BORINGBAY_V2_ENABLED` 开关启停；关闭时返回 404；
- 原页面、排行榜公式、Badge/Icon/Favicon/WebSocket URL 保持兼容；
- `/api/events` 的旧 JSON 仍有效；
- 数据库迁移只新增表和索引；
- 成员无需修改 Badge 或安装新脚本。

## 10. 测试与验收

### 单元测试

- IPv4、压缩/完整 IPv6、非法 IP 掩码；
- 聚合 upsert、维度规范化、总量不重复计算；
- 小于 3 的桶合并为“其他”；
- 7/30/90 天范围、环比和小时分布；
- 90 天清理边界。

### 集成测试

- 历史 NULL 排行数据三种榜单均为 200；
- 空数据库首页为 0 张成员卡，单个今日活跃成员只显示 1 张；
- Badge/RV 去重后只写一次聚合；
- 旧 `/api/events` 请求继续返回 204，新渠道请求正确聚合；
- `/analytics` 和有效成员页为 200，未知成员为 404，V2 关闭为 404；
- 迁移保留原统计并完成幂等日级回填；
- WebSocket JSON 只含脱敏 IP，不含完整 IP。

### 浏览器与发行验收

- 桌面和移动端总览、成员页、排行榜、首页、实时弹窗；
- 无 JavaScript 时分析页仍可读；
- 真实发行二进制下模拟 Cloudflare Badge、链入和导出点击；
- `cargo fmt --check`、严格 Clippy、全量 Rust/Node 测试、release 构建；
- PR 的 amd64、arm64 和多架构容器检查全部通过后才允许合并。

## 11. 交付边界

本 PR 包含前述回归修复、脱敏 IP 实时弹窗、聚合存储、公开总览、成员分析页、文档和测试。它不包含登录、私有看板、原始事件导出、设备识别、页面级追踪或第三方统计脚本。
