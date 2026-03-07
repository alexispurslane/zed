use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use collections::HashMap;
use gpui::{App, AppContext, Context, Entity};
use regex::Regex;
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
    /// Substitutes:
    /// - `` !`shell_command` `` with the output of the shell command (run in worktree)
    /// - `@file_path` with the contents of the file (relative to worktree)
    /// - `$ARGUMENTS` with all args joined by spaces
    /// - `$1`, `$2`, etc. with positional args
    pub fn process(&self, args: &[String], worktree_root: Option<&Path>) -> String {
        let mut result = self.template.clone();

        // Process shell command substitutions !`...`
        result = self.substitute_shell_commands(&result, worktree_root);

        // Process file substitutions @path
        result = self.substitute_file_contents(&result, worktree_root);

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

    /// Substitute `` !`shell_command` `` patterns with command output.
    fn substitute_shell_commands(&self, template: &str, worktree_root: Option<&Path>) -> String {
        let shell_regex = Regex::new(r"!`([^`]+)`").expect("valid regex");
        shell_regex
            .replace_all(template, |caps: &regex::Captures| {
                let command = caps[1].trim();
                match self.execute_shell_command(command, worktree_root) {
                    Ok(output) => output,
                    Err(e) => {
                        log::warn!(
                            "Shell command failed in custom command '{}': {}",
                            self.name,
                            e
                        );
                        format!("[error executing command: {}]", command)
                    }
                }
            })
            .into_owned()
    }

    /// Execute a shell command and return its stdout.
    fn execute_shell_command(&self, command: &str, worktree_root: Option<&Path>) -> Result<String> {
        let cwd = worktree_root
            .or_else(|| self.source_path.parent())
            .unwrap_or(Path::new("/"));
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .output()
            .context("failed to execute shell command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("command exited with status {}: {}", output.status, stderr);
        }

        String::from_utf8(output.stdout)
            .map(|s| s.trim_end().to_string())
            .context("command output is not valid UTF-8")
    }

    /// Substitute `@file_path` patterns with file contents.
    fn substitute_file_contents(&self, template: &str, worktree_root: Option<&Path>) -> String {
        // Match @ followed by a path (non-whitespace characters, or quoted path)
        // Supports: @/absolute/path, @relative/path, @"path with spaces", @'path with spaces'
        let file_regex = Regex::new(r#"@(?:"([^"]+)"|'([^']+)'|(\S+))"#).expect("valid regex");
        file_regex
            .replace_all(template, |caps: &regex::Captures| {
                // Get the matched path from whichever group matched
                let file_path = caps
                    .get(1)
                    .or_else(|| caps.get(2))
                    .or_else(|| caps.get(3))
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                match self.read_file_content(file_path, worktree_root) {
                    Ok(content) => content,
                    Err(e) => {
                        log::warn!("File read failed in custom command '{}': {}", self.name, e);
                        format!("[error reading file: {}]", file_path)
                    }
                }
            })
            .into_owned()
    }

    /// Read file content relative to the worktree root.
    fn read_file_content(&self, file_path: &str, worktree_root: Option<&Path>) -> Result<String> {
        let path = if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else {
            worktree_root
                .or_else(|| self.source_path.parent())
                .unwrap_or(Path::new("/"))
                .join(file_path)
        };

        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read file: {}", path.display()))
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
        Err(e) => {
            log::debug!(
                "Commands directory not readable ({}): {}",
                commands_dir.display(),
                e
            );
            return commands;
        }
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
            Err(e) => {
                log::debug!("Failed to read command file ({}): {}", path.display(), e);
                continue;
            }
        };

        let command = match parse_command_file(&name, &content, path.clone(), is_global) {
            Ok(command) => command,
            Err(e) => {
                log::debug!(
                    "Failed to parse command file ({}): {}. Skipping.",
                    path.display(),
                    e
                );
                continue;
            }
        };

        log::debug!(
            "Loaded custom command '{}' from {}",
            command.name,
            path.display()
        );
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

/// Context entity that holds discovered custom command definitions.
/// Populated asynchronously on creation via cx.spawn → background_spawn → update pattern.
/// Used for: system prompt generation and template processing during LLM request building.
pub struct CommandsContext {
    commands: Option<HashMap<String, Arc<CustomCommand>>>,
}

impl CommandsContext {
    /// Create a new CommandsContext and spawn background task to populate it.
    /// Pattern matches SkillsContext::new() exactly.
    pub fn new(worktree_roots: Vec<PathBuf>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx: &mut Context<Self>| {
            log::debug!(
                "CommandsContext: Starting async discovery from {} worktrees",
                worktree_roots.len()
            );

            // Spawn foreground task that coordinates background work
            cx.spawn(async move |this, cx| {
                log::debug!("CommandsContext: Discovering commands in background...");

                // Do disk I/O on background executor
                let commands = cx
                    .background_spawn(async move {
                        let cmds = discover_custom_commands(&worktree_roots);
                        log::debug!("CommandsContext: Discovered {} commands", cmds.len());
                        cmds
                    })
                    .await;

                // Update entity on main thread
                let count = commands.len();
                this.update(cx, |this, _cx| {
                    log::debug!("CommandsContext: Storing {} commands in context", count);
                    this.commands = Some(commands);
                })
                .ok();
            })
            .detach();

            Self {
                commands: None, // Will be populated asynchronously
            }
        })
    }

    /// Get the commands map if loaded.
    pub fn commands(&self) -> Option<&HashMap<String, Arc<CustomCommand>>> {
        self.commands.as_ref()
    }

    /// Check if commands have been loaded from disk.
    pub fn is_loaded(&self) -> bool {
        self.commands.is_some()
    }

    /// Lookup a command by name. Returns None if not found or not yet loaded.
    pub fn get(&self, name: &str) -> Option<Arc<CustomCommand>> {
        self.commands.as_ref()?.get(name).cloned()
    }
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

        let result = command.process(&["--release".to_string(), "--verbose".to_string()], None);
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

        let result = command.process(&["alpha".to_string(), "beta".to_string()], None);
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

        let result = command.process(&[], None);
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

    #[test]
    fn test_shell_command_substitution() {
        let command = CustomCommand {
            name: "shell-test".to_string(),
            description: "Test shell substitution".to_string(),
            argument_hint: None,
            template: "Current dir: !`pwd`".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], None);
        // Should contain the current directory path
        assert!(result.starts_with("Current dir: /"));
    }

    #[test]
    fn test_shell_command_substitution_multiple() {
        let command = CustomCommand {
            name: "multi-shell".to_string(),
            description: "Test multiple shell substitutions".to_string(),
            argument_hint: None,
            template: "!`echo hello` and !`echo world`".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], None);
        assert_eq!(result, "hello and world");
    }

    #[test]
    fn test_file_substitution() {
        use std::io::Write;

        // Create a temp file with known content
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_command_file.txt");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        writeln!(file, "Hello from file!").unwrap();
        drop(file);

        let command = CustomCommand {
            name: "file-test".to_string(),
            description: "Test file substitution".to_string(),
            argument_hint: None,
            template: format!("Content: @{}", temp_file.display()),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], None);
        assert!(result.contains("Hello from file!"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_shell_error_handling() {
        let command = CustomCommand {
            name: "error-test".to_string(),
            description: "Test shell error handling".to_string(),
            argument_hint: None,
            template: "Result: !`exit 1`".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], None);
        // Should show error message, not panic
        assert!(result.contains("[error executing command"));
    }

    #[test]
    fn test_file_error_handling() {
        let command = CustomCommand {
            name: "file-error-test".to_string(),
            description: "Test file error handling".to_string(),
            argument_hint: None,
            template: "Result: @/nonexistent/path/file.txt".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], None);
        // Should show error message, not panic
        assert!(result.contains("[error reading file"));
    }

    #[test]
    fn test_combined_substitutions() {
        let command = CustomCommand {
            name: "combined-test".to_string(),
            description: "Test all substitutions together".to_string(),
            argument_hint: None,
            template: "!`echo prefix` $1 @/etc/hostname $ARGUMENTS".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&["arg1".to_string(), "arg2".to_string()], None);
        // Shell substitution happens first, then args
        assert!(result.starts_with("prefix "));
        assert!(result.contains("arg1"));
        // hostname should be readable on most systems
        assert!(!result.contains("@/")); // Should be substituted
    }

    #[test]
    fn test_file_substitution_with_spaces() {
        use std::io::Write;

        // Create a temp file with spaces in name
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test file with spaces.txt");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        writeln!(file, "Content with spaces!").unwrap();
        drop(file);

        // Test with quoted path
        let command = CustomCommand {
            name: "file-spaces-test".to_string(),
            description: "Test file substitution with spaces".to_string(),
            argument_hint: None,
            template: r#"Content: @"test file with spaces.txt""#.to_string(),
            source_path: temp_dir.clone(),
            is_global: true,
            allowed_tools: None,
        };

        let result = command.process(&[], Some(&temp_dir));
        assert!(result.contains("Content with spaces!"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_worktree_root_resolution() {
        use std::io::Write;

        // Create a temp worktree with a file
        let temp_dir = std::env::temp_dir();
        let worktree = temp_dir.join("test_worktree");
        let _ = std::fs::create_dir_all(&worktree);
        let temp_file = worktree.join("worktree_file.txt");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        writeln!(file, "Worktree content!").unwrap();
        drop(file);

        let command = CustomCommand {
            name: "worktree-test".to_string(),
            description: "Test worktree root resolution".to_string(),
            argument_hint: None,
            template: "Content: @worktree_file.txt".to_string(),
            source_path: PathBuf::from("/tmp/test.md"),
            is_global: true,
            allowed_tools: None,
        };

        // Pass worktree as parameter
        let result = command.process(&[], Some(&worktree));
        assert!(result.contains("Worktree content!"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
        let _ = std::fs::remove_dir(&worktree);
    }
}
