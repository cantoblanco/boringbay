use chrono::NaiveDate;

use crate::membership_model::Membership;

pub fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn render_route_svg(date: NaiveDate, members: &[Membership]) -> String {
    let stops = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            let y = 230 + index * 62;
            format!(
                r##"<circle cx="105" cy="{y}" r="20" fill="#f5acb9"/><text x="105" y="{text_y}" text-anchor="middle" class="number">{number}</text><text x="145" y="{text_y}" class="stop">{name}</text>"##,
                text_y = y + 8,
                number = index + 1,
                name = escape_xml(&member.name),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 630" role="img" aria-labelledby="title desc">
  <title id="title">无聊湾今日航线 {date}</title>
  <desc id="desc">五站独立博客探索路线</desc>
  <rect width="1200" height="630" rx="42" fill="#fff9fa"/>
  <circle cx="1010" cy="110" r="165" fill="#f5acb9" opacity=".58"/>
  <circle cx="1100" cy="535" r="225" fill="#d0273e" opacity=".09"/>
  <text x="72" y="90" class="eyebrow">BORINGBAY · DAILY ROUTE</text>
  <text x="72" y="158" class="title">无聊湾今日航线</text>
  <text x="74" y="197" class="date">{date} · 五岛巡游</text>
  {stops}
  <g transform="translate(930 190)">
    <circle cx="105" cy="105" r="92" fill="#d0273e"/>
    <circle cx="72" cy="82" r="17" fill="#fff9fa"/><circle cx="138" cy="82" r="17" fill="#fff9fa"/>
    <rect x="60" y="134" width="90" height="17" rx="9" fill="#fff9fa"/>
  </g>
  <text x="1035" y="425" text-anchor="middle" class="challenge">我走完了，你来试试？</text>
  <text x="1035" y="465" text-anchor="middle" class="url">boringbay.com</text>
  <style>
    text{{font-family:system-ui,-apple-system,"Segoe UI",sans-serif;fill:#27181b}}
    .eyebrow{{font-size:22px;font-weight:900;letter-spacing:4px;fill:#d0273e}}
    .title{{font-size:54px;font-weight:900}} .date{{font-size:24px;font-weight:700;fill:#715c61}}
    .number{{font-size:20px;font-weight:900;fill:#541523}} .stop{{font-size:31px;font-weight:850}}
    .challenge{{font-size:25px;font-weight:900}} .url{{font-size:20px;font-weight:800;fill:#d0273e}}
  </style>
</svg>"##
    )
}

pub fn render_member_badge_svg(name: &str, uv: i64, rv: i64, level: i64) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 112" role="img" aria-label="{name} 的无聊湾徽章">
 <rect x="2" y="2" width="616" height="108" rx="54" fill="#fff9fa" stroke="#d0273e" stroke-width="4"/>
 <circle cx="57" cy="56" r="42" fill="#d0273e"/><circle cx="43" cy="46" r="8" fill="#fff9fa"/><circle cx="71" cy="46" r="8" fill="#fff9fa"/><rect x="38" y="70" width="38" height="8" rx="4" fill="#fff9fa"/>
 <text x="118" y="47" class="name">{name}</text><text x="118" y="79" class="stats">UV {uv} · RV {rv} · Lv.{level}</text>
 <text x="578" y="66" text-anchor="end" class="brand">无聊湾</text>
 <style>text{{font-family:system-ui,-apple-system,"Segoe UI",sans-serif;fill:#27181b}}.name{{font-size:27px;font-weight:900}}.stats{{font-size:20px;font-weight:750;fill:#715c61}}.brand{{font-size:22px;font-weight:900;fill:#d0273e}}</style>
</svg>"##,
        name = escape_xml(name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_prevents_markup_injection() {
        assert_eq!(
            escape_xml("<script a=\"b\">&'"),
            "&lt;script a=&quot;b&quot;&gt;&amp;&apos;"
        );
        let badge = render_member_badge_svg("<script>alert(1)</script>", 1, 2, 3);
        assert!(!badge.contains("<script>"));
        assert!(badge.contains("&lt;script&gt;"));
    }
}
