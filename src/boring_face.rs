static SVG_HEADER: &str = r###"<?xml version="1.0" encoding="utf-8"?>
<svg #svg_viewport# xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
        text {
            fill: #fill_black#;
            font-weight: 900 !important;
            font-family: system-ui, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, Arial, sans-serif;
        }
        .fill-black {
            fill: #fill_black#
        }
        .fill-white {
            fill: #fill_white#
        }
        @media (prefers-color-scheme: dark) {
            text {
                fill: #fill_white#
            }
            .fill-black {
                fill: #fill_white#
            }
            .fill-white {
                fill: #fill_black#
            }
        }
    </style>
  </defs>
  #svg_border#
  <g transform="matrix(1, 0, 0, 1, 1.062999, 0.155999)" fill="none" fill-rule="evenodd">
    <circle class="fill-black" cx="54.5" cy="54.5" r="48.5"/>
    <g class="fill-white" transform="translate(19.587 30.779)">
      <circle cx="14.861" cy="13.861" r="8.861"/>
      <circle cx="54.966" cy="13.861" r="8.861"/>
    </g>
    <rect class="fill-white" x="31.5" y="71.2" width="47" height="8.529" rx="5"/>
"###;
static SVG_BORDER: &str = r###"
<rect class="fill-white" width="577.762" height="110" style="stroke: rgb(189, 14, 43); stroke-width: 5;" rx="50"/>
<text style="font-size: 38px; white-space: pre; text-anchor: middle;" x="335" y="48.074">#site_name# 🥱</text>
<text style="font-size: 24px; white-space: pre; font-style: italic; text-anchor: middle;" x="335" y="91.615">UV: #site_uv#  Referrer: #site_referrer#  Level: #site_rank#</text>
"###;
static SVG_FOOTER: &str = r###"
    </g>
  
</svg>"###;

pub struct BoringFace {
    fill_white: String,
    fill_black: String,
    with_border: bool,
}

impl BoringFace {
    pub fn new(fill_white: String, fill_black: String, with_border: bool) -> Self {
        Self {
            fill_white,
            fill_black,
            with_border,
        }
    }

    pub fn render_svg(&self, name: &str, uv: i64, rv: i64, rank: i64) -> String {
        assert!(rank >= 1 && rank <= 10);
        let mut content = SVG_HEADER
            .replace("#fill_white#", &self.fill_white)
            .replace("#fill_black#", &self.fill_black)
            .to_string();

        for i in 0..10 {
            content.push_str(&format!(
                "<rect {} x=\"{}\" y=\"72.5\" width=\"3\" height=\"6\" rx=\"1\" />",
                match i.lt(&rank) {
                    true => "class=\"fill-black\"",
                    false => "fill=\"#d55f6f\"",
                },
                35.5 + (i as f64) * 4f64
            ));
        }

        content.push_str(&SVG_FOOTER);

        content = content.replace(
            "#svg_viewport#",
            match self.with_border {
                true => "viewBox=\"-3.412 -2.5 584.261 115.604\"",
                _ => "viewBox=\"6 6.1 98 97\"",
            },
        );

        if self.with_border {
            content = content.replace(
                "#svg_border#",
                &SVG_BORDER
                    .replace("#site_name#", name)
                    .replace("#site_uv#", &uv.to_string())
                    .replace("#site_referrer#", &rv.to_string())
                    .replace("#site_rank#", &rank.to_string()),
            );
        }

        content
    }
}
