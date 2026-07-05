use openakb_validate::extract_citations;

fn ids(markdown: &str) -> Vec<Vec<String>> {
    extract_citations(markdown)
        .into_iter()
        .map(|citation| citation.ids)
        .collect()
}

#[test]
fn test_simple_marker() {
    assert_eq!(ids("A fact. [cite: s1]"), vec![vec!["s1"]]);
}

#[test]
fn test_comma_list() {
    assert_eq!(ids("[cite:a, b\t,c]"), vec![vec!["a", "b", "c"]]);
}

#[test]
fn test_duplicates_preserved() {
    assert_eq!(ids("[cite:a,a]"), vec![vec!["a", "a"]]);
}

#[test]
fn test_concatenated_markers() {
    assert_eq!(ids("[cite:a][cite:b]"), vec![vec!["a"], vec!["b"]]);
}

#[test]
fn test_code_fence_ignored() {
    assert_eq!(ids("```text\n[cite:a]\n```\n\n[cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_tilde_fence_ignored() {
    assert_eq!(ids("~~~\n[cite:a]\n~~~\n\n[cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_indented_code_ignored() {
    assert_eq!(ids("    [cite:a]\n\n[cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_inline_code_ignored() {
    assert_eq!(
        ids("Literal `[cite:a]` and prose [cite:b]."),
        vec![vec!["b"]]
    );
}

#[test]
fn test_multi_backtick_ignored() {
    assert_eq!(ids("``x ` [cite:a]`` [cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_unterminated_span_live() {
    assert_eq!(ids("Text `x [cite:a] more"), vec![vec!["a"]]);
}

#[test]
fn test_html_block_ignored() {
    assert_eq!(
        ids("<section>\n[cite:a]\n</section>\n\n[cite:b]"),
        vec![vec!["b"]]
    );
}

#[test]
fn test_html_comment_ignored() {
    assert_eq!(ids("<!-- [cite:a] -->\n\n[cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_comment_blocks_ignored() {
    assert!(ids("<!-- hidden [cite:x] --> [cite:y]").is_empty());
    assert!(ids("<!--> [cite:a]").is_empty());
    assert!(ids("<!---> [cite:b]").is_empty());
}

#[test]
fn test_inline_degenerate_comments() {
    assert_eq!(ids("text <!--> [cite:a]"), vec![vec!["a"]]);
    assert_eq!(ids("text <!---> [cite:b]"), vec![vec!["b"]]);
}

#[test]
fn test_unclosed_comment_live() {
    assert!(ids("<!-- [cite:a]").is_empty());
    assert_eq!(ids("text <!-- [cite:a]"), vec![vec!["a"]]);
}

#[test]
fn test_inline_html_not_masked() {
    assert_eq!(
        ids(r#"<span data-x="[cite:a]">label</span>"#),
        vec![vec!["a"]]
    );
}

#[test]
fn test_raw_source_rules() {
    assert_eq!(ids(r"\[cite:a]"), vec![vec!["a"]]);
    assert_eq!(ids("[[cite:a]]"), vec![vec!["a"]]);
    assert!(ids("&#91;cite:a]").is_empty());
}

#[test]
fn test_underscore_id() {
    assert_eq!(ids("[cite:_a_]"), vec![vec!["_a_"]]);
}

#[test]
fn test_malformed_markers_literal() {
    assert!(ids("[cite:] [cite: A] [cite:a b] [cite:a,] [CITE:a]").is_empty());
}

#[test]
fn test_length_boundary() {
    let valid = "a".repeat(64);
    let invalid = "a".repeat(65);
    assert_eq!(ids(&format!("[cite:{valid}]")), vec![vec![valid]]);
    assert!(ids(&format!("[cite:{invalid}]")).is_empty());
}

#[test]
fn test_crlf_normalized() {
    assert_eq!(
        ids("```text\r\n[cite:a]\r\n```\r\n\r\n[cite:b]"),
        vec![vec!["b"]]
    );
}

#[test]
fn test_nul_normalized() {
    assert!(ids("[ci\0te:a]").is_empty());
}
