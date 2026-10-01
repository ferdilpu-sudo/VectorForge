use svg2pdf::{ConversionOptions, PageOptions};

use crate::models::{AppError, ErrorCode};

pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, AppError> {
    let tree = svg2pdf::usvg::Tree::from_str(svg, &svg2pdf::usvg::Options::default()).map_err(
        |error| {
            AppError::with_details(
                ErrorCode::ExportFailed,
                "SVG hasil tracing gagal diparsing untuk PDF.",
                error.to_string(),
            )
        },
    )?;

    svg2pdf::to_pdf(
        &tree,
        ConversionOptions::default(),
        PageOptions { dpi: 96.0 },
    )
    .map_err(|error| {
        AppError::with_details(
            ErrorCode::ExportFailed,
            "Konversi PDF gagal.",
            error.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::svg_to_pdf;

    #[test]
    fn simple_vector_svg_converts_to_pdf() -> Result<(), String> {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48"><path d="M0,0L96,0L96,48Z" fill="#336699"/></svg>"##;
        let pdf = svg_to_pdf(svg).map_err(|error| error.message)?;
        assert!(pdf.starts_with(b"%PDF-"));
        Ok(())
    }
}
