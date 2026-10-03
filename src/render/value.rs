//! The json primitives, hand-written.
//!
//! A line-based renderer needs no serialization framework: an object is a
//! brace, its fields joined by commas, and a brace. Taking a dependency for
//! that would be a decision for the whole crate rather than for one node,
//! and this node is not the place to open it.
//!
//! Documents are COMPACT -- no spaces, no newlines, one line per run. A
//! machine consumer gains nothing from indentation, and a compact document
//! can be asserted exactly in a test, which is what makes the contract in
//! `src/render/SPEC.md:V11` something a test can actually defend.

use crate::render::escape::string;
use std::fmt::{Display, Write};

/// `"name":value`, where `value` is ALREADY rendered json.
///
/// Taking the value pre-rendered is what keeps one function per shape: a
/// field does not need to know whether it holds a number, a string or a
/// whole object.
pub fn field(name: &str, value: &str) -> String {
    let mut out = string(name);
    out.push(':');
    out.push_str(value);
    out
}

/// `{...}` over already-rendered fields.
pub fn object(fields: &[String]) -> String {
    wrap('{', fields, '}')
}

/// `[...]` over already-rendered items.
pub fn array(items: &[String]) -> String {
    wrap('[', items, ']')
}

/// A json number.
///
/// Rendered through `Display` so the integer width stays the caller's
/// business: a byte offset is a `usize` and a token count is a `u64`, and
/// neither should have to be cast to be reported.
pub fn number(value: impl Display) -> String {
    value.to_string()
}

/// A string, or json `null` for the absent case.
///
/// `null` rather than an omitted key: a consumer that reads a fixed set of
/// keys is simpler than one that has to test for their presence, and a key
/// that comes and goes is a contract that changes shape.
pub fn optional(value: Option<&str>) -> String {
    match value {
        Some(text) => string(text),
        None => String::from("null"),
    }
}

fn wrap(open: char, parts: &[String], close: char) -> String {
    let mut out = String::new();
    out.push(open);
    out.push_str(&parts.join(","));
    out.push(close);
    out
}

/// One object written straight into a report, field by field: the same
/// bytes [`object`] over [`field`]s writes, without a `String` per field.
/// The per-finding objects go out this way (R17); the small documents
/// keep the plain functions above, which read better.
pub struct Fields<'o> {
    out: &'o mut String,
    first: bool,
}

impl<'o> Fields<'o> {
    /// `{`, in `out`.
    pub fn open(out: &'o mut String) -> Self {
        out.push('{');
        Self { out, first: true }
    }

    /// `"name":`, and the buffer to write its value into.
    pub fn key(&mut self, name: &str) -> &mut String {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        crate::render::escape::push(self.out, name);
        self.out.push(':');
        self.out
    }

    /// A field whose value is already rendered json.
    pub fn raw(&mut self, name: &str, value: &str) {
        self.key(name).push_str(value);
    }

    /// A string field.
    pub fn text(&mut self, name: &str, value: &str) {
        crate::render::escape::push(self.key(name), value);
    }

    /// A number field, or any value whose `Display` is its json.
    pub fn number(&mut self, name: &str, value: impl Display) {
        let _infallible = write!(self.key(name), "{value}");
    }

    /// `}`.
    pub fn close(self) {
        self.out.push('}');
    }
}

/// `[...]`, written into `out`: each item by `each`, comma-separated.
pub fn list<T>(
    out: &mut String,
    all: impl IntoIterator<Item = T>,
    mut each: impl FnMut(&mut String, T),
) {
    out.push('[');
    for (at, item) in all.into_iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        each(out, item);
    }
    out.push(']');
}

#[cfg(test)]
mod tests {
    use super::{array, field, number, object, optional};

    #[test]
    fn an_object_joins_its_fields_with_commas_and_no_spaces() {
        let fields = [field("a", &number(1)), field("b", &number(2))];
        assert_eq!(object(&fields), "{\"a\":1,\"b\":2}");
    }

    #[test]
    fn an_empty_object_is_still_an_object() {
        assert_eq!(object(&[]), "{}");
        assert_eq!(array(&[]), "[]");
    }

    #[test]
    fn an_array_nests_objects_without_spaces() {
        let items = [object(&[]), object(&[])];
        assert_eq!(array(&items), "[{},{}]");
    }

    #[test]
    fn an_absent_value_is_null_rather_than_a_missing_key() {
        assert_eq!(optional(None), "null");
        assert_eq!(optional(Some("ascii")), "\"ascii\"");
    }

    #[test]
    fn a_field_name_is_escaped_like_any_other_string() {
        assert_eq!(field("a\"b", "1"), "\"a\\\"b\":1");
    }
}
