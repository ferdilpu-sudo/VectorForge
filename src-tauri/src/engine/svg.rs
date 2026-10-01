use std::collections::HashSet;
use std::fmt::Write as _;

use vtracer::VectorDoc;
use vtracer::ir::PathCmd;

pub struct SvgOutput {
    pub svg: String,
    pub path_count: u64,
    pub color_count: u64,
}

pub fn write_alpha_svg(doc: &VectorDoc) -> SvgOutput {
    let mut output = String::new();
    let mut path_count = 0u64;
    let mut colors = HashSet::new();

    let _ = writeln!(
        output,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        doc.width, doc.height, doc.width, doc.height
    );

    for shape in &doc.shapes {
        let color = shape.paint.color();
        if color.a == 0 {
            continue;
        }

        let mut data = String::new();
        for subpath in &shape.path.subpaths {
            data.push_str(&path_data(&subpath.commands));
        }
        if data.is_empty() {
            continue;
        }

        path_count += 1;
        colors.insert((color.r, color.g, color.b, color.a));

        if color.a == 255 {
            let _ = writeln!(
                output,
                "<path d=\"{}\" fill=\"{}\"/>",
                data,
                color.to_hex_string()
            );
        } else {
            let opacity = f64::from(color.a) / 255.0;
            let _ = writeln!(
                output,
                "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                data,
                color.to_hex_string(),
                fmt_number(opacity)
            );
        }
    }

    output.push_str("</svg>\n");

    SvgOutput {
        svg: output,
        path_count,
        color_count: colors.len() as u64,
    }
}

pub fn empty_svg(width: u32, height: u32) -> SvgOutput {
    SvgOutput {
        svg: format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"></svg>\n"
        ),
        path_count: 0,
        color_count: 0,
    }
}

fn path_data(commands: &[PathCmd]) -> String {
    let mut data = String::new();

    for command in commands {
        match command {
            PathCmd::MoveTo(point) => {
                let _ = write!(data, "M{},{}", fmt_number(point.x), fmt_number(point.y));
            }
            PathCmd::LineTo(point) => {
                let _ = write!(data, "L{},{}", fmt_number(point.x), fmt_number(point.y));
            }
            PathCmd::CubicTo(control1, control2, end) => {
                let _ = write!(
                    data,
                    "C{},{} {},{} {},{}",
                    fmt_number(control1.x),
                    fmt_number(control1.y),
                    fmt_number(control2.x),
                    fmt_number(control2.y),
                    fmt_number(end.x),
                    fmt_number(end.y)
                );
            }
            PathCmd::Close => data.push('Z'),
        }
    }

    data
}

fn fmt_number(value: f64) -> String {
    let mut text = format!("{value:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::empty_svg;

    #[test]
    fn empty_svg_contains_no_vector_paths() {
        let output = empty_svg(32, 16);
        assert_eq!(output.path_count, 0);
        assert_eq!(output.color_count, 0);
        assert!(!output.svg.contains("<path"));
        assert!(output.svg.contains("viewBox=\"0 0 32 16\""));
    }
}
