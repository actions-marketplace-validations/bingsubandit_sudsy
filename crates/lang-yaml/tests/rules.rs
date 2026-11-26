//! YAML rules: expected diagnostics (snapshots) and fix output.

use insta::assert_snapshot;
use provider_api::testing::{lint, render};

fn report(text: &str, rules: &str) -> String {
    render(text, &lint(&lang_yaml::provider(), "a.yaml", text, rules))
}

#[test]
fn suppression_comment() {
    let text = "a: 1\n# sudsy-disable-next-line no-duplicate-keys\na: 2\na: 3\n";
    assert_snapshot!(report(text, ""));
}

fn dup_count(text: &str) -> usize {
    report(text, "").matches("no-duplicate-keys").count()
}

#[test]
fn block_line_and_file_suppressions() {
    assert_eq!(dup_count("a: 1\na: 2\n"), 1);
    // Line, with and without a rule list; another rule's name does not apply.
    assert_eq!(
        dup_count("a: 1\na: 2 # sudsy-disable-line no-duplicate-keys\n"),
        0
    );
    assert_eq!(dup_count("a: 1\na: 2 # sudsy-disable-line\n"), 0);
    assert_eq!(
        dup_count("a: 1\na: 2 # sudsy-disable-line max-line-length\n"),
        1
    );
    // Block: only between disable and enable; enable may name one rule.
    let block = |open: &str, close: &str| format!("{open}\na: 1\na: 2\n{close}\nb: 1\nb: 2\n");
    assert_eq!(dup_count(&block("# sudsy-disable", "# sudsy-enable")), 1);
    assert_eq!(
        dup_count(&block(
            "# sudsy-disable no-duplicate-keys",
            "# sudsy-enable"
        )),
        1
    );
    assert_eq!(
        dup_count(&block(
            "# sudsy-disable",
            "# sudsy-enable no-duplicate-keys"
        )),
        1
    );
    assert_eq!(
        dup_count(&block("# sudsy-disable", "# sudsy-enable max-line-length")),
        0
    );
    assert_eq!(dup_count(&block("# sudsy-disable", "# nothing")), 0);
    // File: anywhere in the file.
    assert_eq!(
        dup_count("a: 1\na: 2\n# sudsy-disable-file no-duplicate-keys\n"),
        0
    );
    assert_eq!(dup_count("# sudsy-disable-file\na: 1\na: 2\n"), 0);
}

#[test]
fn ambiguous_booleans_flag_every_casing_and_honor_check_keys_and_allowed() {
    let text = "on: True\nlist: [FALSE, false, true]\n? Yes\n: y\n";
    let all = report(text, "ambiguous-booleans: warn");
    assert_eq!(all.matches("ambiguous-booleans").count(), 4, "{all}");
    let rules = "ambiguous-booleans:\n  level: warn\n  options:\n    check_keys: false";
    let values = report(text, rules);
    assert_eq!(values.matches("ambiguous-booleans").count(), 2, "{values}");
    let rules = "ambiguous-booleans:\n  level: warn\n  options:\n    allowed: [\"True\", \"on\"]";
    let allowed = report(text, rules);
    assert_eq!(
        allowed.matches("ambiguous-booleans").count(),
        2,
        "{allowed}"
    );
}

#[test]
fn max_line_length_allows_non_breakable_words() {
    let u = "u".repeat(30);
    let text = format!("# {u}\n- {u}\n- k: {u}\n");
    let rules = |allow: bool| {
        format!(
            "max-line-length:\n  level: warn\n  options:\n    max: 20\n    allow_non_breakable_words: {allow}"
        )
    };
    let kept = report(&text, &rules(true));
    assert_eq!(kept.matches("max-line-length").count(), 1, "{kept}");
    assert_eq!(
        report(&text, &rules(false))
            .matches("max-line-length")
            .count(),
        3
    );
}

fn rule_lines(path: &str, text: &str, rule: &str) -> Vec<usize> {
    lint(&lang_yaml::provider(), path, text, &format!("{rule}: warn"))
        .iter()
        .filter(|d| d.rule == rule)
        .map(|d| 1 + text[..usize::from(d.range.start())].matches('\n').count())
        .collect()
}

#[test]
fn role_variables_need_the_role_name_as_prefix() {
    let rule = "ansible-role-var-prefix";
    let defaults = "nginx_port: 80\n_nginx_hidden: 1\nwww_port: 8080\nansible_user: x\nBad: 1\n";
    assert_eq!(
        rule_lines("roles/nginx/defaults/main.yml", defaults, rule),
        [3]
    );
    assert!(rule_lines("defaults/main.yml", defaults, rule).is_empty());
    let tasks = "- name: A\n  ansible.builtin.ping:\n  register: result\n- name: B\n  ansible.builtin.set_fact:\n    nginx_ok: 1\n    other: 2\n";
    assert_eq!(
        rule_lines("r/roles/nginx/tasks/main.yml", tasks, rule),
        [1, 4]
    );
    let included =
        "- ansible.builtin.include_role:\n    name: web\n  vars:\n    web_a: 1\n    b: 2\n";
    assert_eq!(rule_lines("play/tasks/main.yml", included, rule), [5]);
}

#[test]
fn flow_closer_may_sit_one_column_left_of_the_content() {
    let parsed = lang_yaml::parse("k:\n  v: [\n    a,\n   ]\n  w: {\n   a: 1,\n  }\n");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
}
