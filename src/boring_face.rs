static SVG_HEADER: &str = r###"<svg #svg_viewport# xmlns="http://www.w3.org/2000/svg">
    #svg_border#
    <g transform="translate(.38)" fill="none" fill-rule="evenodd">
        <circle class="fill-black" cx="54.5" cy="54.5" r="48.5" />
        <g class="fill-white" transform="translate(19.587 30.779)">
            <circle cx="14.861" cy="13.861" r="8.861" />
            <circle cx="54.966" cy="13.861" r="8.861" />
        </g>
        <rect class="fill-white" x="31.5" y="71.2" width="47" height="8.529" rx="5" />"###;
static SVG_BORDER: &str = r###"
<rect class="fill-white" width="711.217" height="110" style="stroke:#bd0e2b; stroke-width:5" rx="50"></rect>
<text x="120" y="78">The Boring Bay 🥱</text>
"###;
static SVG_FOOTER: &str = r###"
    </g>
    <style>
        text {
            fill: #fill_black#;
            font-weight: 900 !important;
            font-family: system-ui, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, Arial, sans-serif;
            font-size: 4.2rem;
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

    pub fn render_svg(&self, length: usize) -> String {
        assert!(length >= 1 && length <= 10);
        let mut content = SVG_HEADER.to_string();

        for i in 0..10 {
            content.push_str(&format!(
                "<rect {} x=\"{}\" y=\"72.5\" width=\"3\" height=\"6\" rx=\"1\" />",
                match i.lt(&length) {
                    true => "class=\"fill-black\"",
                    false => "fill=\"#d55f6f\"",
                },
                35.5 + (i as f64) * 4f64
            ));
        }

        content.push_str(
            &SVG_FOOTER
                .replace("#fill_white#", &self.fill_white)
                .replace("#fill_black#", &self.fill_black),
        );

        content = content.replace(
            "#svg_viewport#",
            match self.with_border {
                true => "viewBox=\"-2.5 -2.5 717 116\"",
                _ => "viewBox=\"6 6.1 98 97\"",
            },
        );

        content = content.replace(
            "#svg_border#",
            match self.with_border {
                true => &SVG_BORDER,
                _ => "",
            },
        );

        content
    }
}
