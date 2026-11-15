//! The formatter against `ruff format` (0.17.4, defaults): each expectation is ruff's output for
//! its input, and every one passes the shared checks (lossless, meaning, comments, idempotency).

use core_config::OptionValue;
use core_format::testing::assert_formats;
use provider_api::format_testing::Formatting;

fn fmt(input: &str) -> String {
    assert_formats(&Formatting::new(lang_python::provider(), "a.py"), input)
}

fn fmt_with(options: Vec<(&'static str, OptionValue)>, input: &str) -> String {
    let mut subject = Formatting::new(lang_python::provider(), "a.py");
    subject.options = options;
    assert_formats(&subject, input)
}

#[test]
fn blank_lines() {
    assert_eq!(
        fmt(
            "import os\nimport sys\nx = 1\ndef f():\n    \"\"\"Doc.\"\"\"\n    y = 1\n    class B: pass\n    return y\nclass C:\n\n    a = 1\n    def g(self): pass\n\n\n\n    # trailing\nif x:\n    pass\nelif y: pass\nelse:\n\n    z = [1, 2,]\n"
        ),
        "import os\nimport sys\n\nx = 1\n\n\ndef f():\n    \"\"\"Doc.\"\"\"\n    y = 1\n\n    class B:\n        pass\n\n    return y\n\n\nclass C:\n    a = 1\n\n    def g(self):\n        pass\n\n    # trailing\n\n\nif x:\n    pass\nelif y:\n    pass\nelse:\n    z = [\n        1,\n        2,\n    ]\n"
    );
}

#[test]
fn brackets() {
    assert_eq!(
        fmt(
            "call(aaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccccc, ddddddddd)\ncall(a, b)\ncall(a, b,)\nx = {\"key\": value_one, \"other\": [1, 2, 3], **rest}\ndef function(argument_number_one, argument_number_two, argument_number_three, argument_four) -> int: pass\nfrom module import (a, b)\nfrom module import aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccc\n"
        ),
        "call(\n    aaaaaaaaaaaaaaaaaaaaaaaaaaaaa,\n    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,\n    cccccccccccccccccccccccccccc,\n    ddddddddd,\n)\ncall(a, b)\ncall(\n    a,\n    b,\n)\nx = {\"key\": value_one, \"other\": [1, 2, 3], **rest}\n\n\ndef function(\n    argument_number_one, argument_number_two, argument_number_three, argument_four\n) -> int:\n    pass\n\n\nfrom module import a, b\nfrom module import (\n    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,\n    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,\n    ccccccccccccccccccccc,\n)\n"
    );
}

#[test]
fn operators() {
    assert_eq!(
        fmt(
            "result = some_function_name(argument_one) + another_function_name(argument_two, argument_three)\nvalue = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + cccccccccccccccccccccccccccccc\nif aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa and bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb or cccccccccccccccccc:\n    pass\nx = a if b else c\ny = a**2 + b ** c.d - e[1:2] + f[a+1 :]\nprint(* args, ** kwargs)\nz = not x\nreturn_ = (yield)\n"
        ),
        "result = some_function_name(argument_one) + another_function_name(\n    argument_two, argument_three\n)\nvalue = (\n    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n    + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n    + cccccccccccccccccccccccccccccc\n)\nif (\n    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n    and bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n    or cccccccccccccccccc\n):\n    pass\nx = a if b else c\ny = a**2 + b**c.d - e[1:2] + f[a + 1 :]\nprint(*args, **kwargs)\nz = not x\nreturn_ = yield\n"
    );
}

#[test]
fn chains() {
    assert_eq!(
        fmt(
            "session.query(User).filter(User.id == 1).filter(User.name == \"aaaaaaaaaaaaaaaaaaaaaaaa\").all()\nx = session.query(User).filter(User.id == 1).filter(User.name == \"aaaaaaaaaaaaaaaaaaaaaaaa\").all()\nsome_object.some_attribute.another_attribute.yet_another.attr.more.attribute.more = some_function_name(argument_one)\n"
        ),
        "session.query(User).filter(User.id == 1).filter(\n    User.name == \"aaaaaaaaaaaaaaaaaaaaaaaa\"\n).all()\nx = (\n    session.query(User)\n    .filter(User.id == 1)\n    .filter(User.name == \"aaaaaaaaaaaaaaaaaaaaaaaa\")\n    .all()\n)\nsome_object.some_attribute.another_attribute.yet_another.attr.more.attribute.more = (\n    some_function_name(argument_one)\n)\n"
    );
}

#[test]
fn strings() {
    assert_eq!(
        fmt(
            "a = 'single'\nb = \"it's\"\nc = 'say \"hi\"'\nd = U\"unicode\" + B\"bytes\" + Rb\"raw\"\ne = \"implicit\" \"concatenation\"\nf = (\"long implicit concatenated string that does not fit on the line \" \"at all because it is long\")\ng = 0XFF + 1E5 + 0O17 + 1_0j\ndef h():\n    '''  Docstring with trailing space.   \n\n       indented more\n    '''\n"
        ),
        "a = \"single\"\nb = \"it's\"\nc = 'say \"hi\"'\nd = \"unicode\" + b\"bytes\" + Rb\"raw\"\ne = \"implicitconcatenation\"\nf = (\n    \"long implicit concatenated string that does not fit on the line \"\n    \"at all because it is long\"\n)\ng = 0xFF + 1e5 + 0o17 + 1_0j\n\n\ndef h():\n    \"\"\"Docstring with trailing space.\n\n    indented more\n    \"\"\"\n"
    );
}

#[test]
fn comments() {
    assert_eq!(
        fmt(
            "#comment without space\nx = 1 # trailing\n#!not a shebang\ndef f(\n    a,  # first\n    b,\n):\n    # leading\n    pass\n    # end of block\n\n# between\ny = [\n    1,\n    # inside\n]\n"
        ),
        "# comment without space\nx = 1  # trailing\n\n\n#!not a shebang\ndef f(\n    a,  # first\n    b,\n):\n    # leading\n    pass\n    # end of block\n\n\n# between\ny = [\n    1,\n    # inside\n]\n"
    );
}

#[test]
fn ignore() {
    assert_eq!(
        fmt(
            "# fmt: off\nx   =   [1,2,\n         3]\n# fmt: on\ny   =   [1,2]\nz   =   {1:2}  # fmt: skip\n"
        ),
        "# fmt: off\nx   =   [1,2,\n         3]\n# fmt: on\ny = [1, 2]\nz   =   {1:2}  # fmt: skip\n"
    );
}

#[test]
fn statements() {
    assert_eq!(
        fmt(
            "with open(\"a\") as f, open(\"b\") as g:\n    pass\nwith (open(\"a\") as f, open(\"b\") as g,):\n    pass\ntry:\n    pass\nexcept (A, B) as e:\n    raise X from e\nelse:\n    pass\nfinally:\n    pass\nfor (i, j) in pairs:\n    pass\nmatch command:\n    case [1, *rest] if rest:\n        pass\n    case {\"k\": v} | None:\n        pass\ndel (a), b\nassert x, \"message\"\nglobal_ = lambda x, *a, k=1: x\nasync def f():\n    async with a as b:\n        await c\nclass A(): ...\nclass B(Base, metaclass=M):\n    x: int = 1\n"
        ),
        "with open(\"a\") as f, open(\"b\") as g:\n    pass\nwith (\n    open(\"a\") as f,\n    open(\"b\") as g,\n):\n    pass\ntry:\n    pass\nexcept (A, B) as e:\n    raise X from e\nelse:\n    pass\nfinally:\n    pass\nfor i, j in pairs:\n    pass\nmatch command:\n    case [1, *rest] if rest:\n        pass\n    case {\"k\": v} | None:\n        pass\ndel (a), b\nassert x, \"message\"\nglobal_ = lambda x, *a, k=1: x\n\n\nasync def f():\n    async with a as b:\n        await c\n\n\nclass A: ...\n\n\nclass B(Base, metaclass=M):\n    x: int = 1\n"
    );
}

#[test]
fn sudsy_ignore_keeps_a_statement() {
    assert_eq!(
        fmt("# sudsy-ignore\nw   =   (1,)\nv   =   (1,)\n"),
        "# sudsy-ignore\nw   =   (1,)\nv = (1,)\n"
    );
}

#[test]
fn options() {
    assert_eq!(
        fmt_with(
            vec![("quote_style", OptionValue::String("single".into()))],
            "a = \"x\"\nb = \"it's\"\n"
        ),
        "a = 'x'\nb = \"it's\"\n"
    );
    assert_eq!(
        fmt_with(
            vec![("magic_trailing_comma", OptionValue::String("ignore".into()))],
            "f(a, b,)\nx = [1, 2,]\n"
        ),
        "f(a, b)\nx = [1, 2]\n"
    );
    assert_eq!(
        fmt_with(
            vec![("line_width", OptionValue::Int(30))],
            "function_name(aaa, bbb, ccc, ddd)\n"
        ),
        "function_name(\n    aaa, bbb, ccc, ddd\n)\n"
    );
    assert_eq!(
        fmt_with(
            vec![("indent_style", OptionValue::String("tab".into()))],
            "if x:\n    y\n"
        ),
        "if x:\n\ty\n"
    );
}
