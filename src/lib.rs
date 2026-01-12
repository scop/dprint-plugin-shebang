use anyhow::Result;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
#[cfg(target_arch = "wasm32")]
use dprint_core::generate_plugin_code;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;
use lazy_regex::regex;
use serde::Serialize;
use std::cmp;

#[derive(Default)]
pub struct ShebangPluginHandler;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {}

impl SyncPluginHandler<Configuration> for ShebangPluginHandler {
    fn resolve_config(
        &mut self,
        _config: ConfigKeyMap,
        _global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        PluginResolveConfigurationResult {
            config: Configuration {},
            diagnostics: Vec::new(),
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
            },
        }
    }

    fn plugin_info(&mut self) -> PluginInfo {
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_key: "shebang".to_string(),
            help_url: "https://github.com/scop/dprint-plugin-shebang".to_string(),
            config_schema_url: "".to_string(),
            update_url: Some("https://plugins.dprint.dev/scop/shebang/latest.json".to_string()),
        }
    }

    fn license_text(&mut self) -> String {
        include_str!("../LICENSE").to_string()
    }

    fn check_config_updates(
        &self,
        _message: dprint_core::plugins::CheckConfigUpdatesMessage,
    ) -> Result<Vec<dprint_core::plugins::ConfigChange>> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<Configuration>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        let bytes = if request.range.is_some() {
            let range = request.range.unwrap();
            if range.start != 0 {
                return Ok(None);
            }
            request.file_bytes[range.start..range.end].to_vec()
        } else {
            request.file_bytes
        };

        let text = String::from_utf8(bytes)?;
        let result = format_shebang(&text)?;
        if result.is_none() {
            return Ok(None);
        }

        let result = result.unwrap();
        if result == text {
            Ok(None)
        } else {
            Ok(Some(result.into_bytes()))
        }
    }
}

pub fn format_shebang(text: &str) -> Result<Option<String>> {
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
    if let Some(captures) = re.captures(&text[..cmp::min(text.len(), 1024)]) {
        let end = captures.name("end").unwrap().start();
        let interpreter = captures.name("interpreter").unwrap().as_str();
        let args = captures.name("args").map_or("", |m| m.as_str());
        if args.is_empty() {
            return Ok(Some(String::from(&format!("#!{}{}", interpreter, &text[end..]))));
        }
        return Ok(Some(String::from(&format!(
            "#!{} {}{}",
            interpreter,
            args,
            &text[end..]
        ))));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::format_shebang;

    #[test]
    fn empty() {
        let text = "";
        assert_eq!(format_shebang(text).unwrap(), None);
    }

    #[test]
    fn foo_bar() {
        let text = "foo\nbar";
        assert_eq!(format_shebang(text).unwrap(), None);
    }

    #[test]
    fn basic() {
        let text = "#!/foo/bar\nquux";
        assert_eq!(format_shebang(text).unwrap(), Some(String::from(text)));
    }

    #[test]
    fn basic_with_args() {
        let text = "#!/foo/bar -quux\nbaz";
        assert_eq!(format_shebang(text).unwrap(), Some(String::from(text)));
    }

    #[test]
    fn pre_post_space() {
        let text = "#! \t /foo/bar \t \n quux";
        assert_eq!(
            format_shebang(text).unwrap(),
            Some(String::from("#!/foo/bar\n quux")) // Note spaces and tabs after /foo/bar is trimmed
        );
    }

    #[test]
    fn pre_mid_post_space() {
        let text = "#! \t /foo/bar\t  -quux\t \nbaz";
        assert_eq!(
            format_shebang(text).unwrap(),
            Some(String::from("#!/foo/bar -quux\t \nbaz")) // Note spaces and tabs after -quux are kept as part of args
        );
    }
}

#[cfg(target_arch = "wasm32")]
generate_plugin_code!(ShebangPluginHandler, ShebangPluginHandler);
