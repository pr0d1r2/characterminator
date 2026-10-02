//! A JSON reader: the smallest one that reads a hook payload WHOLE.
//!
//! `guard` (V35) reads its input from a harness, and the part that matters
//! most -- a tool's output -- arrives as arbitrary JSON whose shape varies
//! by tool. A flat key search (itok's reader, which only needs three
//! fixed fields) cannot see inside that, and it cannot decode escapes, so
//! a tag character sent as the escape pair for U+E0041 would sail past
//! it. The hazard this verb exists to catch is exactly the one an
//! undecoded `\u` escape hides, so the reader decodes every one.
//!
//! No serde, for the reason `src/render` gives for writing json by hand:
//! a reader this size needs no dependency, and a second direct dependency
//! is a decision for the crate rather than for one verb.
//!
//! LENIENT ONLY WHERE LENIENCE CAN ADD A FINDING. A raw control character
//! inside a string is invalid JSON and is kept rather than refused,
//! because refusing it would turn a hazard into a malformed payload, and
//! a malformed payload is the case `guard` lets through (V53). A lone
//! surrogate becomes U+FFFD: it cannot be a `char`, and it is no hazard.

/// One JSON value.
///
/// A number is kept as its source text: nothing the hook reads is a
/// number, so converting one would be a lossy step with no reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Value {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<Value>),
    /// Members in source order, duplicates kept; [`Value::get`] reads the
    /// LAST, which is what a JavaScript harness's own parser would see.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member named `key`, when this is an object holding one.
    pub(super) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .rev()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// The text, when this is a string.
    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Every string inside this value -- member names included, since a
    /// model reads those too -- each labelled with where it sits:
    /// `content[0].text`, or `meta.title (key)`.
    pub(super) fn strings(&self, at: &str) -> Vec<(String, &str)> {
        let mut found = Vec::new();
        self.collect(at, &mut found);
        found
    }

    fn collect<'a>(&'a self, at: &str, found: &mut Vec<(String, &'a str)>) {
        match self {
            Self::Text(text) => found.push((at.to_owned(), text)),
            Self::List(items) => {
                for (index, item) in items.iter().enumerate() {
                    item.collect(&format!("{at}[{index}]"), found);
                }
            }
            Self::Object(members) => members_of(members, at, found),
            Self::Null | Self::Bool(_) | Self::Number(_) => {}
        }
    }
}

fn members_of<'a>(
    members: &'a [(String, Value)],
    at: &str,
    found: &mut Vec<(String, &'a str)>,
) {
    for (name, value) in members {
        let here = format!("{at}.{name}");
        found.push((format!("{here} (key)"), name));
        value.collect(&here, found);
    }
}

/// How deep arrays and objects may nest. A bound, because the reader
/// recurses and a payload is input: without one a hostile document of
/// nothing but `[` would exhaust the stack, and a crash says less than a
/// named refusal. 256 is far past anything a harness writes.
const DEPTH: usize = 256;

/// Read one JSON document, which must be the whole of `text`.
///
/// # Errors
///
/// Anything that is not one JSON value, named with the byte it failed at.
pub(super) fn parse(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        text,
        at: 0,
        depth: 0,
    };
    let value = reader.value()?;
    reader.blank();
    match reader.peek() {
        None => Ok(value),
        Some(_) => Err(reader.error("text after the document")),
    }
}

/// `text` with every `\u` escape decoded -- a surrogate pair to its one
/// character, a lone half to U+FFFD -- and everything else kept as it is
/// written. For a payload [`parse`] refused (`V67`): it is not a
/// document, but an escaped hazard in it is still a hazard. An escaped
/// backslash stays escaped, so a literal backslash-u is not decoded.
pub(super) fn unescaped(text: &str) -> String {
    let mut reader = Reader {
        text,
        at: 0,
        depth: 0,
    };
    let mut out = String::with_capacity(text.len());
    while let Some(next) = reader.bump() {
        if next == '\\' {
            reader.raw_escape(&mut out);
        } else {
            out.push(next);
        }
    }
    out
}

/// A cursor over the document. `at` is a byte offset that only ever moves
/// by a whole character, so every `get(at..)` lands on a boundary.
struct Reader<'a> {
    text: &'a str,
    at: usize,
    depth: usize,
}

/// One container's reader, handed to the depth guard.
type Step<'a> = fn(&mut Reader<'a>) -> Result<Value, String>;

impl<'a> Reader<'a> {
    fn rest(&self) -> &str {
        self.text.get(self.at..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.skip(next.len_utf8());
        Some(next)
    }

    fn skip(&mut self, bytes: usize) {
        self.at = self.at.saturating_add(bytes);
    }

    fn blank(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.bump();
        }
    }

    fn error(&self, what: &str) -> String {
        format!("{what} at byte {}", self.at)
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        if self.bump() == Some(wanted) {
            return Ok(());
        }
        Err(self.error(&format!("expected `{wanted}`")))
    }

