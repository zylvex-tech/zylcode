//! Grammar selection and input preprocessing.
//!
//! `.tsx` is TypeScript plus JSX. The TypeScript grammar cannot parse JSX, and
//! selecting it marked 94 of this repository's 97 `.tsx` files as partially
//! parsed while a TSX pass reported no error node at all. A UTF-8 BOM is not
//! part of any grammar either. Both are handled here, and neither fix may turn
//! a genuine syntax error into a clean parse.

use zylcode_nav::model::{Language, ParseStatus};
use zylcode_nav::parse::parse;

#[test]
fn a_tsx_file_is_parsed_with_the_tsx_grammar() {
    let src = "export function Card() {\n  return <div className=\"c\">hi</div>;\n}\n";
    let out = parse(
        src,
        "src/Card.tsx",
        &Language::TypeScript,
        "src::Card",
        "app",
    )
    .expect("parse");
    assert_eq!(out.status, ParseStatus::Parsed, "error: {:?}", out.error);
    assert!(
        out.symbols.iter().any(|s| s.name == "Card"),
        "the component must be extracted, got {:?}",
        out.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
}

#[test]
fn jsx_inside_a_ts_file_stays_a_parse_problem() {
    let src = "const x = <div>hi</div>;\n";
    let out = parse(src, "src/bad.ts", &Language::TypeScript, "src::bad", "app").expect("parse");
    assert_eq!(
        out.status,
        ParseStatus::Partial,
        "JSX is not TypeScript; it must not be accepted silently"
    );
}

#[test]
fn a_utf8_bom_is_skipped_and_offsets_stay_file_relative() {
    let src = "\u{feff}pub struct Account;\n";
    let out = parse(src, "a.rs", &Language::Rust, "crate", "p").expect("parse");
    assert_eq!(out.status, ParseStatus::Parsed, "error: {:?}", out.error);
    let sym = out
        .symbols
        .iter()
        .find(|s| s.name == "Account")
        .expect("Account is extracted");
    assert_eq!(
        sym.range.start_byte, 3,
        "offsets must address the file on disk, BOM included"
    );
    assert_eq!(sym.range.start_line, 1);
}

#[test]
fn rust_and_typescript_parsing_are_unaffected() {
    let rust = parse("pub fn f() {}\n", "a.rs", &Language::Rust, "crate", "p").expect("parse");
    assert_eq!(rust.status, ParseStatus::Parsed);
    assert!(rust.symbols.iter().any(|s| s.name == "f"));

    let ts = parse(
        "export const x: number = 1;\n",
        "a.ts",
        &Language::TypeScript,
        "a",
        "app",
    )
    .expect("parse");
    assert_eq!(ts.status, ParseStatus::Parsed, "error: {:?}", ts.error);

    let js = parse(
        "export const y = 2;\n",
        "a.js",
        &Language::JavaScript,
        "a",
        "app",
    )
    .expect("parse");
    assert_eq!(js.status, ParseStatus::Parsed, "error: {:?}", js.error);
}
