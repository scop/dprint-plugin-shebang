use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::get_unknown_property_diagnostics;
#[cfg(target_arch = "wasm32")]
use dprint_core::generate_plugin_code;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatError;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;
use lazy_regex::regex;
use serde::Serialize;

#[derive(Default)]
pub struct ShebangPluginHandler;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {}

impl SyncPluginHandler<Configuration> for ShebangPluginHandler {
    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        _global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        PluginResolveConfigurationResult {
            config: Configuration {},
            diagnostics: get_unknown_property_diagnostics(config),
            file_matching: FileMatchingInfo {
                #[rustfmt::skip]
                file_extensions: [
                    // https://en.wikipedia.org/wiki/AWK
                    "awk",
                    // https://bats-core.readthedocs.io
                    "bats",
                    // https://en.wikipedia.org/wiki/Common_Gateway_Interface
                    "cgi",
                    // https://dlang.org/rdmd.html
                    "d",
                    // https://elixir-lang.org
                    "exs",
                    // https://openjdk.org/jeps/330#Shebang_files
                    "java",
                    // https://nodejs.org/en/learn/command-line/run-nodejs-scripts-from-the-command-line
                    "js", "ts",
                    // https://github.com/Kotlin/KEEP/blob/main/proposals/KEEP-0075-scripting-support.md
                    "kts",
                    // https://www.lua.org
                    "lua",
                    // https://en.wikipedia.org/wiki/Make_(software)
                    "mk",
                    // https://www.php.net/manual/en/features.commandline.usage.php
                    "php", "php3", "php4", "php5",
                    // https://perldoc.perl.org/perlrun#Location-of-Perl
                    "pl", "t", "perl",
                    // https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html
                    "postinst", "postrm", "preinst", "prerm",
                    // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_comments#shebang
                    "ps1",
                    // https://docs.python.org/3/using/unix.html#miscellaneous
                    "py",
                    // https://www.ruby-lang.org
                    "rb",
                    // https://www.gnu.org/software/sed
                    "sed",
                    // https://en.wikipedia.org/wiki/Shell_script
                    "sh", "bash", "csh", "fish", "ksh", "tcsh", "zsh",
                    // https://www.slackwiki.com/Writing_A_SlackBuild_Script
                    "SlackBuild",
                    // https://sourceware.org/systemtap/SystemTap_Beginners_Guide/useful-systemtap-scripts.html
                    "stp",
                ].into_iter().map(String::from).collect(),
                #[rustfmt::skip]
                file_names: vec![
                    // https://en.wikipedia.org/wiki/Make_(software)
                    "Makefile", "GNUmakefile",
                ].into_iter().map(String::from).collect(),
                // only the shebang line is touched, so let the file's usual formatter run too
                additive: true,
            },
        }
    }

    fn plugin_info(&mut self) -> PluginInfo {
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_key: "shebang".to_string(),
            help_url: "https://github.com/scop/dprint-plugin-shebang".to_string(),
            config_schema_url: format!(
                "https://plugins.dprint.dev/scop/dprint-plugin-shebang/{}/schema.json",
                env!("CARGO_PKG_VERSION")
            ),
            update_url: Some("https://plugins.dprint.dev/scop/shebang/latest.json".to_string()),
        }
    }

    fn license_text(&mut self) -> String {
        include_str!("../LICENSE").to_string()
    }

    fn check_config_updates(
        &self,
        _message: dprint_core::plugins::CheckConfigUpdatesMessage,
    ) -> Result<Vec<dprint_core::plugins::ConfigChange>, FormatError> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<Configuration>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        let text = String::from_utf8(request.file_bytes)?;
        if let Some(range) = request.range {
            // the shebang can only be on the first line, so skip ranges that don't touch it
            if range.start > first_line_end(&text) {
                return Ok(None);
            }
        }
        Ok(format_shebang(&text).map(String::into_bytes))
    }
}

/// Formats the shebang line at the start of the text, returning `None` when
/// there's no shebang or it's already formatted.
pub fn format_shebang(text: &str) -> Option<String> {
    let re = regex!(
        r#"^#!(?x)                          # hashbang
        [\ \t]*                             # optional whitespace
        (?<interpreter>[^\ \t]+)            # interpreter
        (?:[\ \t]*?                         # optional whitespace
          |[\ \t]+(?<args>[^\ \t][^\r\n]*?) # optional whitespace, then args; note that whitespace after args is part of args, not stripped
        )?
        (?<end>[\r\n]|$)                    # end of line
    "#
    );
    // slicing at a line break is always on a char boundary since `\r` and `\n` are single bytes
    let captures = re.captures(&text[..first_line_end(text)])?;
    let end = captures.name("end").unwrap().start();
    let interpreter = captures.name("interpreter").unwrap();
    let args = captures.name("args");
    // check the match positions to avoid allocating when the shebang is already formatted
    let is_formatted = interpreter.start() == "#!".len()
        && match args {
            Some(args) => &text[interpreter.end()..args.start()] == " ",
            None => interpreter.end() == end,
        };
    if is_formatted {
        return None;
    }
    let interpreter = interpreter.as_str();
    Some(match args {
        Some(args) => format!("#!{} {}{}", interpreter, args.as_str(), &text[end..]),
        None => format!("#!{}{}", interpreter, &text[end..]),
    })
}

