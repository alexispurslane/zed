Revised Implementation Plan: Custom Slash Commands for Zed Agent

### Key Findings from Research

1. **Two Slash Command Systems Exist:**
   - **ACP/Context Server commands** - Used by new NativeAgent, simple `AvailableCommand` structs
   - **Text Thread commands** - Rich `SlashCommand` trait with local execution (legacy)

2. **For Built-in Agent:** We should **NOT** use the ACP slash command infrastructure. Instead, we'll integrate with the **completion provider + crease system** used for `@`-mentions.

3. **Discovery Pattern:** Follow skills exactly - discover at thread creation from `~/.config/zed/commands/` and `.agents/commands/`, with worktree precedence.

4. **YAML Parsing:** Use `serde_yml` (already in codebase).

---

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Directory Structure                              │
│  ~/.config/zed/commands/*.md          # Global commands (user-wide)     │
│  <project>/.agents/commands/*.md      # Worktree commands (override)    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    CommandsContext (on Thread)                           │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Populated async at thread creation via cx.spawn pattern        │   │
│  │  - Stores: HashMap<String, Arc<CustomCommand>> (definitions)   │   │
│  │  - Used for: completions, system prompt, LLM request processing │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
┌──────────────────────────────────┐  ┌──────────────────────────────────┐
│   PromptCompletionProvider       │  │   System Prompt Builder          │
│   (dereferences CommandsContext) │  │   (dereferences CommandsContext) │
│   - User types `/`               │  │   - Lists available commands     │
│   - Shows completions from       │  │     for LLM awareness            │
│     CommandsContext              │  │                                  │
│   - On confirm: inserts crease   │  │                                  │
└──────────┬───────────────────────┘  └──────────────────────────────────┘
           │
           ▼
┌──────────────────────────────────┐
│   Message Content (Creases)      │
│   - Stores: command_name + args  │
│   - Renders as bubble UI         │
│   - NOT processed template       │
│   - Preserved in message history │
└──────────┬───────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              Thread::build_user_message()                                │
│  - Extracts command invocations from creases                             │
│  - Looks up command definition in CommandsContext                        │
│  - Processes template: $ARGUMENTS, $1, $2 substitution                  │
│  - Returns expanded text for LLM request                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key Architectural Principles:**
1. **Separation of concerns**: Command definitions (CommandsContext) are separate from command invocations (message creases)
2. **Lazy processing**: Templates are expanded only when building the LLM request, not on message submit
3. **Async loading**: CommandsContext populates asynchronously at thread creation (non-blocking)
4. **Worktree precedence**: Commands in `.agents/commands/` override global `~/.config/zed/commands/`

---

### Phase 1: Core Data Structures

**Context:** This phase establishes the foundation for custom slash commands by defining the data structures needed to represent commands and implementing the discovery mechanism. Without this phase, the system has no way to load or represent custom commands from the filesystem.

**Technical Implementation:**
- Define `CustomCommand` struct to hold command metadata (name, description, template, etc.)
- Create `CommandFrontmatter` struct for YAML deserialization using `serde_yml`
- Implement `discover_custom_commands()` to scan directories and load command files
- Build precedence logic where worktree commands override global commands
- Implement template processing with `$ARGUMENTS` and positional parameter substitution

#### 1.1 Create `CustomCommand` Struct

**Location:** `crates/agent/src/custom_commands.rs` (new file)

```rust
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CustomCommand {
    pub name: String,              // From filename
    pub description: String,       // From frontmatter
    pub argument_hint: Option<String>, // From frontmatter
    pub template: String,          // Markdown body
    pub source_path: PathBuf,      // Where loaded from
    pub is_global: bool,           // Global vs worktree
}

#[derive(Deserialize)]
struct CommandFrontmatter {
    description: String,
    #[serde(default)]
    argument_hint: Option<String>,
    #[serde(default)]
    allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    metadata: Option<HashMap<String, String>>,
}
```

#### 1.2 Discovery Function

```rust
pub fn discover_custom_commands(worktree_roots: &[PathBuf]) -> HashMap<String, Arc<CustomCommand>> {
    // Phase 1: Global commands from ~/.config/zed/commands
    let mut all_commands = discover_commands_sync(&global_commands_dir());

    // Phase 2: Worktree commands override global
    for worktree in worktree_roots {
        let worktree_cmds = discover_commands_sync(&worktree.join(".agents").join("commands"));
        for (name, cmd) in worktree_cmds {
            all_commands.insert(name, cmd);
        }
    }
    all_commands
}

fn global_commands_dir() -> PathBuf {
    paths::config_dir().join("commands")
}

fn discover_commands_sync(commands_dir: &Path) -> HashMap<String, Arc<CustomCommand>> {
    // For each .md file in commands_dir:
    //   1. Parse filename as command name (e.g., "test.md" -> "test")
    //   2. Parse YAML frontmatter using serde_yml
    //   3. Validate name matches filename (without extension)
    //   4. Create CustomCommand struct
}
```

#### 1.3 Template Processing

```rust
impl CustomCommand {
    pub fn process(&self, args: &[String]) -> String {
        let mut result = self.template.clone();

        // Replace $ARGUMENTS with all args joined
        let all_args = args.join(" ");
        result = result.replace("$ARGUMENTS", &all_args);

        // Replace $1, $2, $3 with positional args
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            result = result.replace(&placeholder, arg);
        }

        // TODO: Phase 2 - Shell injection !`cmd`
        // TODO: Phase 2 - File references @path

        result
    }
}
```

**Phase 1 Deliverables:**

| Deliverable | How to Test |
|-------------|-------------|
| Command files in `~/.config/zed/commands/*.md` are parsed | Create a test file and verify it's loaded |
| YAML frontmatter is correctly extracted | Check that description/argument_hint appear in logs |
| Worktree commands override global commands | Create same command in both locations, verify worktree wins |
| Template variables `$ARGUMENTS`, `$1`, `$2` are substituted | Write unit test for `CustomCommand::process()` |
| Discovery returns HashMap of all commands | Unit test with temp directories |

---

### Phase 2: Integration with Completion System

**Context:** This phase connects custom commands to Zed's existing completion UI, allowing users to discover and select slash commands through the familiar `Ctrl+Space` or auto-trigger interface. The completion system is the entry point for users to invoke custom commands.

**Technical Implementation:**
- Extend `PromptCompletion` enum with new `SlashCommand` variant holding command metadata
- Modify `try_parse()` to detect `/` at start of input and parse command names
- Update `completions()` method to generate completion items from discovered commands
- Implement `confirm_slash_command_callback()` to handle selection and insert crease

#### 2.1 Extend `PromptCompletion`

**Location:** `crates/agent_ui/src/completion_provider.rs`

Add to the `PromptCompletion` enum:

```rust
pub enum PromptCompletion {
    Mention { ... },
    Thread { ... },
    // NEW:
    SlashCommand {
        command_name: String,
        description: String,
        argument_hint: Option<String>,
        custom_command: Option<Arc<CustomCommand>>, // If from file
    },
}
```

#### 2.2 Update `try_parse` for Slash Commands

Extend the existing slash command parsing (around line 1492):

```rust
pub fn try_parse(line: &str, offset_to_line: usize, commands: &HashMap<String, Arc<CustomCommand>>) -> Option<Self> {
    if !line.starts_with('/') || offset_to_line != 0 {
        return None;
    }

    // Parse: /command or /command arg1 arg2
    let after_slash = &line[1..];
    let (command_name, rest) = after_slash.split_once(' ')
        .map(|(cmd, rest)| (cmd, Some(rest)))
        .unwrap_or_else(|| (after_slash, None));

    // Check if it's a custom command
    if let Some(cmd) = commands.get(command_name) {
        return Some(PromptCompletion::SlashCommand {
            command_name: command_name.to_string(),
            description: cmd.description.clone(),
            argument_hint: cmd.argument_hint.clone(),
            custom_command: Some(cmd.clone()),
        });
    }

    // Fall back to built-in commands...
    None
}
```

#### 2.3 Generate Completions

In `PromptCompletionProvider::completions()`:

```rust
fn completions(&self, buffer: &BufferSnapshot, ...) -> Task<Result<Vec<Completion>>> {
    // ... existing code ...

    // Check if at start of line with `/`
    if let Some(completion_type) = PromptCompletion::try_parse(&line, offset_to_line, &self.custom_commands) {
        match completion_type {
            PromptCompletion::SlashCommand { command_name, description, argument_hint, custom_command } => {
                let completion = Completion {
                    replace_range: position..position,
                    new_text: format!("/{} ", command_name),
                    label: CodeLabel::plain(command_name.clone(), None),
                    description: Some(description.clone()),
                    icon_path: Some(IconName::Slash.path().into()),
                    confirm: Some(self.confirm_slash_command_callback(
                        command_name,
                        custom_command,
                        source_range,
                    )),
                    // ... other fields
                };
                return Task::ready(Ok(vec![completion]));
            }
            // ... other cases
        }
    }
}
```

#### 2.4 Confirm Callback

**Simplified approach:** The completion callback just inserts text. Crease insertion happens separately via MessageEditor detection (same pattern as @-mentions).

```rust
// In completion provider - simplified, no crease insertion here
confirm: Some(Arc::new({
    let source = source.clone();
    move |intent, _window, cx| {
        if !is_missing_argument {
            cx.defer(move |cx| {
                match intent {
                    CompletionIntent::Complete
                    | CompletionIntent::CompleteWithInsert
                    | CompletionIntent::CompleteWithReplace => {
                        source.confirm_command(cx);
                    }
                    CompletionIntent::Compose => {}
                }
            });
        }
        false
    }
})),
```

**Note:** We moved crease insertion to Phase 3 (MessageEditor detection) to avoid complex closure lifetime issues and follow the @-mention pattern.

**Phase 2 Deliverables:**

| Deliverable | How to Test |
|-------------|-------------|
| Typing `/` shows completion menu with custom commands | Type `/` in message editor, see commands listed |
| Command descriptions appear in completion UI | Verify description from YAML shows in dropdown |
| Selecting a command inserts text with trailing space | Select `/test` from completions, see `/test ` inserted |
| Built-in commands still work | Verify `/default`, `/cargo` etc. still appear |

**Implementation Changes:**
- Added `custom_command: Option<Arc<CustomCommand>>` field to `AvailableCommand` struct
- Added `custom_commands()` method to `PromptCompletionProviderDelegate` trait
- Modified `search_slash_commands()` to include custom commands alongside built-in commands
- Simplified confirm callback to just insert text (following @-mention pattern)

---

### Phase 3: Crease/Bubble Implementation

**Context:** This phase creates the visual representation of slash commands in the editor. When a user selects a command from completions, it transforms into a compact "bubble" UI (similar to @-mentions). The crease system collapses the text into a rendered element, providing a clean, non-editable visual indicator that a command is active.

**Technical Implementation:**
- Create `SlashCommandCrease` component implementing `RenderOnce` for GPUI
- Style as a tinted button with slash icon, command name, and optional argument hint
- **Architecture:** Follow @-mention pattern - completion inserts text, MessageEditor detects and renders crease
- Implement `insert_crease_for_slash_command()` to create editor folds
- Use `Crease::inline()` with `FoldPlaceholder` to render the bubble in place of text
- Store command metadata in the crease for later execution

#### 3.1 Create `SlashCommandCrease` UI Component

**Location:** `crates/agent_ui/src/ui/slash_command_crease.rs` (new file)

```rust
#[derive(IntoElement)]
pub struct SlashCommandCrease {
    id: ElementId,
    command_name: SharedString,
    argument_hint: Option<SharedString>,
    is_loading: bool,
}

impl RenderOnce for SlashCommandCrease {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        ButtonLike::new(self.id)
            .style(ButtonStyle::Tinted(TintColor::Accent))
            .child(
                h_flex()
                    .gap_1()
                    .child(Icon::new(IconName::Slash))
                    .child(Label::new(self.command_name.clone()))
                    .when_some(self.argument_hint, |this, hint| {
                        this.child(Label::new(format!(" {}", hint)).color(Color::Muted))
                    })
            )
            .when(self.is_loading, |this| {
                this.child(Indicator::new())
            })
    }
}
```

#### 3.2 Insert Slash Command Crease

**Location:** `crates/agent_ui/src/mention_set.rs`

```rust
pub(crate) fn insert_crease_for_slash_command(
    range: Range<Anchor>,
    command_name: SharedString,
    argument_hint: Option<SharedString>,
    editor: Entity<Editor>,
    window: &mut Window,
    cx: &mut App,
) -> Option<CreaseId> {
    let placeholder = FoldPlaceholder {
        render: Arc::new(move |_fold_id, _range, _cx: &mut App| {
            SlashCommandCrease::new(command_name.clone(), command_name.clone())
                .argument_hint(argument_hint.clone())
                .into_any_element()
        }),
        merge_adjacent: false,
        ..Default::default()
    };

    let crease = Crease::Inline {
        range,
        placeholder,
        render_toggle: None,
        render_trailer: None,
        metadata: None,
    };

    editor.update(cx, |editor, cx| {
        let ids = editor.insert_creases(vec![crease.clone()], cx);
        editor.fold_creases(vec![crease], false, window, cx);
        ids.get(0).copied()
    })
}
```

**Detection in MessageEditor:**

Add to `MessageEditor::new()` in the `EditorEvent::Edited` handler:

```rust
// In MessageEditor's subscribe_in handler for EditorEvent::Edited
if let EditorEvent::Edited { .. } = event {
    // Detect slash commands and insert creases
    this.detect_and_insert_slash_command_creases(&snapshot, window, cx);
}

// Method implementation:
fn detect_and_insert_slash_command_creases(
    &self,
    snapshot: &EditorSnapshot,
    window: &mut Window,
    cx: &mut App,
) {
    let buffer = snapshot.buffer();
    let text = buffer.text();
    
    // Parse for slash command at start of text
    let Some(parsed) = SlashCommandCompletion::try_parse(&text, 0) else {
        return;
    };
    
    let Some(command_name) = parsed.command else {
        return;
    };
    
    // Check if custom command
    let custom_commands = self.custom_commands.borrow();
    let Some(custom_command) = custom_commands.get(&command_name) else {
        return;
    };
    
    // Convert range to anchors and insert crease
    let start = buffer.anchor_before(MultiBufferOffset(parsed.source_range.start));
    let end = buffer.anchor_after(MultiBufferOffset(parsed.source_range.end));
    
    insert_crease_for_slash_command(
        start..end,
        SharedString::from(command_name),
        custom_command.argument_hint.clone().map(SharedString::from),
        self.editor.clone(),
        window,
        cx,
    );
}
```

**Phase 3 Deliverables:**

| Deliverable | How to Test |
|-------------|-------------|
| Typing `/test` renders a colored bubble | Type `/test`, see a button-like bubble appear |
| Bubble shows command name and optional argument hint | Verify the visual matches command's `argument_hint` from YAML |
| Bubble is not editable as raw text (it's a fold/crease) | Try to click inside bubble - it should not behave like text |
| Multiple commands in one message each render as separate bubbles | Type `/test /review`, see two distinct bubbles |

**Architecture Note:** Following the @-mention pattern, creases are inserted by MessageEditor on `EditorEvent::Edited`, not by the completion callback. This avoids complex closure lifetime issues.

---

### Phase 4: Command Execution

**Context:** This phase defines how command definitions are stored on Thread and how command invocations are processed into the LLM prompt. There are two distinct concepts:
1. **Available command definitions** - stored in `CommandsContext` on Thread, loaded async at thread creation, used for completions and system prompt
2. **Command invocations** - stored in message content as crease/bubble references (name + args), rendered as bubbles in the UI
3. **Processing** - template expansion happens ONLY when building the LLM request in `thread.rs`, not on message submit

The message editor does NOT process commands on submit—it sends the raw invocation (creased as a bubble) to the Thread. The Thread stores the available command definitions in `CommandsContext`. When building the LLM prompt, the Thread looks up the command definition, processes the template with the stored arguments, and injects the expanded text.

**Technical Implementation:**
- `CommandsContext` entity stores available command definitions (HashMap<String, Arc<CustomCommand>>)
- CommandsContext populated via `cx.spawn` → `cx.background_spawn` → `this.update` pattern (same as SkillsContext)
- CommandsContext is ONLY dereferenced for: (1) completion generation, (2) system prompt building, (3) LLM request processing
- Messages store raw command invocations (`/command args`) as creases/bubbles—NOT processed template output
- Template processing (`$ARGUMENTS`, `$1`, `$2` substitution) happens in `thread.rs` when constructing the LLM request
- Command lookup: message stores command name → Thread queries CommandsContext by name → processes template with stored args

#### 4.1 Store Command Invocation in Message

When the user submits a message containing `/command arg1 arg2`, the message stores the **raw invocation** (with the crease/bubble marker), NOT the processed template output. The template expansion happens later when the Thread builds the LLM request.

**In `message_editor.rs` on submit:**

```rust
fn submit_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    // The editor text contains crease markers for slash commands
    // We send this text AS-IS to the Thread—the creases are preserved
    let text_with_creases = self.editor.read(cx).text(cx);

    // The Thread will handle processing when building the LLM prompt
    self.thread.update(cx, |thread, cx| {
        thread.insert_user_message(text_with_creases, cx);
    });
}
```

The message content preserves the slash command as a crease/bubble reference. The crease metadata stores:
- `command_name`: The command identifier (e.g., "test")
- `args`: The arguments provided (e.g., ["arg1", "arg2"])

#### 4.2 Store Available Commands in Thread (Entity Pattern)

Following the `SkillsContext` pattern exactly, create a `CommandsContext` entity that holds the **available command definitions**. This is populated asynchronously when the Thread is created and is ONLY used for: (1) generating completions, (2) building the system prompt, and (3) processing invocations when building LLM requests.

**In `crates/agent/src/custom_commands.rs`:**

```rust
/// Context entity that holds discovered custom command definitions.
/// Populated asynchronously on creation via cx.spawn → background_spawn → update pattern.
/// Used for: completions, system prompt, and template processing during LLM request building.
pub struct CommandsContext {
    commands: Option<HashMap<String, Arc<CustomCommand>>>,
}

impl CommandsContext {
    /// Create a new CommandsContext and spawn background task to populate it.
    /// Pattern matches SkillsContext::new() exactly.
    pub fn new(worktree_roots: Vec<PathBuf>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx: &mut Context<Self>| {
            // Spawn foreground task that coordinates background work
            cx.spawn(async move |this, cx| {
                // Do disk I/O on background executor
                let commands = cx
                    .background_spawn(async move {
                        discover_custom_commands(&worktree_roots)
                    })
                    .await;

                // Update entity on main thread
                this.update(cx, |this, _cx| {
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

    /// Get the commands map. Returns empty map if not yet loaded.
    /// Callers should handle the not-loaded case gracefully (e.g., no completions yet).
    pub fn commands(&self) -> &HashMap<String, Arc<CustomCommand>> {
        self.commands.as_ref().unwrap_or(&EMPTY_MAP)
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

lazy_static::lazy_static! {
    static ref EMPTY_MAP: HashMap<String, Arc<CustomCommand>> = HashMap::default();
}
```

**Key architectural points:**
- `CommandsContext` stores command **definitions** (the templates), NOT invocations
- It is populated asynchronously at thread creation time (non-blocking)
- It is dereferenced in three places only:
  1. `completion_provider.rs` - to show available commands when user types `/`
  2. `system_prompt.hbs` - to list available commands for the LLM
  3. `thread.rs` - to lookup and process command invocations when building prompts

#### 4.3 Process Commands When Building LLM Request

When the Thread builds the request to send to the LLM, it processes any command invocations in the message history:

**In `crates/agent/src/thread.rs`:**

```rust
impl Thread {
    /// Build the user message content for the LLM, processing any slash commands.
    fn build_user_message(&self, message_text: &str, cx: &App) -> String {
        // For each slash command crease in the message:
        // 1. Extract command_name and args from the crease metadata
        // 2. Lookup the command definition in available_commands
        // 3. Process the template: cmd.process(&args)
        // 4. Replace the command invocation with the expanded template

        let mut result = message_text.to_string();

        // Extract slash command invocations from creases
        for (command_name, args) in self.extract_command_invocations(&message_text) {
            if let Some(cmd) = self.available_commands.read(cx).get(&command_name) {
                let processed = cmd.process(&args);
                // Replace the /command args with the processed template
                result = result.replace(&format!("/{} {}", command_name, args.join(" ")), &processed);
            }
        }

        result
    }
}
```

**Phase 4 Deliverables:**

| Deliverable | How to Test |
|-------------|-------------|
| `CommandsContext` is populated async at thread creation | Add logging, verify disk discovery happens in background |
| CommandsContext is empty until loaded, then contains commands | Check `is_loaded()` and `commands()` behavior |
| Message stores `/command args` as crease, not processed text | Submit message, inspect Thread's stored message content |
| Command template is expanded when building LLM request | Log `build_user_message()` output, verify `$1`, `$2` replaced |
| Message history shows bubble UI, not expanded template | View thread history, verify bubbles render correctly |

**In `crates/agent/src/thread.rs`:**

Add to `Thread` struct:

```rust
pub struct Thread {
    // ... existing fields ...
    pub(crate) templates: Arc<Templates>,
    /// Formatted available skills for the system prompt.
    available_skills: Entity<SkillsContext>,
    /// Discovered custom slash commands.
    available_commands: Entity<CommandsContext>,
    model: Option<Arc<dyn LanguageModel>>,
    // ... rest of fields ...
}
```

Create in `Thread::new_internal()`:

```rust
let worktree_roots: Vec<PathBuf> = project
    .worktrees(cx)
    .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf())
    .collect();
let available_skills = SkillsContext::new(worktree_roots.clone(), templates.clone(), cx);
let available_commands = CommandsContext::new(worktree_roots, cx); // NEW

Self {
    id: acp::SessionId::new(uuid::Uuid::new_v4().to_string()),
    // ... other fields ...
    available_skills,
    available_commands, // NEW
    // ... rest ...
}
```

Same pattern in `Thread::from_db()` when loading from database.

---

### Phase 5: System Prompt Integration

**Context:** This phase makes the LLM aware of available custom commands by including them in the system prompt. Just as the agent needs to know about available tools and skills, it should understand what slash commands the user has defined so it can respond appropriately when commands are invoked.

**Technical Implementation:**
- Add `CommandSummary` struct for serializing command info to the prompt template
- Extend `SystemPromptTemplate` with `custom_commands` field
- Update Handlebars template to render available commands section
- Format command list in `thread.rs` when building the request
- Filter and pass command metadata (name, description) to the template

Similar to skills, list available custom commands in the system prompt so the agent knows they exist.

**In `system_prompt.hbs`:**

```handlebars
{{#if custom_commands}}
## Available Custom Slash Commands

The user has defined the following custom slash commands that can be invoked with `/command-name`:

{{#each custom_commands}}
- `/{{this.name}}` - {{this.description}}
{{/each}}

These commands will be processed before being sent to you.
{{/if}}
```

**In `SystemPromptTemplate`:**

```rust
pub struct CommandSummary {
    pub name: String,
    pub description: String,
}

pub struct SystemPromptTemplate<'a> {
    // ... existing fields ...
    pub available_skills: String,
    pub custom_commands: Vec<CommandSummary>,
    // ... other fields ...
}
```

**Format commands for prompt** (in `thread.rs` when building request):

```rust
let custom_commands: Vec<CommandSummary> = self
    .available_commands
    .read(cx)
    .commands()
    .values()
    .map(|cmd| CommandSummary {
        name: cmd.name.clone(),
        description: cmd.description.clone(),
    })
    .collect();

let system_prompt = SystemPromptTemplate {
    // ... existing fields ...
    available_skills: self.available_skills.read(cx).formatted().to_string(),
    custom_commands,
    // ... other fields ...
};
```

**Phase 5 Deliverables:**

| Deliverable | How to Test |
|-------------|-------------|
| System prompt includes "Available Custom Slash Commands" section | Inspect full system prompt sent to LLM |
| Commands are listed with name and description | Verify format matches: `/name - description` |
| Section only appears when commands exist | Test with empty commands dir, verify no section |
| Both global and worktree commands appear | Create commands in both locations, verify both listed |

---



### Implementation Checklist

| Task | Location | Priority |
|------|----------|----------|
| Create `CustomCommand` struct with serde_yml parsing | `agent/src/custom_commands.rs` | P0 |
| Add `discover_custom_commands()` function | `agent/src/custom_commands.rs` | P0 |
| Add template processing (`$ARGUMENTS`, `$N`) | `agent/src/custom_commands.rs` | P0 |
| Extend `PromptCompletion` enum | `agent_ui/src/completion_provider.rs` | P0 |
| Update `try_parse()` for slash commands | `agent_ui/src/completion_provider.rs` | P0 |
| Generate completions in `completions()` | `agent_ui/src/completion_provider.rs` | P0 |
| Create `confirm_slash_command_callback()` | `agent_ui/src/completion_provider.rs` | P0 |
| Create `SlashCommandCrease` component | `agent_ui/src/ui/slash_command_crease.rs` | P0 |
| Add `insert_crease_for_slash_command()` | `agent_ui/src/mention_set.rs` or new file | P0 |
| Add custom commands to `Thread` | `agent/src/agent.rs` | P1 |
| Process commands on submit | `agent_ui/src/message_editor.rs` | P1 |
| Add to system prompt template | `agent/src/templates/system_prompt.hbs` | P1 |
| Create example commands in `~/.config/zed/commands/` | docs | P2 |

---

### Key Design Decisions

1. **Use `serde_yml`** for YAML parsing (user requirement)
2. **Follow skills pattern** for discovery and precedence
3. **Use completion provider + crease system** (not ACP slash commands)
4. **Static discovery** at thread creation (no hot reload initially, like skills)
5. **Worktree precedence** via HashMap insert (last wins)
6. **Bubble rendering** via editor creases/folds (like mentions)
7. **Template processing** before sending to LLM

---

### Testing Strategy

1. **Unit tests** for `CustomCommand::process()` with various args
2. **Unit tests** for YAML frontmatter parsing with `serde_yml`
3. **Integration tests** for discovery from directories
4. **Manual testing** for completion UI and bubble rendering
5. **E2E tests** for full command execution flow

---

### Future Enhancements (Phase 3+)

1. **Hot reload** - Watch commands directory and update without thread restart
2. **Shell injection** - `!`command` ` syntax execution
3. **File references** - `@path` injection
4. **Argument validation** - Validate arguments match hint pattern
5. **Nested namespaces** - Full support for `category:command` namespacing

---

### Summary

This plan integrates custom slash commands into the **built-in Zed Agent** (not ACP) by:

1. **Discovering** `.md` files from `~/.config/zed/commands/` and `.agents/commands/`
2. **Parsing** YAML frontmatter with `serde_yml` and markdown body
3. **Providing completions** via `PromptCompletionProvider` when user types `/`
4. **Rendering bubbles** via editor creases (like `@`-mentions)
5. **Processing templates** with `$ARGUMENTS` and `$N` before sending to LLM

The directory structure (`~/.config/zed/commands/` and `.agents/commands/`) is defined implicitly by the discovery code and documented in the Architecture Overview.
