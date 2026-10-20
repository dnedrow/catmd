//! End-to-end tests of the `catmd` binary through a pipe (never a terminal).

use assert_cmd::Command;
use predicates::prelude::*;

/// A `catmd` command with every environment variable that influences output removed,
/// so each test controls exactly what the binary sees.
fn catmd() -> Command {
    let mut cmd = Command::cargo_bin("catmd").expect("binary builds");
    for var in [
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "TERM",
        "COLORTERM",
        "COLUMNS",
        "LINES",
        "LC_ALL",
        "LC_CTYPE",
        "LANG",
        "CATMD_PAGER",
        "PAGER",
        "LESS",
        "CATMD_THEME",
        "XDG_CONFIG_HOME",
        "HOME",
    ] {
        cmd.env_remove(var);
    }
    cmd
}

fn usage_error() -> impl Predicate<str> {
    predicate::str::contains("Usage:").or(predicate::str::contains("For more information"))
}

#[test]
fn help_lists_every_flag() {
    catmd().arg("--help").assert().success().stdout(
        predicate::str::contains("--color")
            .and(predicate::str::contains("--width"))
            .and(predicate::str::contains("--height"))
            .and(predicate::str::contains("--ascii"))
            .and(predicate::str::contains("--no-pager"))
            .and(predicate::str::contains("[FILE]")),
    );
}

#[test]
fn version_prints_cargo_version() {
    catmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn invalid_color_value_is_a_usage_error() {
    catmd()
        .args(["--color", "sometimes"])
        .assert()
        .code(2)
        .stderr(usage_error());
}

#[test]
fn non_numeric_width_is_a_usage_error() {
    catmd()
        .args(["--width", "wide"])
        .assert()
        .code(2)
        .stderr(usage_error());
}

#[test]
fn zero_height_is_a_usage_error() {
    catmd()
        .args(["--height", "0"])
        .assert()
        .code(2)
        .stderr(usage_error());
}

#[test]
fn invalid_color_depth_is_a_usage_error() {
    catmd()
        .args(["--color-depth", "88"])
        .assert()
        .code(2)
        .stderr(usage_error());
}

#[test]
fn help_lists_color_depth() {
    catmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--color-depth <DEPTH>"));
}

#[test]
fn zero_width_is_a_usage_error() {
    catmd()
        .args(["--width", "0"])
        .assert()
        .code(2)
        .stderr(usage_error());
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A throwaway directory removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "catmd-test-{}-{}-{}",
            std::process::id(),
            name,
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        TempDir(dir)
    }

    fn file(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).expect("write fixture");
        path
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// input-sources
// ---------------------------------------------------------------------------

#[test]
fn single_file_is_rendered() {
    let dir = TempDir::new("single");
    let a = dir.file("a.md", "# Alpha\n");
    catmd().arg(&a).assert().success().stdout("# Alpha\n");
}

#[test]
fn multiple_files_keep_argument_order() {
    let dir = TempDir::new("order");
    let a = dir.file("a.md", "# Alpha\n");
    let b = dir.file("b.md", "# Beta\n");
    catmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout("# Alpha\n\n# Beta\n");
}

#[test]
fn stdin_is_read_when_no_file_is_given() {
    catmd()
        .write_stdin("# Hi\n")
        .assert()
        .success()
        .stdout("# Hi\n");
}

#[test]
fn dash_reads_stdin_in_position() {
    let dir = TempDir::new("dash");
    let a = dir.file("a.md", "A\n");
    let b = dir.file("b.md", "B\n");
    catmd()
        .arg(&a)
        .arg("-")
        .arg(&b)
        .write_stdin("middle")
        .assert()
        .success()
        .stdout("A\n\nmiddle\n\nB\n");
}

#[test]
fn paragraphs_do_not_merge_across_files() {
    let dir = TempDir::new("merge");
    let a = dir.file("a.md", "end of a");
    let b = dir.file("b.md", "start of b\n");
    catmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout("end of a\n\nstart of b\n");
}

#[test]
fn reference_links_do_not_leak_across_inputs() {
    let dir = TempDir::new("refs");
    let a = dir.file("a.md", "[ref]: https://example.com\n");
    let b = dir.file("b.md", "[text][ref]\n");
    catmd()
        .args([&a, &b])
        .assert()
        .success()
        .stdout("[text][ref]\n");
}

#[test]
fn missing_file_is_reported_and_the_rest_still_renders() {
    let dir = TempDir::new("missing");
    let a = dir.file("a.md", "A\n");
    let b = dir.file("b.md", "B\n");
    let missing = dir.path().join("missing.md");
    catmd()
        .arg(&a)
        .arg(&missing)
        .arg(&b)
        .assert()
        .code(1)
        .stdout("A\n\nB\n")
        .stderr(predicate::str::contains("missing.md").and(predicate::str::starts_with("catmd: ")));
}

#[test]
fn directory_input_is_an_error() {
    let dir = TempDir::new("dir");
    catmd()
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            dir.path().to_string_lossy().as_ref(),
        ));
}