fn first_line_end(text: &str) -> usize {
    text.find(['\r', '\n']).unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use crate::Configuration;
    use crate::ShebangPluginHandler;
    use crate::format_shebang;
    use dprint_core::plugins::FormatConfigId;
    use dprint_core::plugins::FormatRange;
    use dprint_core::plugins::FormatResult;
    use dprint_core::plugins::NullCancellationToken;
    use dprint_core::plugins::SyncFormatRequest;
    use dprint_core::plugins::SyncPluginHandler;
    use std::path::Path;

    #[test]
    fn empty() {
        let text = "";
        assert_eq!(format_shebang(text), None);
    }

    #[test]
    fn foo_bar() {
        let text = "foo\nbar";
        assert_eq!(format_shebang(text), None);
    }

    #[test]
    fn basic() {
        let text = "#!/foo/bar\nquux";
        assert_eq!(format_shebang(text), None);
    }

    #[test]
    fn basic_with_args() {
        let text = "#!/foo/bar -quux\nbaz";
        assert_eq!(format_shebang(text), None);
    }

    #[test]
    fn tab_before_args() {
        let text = "#!/foo/bar\t-quux\nbaz";
        assert_eq!(
            format_shebang(text),
            Some(String::from("#!/foo/bar -quux\nbaz"))
        );
    }

    #[test]
    fn post_space_only() {
        let text = "#!/foo/bar \r\nbaz";
        assert_eq!(
            format_shebang(text),
            Some(String::from("#!/foo/bar\r\nbaz"))
        );
    }

    #[test]
    fn pre_post_space() {
        let text = "#! \t /foo/bar \t \n quux";
        assert_eq!(
            format_shebang(text),
            Some(String::from("#!/foo/bar\n quux")) // Note spaces and tabs after /foo/bar is trimmed
        );
    }

    #[test]
    fn pre_mid_post_space() {
        let text = "#! \t /foo/bar\t  -quux\t \nbaz";
        assert_eq!(
            format_shebang(text),
            Some(String::from("#!/foo/bar -quux\t \nbaz")) // Note spaces and tabs after -quux are kept as part of args
        );
    }

    #[test]
    fn multibyte_char_across_byte_1024() {
        let text = format!("#!/foo/bar {}é\nbaz", "a".repeat(1012));
        assert!(!text.is_char_boundary(1024));
        assert_eq!(format_shebang(&text), None);
    }

    #[test]
    fn long_first_line() {
        let args = format!("{} \t {}", "a".repeat(1012), "b".repeat(10));
        let text = format!("#!  /foo/bar {args}\nbaz");
        assert_eq!(
            format_shebang(&text),
            Some(format!("#!/foo/bar {args}\nbaz"))
        );
    }

    #[test]
    fn handler_formats_whole_file() {
        let result = handler_format("#!  /foo/bar\nbaz\nquux\n", None).unwrap();
        assert_eq!(result, Some(b"#!/foo/bar\nbaz\nquux\n".to_vec()));
    }

    #[test]
    fn handler_unchanged() {
        let result = handler_format("#!/foo/bar\nbaz\n", None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn handler_range_on_first_line() {
        let result = handler_format("#!  /foo/bar\nbaz\nquux\n", Some(3..5)).unwrap();
        assert_eq!(result, Some(b"#!/foo/bar\nbaz\nquux\n".to_vec()));
    }

    #[test]
    fn handler_range_at_end_of_first_line() {
        let result = handler_format("#!  /foo/bar\nbaz\nquux\n", Some(12..12)).unwrap();
        assert_eq!(result, Some(b"#!/foo/bar\nbaz\nquux\n".to_vec()));
    }

    #[test]
    fn handler_range_after_first_line() {
        let result = handler_format("#!  /foo/bar\nbaz\nquux\n", Some(13..16)).unwrap();
        assert_eq!(result, None);
    }

    fn handler_format(text: &str, range: FormatRange) -> FormatResult {
        ShebangPluginHandler.format(
            SyncFormatRequest {
                file_path: Path::new("file.sh"),
                file_bytes: text.as_bytes().to_vec(),
                config_id: FormatConfigId::from_raw(1),
                config: &Configuration {},
                range,
                token: &NullCancellationToken,
            },
            |_| unreachable!(),
        )
    }
}

#[cfg(target_arch = "wasm32")]
generate_plugin_code!(ShebangPluginHandler, ShebangPluginHandler);
