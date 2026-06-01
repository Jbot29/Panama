use resvg::{tiny_skia, usvg};

fn escape_bare_ampersands(svg: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let mut remaining = svg;
    while let Some(pos) = remaining.find('&') {
        result.push_str(&remaining[..pos]);
        let after = &remaining[pos + 1..];
        let valid = after.starts_with("amp;")
            || after.starts_with("lt;")
            || after.starts_with("gt;")
            || after.starts_with("quot;")
            || after.starts_with("apos;")
            || after.starts_with('#')
            || {
                let semi = after.find(';').unwrap_or(100);
                semi < 20 && after[..semi].chars().all(|c| c.is_ascii_alphanumeric())
            };
        if valid {
            result.push('&');
        } else {
            result.push_str("&amp;");
        }
        remaining = &remaining[pos + 1..];
    }
    result.push_str(remaining);
    result
}

pub fn render_svg(svg_str: &str) -> Option<egui::ColorImage> {
    let sanitized = escape_bare_ampersands(svg_str);
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = match usvg::Tree::from_str(&sanitized, &opt) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[diagram] usvg parse error: {e:?}");
            return None;
        }
    };

    let size = tree.size().to_int_size();
    eprintln!("[diagram] parsed SVG size: {}x{}", size.width(), size.height());

    let max_width = 700u32;
    let scale = if size.width() > max_width {
        max_width as f32 / size.width() as f32
    } else {
        1.0
    };
    let width = (size.width() as f32 * scale) as u32;
    let height = (size.height() as f32 * scale) as u32;
    eprintln!("[diagram] rendering at {width}x{height}");

    let mut pixmap = match tiny_skia::Pixmap::new(width, height) {
        Some(p) => p,
        None => {
            eprintln!("[diagram] Pixmap::new({width}, {height}) returned None");
            return None;
        }
    };

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

pub fn extract_svg(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find("<svg");
    let close = lower.rfind("</svg");
    eprintln!("[diagram] extract_svg: len={} start={start:?} close={close:?}", text.len());
    let start = start?;
    let close_start = close?;
    let close_end = lower[close_start..].find('>')? + close_start + 1;
    Some(text[start..close_end].to_string())
}
