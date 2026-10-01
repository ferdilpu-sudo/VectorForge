use std::fmt::Write as _;

use vtracer::VectorDoc;
use vtracer::ir::PathCmd;

use crate::models::{AppError, ErrorCode};

pub fn write_eps(doc: &VectorDoc) -> Result<Vec<u8>, AppError> {
    if doc.width == 0 || doc.height == 0 {
        return Err(AppError::new(
            ErrorCode::ExportFailed,
            "Dokumen EPS memiliki bounds nol.",
        ));
    }

    let mut output = String::new();
    let _ = writeln!(output, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(output, "%%BoundingBox: 0 0 {} {}", doc.width, doc.height);
    let _ = writeln!(output, "%%LanguageLevel: 2");
    let _ = writeln!(output, "%%Pages: 1");
    let _ = writeln!(output, "%%EndComments");
    let _ = writeln!(output, "gsave");
    let _ = writeln!(output, "0 {} translate", doc.height);
    let _ = writeln!(output, "1 -1 scale");

    for shape in &doc.shapes {
        let color = shape.paint.color();
        if color.a != 255 {
            return Err(AppError::new(
                ErrorCode::ExportFailed,
                "EPS menerima transparansi yang belum dikomposit ke putih.",
            ));
        }

        let red = f64::from(color.r) / 255.0;
        let green = f64::from(color.g) / 255.0;
        let blue = f64::from(color.b) / 255.0;
        let _ = writeln!(output, "{red:.6} {green:.6} {blue:.6} setrgbcolor");
        output.push_str("newpath\n");

        for subpath in &shape.path.subpaths {
            for command in &subpath.commands {
                match command {
                    PathCmd::MoveTo(point) => {
                        let _ = writeln!(
                            output,
                            "{} {} moveto",
                            fmt_number(point.x),
                            fmt_number(point.y)
                        );
                    }
                    PathCmd::LineTo(point) => {
                        let _ = writeln!(
                            output,
                            "{} {} lineto",
                            fmt_number(point.x),
                            fmt_number(point.y)
                        );
                    }
                    PathCmd::CubicTo(control1, control2, end) => {
                        let _ = writeln!(
                            output,
                            "{} {} {} {} {} {} curveto",
                            fmt_number(control1.x),
                            fmt_number(control1.y),
                            fmt_number(control2.x),
                            fmt_number(control2.y),
                            fmt_number(end.x),
                            fmt_number(end.y)
                        );
                    }
                    PathCmd::Close => output.push_str("closepath\n"),
                }
            }
        }

        output.push_str("fill\n");
    }

    output.push_str("grestore\nshowpage\n%%EOF\n");
    Ok(output.into_bytes())
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