#[test]
fn all_readable_inputs_exit_zero() {
    let dir = TempDir::new("ok");
    let a = dir.file("a.md", "A\n");
    let b = dir.file("b.md", "B\n");
    catmd().args([&a, &b]).assert().code(0);
}

#[test]
fn invalid_utf8_is_replaced() {
    let dir = TempDir::new("utf8");
    let f = dir.file("bad.md", b"hello \xff world\n");
    catmd()
        .arg(&f)
        .assert()
        .success()
        .stdout("hello \u{FFFD} world\n");
}

#[test]
fn empty_file_produces_empty_output() {
    let dir = TempDir::new("empty");
    let f = dir.file("empty.md", "");
    catmd().arg(&f).assert().success().stdout("");
    let ws = dir.file("ws.md", "  \n\n");
    catmd().arg(&ws).assert().success().stdout("");
}

// ---------------------------------------------------------------------------
// color-output through a pipe
// ---------------------------------------------------------------------------

#[test]
fn color_always_forces_styling_into_a_pipe() {
    catmd()
        .args(["--color", "always"])
        .write_stdin("**bold**\n")
        .assert()
        .success()
        .stdout("\x1b[1mbold\x1b[0m\n");
}

#[test]
fn auto_mode_emits_no_escapes_into_a_pipe() {
    catmd()
        .env("TERM", "xterm-256color")
        .write_stdin("# Title\n\n**bold** and `code`\n")
        .assert()
        .success()
        .stdout(predicate::str::contains('\x1b').not());
}

#[test]
fn color_never_emits_no_escapes() {
    catmd()
        .args(["--color", "never"])
        .env("CLICOLOR_FORCE", "1")
        .write_stdin("**bold**\n")
        .assert()
        .success()
        .stdout("bold\n");
}

#[test]
fn clicolor_force_styles_a_pipe() {
    catmd()
        .env("CLICOLOR_FORCE", "1")
        .write_stdin("**bold**\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1m"));
}

#[test]
fn no_color_is_overridden_by_color_always() {
    catmd()
        .env("NO_COLOR", "1")
        .args(["--color", "always"])
        .write_stdin("**bold**\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1m"));
}

#[test]
fn no_color_beats_clicolor_force_in_auto_mode() {
    catmd()
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .write_stdin("**bold**\n")
        .assert()
        .success()
        .stdout("bold\n");
}

const EVERY_CONSTRUCT: &str = "# H1\n\n## H2\n\n### H3\n\n**b** *i* ~~s~~ `c` [l](u) ![a](s)\n\n> [!CAUTION]\n> x\n\n- [ ] t\n\n1. n\n\n| A |\n|---|\n| 1 |\n\n```\ncode\n```\n\n---\n\ntext[^1]\n\n[^1]: note\n";

fn no_deep_colors() -> impl Predicate<str> {
    predicate::str::contains("38;5;")
        .not()
        .and(predicate::str::contains("38;2;").not())
        .and(predicate::str::contains("48;").not())
}

#[test]
fn styled_output_uses_only_the_16_color_palette_at_16_colors() {
    catmd()
        .env("TERM", "xterm")
        .args(["--color", "always"])
        .write_stdin(EVERY_CONSTRUCT)
        .assert()
        .success()
        .stdout(no_deep_colors());
}

