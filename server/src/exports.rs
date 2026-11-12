//! Writes a report [Table] as CSV, XLSX or PDF (ADR-0010), the same table the page shows. Times in Warsaw.
//!  - CSV for a Polish Excel: `;`, UTF-8 with BOM, CRLF, decimal comma, and a `'` before a value starting with
//!    = + - @ (a formula a spreadsheet would otherwise run).
//!  - XLSX (`rust_xlsxwriter`): real dates and numbers, a frozen header row with an autofilter.
//!  - PDF (`krilla`): A4 landscape, A3 for more than 13 columns, the header repeated on every page, Lato embedded
//!    (subset) so Polish letters show everywhere.

use anyhow::{Context, anyhow};
use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Europe::Warsaw;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::reports::{Cell, Lang, Table};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportFormat {
    Csv,
    Xlsx,
    Pdf,
}

impl ExportFormat {
    pub fn of(extension: &str) -> Option<ExportFormat> {
        match extension.to_ascii_lowercase().as_str() {
            "csv" => Some(ExportFormat::Csv),
            "xlsx" => Some(ExportFormat::Xlsx),
            "pdf" => Some(ExportFormat::Pdf),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Pdf => "pdf",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            ExportFormat::Csv => "text/csv; charset=UTF-8",
            ExportFormat::Xlsx => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
            ExportFormat::Pdf => "application/pdf",
        }
    }
}

pub fn write(table: &Table, format: ExportFormat, lang: Lang) -> anyhow::Result<Vec<u8>> {
    match format {
        ExportFormat::Csv => Ok(csv(table)),
        ExportFormat::Xlsx => xlsx(table),
        ExportFormat::Pdf => pdf(table, lang),
    }
}

fn rows_with_totals(table: &Table) -> impl Iterator<Item = &Vec<Option<Cell>>> {
    table.rows.iter().chain(table.totals.iter())
}

/// "2026-11-12 09:30" in Warsaw.
fn local_time(at: &DateTime<Utc>) -> String {
    at.with_timezone(&Warsaw)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Two decimals, `decimal` between the whole part and the fraction, `group` (if any) every three digits.
fn amount(value: &Decimal, decimal: char, group: Option<char>) -> String {
    let rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointNearestEven);
    let plain = format!("{:.2}", rounded.abs());
    let (whole, fraction) = plain.split_once('.').unwrap_or((&plain, "00"));
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if let Some(g) = group
            && i > 0
            && (whole.len() - i) % 3 == 0
        {
            grouped.push(g);
        }
        grouped.push(digit);
    }
    let sign = if rounded.is_sign_negative() && !rounded.is_zero() {
        "-"
    } else {
        ""
    };
    format!("{sign}{grouped}{decimal}{fraction}")
}

// ---- CSV -----------------------------------------------------------------------------------------------------

fn csv(table: &Table) -> Vec<u8> {
    let mut text = String::from("\u{feff}");
    let mut line = |cells: Vec<String>| {
        let quoted: Vec<String> = cells.iter().map(|c| quote(&guard(c))).collect();
        text.push_str(&quoted.join(";"));
        text.push_str("\r\n");
    };
    line(table.columns.iter().map(|c| c.header()).collect());
    for row in rows_with_totals(table) {
        line(
            row.iter()
                .map(|value| match value {
                    None => String::new(),
                    Some(Cell::Instant(at)) => local_time(at),
                    Some(Cell::Amount(a)) => amount(a, ',', None),
                    Some(other) => other.text(),
                })
                .collect(),
        );
    }
    text.into_bytes()
}

/// A value a spreadsheet would read as a formula gets a leading apostrophe.
fn guard(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_owned(),
    }
}

