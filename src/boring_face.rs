use std::collections::HashMap;

static SVG_HEADER: &str = r###"<svg width="98" height="97" xmlns="http://www.w3.org/2000/svg">
    <g transform="translate(.38)" fill="none" fill-rule="evenodd">
        <circle class="fill-black" cx="48.5" cy="48.5" r="48.5" />
        <g class="fill-white" transform="translate(19.587 30.779)">
            <circle cx="8.861" cy="8.861" r="8.861" />
            <circle cx="48.966" cy="8.861" r="8.861" />
        </g>
        <rect class="fill-white" x="25.5" y="66.2" width="47" height="8.529" rx="5" />"###;

static SVG_FOOTER: &str = r###"</g>
    <style>
        .fill-black {
            fill: #fill_black#
        }
        .fill-white {
            fill: #fill_white#
        }
        @media (prefers-color-scheme: dark) {
            .fill-black {
                fill: #fill_white#
            }
            .fill-white {
                fill: #fill_black#
            }
        }
    </style>
</svg>"###;

struct BoringFace {
    fill_white: String,
    fill_black: String,
    render_cache: HashMap<usize, String>,
}

impl BoringFace {
    pub fn new(fill_white: String, fill_black: String) -> Self {
        Self {
            fill_white,
            fill_black,
            render_cache: HashMap::new(),
        }
    }

    fn render_svg(&mut self, length: usize) -> String {
        assert!(length >= 1 && length <= 10);
        if let Some(cache) = self.render_cache.get(&length) {
            return cache.to_string();
        }
        let mut content = SVG_HEADER.to_string();

        for i in 0..10 {
            content.push_str(&format!(
                "<rect {} x=\"{}\" y=\"67.5\" width=\"3\" height=\"6\" rx=\"1\" />",
                match i.lt(&length) {
                    true => "class=\"fill-black\"",
                    false => "fill=\"gray\"",
                },
                29.5 + (i as f64) * 4f64
            ));
        }

        content.push_str(SVG_FOOTER);

        self.render_cache.insert(length, content.clone());

        content
    }
}