#[test]
fn default_theme_stays_portable_on_a_truecolor_terminal() {
    catmd()
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .args(["--color", "always"])
        .write_stdin(EVERY_CONSTRUCT)
        .assert()
        .success()
        .stdout(no_deep_colors().and(predicate::str::contains("\x1b[1m")));
}

// ---------------------------------------------------------------------------
// terminal-geometry through a pipe
// ---------------------------------------------------------------------------

#[test]
fn columns_sets_the_width_when_piped() {
    catmd()
        .env("COLUMNS", "60")
        .arg("--ascii")
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "-".repeat(60)));
}

#[test]
fn width_defaults_to_80_when_piped_without_columns() {
    catmd()
        .arg("--ascii")
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "-".repeat(80)));
}

#[test]
fn width_flag_overrides_columns() {
    catmd()
        .env("COLUMNS", "60")
        .args(["--ascii", "--width", "30"])
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "-".repeat(30)));
}

#[test]
fn tiny_width_is_clamped_to_20() {
    catmd()
        .args(["--ascii", "--width", "5"])
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "-".repeat(20)));
}

// ---------------------------------------------------------------------------
// glyph selection
// ---------------------------------------------------------------------------

#[test]
fn utf8_locale_selects_unicode_glyphs() {
    catmd()
        .env("LANG", "en_US.UTF-8")
        .args(["--width", "20"])
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "─".repeat(20)));
}

#[test]
fn c_locale_selects_ascii_glyphs() {
    catmd()
        .env("LC_ALL", "C")
        .env("LANG", "en_US.UTF-8")
        .args(["--width", "20"])
        .write_stdin("---\n")
        .assert()
        .success()
        .stdout(format!("{}\n", "-".repeat(20)));
}

#[test]
fn no_locale_variables_select_unicode_glyphs() {
    catmd()
        .write_stdin("- item\n")
        .assert()
        .success()
        .stdout("• item\n");
}

#[test]
fn ascii_flag_overrides_a_utf8_locale() {
    catmd()
        .env("LANG", "en_US.UTF-8")
        .arg("--ascii")
        .write_stdin("- item\n")
        .assert()
        .success()
        .stdout("* item\n");
}

// ---------------------------------------------------------------------------
// broken pipe
// ---------------------------------------------------------------------------