fn quote(value: &str) -> String {
    if value.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

// ---- XLSX ----------------------------------------------------------------------------------------------------

fn xlsx(table: &Table) -> anyhow::Result<Vec<u8>> {
    use rust_xlsxwriter::{ExcelDateTime, Format, FormatPattern, Workbook};

    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    let sheet_name: String = table
        .title
        .chars()
        .take(31)
        .map(|c| if "[]*?/\\:".contains(c) { ' ' } else { c })
        .collect();
    sheet.set_name(sheet_name.trim_matches('\''))?;
    let head = Format::new()
        .set_bold()
        .set_pattern(FormatPattern::Solid)
        .set_background_color("#C0C0C0");
    let date = Format::new().set_num_format("yyyy-mm-dd");
    let time = Format::new().set_num_format("yyyy-mm-dd hh:mm");
    let money = Format::new().set_num_format("#,##0.00");
    let plain = Format::new();
    let total = Format::new().set_bold();

    for (c, column) in table.columns.iter().enumerate() {
        sheet.write_string_with_format(0, c as u16, column.header(), &head)?;
    }
    let total_row = table.totals.as_ref().map(|_| table.rows.len());
    for (r, values) in rows_with_totals(table).enumerate() {
        let row = (r + 1) as u32;
        let is_total = total_row == Some(r);
        for (c, value) in values.iter().enumerate() {
            let col = c as u16;
            let pick = |f: &'_ Format| if is_total { total.clone() } else { f.clone() };
            match value {
                None => {}
                Some(Cell::Instant(at)) => {
                    let local = at.with_timezone(&Warsaw);
                    let when = ExcelDateTime::from_ymd(
                        local.year() as u16,
                        local.month() as u8,
                        local.day() as u8,
                    )?
                    .and_hms(
                        local.hour() as u16,
                        local.minute() as u8,
                        local.second(),
                    )?;
                    sheet.write_datetime_with_format(row, col, &when, &pick(&time))?;
                }
                Some(Cell::Date(d)) => {
                    let when =
                        ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8)?;
                    sheet.write_datetime_with_format(row, col, &when, &pick(&date))?;
                }
                Some(Cell::Amount(a)) => {
                    let n: f64 = a.to_string().parse().context("an amount")?;
                    sheet.write_number_with_format(row, col, n, &pick(&money))?;
                }
                Some(Cell::Number(n)) => {
                    sheet.write_number_with_format(row, col, *n as f64, &pick(&plain))?;
                }
                Some(Cell::Text(s)) => {
                    sheet.write_string_with_format(row, col, s, &pick(&plain))?;
                }
            }
        }
    }
    sheet.set_freeze_panes(1, 0)?;
    if !table.columns.is_empty() {
        sheet.autofilter(
            0,
            0,
            table.rows.len().max(1) as u32,
            (table.columns.len() - 1) as u16,
        )?;
    }
    for (c, column) in table.columns.iter().enumerate() {
        let longest = table
            .rows
            .iter()
            .map(|r| r[c].as_ref().map(|v| v.text().chars().count()).unwrap_or(0))
            .chain(std::iter::once(column.header().chars().count()))
            .max()
            .unwrap_or(0);
        sheet.set_column_width(c as u16, (longest.clamp(8, 50) + 2) as f64)?;
    }
    Ok(book.save_to_buffer()?)
}

// ---- PDF -----------------------------------------------------------------------------------------------------

static LATO_REGULAR: &[u8] = include_bytes!("../assets/fonts/Lato-Regular.ttf");
static LATO_BOLD: &[u8] = include_bytes!("../assets/fonts/Lato-Bold.ttf");

/// A font for drawing (krilla) and for measuring text (ttf-parser).
struct Typeface {
    font: krilla::text::Font,
    face: ttf_parser::Face<'static>,
}

impl Typeface {
    fn load(bytes: &'static [u8]) -> anyhow::Result<Self> {
        Ok(Typeface {
            font: krilla::text::Font::new(bytes.into(), 0).ok_or_else(|| anyhow!("a TTF font"))?,
            face: ttf_parser::Face::parse(bytes, 0)?,
        })
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        let units: u32 = text
            .chars()
            .map(|ch| {
                self.face
                    .glyph_index(ch)
                    .and_then(|g| self.face.glyph_hor_advance(g))
                    .unwrap_or(0) as u32
            })
            .sum();
        units as f32 * size / self.face.units_per_em() as f32
    }

