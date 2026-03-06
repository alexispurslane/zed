use collections::HashMap;
use editor::display_map::CreaseId;
use gpui::SharedString;

/// Information about a slash command stored in CommandsSet.
pub struct SlashCommandInfo {
    pub name: SharedString,
}

/// Tracks slash command creases in the editor, similar to how MentionSet tracks @-mentions.
/// This allows slash command creases to be persisted in message history and reconstructed
/// when messages are displayed.
pub struct CommandsSet {
    commands: HashMap<CreaseId, SlashCommandInfo>,
}

impl CommandsSet {
    /// Create a new empty CommandsSet.
    pub fn new() -> Self {
        Self {
            commands: HashMap::default(),
        }
    }

    /// Insert a slash command into the set.
    pub fn insert(&mut self, crease_id: CreaseId, name: SharedString) {
        self.commands.insert(crease_id, SlashCommandInfo { name });
    }

    /// Remove a slash command from the set.
    pub fn remove(&mut self, crease_id: &CreaseId) {
        self.commands.remove(crease_id);
    }

    /// Get information about a slash command by its crease ID.
    pub fn get(&self, crease_id: &CreaseId) -> Option<&SlashCommandInfo> {
        self.commands.get(crease_id)
    }

    /// Check if a crease ID is in the set.
    pub fn contains(&self, crease_id: &CreaseId) -> bool {
        self.commands.contains_key(crease_id)
    }

    /// Clear all commands and return them as an iterator.
    pub fn clear(&mut self) -> impl Iterator<Item = (CreaseId, SlashCommandInfo)> + '_ {
        self.commands.drain()
    }

    /// Get the number of commands in the set.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Iterate over all commands in the set.
    pub fn iter(&self) -> impl Iterator<Item = (&CreaseId, &SlashCommandInfo)> {
        self.commands.iter()
    }
}

impl Default for CommandsSet {
    fn default() -> Self {
        Self::new()
    }
}
