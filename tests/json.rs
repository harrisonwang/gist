use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn pith_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pith"))
}

fn fixture_path(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

fn pith_json(args: &[String]) -> Value {
    let output = pith_bin().args(args).output().expect("run pith json");

    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn json_args(paths: &[&str]) -> Vec<String> {
    let mut args = vec!["--mode".to_string(), "json".to_string()];
    args.extend(paths.iter().map(|path| fixture_path(path)));
    args
}

fn blocks(value: &Value) -> &[Value] {
    value["documents"][0]["blocks"].as_array().unwrap()
}

#[test]
fn plain_text_uses_json_v1_document_and_paragraph_block() {
    let value = pith_json(&json_args(&["plain/01_ascii.txt"]));

    assert_eq!(value["mode"], "json");
    assert_eq!(value["schema_version"], "pith-json-v1");
    assert_eq!(value["warnings"], json!([]));
    assert!(!value.as_object().unwrap().contains_key("status"));
    assert!(!value.as_object().unwrap().contains_key("content"));

    let document = &value["documents"][0];
    assert_eq!(document["format"], "text");
    assert_eq!(document["blocks"][0]["id"], "b1");
    assert_eq!(document["blocks"][0]["kind"], "paragraph");
    assert_eq!(document["blocks"][0]["text"], "Hello world\nLine two");
    assert_eq!(document["blocks"][0]["source_anchor"]["path"], json!([]));
    assert_eq!(document["blocks"][0]["truncated"], false);
    assert_eq!(document["blocks"][0]["warnings"], json!([]));
}

#[test]
fn multi_input_json_always_uses_documents_array() {
    let value = pith_json(&json_args(&["plain/01_ascii.txt", "plain/02_utf8.txt"]));

    let documents = value["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["format"], "text");
    assert_eq!(documents[1]["format"], "text");
    assert!(
        documents[1]["blocks"][0]["text"]
            .as_str()
            .unwrap()
            .contains("中文")
    );
}

#[test]
fn docx_headings_set_level_and_path_anchor() {
    let value = pith_json(&json_args(&["docx/01_basic.docx"]));
    let blocks = blocks(&value);

    let heading = blocks
        .iter()
        .find(|block| block["text"] == "First Heading")
        .unwrap();
    assert_eq!(heading["kind"], "heading");
    assert_eq!(heading["level"], 1);
    assert_eq!(heading["source_anchor"]["path"], json!(["First Heading"]));

    let paragraph = blocks
        .iter()
        .find(|block| block["text"] == "A normal paragraph.")
        .unwrap();
    assert_eq!(paragraph["kind"], "paragraph");
    assert_eq!(paragraph["source_anchor"]["path"], json!(["First Heading"]));
}

#[test]
fn docx_lists_are_list_blocks() {
    let value = pith_json(&json_args(&["docx/02_lists.docx"]));
    let blocks = blocks(&value);

    let list = blocks.iter().find(|block| block["kind"] == "list").unwrap();
    assert_eq!(list["text"], "- Bullet 1");
    assert_eq!(list["source_anchor"]["path"], json!(["Lists"]));
}

#[test]
fn pptx_blocks_carry_slide_anchor() {
    let value = pith_json(&json_args(&["pptx/01_basic.pptx"]));
    let blocks = blocks(&value);

    let slide_heading = blocks
        .iter()
        .find(|block| block["text"] == "Slide 1")
        .unwrap();
    assert_eq!(slide_heading["kind"], "heading");
    assert_eq!(slide_heading["slide"], 1);

    let slide_body = blocks
        .iter()
        .find(|block| block["text"] == "Title Slide")
        .unwrap();
    assert_eq!(slide_body["kind"], "paragraph");
    assert_eq!(slide_body["slide"], 1);
}

#[test]
fn xlsx_tables_carry_sheet_anchor() {
    let value = pith_json(&json_args(&["xlsx/02_multi_sheets.xlsx"]));
    let blocks = blocks(&value);

    let heading = blocks
        .iter()
        .find(|block| block["text"] == "Sheet: First")
        .unwrap();
    assert_eq!(heading["kind"], "heading");
    assert_eq!(heading["sheet"], "First");

    let table = blocks
        .iter()
        .find(|block| block["kind"] == "table")
        .unwrap();
    assert_eq!(table["sheet"], "First");
    assert!(table["text"].as_str().unwrap().contains("| a | b |"));
}

#[test]
fn ipynb_fenced_code_blocks_keep_language() {
    let value = pith_json(&json_args(&["ipynb/01_basic.ipynb"]));
    let blocks = blocks(&value);

    let code = blocks.iter().find(|block| block["kind"] == "code").unwrap();
    assert_eq!(code["language"], "python");
    assert_eq!(code["text"], "print('hello')\n42");
    assert_eq!(code["source_anchor"]["path"], json!(["Notebook title"]));
}

#[test]
fn large_csv_marks_table_truncated_with_warning() {
    let value = pith_json(&json_args(&["csv/10_large.csv"]));
    let document = &value["documents"][0];
    let table = &document["blocks"][0];

    assert_eq!(table["kind"], "table");
    assert_eq!(table["truncated"], true);
    assert_eq!(table["warnings"], json!(["truncated at 1000 rows"]));
    assert_eq!(document["warnings"], json!(["truncated at 1000 rows"]));
}
