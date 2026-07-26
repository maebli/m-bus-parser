use std::sync::OnceLock;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntax_for<'a>(syntax_set: &'a SyntaxSet, language: &str) -> &'a SyntaxReference {
    let extension = match language.trim().to_ascii_lowercase().as_str() {
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "xml" => "xml",
        _ => return syntax_set.find_syntax_plain_text(),
    };
    syntax_set
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

pub(crate) fn highlight_source(source: &str, language: &str) -> Result<String, String> {
    if language.trim().eq_ignore_ascii_case("csv") {
        return Ok(highlight_csv(source));
    }

    let syntax_set = syntax_set();
    let syntax = syntax_for(syntax_set, language);
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        syntax_set,
        ClassStyle::SpacedPrefixed { prefix: "syn-" },
    );

    let appended_newline = !source.ends_with('\n');
    let owned_source;
    let parse_source = if appended_newline {
        owned_source = format!("{source}\n");
        owned_source.as_str()
    } else {
        source
    };

    for line in LinesWithEndings::from(parse_source) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .map_err(|error| error.to_string())?;
    }

    let mut html = generator.finalize();
    if appended_newline {
        let final_newline = html
            .rfind('\n')
            .ok_or_else(|| "highlighter omitted the final line separator".to_string())?;
        html.remove(final_newline);
    }
    Ok(html)
}

fn highlight_csv(source: &str) -> String {
    let mut html = String::from("<span class=\"syn-source syn-csv\">");
    let mut field = String::new();
    let mut characters = source.chars().peekable();
    let mut column = 0usize;
    let mut quoted = false;

    while let Some(character) = characters.next() {
        match character {
            '"' => {
                field.push(character);
                if quoted && characters.peek() == Some(&'"') {
                    field.push(characters.next().unwrap_or('"'));
                } else {
                    quoted = !quoted;
                }
            }
            ',' if !quoted => {
                push_csv_field(&mut html, &field, column);
                field.clear();
                html.push_str("<span class=\"syn-punctuation syn-csv-delimiter\">,</span>");
                column += 1;
            }
            '\r' if !quoted && characters.peek() == Some(&'\n') => {
                push_csv_field(&mut html, &field, column);
                field.clear();
                html.push('\r');
                html.push(characters.next().unwrap_or('\n'));
                column = 0;
            }
            '\n' | '\r' if !quoted => {
                push_csv_field(&mut html, &field, column);
                field.clear();
                html.push(character);
                column = 0;
            }
            _ => field.push(character),
        }
    }

    push_csv_field(&mut html, &field, column);
    html.push_str("</span>");
    html
}

fn push_csv_field(html: &mut String, field: &str, column: usize) {
    html.push_str(&format!(
        "<span class=\"syn-csv-column syn-csv-column-{}\">",
        column % 8
    ));
    push_escaped_html(html, field);
    html.push_str("</span>");
}

fn push_escaped_html(html: &mut String, source: &str) {
    for character in source.chars() {
        match character {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            _ => html.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_formats_preserve_every_source_line() {
        for (language, source) in [
            ("json", "{\n  \"meter\": \"02205100\"\n}"),
            ("yaml", "meter: 02205100\nrecords: 10"),
            ("csv", "meter_id,record_count\n02205100,10"),
            ("xml", "<MBusData>\n  <SlaveInformation />\n</MBusData>"),
        ] {
            let html = highlight_source(source, language).unwrap();
            assert_eq!(
                html.matches('\n').count(),
                source.matches('\n').count(),
                "{language} must retain exactly one separator per source line"
            );
            assert!(
                html.contains("syn-"),
                "{language} must contain semantic token classes"
            );
            if language == "csv" {
                assert!(html.contains("syn-csv-column-0"));
                assert!(html.contains("syn-csv-column-1"));
                assert!(!html.contains("syn-text syn-plain"));
            }
        }
    }

    #[test]
    fn highlighted_html_escapes_untrusted_source() {
        let html = highlight_source("\"</script><img src=x>\"", "json").unwrap();
        assert!(!html.contains("<script"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;"));
    }

    #[test]
    fn csv_highlighting_respects_quotes_and_embedded_newlines() {
        let source = "a,b\n\"one,two\",\"<tag>\"\n\"line\nbreak\",x";
        let html = highlight_source(source, "csv").unwrap();

        assert_eq!(html.matches('\n').count(), source.matches('\n').count());
        assert_eq!(html.matches("syn-csv-delimiter").count(), 3);
        assert!(html.contains("&quot;one,two&quot;"));
        assert!(html.contains("&lt;tag&gt;"));
        assert!(!html.contains("<tag>"));
    }

    #[test]
    fn syntax_palette_meets_wcag_aa_in_both_themes() {
        const REQUIRED_RATIO: f64 = 4.5;
        const LIGHT_BACKGROUND: &str = "#ffffff";
        const DARK_BACKGROUND: &str = "#0d1117";
        const LIGHT_TOKENS: &[&str] = &[
            "#0d1117", "#4b5563", "#0a3069", "#953800", "#a0111f", "#0349b4", "#622cbc", "#024c1a",
            "#005a5a", "#704800", "#3b3f99",
        ];
        const DARK_TOKENS: &[&str] = &[
            "#f0f3f6", "#c7cdd5", "#a5d6ff", "#ffb77c", "#ff9492", "#71b7ff", "#cb9eff", "#6de080",
            "#5eead4", "#f8d866", "#b6c7ff",
        ];

        for (background, tokens) in [
            (LIGHT_BACKGROUND, LIGHT_TOKENS),
            (DARK_BACKGROUND, DARK_TOKENS),
        ] {
            for foreground in tokens {
                let ratio = contrast_ratio(foreground, background);
                assert!(
                    ratio >= REQUIRED_RATIO,
                    "{foreground} on {background} has only {ratio:.2}:1 contrast"
                );
            }
        }
    }

    fn contrast_ratio(foreground: &str, background: &str) -> f64 {
        let foreground = luminance(foreground);
        let background = luminance(background);
        (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
    }

    fn luminance(color: &str) -> f64 {
        let hex = color.strip_prefix('#').unwrap();
        let channels = [0, 2, 4].map(|offset| {
            let value = u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap() as f64 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
    }
}