    fn value(&mut self) -> Result<Value, String> {
        self.blank();
        match self.peek() {
            Some('{') => self.nested(Self::object),
            Some('[') => self.nested(Self::list),
            Some('"') => self.string().map(Value::Text),
            Some('t') => self.word("true", Value::Bool(true)),
            Some('f') => self.word("false", Value::Bool(false)),
            Some('n') => self.word("null", Value::Null),
            Some('-' | '0'..='9') => self.number(),
            _ => Err(self.error("expected a value")),
        }
    }

    fn nested(&mut self, inner: Step<'a>) -> Result<Value, String> {
        if self.depth >= DEPTH {
            return Err(self.error("nested too deep"));
        }
        self.depth = self.depth.saturating_add(1);
        let value = inner(self);
        self.depth = self.depth.saturating_sub(1);
        value
    }

    fn word(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if !self.rest().starts_with(word) {
            return Err(self.error("expected a value"));
        }
        self.skip(word.len());
        Ok(value)
    }

    /// A number's text, checked by Rust's float grammar: a shade laxer
    /// than JSON's (`1.` passes), which costs nothing when no number is
    /// ever read for its value.
    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        while matches!(
            self.peek(),
            Some('0'..='9' | '-' | '+' | '.' | 'e' | 'E')
        ) {
            self.bump();
        }
        let text = self.text.get(start..self.at).unwrap_or_default();
        match text.parse::<f64>() {
            Ok(_) => Ok(Value::Number(text.to_owned())),
            Err(_) => Err(self.error("not a number")),
        }
    }

    fn list(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        if self.closes(']') {
            return Ok(Value::List(items));
        }
        loop {
            items.push(self.value()?);
            if !self.more(']')? {
                return Ok(Value::List(items));
            }
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut members = Vec::new();
        if self.closes('}') {
            return Ok(Value::Object(members));
        }
        loop {
            members.push(self.member()?);
            if !self.more('}')? {
                return Ok(Value::Object(members));
            }
        }
    }

    fn member(&mut self) -> Result<(String, Value), String> {
        self.blank();
        let name = self.string()?;
        self.blank();
        self.expect(':')?;
        Ok((name, self.value()?))
    }

    /// Whether the container closes at once, consuming the bracket if so.
    fn closes(&mut self, close: char) -> bool {
        self.blank();
        let empty = self.peek() == Some(close);
        if empty {
            self.bump();
        }
        empty
    }

    /// After an item: `true` on a comma, `false` on the closing bracket.
    fn more(&mut self, close: char) -> Result<bool, String> {
        self.blank();
        match self.bump() {
            Some(',') => Ok(true),
            Some(found) if found == close => Ok(false),
            _ => Err(self.error(&format!("expected `,` or `{close}`"))),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(out),
                Some('\\') => self.escape(&mut out)?,
                Some(character) => out.push(character),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        let named = match self.bump() {
            Some('u') => return self.unicode(out),
            Some('b') => '\u{8}',
            Some('f') => '\u{c}',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some(same @ ('"' | '\\' | '/')) => same,
            _ => return Err(self.error("unknown escape")),
        };
        out.push(named);
        Ok(())
    }

    /// One `\u` escape, and its partner when it opens a surrogate pair:
    /// an astral character -- every tag character among them -- arrives
    /// as two escapes, and decoding them apart would lose it.
    fn unicode(&mut self, out: &mut String) -> Result<(), String> {
        let mut units = vec![self.hex()?];
        let high = units.first().is_some_and(|u| (0xD800..0xDC00).contains(u));
        if high && self.rest().starts_with("\\u") {
            self.skip(2);
            units.push(self.hex()?);
        }
        let decoded = char::decode_utf16(units);
        out.extend(decoded.map(|u| u.unwrap_or(char::REPLACEMENT_CHARACTER)));
        Ok(())
    }

    /// After a backslash outside any document: a `\u` escape decoded,
    /// any other escape kept whole, so the character after it is never
    /// read as the start of one.
    fn raw_escape(&mut self, out: &mut String) {
        let start = self.at;
        if self.peek() == Some('u') {
            self.skip(1);
            if self.unicode(out).is_ok() {
                return;
            }
            self.at = start;
        }
        out.push('\\');
        out.extend(self.bump());
    }

    fn hex(&mut self) -> Result<u16, String> {
        let digits = self.rest().get(..4).filter(|digits| {
            digits.chars().all(|digit| digit.is_ascii_hexdigit())
        });
        let unit = digits.and_then(|d| u16::from_str_radix(d, 16).ok());
        let unit =
            unit.ok_or_else(|| self.error("expected four hex digits"))?;
        self.skip(4);
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::{DEPTH, Value, parse, unescaped};

    fn read(text: &str) -> Value {
        let parsed = parse(text);
        assert!(parsed.is_ok(), "{parsed:?}");
        parsed.unwrap_or(Value::Null)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    #[test]
    fn scalars_read_as_themselves() {
        assert_eq!(read("null"), Value::Null);
        assert_eq!(read(" true "), Value::Bool(true));
        assert_eq!(read("false"), Value::Bool(false));
        assert_eq!(read("-1.5e3"), Value::Number(String::from("-1.5e3")));
        assert_eq!(read(r#""hi""#), text("hi"));
    }

    #[test]
    fn containers_nest() {
        let got = read(r#"{"a":[1,{"b":"c"}],"d":{}, "e":[]}"#);
        let inner = Value::Object(vec![(String::from("b"), text("c"))]);
        let list = Value::List(vec![Value::Number(String::from("1")), inner]);
        assert_eq!(got.get("a"), Some(&list));
        assert_eq!(got.get("d"), Some(&Value::Object(Vec::new())));
        assert_eq!(got.get("e"), Some(&Value::List(Vec::new())));
        assert_eq!(got.get("zz"), None);
    }

    #[test]
    fn the_last_duplicate_member_wins() {
        let got = read(r#"{"a":"first","a":"last"}"#);
        assert_eq!(got.get("a").and_then(Value::text), Some("last"));
    }

    #[test]
    fn the_named_escapes_decode() {
        let got = read(r#""q\" s\\ /\/ \b\f\n\r\t""#);
        assert_eq!(got, text("q\" s\\ // \u{8}\u{c}\n\r\t"));
    }

    /// The case the whole reader exists for: a tag character escaped as a
    /// surrogate pair decodes to the character, not to two halves.
    #[test]
    fn a_surrogate_pair_decodes_to_one_astral_character() {
        assert_eq!(read(r#""a\udb40\udc41b""#), text("a\u{E0041}b"));
        assert_eq!(read(r#""\u202e""#), text("\u{202E}"));
    }

    #[test]
    fn a_lone_surrogate_becomes_the_replacement_character() {
        assert_eq!(read(r#""\ud800x""#), text("\u{FFFD}x"));
        assert_eq!(read(r#""\udc00""#), text("\u{FFFD}"));
        // A high half followed by an escape that is not a low half keeps
        // the second character rather than swallowing it.
        assert_eq!(read(r#""\ud800\u0041""#), text("\u{FFFD}A"));
    }

    /// Lenient where lenience can only ADD a finding: a raw control kept
    /// is a hazard reported, where a refusal would be a payload let by.
    #[test]
    fn a_raw_control_character_in_a_string_is_kept() {
        assert_eq!(read("\"a\u{1B}b\""), text("a\u{1B}b"));
    }

    #[test]
    fn non_ascii_passes_through_raw() {
        assert_eq!(read("\"a\u{2014}\u{E0041}\""), text("a\u{2014}\u{E0041}"));
    }

    /// One of each way a document can be wrong.
    const MALFORMED: [&str; 15] = [
        "",
        "{",
        "[1,",
        "[1 2]",
        r#"{"a" 1}"#,
        r#"{"a":}"#,
        "nul",
        r#""open"#,
        r#""\x""#,
        r#""\u12""#,
        r#""\u12G4""#,
        "1 2",
        "--1",
        "{1:2}",
        "[1,]",
    ];

    #[test]
    fn malformed_documents_are_refused_by_name() {
        for bad in MALFORMED {
            let why = parse(bad).err().unwrap_or_default();
            assert!(why.contains("at byte"), "{bad:?} -> {why:?}");
        }
    }

    /// A JSON `\u` escape of `hex`, built so no escape sits in this source.
    fn esc(hex: &str) -> String {
        format!("\\u{hex}")
    }

    /// V67: a refused payload still has its escapes decoded -- a pair to
    /// one character, a lone half to U+FFFD -- and nothing else changed.
    #[test]
    fn unescaping_raw_text_decodes_only_unicode_escapes() {
        let pair = format!("[[\"a{}{}\"", esc("db40"), esc("dc41"));
        assert_eq!(unescaped(&pair), "[[\"a\u{E0041}\"");
        let lone = format!("{}x{}", esc("d800"), esc("202E"));
        assert_eq!(unescaped(&lone), "\u{FFFD}x\u{202E}");
        let kept = format!("\\\\{} \\n \\u12G4 \\", "u202e");
        assert_eq!(unescaped(&kept), kept);
    }

    #[test]
    fn nesting_past_the_bound_is_refused_not_overflowed() {
        let deep = "[".repeat(DEPTH.saturating_add(1));
        let why = parse(&deep).err().unwrap_or_default();
        assert!(why.starts_with("nested too deep"), "{why}");
        let ok = format!("{}{}", "[".repeat(DEPTH), "]".repeat(DEPTH));
        assert!(parse(&ok).is_ok());
    }

    /// Where each string of `{"content":[{"type":..,"text":..}],"n":1}`
    /// sits, member names included.
    const LABELS: [&str; 6] = [
        "r.content (key)",
        "r.content[0].type (key)",
        "r.content[0].type",
        "r.content[0].text (key)",
        "r.content[0].text",
        "r.n (key)",
    ];

    #[test]
    fn every_string_is_found_with_where_it_sits() {
        let got = read(r#"{"content":[{"type":"text","text":"x"}],"n":1}"#);
        let found = got.strings("r");
        let labels: Vec<&str> =
            found.iter().map(|(at, _)| at.as_str()).collect();
        assert_eq!(labels, LABELS);
        assert_eq!(text("y").strings("r"), [(String::from("r"), "y")]);
    }
}