    /// Greedy wrap at spaces; a word wider than the line is broken between letters.
    fn wrap(&self, text: &str, size: f32, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if self.width(&candidate, size) <= width {
                    line = candidate;
                    continue;
                }
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                for ch in word.chars() {
                    let mut next = line.clone();
                    next.push(ch);
                    if !line.is_empty() && self.width(&next, size) > width {
                        lines.push(std::mem::take(&mut line));
                        line.push(ch);
                    } else {
                        line = next;
                    }
                }
            }
            lines.push(line);
        }
        lines
    }
}

const MARGIN_X: f32 = 24.0;
const MARGIN_Y: f32 = 28.0;
const CELL_SIZE: f32 = 7.5;
const PADDING: f32 = 3.0;
const LEADING: f32 = CELL_SIZE * 1.35;

struct PdfRow {
    cells: Vec<(Vec<String>, bool)>,
    bold: bool,
    fill: bool,
    /// One cell over the whole width (the "no tickets" line).
    span: bool,
    height: f32,
}

fn pdf(table: &Table, lang: Lang) -> anyhow::Result<Vec<u8>> {
    use krilla::{
        Document,
        color::luma,
        geom::{PathBuilder, Point, Rect},
        page::PageSettings,
        paint::{Fill, Stroke},
        text::TextDirection,
    };

    let regular = Typeface::load(LATO_REGULAR)?;
    let bold = Typeface::load(LATO_BOLD)?;
    let (page_w, page_h) = if table.columns.len() > 13 {
        (1190.55, 841.89)
    } else {
        (841.89, 595.28)
    };
    let columns = table.columns.len().max(1);
    let col_w = (page_w - 2.0 * MARGIN_X) / columns as f32;
    let text_w = col_w - 2.0 * PADDING;
    let (decimal, group) = if lang == Lang::Pl {
        (',', '\u{a0}')
    } else {
        ('.', ',')
    };

    let measure = |cells: Vec<(String, bool)>, bold_row: bool, fill: bool| -> PdfRow {
        let face = if bold_row { &bold } else { &regular };
        let cells: Vec<(Vec<String>, bool)> = cells
            .into_iter()
            .map(|(t, right)| (face.wrap(&t, CELL_SIZE, text_w), right))
            .collect();
        let lines = cells.iter().map(|(l, _)| l.len()).max().unwrap_or(1).max(1);
        PdfRow {
            cells,
            bold: bold_row,
            fill,
            span: false,
            height: lines as f32 * LEADING + 2.0 * PADDING,
        }
    };
    let header = measure(
        table.columns.iter().map(|c| (c.header(), false)).collect(),
        true,
        true,
    );
    let total_row = table.totals.as_ref().map(|_| table.rows.len());
    let mut body: Vec<PdfRow> = rows_with_totals(table)
        .enumerate()
        .map(|(r, row)| {
            let cells = row
                .iter()
                .map(|value| match value {
                    None => (String::new(), false),
                    Some(Cell::Instant(at)) => (local_time(at), false),
                    Some(Cell::Amount(a)) => (amount(a, decimal, Some(group)), true),
                    Some(n @ Cell::Number(_)) => (n.text(), true),
                    Some(other) => (other.text(), false),
                })
                .collect();
            measure(cells, total_row == Some(r), false)
        })
        .collect();
    if table.rows.is_empty() {
        let empty = lang.t("Brak zgłoszeń w tym okresie.", "No tickets in this period.");
        let lines = regular.wrap(empty, CELL_SIZE, page_w - 2.0 * MARGIN_X - 2.0 * PADDING);
        body.push(PdfRow {
            height: lines.len() as f32 * LEADING + 2.0 * PADDING,
            cells: vec![(lines, false)],
            bold: false,
            fill: false,
            span: true,
        });
    }

    let mut document = Document::new();
    let settings = || PageSettings::from_wh(page_w, page_h).ok_or_else(|| anyhow!("a page size"));
    let mut rows = body.iter().peekable();
    let mut first = true;
    while first || rows.peek().is_some() {
        let mut page = document.start_page_with(settings()?);
        let mut surface = page.surface();
        let mut y = MARGIN_Y;
        if first {
            surface.draw_text(
                Point::from_xy(MARGIN_X, y + 16.0),
                bold.font.clone(),
                16.0,
                &table.title,
                false,
                TextDirection::Auto,
            );
            y += 16.0 * 1.5;
            surface.draw_text(
                Point::from_xy(MARGIN_X, y + 10.0),
                regular.font.clone(),
                10.0,
                &table.subtitle,
                false,
                TextDirection::Auto,
            );
            y += 10.0 * 1.5 + 10.0;
            first = false;
        }
        let mut draw = |row: &PdfRow, y: f32| {
            let face = if row.bold { &bold } else { &regular };
            let widths: Vec<f32> = if row.span {
                vec![page_w - 2.0 * MARGIN_X]
            } else {
                vec![col_w; row.cells.len()]
            };
            let mut x = MARGIN_X;
            for ((lines, right), w) in row.cells.iter().zip(widths) {
                let mut pb = PathBuilder::new();
                if let Some(rect) = Rect::from_xywh(x, y, w, row.height) {
                    pb.push_rect(rect);
                }
                if let Some(path) = pb.finish() {
                    surface.set_fill(row.fill.then(|| Fill {
                        paint: luma::Color::new(230).into(),
                        ..Fill::default()
                    }));
                    surface.set_stroke(Some(Stroke {
                        width: 0.5,
                        ..Stroke::default()
                    }));
                    surface.draw_path(&path);
                }
                surface.set_stroke(None);
                surface.set_fill(Some(Fill::default()));
                for (i, line) in lines.iter().enumerate() {
                    let left = if *right {
                        x + w - PADDING - face.width(line, CELL_SIZE)
                    } else {
                        x + PADDING
                    };
                    surface.draw_text(
                        Point::from_xy(left, y + PADDING + CELL_SIZE + i as f32 * LEADING),
                        face.font.clone(),
                        CELL_SIZE,
                        line,
                        false,
                        TextDirection::Auto,
                    );
                }
                x += w;
            }
        };
        draw(&header, y);
        y += header.height;
        while let Some(row) = rows.peek() {
            // A row that does not fit goes to the next page, unless it is the first one under the header.
            if y + row.height > page_h - MARGIN_Y && y > MARGIN_Y + header.height {
                break;
            }
            draw(row, y);
            y += row.height;
            rows.next();
        }
        surface.finish();
        page.finish();
    }
    document
        .finish()
        .map_err(|e| anyhow!("writing the PDF: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_round_and_group() {
        let d = |s: &str| s.parse::<Decimal>().unwrap();
        assert_eq!(amount(&d("1234.5"), ',', None), "1234,50");
        assert_eq!(amount(&d("1234567.891"), ',', Some(' ')), "1 234 567,89");
        assert_eq!(amount(&d("-12.345"), '.', Some(',')), "-12.34");
        assert_eq!(amount(&d("999"), '.', Some(',')), "999.00");
    }

    #[test]
    fn formulas_are_neutralised_and_separators_quoted() {
        assert_eq!(
            quote(&guard("=HYPERLINK(\"x\")")),
            "\"'=HYPERLINK(\"\"x\"\")\""
        );
        assert_eq!(quote(&guard("a;b")), "\"a;b\"");
        assert_eq!(quote(&guard("zwykły")), "zwykły");
    }
}
