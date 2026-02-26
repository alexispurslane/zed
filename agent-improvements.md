# Agent Improvement Feasibility Report

### 1. Agent Skills (agentskills.io)

### 2. Task Tool with Checkbox UI

**Difficulty: Medium** (increased from Low-Medium due to UI)
**Feasibility: High**

**Updated Requirements:**
- Render as actual checkboxes (not just text)
- Support nesting (parent-child relationships)
- Actions: `add`, `update_status`, `create` (for initial list)

**Required Changes:**

1. **Data Model** (~100 lines):
   ```rust
   struct Task {
       id: Uuid,
       description: String,
       status: TaskStatus, // NotStarted, InProgress, Done
       parent_id: Option<Uuid>,
       created_at: DateTime<Utc>,
   }
   ```

2. **Tool Implementation** (~150 lines):
   - `TaskTool` with input schema supporting the three actions
   - `add`: Add single task, optionally with parent_id
   - `update_status`: Change status by task ID
   - `create`: Bulk create (replaces entire list)

3. **UI Rendering** (~200 lines):
   - New component in `agent_ui` crate
   - Tree structure for nesting
   - Checkboxes with status indicators
   - Events: `TaskUpdated`, `TaskAdded`

4. **Thread State** (~50 lines):
   - Add `tasks: Vec<Task>` to `Thread`
   - Persist to database (optional, can start ephemeral)

**Key UI Files to Modify:**
- `crates/agent_ui/src/tool_call_item.rs` or new `crates/agent_ui/src/task_list.rs`

---

### 4. Template Arguments for @-Rules (Premade Prompts)

**Clarification:** These are the `MentionUri::Rule` from `PromptStore`, not `.rules` files.

**Current Flow:**
1. User types `@RuleName` in input
2. Parsed as `MentionUri::Rule { id, name }`
3. Loaded via `PromptStore::default_prompt_metadata()`
4. Rendered into system prompt as `UserRulesContext`

**Required Changes:**

1. **Capture Arguments** (~100 lines):
   - When user types `@RuleName arg1 arg2`, capture everything after the rule name
   - Store in new `RuleInvocation` struct alongside the rule content

2. **Template Substitution** (~100 lines):
   - Detect `{}` placeholder in rule content
   - Substitute captured arguments
   - Handle case where no arguments provided (leave `{}` or error?)

3. **Parsing** (~50 lines):
   - Extend mention parsing or handle at input processing stage
   - Support quoted arguments for multi-word values

**Modified Structures:**
```rust
// In UserRulesContext or new struct
pub struct RuleWithArguments {
    pub rule: UserRulesContext,
    pub arguments: Vec<String>, // Template arguments
}

// Template substitution
fn apply_template(content: &str, args: &[String]) -> String {
    // Replace {} with args[0], or support named placeholders {0}, {1}, etc.
}
```

**Example Usage:**
```
User input: "@coding-style use ? for error propagation"
Rule content: "When handling errors, prefer `{}` over unwrap()"
Result: "When handling errors, prefer `?` over unwrap()"
```

**Difficulty: Medium** (mostly parsing and substitution logic)

---

## Summary Table

| Feature | Difficulty | Key Files |
|---------|-----------|-----------|
| Agent Skills | Low-Medium | New skill scanner, `templates.rs`, `system_prompt.hbs` |
| Task Tool (with UI) | Medium | `tools/task_tool.rs`, `thread.rs`, `agent_ui/src/` |
| Sub-agent filtering | Very Low | `tools/subagent_tool.rs:L497-568` |
| Rule Templates | Medium | Mention parsing, `agent.rs`, template substitution |

---

## Recommended Implementation Order

1. **Agent Skills** - Pure additive, no UI changes needed
2. **Sub-agent event filtering** - Single file change
3. **Rule Templates** - Requires input parsing changes
4. **Task Tool** - Most complex due to UI component

Would you like me to start implementing any of these features?
