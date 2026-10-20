//! The licenses of the grammars embedded for syntax highlighting.
//!
//! `THIRD_PARTY_LICENSES.md` at the repository root is generated from this
//! module and kept in sync by a test, so a grammar-set update cannot land
//! without refreshing it.

use std::fmt::Write;

/// Environment variable that makes the sync test rewrite the file.
pub const UPDATE_ENV: &str = "CATMD_UPDATE_LICENSES";

/// The generated Markdown listing every embedded grammar license that
/// requires acknowledgement, sorted by source path.
pub fn third_party_licenses() -> String {
    let listing = two_face::acknowledgement::listing();
    let mut licenses: Vec<_> = listing.for_syntaxes().iter().collect();
    licenses.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let mut md = String::new();
    writeln!(md, "# Third-party licenses").unwrap();
    writeln!(md).unwrap();
    writeln!(
        md,
        "`catmd` highlights code with the syntax definitions curated by the \
         [`bat`](https://github.com/sharkdp/bat) project and embedded through the \
         [`two-face`](https://codeberg.org/CosmicHarper/two-face) crate. The \
         definitions below carry their own licenses, reproduced as `two-face` \
         reports them; the complete listing, including licenses that do not \
         require acknowledgement, is at <{}>.",
        two_face::acknowledgement::url()
    )
    .unwrap();
    writeln!(md).unwrap();
    writeln!(
        md,
        "This file is generated. Refresh it with `{UPDATE_ENV}=1 cargo test third_party_licenses`."
    )
    .unwrap();

    for license in licenses {
        writeln!(md).unwrap();
        writeln!(md, "## {}", license.rel_path.display()).unwrap();
        writeln!(md).unwrap();
        writeln!(md, "````text").unwrap();
        writeln!(md, "{}", license.text.trim_end()).unwrap();
        writeln!(md, "````").unwrap();
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/THIRD_PARTY_LICENSES.md");

    #[test]
    fn third_party_licenses_file_is_in_sync() {
        let expected = third_party_licenses();
        assert!(expected.contains("\n## "), "no licenses were listed");
        if std::env::var_os(UPDATE_ENV).is_some_and(|v| !v.is_empty()) {
            std::fs::write(FILE, &expected).expect("write THIRD_PARTY_LICENSES.md");
            return;
        }
        let actual = std::fs::read_to_string(Path::new(FILE)).unwrap_or_default();
        assert!(
            actual == expected,
            "THIRD_PARTY_LICENSES.md is out of date with the embedded grammar set; \
             run `{UPDATE_ENV}=1 cargo test third_party_licenses` to regenerate it"
        );
    }

    #[test]
    fn listing_is_sorted_and_each_license_has_text() {
        let md = third_party_licenses();
        // Headings outside the license-text fences (the texts may contain `## ` lines).
        let mut in_fence = false;
        let mut headings = Vec::new();
        for line in md.lines() {
            if line == "````text" || line == "````" {
                in_fence = line == "````text";
            } else if let (false, Some(heading)) = (in_fence, line.strip_prefix("## ")) {
                headings.push(heading);
            }
        }
        assert!(headings.len() > 1);
        assert!(
            headings
                .windows(2)
                .all(|w| Path::new(w[0]) < Path::new(w[1])),
            "{headings:?}"
        );
        assert_eq!(md.matches("\n````text\n").count(), headings.len());
    }
}