#[test]
fn closed_stdout_exits_quietly_with_success() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command as StdCommand, Stdio};

    let dir = TempDir::new("pipe");
    let big: String = (0..20_000).map(|i| format!("line {i}\n\n")).collect();
    let f = dir.file("big.md", big);

    let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin("catmd"))
        .arg(&f)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn catmd");
    {
        let stdout = child.stdout.take().expect("piped stdout");
        let mut first = String::new();
        BufReader::new(stdout)
            .read_line(&mut first)
            .expect("read a line");
        assert_eq!(first, "line 0\n");
        // Dropping the reader closes the pipe while catmd still has data to write.
    }
    let output = child.wait_with_output().expect("wait for catmd");
    assert!(output.status.success(), "{:?}", output.status);
    assert!(
        output.stderr.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// output-paging through a pipe
// ---------------------------------------------------------------------------

#[test]
fn piped_output_is_never_paged_even_with_height() {
    let doc: String = (0..60).map(|i| format!("line {i}\n\n")).collect();
    // A bogus pager would produce a stderr message if catmd ever tried to start it.
    catmd()
        .env("PAGER", "definitely-not-a-real-pager")
        .args(["--height", "5"])
        .write_stdin(doc.clone())
        .assert()
        .success()
        .stdout(doc.trim_end_matches('\n').to_string() + "\n")
        .stderr("");
}

// ---------------------------------------------------------------------------
// theming
// ---------------------------------------------------------------------------

/// `catmd --color always` at 16 colors (TERM=xterm, no COLORTERM).
fn styled16() -> Command {
    let mut cmd = catmd();
    cmd.env("TERM", "xterm").args(["--color", "always"]);
    cmd
}

fn theme_fails(dir: &TempDir, toml: &str, token: &str) {
    let t = dir.file("bad.toml", toml);
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("`x`\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("bad.toml").and(predicate::str::contains(token)));
}

#[test]
fn minimal_theme_restyles_one_role() {
    let dir = TempDir::new("theme-min");
    let t = dir.file("t.toml", "code = \"red\"\n");
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
}

#[test]
fn theme_is_ignored_when_styling_is_off() {
    let dir = TempDir::new("theme-off");
    let t = dir.file("t.toml", "code = \"red\"\nstrong = \"red\"\n");
    let md = "# T\n\n**b** and `x`\n";
    let plain = catmd()
        .args(["--color", "never"])
        .write_stdin(md)
        .output()
        .unwrap()
        .stdout;
    catmd()
        .args(["--color", "never", "--theme"])
        .arg(&t)
        .write_stdin(md)
        .assert()
        .success()
        .stdout(String::from_utf8(plain).unwrap());
}

#[test]
fn partial_theme_leaves_unmentioned_roles_identical_to_the_default() {
    let dir = TempDir::new("theme-partial");
    let t = dir.file("t.toml", "code = \"red\"\n");
    let md = "# T\n";
    let without_theme = styled16().write_stdin(md).output().unwrap().stdout;
    let without_theme = String::from_utf8(without_theme).unwrap();
    // The heading keeps its built-in bold + underline + bright magenta.
    assert!(
        without_theme.contains("\x1b[1m\x1b[4m\x1b[95mT\x1b[0m"),
        "{without_theme:?}"
    );
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin(md)
        .assert()
        .success()
        .stdout(without_theme);
}

#[test]
fn table_form_sets_color_and_modifiers() {
    let dir = TempDir::new("theme-table");
    let t = dir.file(
        "t.toml",
        "heading1 = { fg = \"cyan\", modifiers = [\"bold\", \"underline\"] }\n",
    );
    let out = styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("# T\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.contains("\x1b[1m") && out.contains("\x1b[4m") && out.contains("\x1b[36m"),
        "{out:?}"
    );
    assert!(
        !out.contains("\x1b[35m") && !out.contains("\x1b[95m"),
        "{out:?}"
    );
}

#[test]
fn bare_string_replaces_modifiers_too() {
    let dir = TempDir::new("theme-bare");
    let t = dir.file("t.toml", "strong = \"red\"\n");
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("**b**\n")
        .assert()
        .success()
        .stdout("\x1b[31mb\x1b[0m\n");
}

#[test]
fn modifiers_without_a_color() {
    let dir = TempDir::new("theme-mods");
    let t = dir.file("t.toml", "link_url = { modifiers = [\"dim\"] }\n");
    let out = styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("[d](https://x.y)\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(out.contains("(\x1b[2mhttps://x.y\x1b[0m)"), "{out:?}");
}

#[test]
fn every_color_form_is_accepted_and_emitted_at_truecolor() {
    let dir = TempDir::new("theme-forms");
    let t = dir.file(
        "t.toml",
        "code = \"bright_yellow\"\nemph = 208\nstrong = \"#FF8800\"\nlink_text = \"accent\"\n[palette]\naccent = \"#00aaff\"\n",
    );
    let out = catmd()
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .args(["--color", "always", "--theme"])
        .arg(&t)
        .write_stdin("`c` *e* **s** [l](https://x.y)\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(out.contains("\x1b[93mc\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[38;5;208me\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[38;2;255;136;0ms\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[38;2;0;170;255ml\x1b[0m"), "{out:?}");
}

#[test]
fn palette_names_are_resolved_before_ansi_names() {
    let dir = TempDir::new("theme-palette");
    let t = dir.file("t.toml", "code = \"red\"\n[palette]\nred = 196\n");
    catmd()
        .env("TERM", "xterm-256color")
        .args(["--color", "always", "--theme"])
        .arg(&t)
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[38;5;196mx\x1b[0m\n");
}

#[test]
fn theme_setting_every_role_loads() {
    let dir = TempDir::new("theme-all");
    assert_eq!(catmd::Theme::ROLES.len(), 36);
    let toml: String = catmd::Theme::ROLES
        .iter()
        .map(|r| format!("{r} = \"green\"\n"))
        .collect();
    let t = dir.file("all.toml", toml);
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("# T\n\n- a\n")
        .assert()
        .success();
}

/// A theme written for the 29 roles that predate the `syntax_*` roles.
fn pre_syntax_roles_theme() -> String {
    let roles: Vec<&str> = catmd::Theme::ROLES
        .iter()
        .copied()
        .filter(|r| !r.starts_with("syntax_"))
        .collect();
    assert_eq!(roles.len(), 29);
    roles.iter().map(|r| format!("{r} = \"green\"\n")).collect()
}

#[test]
fn theme_predating_the_syntax_roles_still_loads() {
    let dir = TempDir::new("theme-pre-syntax");
    let t = dir.file("old.toml", pre_syntax_roles_theme());
    styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("# T\n\n```rust\nfn main() {}\n```\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("main"))
        .stderr("");
}

#[test]
fn syntax_roles_take_effect_through_the_cli() {
    let dir = TempDir::new("theme-syntax");
    let t = dir.file(
        "t.toml",
        "syntax_keyword = \"red\"\nsyntax_comment = { modifiers = [\"italic\"] }\n",
    );
    let out = styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("```rust\nfn main() {} // x\n```\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(out.contains("\x1b[31mfn\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[3m// x\x1b[0m"), "{out:?}");
}

#[test]
fn syntax_roles_are_independent_of_the_code_block_role() {
    let dir = TempDir::new("theme-code-block-only");
    let t = dir.file("t.toml", "code_block = \"white\"\n");
    let out = styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("```rust\nfn main() {}\n```\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(out.contains("\x1b[37m() {}\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[35mfn\x1b[0m"), "{out:?}");
}

#[test]
fn theme_predating_the_syntax_roles_keeps_built_in_highlighting() {
    let dir = TempDir::new("theme-pre-syntax-highlight");
    let t = dir.file("old.toml", pre_syntax_roles_theme());
    let out = styled16()
        .arg("--theme")
        .arg(&t)
        .write_stdin("```rust\nfn main() {}\n```\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(out.contains("\x1b[35mfn\x1b[0m"), "{out:?}");
    // `code_block` was set to green by the old theme; plain code follows it.
    assert!(out.contains("\x1b[32m() {}\x1b[0m"), "{out:?}");
}

#[test]
fn newly_exposed_roles_take_effect_through_the_cli() {
    let dir = TempDir::new("theme-roles");
    let t = dir.file(
        "t.toml",
        "bullet = \"green\"\nlist_number = \"green\"\ntask_checkbox = \"green\"\ntable_border = \"green\"\n",
    );
    let out = styled16()
        .arg("--ascii")
        .arg("--theme")
        .arg(&t)
        .write_stdin("- a\n\n1. b\n\n- [ ] c\n\n| A |\n|---|\n| 1 |\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    for needle in [
        "\x1b[32m* \x1b[0ma",
        "\x1b[32m1. \x1b[0mb",
        "\x1b[32m[ ] \x1b[0mc",
        "\x1b[32m-\x1b[0m",
    ] {
        assert!(out.contains(needle), "{needle:?} not in {out:?}");
    }
}

#[test]
fn theme_by_relative_path_and_bare_filename() {
    let dir = TempDir::new("theme-path");
    fs::create_dir_all(dir.path().join("themes")).unwrap();
    dir.file("themes/dark.toml", "code = \"red\"\n");
    dir.file("dark.toml", "code = \"green\"\n");
    styled16()
        .current_dir(dir.path())
        .args(["--theme", "./themes/dark.toml"])
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
    styled16()
        .current_dir(dir.path())
        .args(["--theme", "dark.toml"])
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[32mx\x1b[0m\n");
}

#[test]
fn theme_by_name_resolves_through_xdg_config_home() {
    let dir = TempDir::new("theme-xdg");
    fs::create_dir_all(dir.path().join("catmd/themes")).unwrap();
    dir.file("catmd/themes/dark.toml", "code = \"red\"\n");
    styled16()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--theme", "dark"])
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
}

#[test]
fn theme_by_name_falls_back_to_home_config() {
    let dir = TempDir::new("theme-home");
    fs::create_dir_all(dir.path().join(".config/catmd/themes")).unwrap();
    dir.file(".config/catmd/themes/dark.toml", "code = \"red\"\n");
    styled16()
        .env("HOME", dir.path())
        .args(["--theme", "dark"])
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
}

#[test]
fn catmd_theme_env_supplies_a_default_and_the_flag_wins() {
    let dir = TempDir::new("theme-env");
    let a = dir.file("a.toml", "this is not toml\n");
    let good = dir.file("t.toml", "code = \"red\"\n");
    styled16()
        .env("CATMD_THEME", &good)
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
    // a.toml is invalid, so it would fail if it were read.
    styled16()
        .env("CATMD_THEME", &a)
        .arg("--theme")
        .arg(&good)
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[31mx\x1b[0m\n");
}

#[test]
fn missing_theme_file_fails_fast() {
    let dir = TempDir::new("theme-missing");
    let nope = dir.path().join("nope.toml");
    styled16()
        .arg("--theme")
        .arg(&nope)
        .write_stdin("`x`\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(
            predicate::str::contains("nope.toml").and(predicate::str::starts_with("catmd: theme ")),
        );
}

#[test]
fn missing_named_theme_reports_the_searched_path() {
    let dir = TempDir::new("theme-missing-name");
    let expected = dir.path().join("catmd/themes/nope.toml");
    styled16()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--theme", "nope"])
        .write_stdin("`x`\n")
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            expected.to_string_lossy().as_ref(),
        ));
}

#[test]
fn invalid_themes_fail_fast_with_the_offending_token() {
    let dir = TempDir::new("theme-invalid");
    theme_fails(&dir, "heading7 = \"red\"\n", "heading7");
    theme_fails(&dir, "strong = { modifiers = [\"blink\"] }\n", "blink");
    theme_fails(&dir, "code = \"#f80\"\n", "#f80");
    theme_fails(&dir, "code = 256\n", "256");
    theme_fails(&dir, "code = \"red\n", "invalid TOML");
}

// ---------------------------------------------------------------------------
// color depth through the CLI
// ---------------------------------------------------------------------------

fn hex_theme(dir: &TempDir) -> PathBuf {
    dir.file("hex.toml", "code = \"#ff0000\"\n")
}

#[test]
fn term_256color_emits_256_color_sequences_for_an_index_theme() {
    let dir = TempDir::new("depth-256");
    let t = dir.file("t.toml", "code = 196\n");
    catmd()
        .env("TERM", "xterm-256color")
        .args(["--color", "always", "--theme"])
        .arg(&t)
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[38;5;196mx\x1b[0m\n");
}

#[test]
fn colorterm_truecolor_emits_rgb_sequences_for_a_hex_theme() {
    let dir = TempDir::new("depth-true");
    catmd()
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .args(["--color", "always", "--theme"])
        .arg(hex_theme(&dir))
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[38;2;255;0;0mx\x1b[0m\n");
}

#[test]
fn hex_theme_is_downgraded_to_an_index_at_256_colors() {
    let dir = TempDir::new("depth-hex256");
    catmd()
        .env("TERM", "xterm-256color")
        .args(["--color", "always", "--theme"])
        .arg(hex_theme(&dir))
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[38;5;196mx\x1b[0m\n");
}

#[test]
fn hex_theme_is_downgraded_to_ansi_at_16_colors() {
    let dir = TempDir::new("depth-hex16");
    let out = styled16()
        .arg("--theme")
        .arg(hex_theme(&dir))
        .write_stdin("`x`\n")
        .output()
        .unwrap();
    let out = String::from_utf8(out.stdout).unwrap();
    assert!(
        out == "\x1b[31mx\x1b[0m\n" || out == "\x1b[91mx\x1b[0m\n",
        "{out:?}"
    );
}

#[test]
fn color_depth_16_overrides_a_truecolor_terminal() {
    let dir = TempDir::new("depth-force16");
    catmd()
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .args(["--color", "always", "--color-depth", "16", "--theme"])
        .arg(hex_theme(&dir))
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout(no_deep_colors());
}

#[test]
fn color_depth_truecolor_overrides_a_plain_terminal() {
    let dir = TempDir::new("depth-forcetrue");
    catmd()
        .env("TERM", "screen")
        .args(["--color", "always", "--color-depth", "truecolor", "--theme"])
        .arg(hex_theme(&dir))
        .write_stdin("`x`\n")
        .assert()
        .success()
        .stdout("\x1b[38;2;255;0;0mx\x1b[0m\n");
}
