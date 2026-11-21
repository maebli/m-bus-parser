#[derive(Clone, Copy)]
pub(super) enum TableStyle {
    Plain,
    Terminal,
    Html,
}

#[derive(Clone, Copy)]
pub(super) enum Cell<'a> {
    Text,
    Label,
    Reading,
    Bytes,
    Website(&'a str),
}

impl TableStyle {
    pub(super) fn write(self, output: &mut String, cell: Cell<'_>, text: &str) {
        if text.is_empty() {
            return;
        }
        if matches!(self, Self::Plain) {
            output.push_str(text);
            return;
        }

        let (class, ansi) = match cell {
            Cell::Text => ("", ""),
            Cell::Label => ("label", "\x1b[36m"),
            Cell::Reading => ("reading", "\x1b[32m"),
            Cell::Bytes => ("bytes", "\x1b[35m"),
            Cell::Website(_) => ("link", "\x1b[4;34m"),
        };
        let website = match cell {
            Cell::Website(host)
                if !host.is_empty()
                    && host.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')
                    }) =>
            {
                Some(host)
            }
            _ => None,
        };
        match self {
            Self::Terminal => {
                if let Some(host) = website {
                    output.push_str(&format!("\x1b]8;;https://{host}\x1b\\"));
                }
                output.push_str(ansi);
                for ch in text.chars() {
                    if ch.is_control() {
                        output.extend(ch.escape_default());
                    } else {
                        output.push(ch);
                    }
                }
                if !ansi.is_empty() {
                    output.push_str("\x1b[0m");
                }
                if website.is_some() {
                    output.push_str("\x1b]8;;\x1b\\");
                }
            }
            Self::Html => {
                if let Some(host) = website {
                    output.push_str(&format!("<a class=\"table-link\" href=\"https://{host}\" target=\"_blank\" rel=\"noopener noreferrer\">"));
                } else if !class.is_empty() {
                    output.push_str(&format!("<span class=\"table-{class}\">"));
                }
                for ch in text.chars() {
                    match ch {
                        '&' => output.push_str("&amp;"),
                        '<' => output.push_str("&lt;"),
                        '>' => output.push_str("&gt;"),
                        _ => output.push(ch),
                    }
                }
                if website.is_some() {
                    output.push_str("</a>");
                } else if !class.is_empty() {
                    output.push_str("</span>");
                }
            }
            Self::Plain => unreachable!(),
        }
    }
}
