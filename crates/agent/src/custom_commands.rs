use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use collections::HashMap;
use serde::Deserialize;

/// A custom slash command loaded from a markdown file.
#[derive(Debug, Clone)]
pub struct CustomCommand {
    /// The command name, derived from the filename (e.g., "test.md" -> "test").
    pub name: String,
    /// Description from YAML frontmatter.
    pub description: String,
    /// Optional hint shown to the user about expected arguments.
    pub argument_hint: Option<String>,
    /// The markdown body of the command file (template content).
    pub template: String,
    /// Full path to the source file.
    pub source_path: PathBuf,
    /// Whether this command was loaded from the global config directory.
    pub is_global: bool,
    /// Optional list of tools this command is allowed to use.
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CommandFrontmatter {
    description: String,
    #[serde(default)]
    argument_hint: Option<String>,
    #[serde(default)]
    allowed_tools: Option<Vec<String>>,
}

impl CustomCommand {
    /// Process the template with the provided arguments.
    /// Substitutes `$ARGUMENTS` with all args joined, and `$1`, `$2`, etc. with positional args.
    pub fn process(&self, args: &[String]) -> String {
        let mut result = self.template.clone();

        // Replace `$ARGUMENTS` with all args joined by spaces.
        let all_args = args.join(" ");
        result = result.replace("$ARGUMENTS", &all_args);

        // Replace positional placeholders `$1`, `$2`, etc.
        for (index, arg) in args.iter().enumerate() {
            let placeholder = format!("${}", index + 1);
            result = result.replace(&placeholder, arg);
        }

        result
    }
}

/// Discover custom commands from global and worktree directories.
/// Worktree commands override global commands with the same name.
pub fn discover_custom_commands(worktree_roots: &[PathBuf]) -> HashMap<String, Arc<CustomCommand>> {
    let mut all_commands = discover_commands_sync(&global_commands_dir(), true);

    // Worktree commands override global commands.
    for worktree_root in worktree_roots {
        let worktree_commands_dir = worktree_root.join(".agents").join("commands");
        let worktree_commands = discover_commands_sync(&worktree_commands_dir, false);
        for (name, command) in worktree_commands {
            all_commands.insert(name, command);
        }
    }

    all_commands
}

/// Returns the global commands directory: `~/.config/zed/commands`.
fn global_commands_dir() -> PathBuf {
    paths::config_dir().join("commands")
}

/// Synchronously discover commands from a directory.
/// Each `.md` file is parsed for YAML frontmatter and template content.
fn discover_commands_sync(
    commands_dir: &Path,
    is_global: bool,
) -> HashMap<String, Arc<CustomCommand>> {
    let mut commands = HashMap::default();

    let entries = match std::fs::read_dir(commands_dir) {
        Ok(entries) => entries,
        Err(_) => return commands, // Directory doesn't exist or isn't readable.
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let extension = path.extension().and_then(|ext| ext.to_str());

        if extension != Some("md") {
            continue;
        }

        let Some(name) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
        else {
            continue;
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let command = match parse_command_file(&name, &content, path.clone(), is_global) {
            Ok(command) => command,
            Err(_) => continue, // Skip malformed files.
        };

        commands.insert(name, Arc::new(command));
    }

    commands
}

/// Parse a command markdown file with YAML frontmatter.
/// Expects format:
/// ```text
/// ---
/// description: "Some description"
/// argument_hint: "optional hint"
/// ---
/// Template content here...
/// ```
fn parse_command_file(
    name: &str,
    content: &str,
    source_path: PathBuf,
    is_global: bool,
) -> Result<CustomCommand> {
    let (frontmatter, template) = split_frontmatter(content)?;

    let frontmatter: CommandFrontmatter =
        serde_yml::from_str(frontmatter).context("failed to parse YAML frontmatter")?;

    Ok(CustomCommand {
        name: name.to_string(),
        description: frontmatter.description,
        argument_hint: frontmatter.argument_hint,
        template: template.to_string(),
        source_path,
        is_global,
        allowed_tools: frontmatter.allowed_tools,
    })
}

/// Split content into frontmatter and template body.
/// Frontmatter is delimited by `---` at the start of the file.
fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
    let content = content
        .strip_prefix("---")
        .context("missing frontmatter start")?;

    let Some(delimiter_end) = content.find("---") else {
        anyhow::bail!("missing frontmatter end delimiter");
    };

    let frontmatter = &content[..delimiter_end];
    let template = &content[delimiter_end + 3..].trim_start();

    Ok((frontmatter, template))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_substitutes_arguments() {
        let command = CustomCommand {
            name: "test".to_string(),
            description: "Test command".to_string(),
            argument_hint: None,
            template: "Run tests with: $ARGUMENTS".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&["--release".to_string(), "--verbose".to_string()]);
        assert_eq!(result, "Run tests with: --release --verbose");
    }

    #[test]
    fn test_process_substitutes_positional_args() {
        let command = CustomCommand {
            name: "test".to_string(),
            description: "Test command".to_string(),
            argument_hint: None,
            template: "First: $1, Second: $2, First again: $1".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&["alpha".to_string(), "beta".to_string()]);
        assert_eq!(result, "First: alpha, Second: beta, First again: alpha");
    }

    #[test]
    fn test_process_no_args() {
        let command = CustomCommand {
            name: "test".to_string(),
            description: "Test command".to_string(),
            argument_hint: None,
            template: "Just a template".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[]);
        assert_eq!(result, "Just a template");
    }

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
description: Run the test suite
argument_hint: <test_name>
---
Run tests with `cargo test $ARGUMENTS`.
"#;

        let (frontmatter, template) = split_frontmatter(content).unwrap();
        assert!(frontmatter.contains("description:"));
        assert!(template.contains("Run tests with"));
    }

    #[test]
    fn test_parse_frontmatter_missing_delimiter() {
        let content = "No frontmatter here";
        assert!(split_frontmatter(content).is_err());
    }
}
