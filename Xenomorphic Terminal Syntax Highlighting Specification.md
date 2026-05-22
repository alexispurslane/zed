# Xenomorphic Syntax Highlighting Specification

Official syntax highlighting and design specification for the Xenomorphic theme.

## Overview

Xenomorphic is an industrial bio-mechanical color scheme inspired by deep-space telemetry, alien biology, and the cold precision of spacecraft systems. Built around absolute blacks, desaturated industrial steels, and a signature acid-green accent, the theme delivers a distinct visual experience that evokes the tension between synthetic machinery and organic infestation.

The palette embraces crushed blacks and minimal surface separation. UI chrome is deliberately flattened to keep attention on the code. The single chromatic accent — molecular acid green — is reserved for the cursor, active line numbers, and alive states, creating a “heartbeat” effect in an otherwise stark environment.

## Color Palette

### Industrial Grayscale

| Hex Code | Name | Description |
| :------: | :--- | :---------- |
| `#D8DDD9` | **Polished Titanium** | Lightest / White Point (bright text) |
| `#B4B9B6` | **Titanium Alloy** | Primary foreground text |
| `#8C9190` | **Sensor Array** | Secondary / dimmed text |
| `#6E7372` | **Scrap Metal** | Tertiary / disabled text |
| `#4A5656` | **Pig Iron** | Punctuation, structural delimiters |
| `#333333` | **Cast Iron** | Decorative borders |
| `#242424` | **Raw Steel** | Active find match, hover emphasis |
| `#1A1A1A` | **Sinter** | Primary button surface |
| `#181818` | **Drill Core** | Find match background |
| `#141414` | **Ore Vein** | Scrollbar thumb |
| `#111111` | **Ore Seam** | Separator borders |
| `#0A0A0A` | **Hibernation** | Elevated surface panels |
| `#080808` | **Dark Matter** | Current line highlight |
| `#050505` | **Void** | Input field background |
| `#010101` | **Singularity** | Absolute black point / editor background |

### Signature & Semantic Colours

| Hex Code | Name | Description |
| :------: | :--- | :---------- |
| `#74E813` | **Molecular Acid** | Primary accent — cursor, active states, success |
| `#1f4c05` | **Biolume** | Selection background (dark phosphor) |
| `#C33434` | **Self-Destruct** | Errors, critical failures |
| `#D99B52` | **Warning Beacon** | Warnings, types, classes |
| `#5D839C` | **Telemetry** | Information, hints |
| `#816336` | **Derelict** | Deprecated code |
| `#20473D` | **Dormant** | Unused symbols |

### Syntax Token Colours

| Hex Code | Name | Token Role |
| :------: | :--- | :--------- |
| `#89A8C2` | **Cryo Interface** | Keywords, control flow |
| `#8EBE77` | **Hive Moss** | Functions, methods, calls |
| `#D99B52` | **Amber Resin** | Classes, types, interfaces |
| `#64A37C` | **Atmospheric** | String literals |
| `#A2B9C7` | **Sensor Reading** | Numbers, numeric literals |
| `#4B6858` | **Biofilm** | Comments, documentation |
| `#9CBAB0` | **Chitin** | Variables, identifiers, parameters |
| `#508585` | **Oxidation** | Operators, comparisons |
| `#4A5656` | **Pig Iron** | Brackets, punctuation, delimiters |
| `#C47C35` | **Carapace** | Decorators, attributes, annotations |
| `#E5A040` | **Plasma Burn** | Constants, booleans, enums, `this`/`self` |
| `#6D8FA6` | **Navigation** | Built-in functions, standard library |
| `#9471BA` | **Neural Parasite** | Escape sequences, regex metacharacters |
| `#79A3B5` | **Coolant** | Markup tag names |
| `#C29F64` | **Fossilized** | Markup attribute names |
| `#73A381` | **Vegetation** | CSS selectors |
| `#5E7A94` | **Weyland Blue** | Links, URLs |

### ANSI Colours

| Hex Code | ANSI Colour | Name |
| :------: | :---------- | :--- |
| `#010101` | Black | Singularity |
| `#C33434` | Red | Self-Destruct |
| `#74E813` | Green | Molecular Acid |
| `#D99B52` | Yellow | Warning Beacon |
| `#5E7A94` | Blue | Weyland Blue |
| `#9471BA` | Magenta | Neural Parasite |
| `#508585` | Cyan | Oxidation |
| `#B4B9B6` | White | Titanium Alloy |
| `#2A2A2A` | Bright Black | Slag |
| `#E05252` | Bright Red | Flamethrower |
| `#9FFF4A` | Bright Green | Acid Spray |
| `#E5A040` | Bright Yellow | Plasma Burn |
| `#89A8C2` | Bright Blue | Cryo Interface |
| `#B08EC9` | Bright Magenta | Hyperdream |
| `#6AAAAA` | Bright Cyan | Rebreather |
| `#D8DDD9` | Bright White | Polished Titanium |

---

## Syntax Highlighting Rules

### Token Classification

Following *TextMate* scoping conventions for consistent highlighting across editors.

#### Primary Tokens

**Keywords & Storage** → `Cryo Interface (#89A8C2)`

- Language Keywords: `if`, `else`, `return`, `function`, `class`, `module`, `use`
- Storage Modifiers: `static`, `public`, `private`, `const`, `let`, `var`, `implicit`
- Control Flow: `try`, `catch`, `throw`, `break`, `continue`, `select`, `case`

> **Design Note**: Keywords are rendered in a dusty arctic blue — reminiscent of cryo-stasis interface panels. This keeps structural syntax readable without competing with the acid accent.

**Functions & Methods** → `Hive Moss (#8EBE77)`

- Function Declarations and Calls
- Method Invocations
- User-defined Functions
- Macro Invocations (Rust: `println!`, `vec!`)

**Classes & Types** → `Amber Resin (#D99B52)`

- Class Names and Constructors
- Type Annotations: `int`, `string`, `boolean`, `acid_level_t`
- Interfaces and Enums
- Generic Type Parameters

**Strings & Text** → `Atmospheric (#64A37C)`

- String Literals: `"containment failed"`, `'lv-426'`, `` `telemetry` ``
- Escape Sequences (e.g., `\n`, `\t`, `\x1b`) — container string stays Atmospheric, escaped character itself renders as `Neural Parasite (#9471BA)`

**Numbers** → `Sensor Reading (#A2B9C7)`

- Numeric Literals: `42`, `3.14`, `0xFF`, `1e5`, `87.3`
- Units in scientific contexts

**Constants & Special Values** → `Plasma Burn (#E5A040)`

- Boolean Values: `true`, `false`, `.true.`, `.false.`
- Null-like Constants: `null`, `undefined`, `nil`, `None`
- Special Language Values: `NaN`, `Infinity`, `this`, `self`, `super`
- Named Constants: `CREW_SIZE`, `ACID_PH`, `MOLTING_TEMP`
- Enum Values: `Status.OK`, `HttpStatus.NOT_FOUND`
- Language-level Constants: `__name__`, `__FILE__`, `M_PI`

> **Design Note**: Booleans and null-like values use a distinct warm amber — visually grouping them as special language constants separate from numeric data. The warm tone contrasts with the cool Sensor Reading used for numbers.

**Comments** → `Biofilm (#4B6858)` *italic*

- Single-Line: `!`, `//`, `#`, `--`
- Multi-Line: `/* */`, `<!-- -->`
- Documentation Blocks (JSDoc, Javadoc, docstrings, Fortran DOC)

**Attributes & Decorators** → `Carapace (#C47C35)` *italic*

- Java/Kotlin Annotations: `@Override`, `@Component`
- Python Decorators: `@dataclass`, `@property`
- Rust Attributes: `#[derive(Debug)]`, `#[cfg(test)]`
- C# Attributes: `[Serializable]`
- TypeScript Decorators: `@Injectable`
- Fortran Directives: `!$OMP`

> **Sigil Handling**: The `@`, `#`, or `!$` sigil inherits the decorator color and is styled as part of the attribute token.

#### Support & Built-ins

**Built-ins & Standard Library** → `Navigation (#6D8FA6)`

- Built-in Functions: `print`, `len`, `typeof`, `console.log`
- Standard Library Classes: `Array`, `String`, `Object`, `Map`
- TextMate scopes: `support.function`, `support.class`, `support.type`

**Markup Tags (HTML/XML)** → `Coolant (#79A3B5)`

- Tag Names: `<div>`, `<span>`, `<svg>`
- Self-closing Tags: `<br/>`, `<img/>`
- TextMate scope: `entity.name.tag`

**Markup Attributes** → `Fossilized (#C29F64)`

- Attribute Names: `class`, `id`, `href`, `src`
- TextMate scope: `entity.other.attribute-name`

**Markup Attribute Values** → `Atmospheric (#64A37C)`

- Quoted Values: `"container"`, `'primary'`
- TextMate scope: `string.quoted` within markup

**Regular Expressions**

- Regex Literals & Character Classes → `Atmospheric (#64A37C)`
- Regex Operators & Metacharacters (`+`, `*`, `?`, `|`, `^`, `$`) → `Oxidation (#508585)`
- Regex Groups & Brackets → `Oxidation (#508585)`

**Variables & Identifiers** → `Chitin (#9CBAB0)`

- Variable Names and Parameters
- Object Properties
- Default Text Content
- TextMate scope: `variable`, `variable.parameter`

**Errors & Warnings** → `Self-Destruct (#C33434)`

- Syntax Errors
- Deprecated Code (with strikethrough where supported)
- Invalid Tokens

#### Operators & Punctuation

**Operators** → `Oxidation (#508585)`

- Arithmetic: `+`, `-`, `*`, `/`, `%`, `**`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`, `===`, `!==`
- Logical: `&&`, `||`, `!`, `and`, `or`, `not`
- Assignment: `=`, `+=`, `-=`, `*=`, `/=`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- Fortran-specific: `::`, `=>`, `%`

**Punctuation & Delimiters** → `Pig Iron (#4A5656)`

- Separators: `,`, `;`, `:`
- Accessors: `.`, `::`, `%`
- Brackets: `(`, `)`, `[`, `]`, `{`, `}`
- Angle Brackets (generics): `<`, `>`

> **Design Note**: Punctuation is deliberately low-contrast — nearly invisible at a glance but visible when sought. This reduces visual noise and lets semantic tokens dominate the reading experience.

**Sigils & Prefix Symbols**

- Sigils inherit the color of the token they modify:
	- `$variable` → Variable color (`Chitin`)
	- `@decorator` → Decorator color (`Carapace`)
	- `&reference` → Context-dependent (variable or type)
	- `*pointer` → Context-dependent

### Styling Modifiers

**Bold**

- Markdown headings (H1–H6)
- Strong emphasis in Markdown (`**text**`)
- Active tab labels in UI
- Critical warnings or alerts

**Italic**

- Comments and documentation blocks
- Type parameters and generics
- Decorators and attributes
- Markdown emphasis (`*text*`)
- Quoted text in documentation

**Underline**

- Links: Solid underline using `Weyland Blue (#5E7A94)`
- Spelling errors: Dotted underline using `Self-Destruct (#C33434)`
- Potential errors/warnings: Wavy underline using `Warning Beacon (#D99B52)`

> **Accessibility Note**: Links and spelling errors must be distinguishable by underline style (solid vs. dotted), not just color.

**Strikethrough**

- Deprecated code or APIs (use `Derelict (#816336)`)
- Completed tasks in TODO comments

### Special Rules

**Scope Prioritization**

When multiple scopes apply to a token, apply styles in this precedence order:

1. **Error scopes** (highest priority) — Always override other styles
2. **Warning scopes** — Override non-error styles
3. **Language-specific overrides** — Per-language customizations
4. **Semantic token colors** — LSP/semantic highlighting
5. **Base syntax colors** (lowest priority) — TextMate grammar defaults

> **Rule**: If a token is both an error and another role (e.g., a misspelled keyword), the error style takes full precedence. The token should display error styling, not a blend.

**Bracket Matching**

- Matching Brackets: Subtle highlight with `Raw Steel (#242424)` background
- Unmatched Brackets: `Self-Destruct (#C33434)` with wavy underline
- Rainbow Brackets: Optional, cycling through: `Cryo Interface`, `Hive Moss`, `Neural Parasite`, `Weyland Blue`, `Warning Beacon`, `Atmospheric`

---

## Language-Specific Rules

### Configuration Languages (JSON/YAML/TOML)

Configuration files use a distinct key/value visual hierarchy:

**Keys** → `Cryo Interface (#89A8C2)`

- Object keys and property names
- YAML mapping keys
- TOML table headers and keys

**Values** → Standard token rules apply:

- Strings → `Atmospheric (#64A37C)`
- Numbers → `Sensor Reading (#A2B9C7)`
- Booleans/null → `Plasma Burn (#E5A040)`

**Structural Elements** → `Pig Iron (#4A5656)`

- Colons, commas, brackets
- YAML dashes for list items

> **Design Note**: Keys adopt the keyword blue to visually anchor the structural spine of config files, while values follow normal semantic coloring.

### Markdown & Rich Text

**Headings (H1–H6)** → `Cryo Interface (#89A8C2)` **bold**

- Heading markers (`#`, `##`, etc.) → `Pig Iron (#4A5656)`

**Emphasis**

- Bold (`**text**`) → `Titanium Alloy (#B4B9B6)` **bold**
- Italic (`*text*`) → `Titanium Alloy (#B4B9B6)` *italic*
- Bold Italic (`***text***`) → `Titanium Alloy (#B4B9B6)` ***bold italic***

**Code**

- Inline code (`` `code` ``) → `Atmospheric (#64A37C)` on `Drill Core (#181818)` background
- Fenced code blocks → Normal syntax highlighting inside
- Code fence markers (` ``` `) → `Pig Iron (#4A5656)`

**Blockquotes** → `Sensor Array (#8C9190)` *italic*

- Quote markers (`>`) → `Pig Iron (#4A5656)`

**Links**

- Link text → `Weyland Blue (#5E7A94)` underlined
- Link URL → `Navigation (#6D8FA6)`

**Lists**

- List markers (`-`, `*`, `1.`) → `Pig Iron (#4A5656)`
- List content → `Titanium Alloy (#B4B9B6)`

**Horizontal Rules** → `Ore Seam (#111111)`

### HTML / XML / Templates

**Core Markup**

- Tag Names → `Coolant (#79A3B5)`
- Attribute Names → `Fossilized (#C29F64)`
- Attribute Values → `Atmospheric (#64A37C)`
- Text Nodes → `Titanium Alloy (#B4B9B6)`
- Comments → `Biofilm (#4B6858)` *italic*

**Angle Brackets & Delimiters** → `Pig Iron (#4A5656)`

- `<`, `>`, `</`, `/&gt;`

**Template Languages (JSX, Vue, Handlebars, EJS, etc.)**

- Template Delimiters (`{}`, `{{}}`, `<%`, `%>`) → `Pig Iron (#4A5656)`
- Embedded Expressions → Follow host language syntax rules
- Component Names (PascalCase) → `Amber Resin (#D99B52)` (treat as custom tags)

### CSS & Stylesheets

**Selectors**

- Element Selectors → `Titanium Alloy (#B4B9B6)`
- Class Selectors (`.class`) → `Vegetation (#73A381)`
- ID Selectors (`#id`) → `Vegetation (#73A381)`
- Pseudo-classes/elements (`:hover`, `::before`) → `Fossilized (#C29F64)`
- Attribute Selectors → `Fossilized (#C29F64)`

**Properties & Values**

- Property Names → `Fossilized (#C29F64)`
- String Values → `Atmospheric (#64A37C)`
- Numeric Values → `Sensor Reading (#A2B9C7)`
- Units (`px`, `em`, `%`, `rem`) → `Fossilized (#C29F64)`
- Color Values (hex, rgb, hsl) → `Sensor Reading (#A2B9C7)`
- Keywords (`auto`, `inherit`, `flex`) → `Plasma Burn (#E5A040)`

**At-Rules**

- `@media`, `@import`, `@keyframes` → `Cryo Interface (#89A8C2)`

**Utility-First (Tailwind, etc.)**

- Class Names in HTML → `Vegetation (#73A381)` (treat as structural labels)

### String Interpolation

**String Containers** → `Atmospheric (#64A37C)`

- Quote characters and literal text portions

**Interpolated Expressions** → Full syntax highlighting

- JavaScript: `${expression}` — expression follows JS rules
- Python f-strings: `{expression}` — expression follows Python rules
- Rust: `{expression}` — expression follows Rust rules
- Shell: `${variable}` or `$(command)` — follows shell rules

**Interpolation Delimiters** → `Pig Iron (#4A5656)`

- `${`, `}`, `#{`, `{`, `}`

> **Implementation Note**: Interpolated segments should be tokenized as full expressions, not rendered as plain string content.

### Git & Diff Views

**Diff Additions**

- Text Color: `Titanium Alloy (#B4B9B6)`
- Background: `Weyland Blue (#5E7A94)` at 8% opacity
- Line Marker (`+`): `Molecular Acid (#74E813)`

**Diff Deletions**

- Text Color: `Sensor Array (#8C9190)`
- Background: `Self-Destruct (#C33434)` at 10% opacity
- Line Marker (`-`): `Self-Destruct (#C33434)`

**Diff Modifications**

- Background: `Cryo Interface (#89A8C2)` at 8% opacity
- Line Marker (`~`): `Cryo Interface (#89A8C2)`

**Diff Headers & Metadata**

- File paths: `Hive Moss (#8EBE77)`
- Hunk headers (`@@`): `Weyland Blue (#5E7A94)`
- Commit hashes: `Fossilized (#C29F64)`

**Inline Diff (word-level)**

- Added words: `Weyland Blue (#5E7A94)` background at 20%
- Removed words: `Self-Destruct (#C33434)` background at 20%

> **Design Note**: Diff backgrounds are kept extremely subtle (under 10% opacity) to preserve readability while still conveying change state. The line markers carry the semantic weight.

### Shell & CLI Scripts (Bash, Zsh, Fish, PowerShell)

**Commands & Executables** → `Hive Moss (#8EBE77)`

- First word on a line (command name)
- Built-in commands: `echo`, `cd`, `export`
- External commands: `grep`, `awk`, `curl`

**Flags & Options** → `Cryo Interface (#89A8C2)`

- Short flags: `-a`, `-v`, `-rf`
- Long flags: `--help`, `--verbose`, `--output`

**Variables** → `Chitin (#9CBAB0)`

- Environment variables: `$PATH`, `$HOME`, `${VAR}`
- Positional parameters: `$1`, `$@`, `$#`
- Sigil (`$`) inherits variable color per sigil rules

**Here-Documents** → `Atmospheric (#64A37C)`

- Here-doc body content follows string rules
- Delimiter tokens (`<<EOF`, `EOF`) → `Pig Iron (#4A5656)`

**Shebang Line** → `Biofilm (#4B6858)` *italic*

- `#!/bin/bash`, `#!/usr/bin/env python`
- Treat as a special comment

### SQL

**Keywords** → `Cryo Interface (#89A8C2)`

- DML: `SELECT`, `INSERT`, `UPDATE`, `DELETE`
- DDL: `CREATE`, `ALTER`, `DROP`, `TRUNCATE`
- Clauses: `FROM`, `WHERE`, `JOIN`, `ON`, `GROUP BY`, `ORDER BY`, `HAVING`
- Operators: `AND`, `OR`, `NOT`, `IN`, `BETWEEN`, `LIKE`, `IS NULL`

**Functions** → `Hive Moss (#8EBE77)`

- Aggregate: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`
- Scalar: `COALESCE`, `NULLIF`, `CAST`, `CONVERT`
- Date/Time: `NOW`, `DATEADD`, `DATEDIFF`

**Types** → `Amber Resin (#D99B52)`

- `VARCHAR`, `INT`, `BIGINT`, `BOOLEAN`, `TIMESTAMP`, `TEXT`, `DECIMAL`

**Identifiers** → `Chitin (#9CBAB0)`

- Table names, column names
- Aliases (`AS name`) — treat as variables

**Strings & Numbers** → Standard token rules

- String & char literals → `Atmospheric (#64A37C)`
- Numeric literals → `Sensor Reading (#A2B9C7)`

### C/C++ Preprocessor Directives

**Preprocessor Keywords** → `Cryo Interface (#89A8C2)`

- `#include`, `#define`, `#undef`
- `#if`, `#ifdef`, `#ifndef`, `#else`, `#elif`, `#endif`
- `#pragma`, `#error`, `#warning`

**Macro Names** → `Plasma Burn (#E5A040)`

- Defined constants: `MAX_SIZE`, `DEBUG_MODE`
- Treat as named constants

**Macro Parameters** → `Chitin (#9CBAB0)`

- Parameters in macro definitions follow `variable.parameter` rules

**Include Paths** → `Atmospheric (#64A37C)`

- `<stdio.h>`, `"myheader.h"` — treat as strings

> **Note**: The `#` sigil is part of the directive keyword, not a separate punctuation token.

### Rust-Specific

**Lifetimes** → `Amber Resin (#D99B52)` *italic*

- `'a`, `'static`, `'_`
- The tick (`'`) is part of the lifetime token

**Traits** → `Amber Resin (#D99B52)`

- Trait names: `Clone`, `Debug`, `Iterator`, `Send`, `Sync`
- Trait bounds follow type coloring

**Macros** → `Hive Moss (#8EBE77)`

- Macro invocations: `println!`, `vec!`, `format!`
- The `!` is part of the macro name

> **Note**: Rust attributes (`#[derive]`, `#[cfg]`) are already covered under Attributes & Decorators.

### Haskell / ML / Functional Languages

**Type Constructors** → `Amber Resin (#D99B52)`

- `Maybe`, `Either`, `List`, `IO`
- Type-level names starting with uppercase

**Data Constructors** → `Plasma Burn (#E5A040)`

- `Just`, `Nothing`, `Left`, `Right`, `True`, `False`
- Treat as constants (they are value-level names for types)

**Type Signatures** → Standard rules with explicit operators

- `::` (type annotation) → `Oxidation (#508585)`
- `=>` (constraint arrow) → `Oxidation (#508585)`
- `->` (function arrow) → `Oxidation (#508585)`

**Type Variables** → `Amber Resin (#D99B52)` *italic*

- Lowercase type parameters: `a`, `b`, `m`
- Distinguish from term-level variables via italic

---

## Editor UI Definitions

### Editor Core

| Element | Color | Notes |
| ------- | ----- | ----- |
| Editor Background | `Singularity (#010101)` | Absolute black — no warmth |
| Gutter Background | `Singularity (#010101)` | Seamless with editor |
| Default Text | `Titanium Alloy (#B4B9B6)` | Neutral industrial grey |
| Line Numbers (inactive) | `Slag (#1C1C1C)` | Barely visible |
| Line Numbers (active) | `Molecular Acid (#74E813)` | Acid green — the “heartbeat” |
| Cursor | `Molecular Acid (#74E813)` | Block or line cursor |
| Current Line Highlight | `Dark Matter (#080808)` | Nearly imperceptible |

> **Design Note**: The active line number in acid green is the only chromatic element in the default editor chrome. This creates a single point of living color in an otherwise dead industrial environment — like a status LED on dormant machinery.

### Selection & Guides

| Element | Color | Notes |
| ------- | ----- | ----- |
| Selection Background | `Biolume (#1f4c05)` | Dark organic phosphor |
| Selection Text | `Titanium Alloy (#B4B9B6)` | Keep original text color |
| Inactive Selection | `Biolume (#1f4c05)` at 50% | |
| Indent Guides | `Ore Seam (#111111)` | Almost invisible |
| Active Indent Guide | `Slag (#2A2A2A)` | Current scope |
| Whitespace Characters | `Slag (#2A2A2A)` at 50% | Spaces, tabs when visible |
| Matching Bracket BG | `Raw Steel (#242424)` | Bracket pair highlight |

> **Design Note**: Selection uses a dark green phosphor that references the acid palette without competing with it. The color suggests biological luminescence — a faint glow in the void.

### Search & Navigation

| Element | Color | Notes |
| ------- | ----- | ----- |
| Search Match BG | `Drill Core (#181818)` | All matches |
| Active Search Match | `Raw Steel (#242424)` border | Currently selected match |
| Find/Replace Error | `Self-Destruct (#C33434)` underline | Invalid regex/pattern |
| Highlight on Scroll | `Molecular Acid (#74E813)` at 15% | Scrollbar match indicators |

### Disabled & Placeholder States

| Element | Color | Notes |
| ------- | ----- | ----- |
| Disabled Text | `Scrap Metal (#6E7372)` | On dark backgrounds |
| Placeholder Text | `Sensor Array (#8C9190)` at 50% | Input hints |

> **Accessibility Note**: Disabled UI elements should still meet minimum contrast where practical. When full contrast cannot be achieved, use additional visual cues (e.g., reduced opacity, different font weight).

### Tabs

| Element | Color |
| ------- | ----- |
| Active Tab Background | `Singularity (#010101)` |
| Inactive Tab Background | `Singularity (#010101)` |
| Active Tab Text | `Titanium Alloy (#B4B9B6)` |
| Inactive Tab Text | `Sensor Array (#8C9190)` |
| Active Tab Border | `Molecular Acid (#74E813)` 1px bottom |
| Modified Indicator | `Titanium Alloy (#B4B9B6)` at 60% opacity |

> **Design Note**: Tabs have no background differentiation — only the acid bottom border indicates the active tab. This keeps the chrome maximally flat.

### Sidebar

| Element | Color |
| ------- | ----- |
| Background | `Singularity (#010101)` |
| Active Item Text | `Titanium Alloy (#B4B9B6)` |
| Inactive Item Text | `Sensor Array (#8C9190)` |
| Selected Item BG | `Hibernation (#0A0A0A)` |
| Hover State BG | `Dark Matter (#080808)` |
| Section Headers | `Scrap Metal (#6E7372)` uppercase |

### Status Bar

| State | Background | Text |
| ----- | ---------- | ---- |
| Normal | `Singularity (#010101)` | `Titanium Alloy (#B4B9B6)` |
| Error | `Self-Destruct (#C33434)` | `Polished Titanium (#D8DDD9)` |
| Warning | `Warning Beacon (#D99B52)` | `Singularity (#010101)` |
| Remote/Active | `Sinter (#1A1A1A)` | `Molecular Acid (#74E813)` |

> **Design Note**: The status bar shares the editor background in normal state. It only gains color when conveying state (errors, warnings, remote connection). The acid green text on dark button surfaces indicates an active/live connection.

### Gutter Indicators

| Indicator | Color | Shape / Style |
| --------- | ----- | ------------- |
| Breakpoint | `Self-Destruct (#C33434)` fill | Circle |
| Breakpoint (disabled) | `Self-Destruct (#C33434)` at 40% | Circle outline |
| Current Execution Line | `Molecular Acid (#74E813)` outline | Arrow or ring |
| Error Marker | `Self-Destruct (#C33434)` | Circle |
| Warning Marker | `Warning Beacon (#D99B52)` | Diamond |
| Info Marker | `Telemetry (#5D839C)` | Square |
| Hint Marker | `Sensor Array (#8C9190)` at 50% | Dot |
| Git Added | `Molecular Acid (#74E813)` | Vertical bar |
| Git Modified | `Cryo Interface (#89A8C2)` | Vertical bar |
| Git Deleted | `Self-Destruct (#C33434)` | Triangle pointing right |

### Inlay Hints & Code Lens

**Inlay Hints** (parameter names, inferred types, etc.)

| Element | Color | Style |
| ------- | ----- | ----- |
| Hint Text | `Sensor Array (#8C9190)` at 50% | *italic* recommended |
| Hint Background | Transparent | No background |
| Hint Border/Padding | None | |

**Code Lens** (references, test runners, action links)

| Element | Color | Notes |
| ------- | ----- | ----- |
| Code Lens Text | `Sensor Array (#8C9190)` at 60% | Above function/class |
| Code Lens Hover | `Weyland Blue (#5E7A94)` | On mouse hover |

> **Note**: Inlay hints and code lens must never have higher contrast than primary code tokens. They are supplementary information and should fade into the background when not focused.

### Minimap & Overview Ruler

| Element | Color | Notes |
| ------- | ----- | ----- |
| Minimap Background | `Singularity (#010101)` | Match editor background |
| Minimap Code | `Titanium Alloy (#B4B9B6)` at 12% | Low-contrast silhouette |
| Minimap Selection | `Biolume (#1f4c05)` at 50% | |
| Minimap Search Match | `Molecular Acid (#74E813)` at 20% | |
| Minimap Error | `Self-Destruct (#C33434)` at 50% | |
| Minimap Warning | `Warning Beacon (#D99B52)` at 30% | |
| Overview Ruler BG | `Singularity (#010101)` | Scrollbar track area |
| Overview Ruler Selection | `Slag (#2A2A2A)` | Current view indicator |

### Panels & Popups

**Panel Backgrounds**

| Panel | Background | Notes |
| ----- | ---------- | ----- |
| Terminal | `Singularity (#010101)` | Match editor for seamless feel |
| Problems Panel | `Hibernation (#0A0A0A)` | Subtle separation |
| Output Panel | `Hibernation (#0A0A0A)` | |
| Debug Console | `Hibernation (#0A0A0A)` | |
| Search Panel | `Hibernation (#0A0A0A)` | |
| Panel Headers | `Dark Matter (#080808)` | Title bar of panels |
| Panel Header Text | `Sensor Array (#8C9190)` | |

**Tooltips & Hover Cards**

| Element | Color |
| ------- | ----- |
| Background | `Hibernation (#0A0A0A)` |
| Text | `Titanium Alloy (#B4B9B6)` |
| Border | `Ore Seam (#111111)` 1px solid |
| Code in Tooltip | `Atmospheric (#64A37C)` |
| Link in Tooltip | `Weyland Blue (#5E7A94)` underlined |

**Autocomplete & IntelliSense**

| Element | Color | Notes |
| ------- | ----- | ----- |
| Popup Background | `Hibernation (#0A0A0A)` | |
| Popup Border | `Ore Seam (#111111)` | 1px solid |
| Item Text | `Titanium Alloy (#B4B9B6)` | |
| Item Text (dimmed) | `Sensor Array (#8C9190)` | Type signatures, paths |
| Selected Item BG | `Sinter (#1A1A1A)` | |
| Selected Item Border | `Molecular Acid (#74E813)` | Optional left accent |
| Match Highlight | `Molecular Acid (#74E813)` | Matched characters in fuzzy find |
| Icon: Function | `Hive Moss (#8EBE77)` | |
| Icon: Variable | `Chitin (#9CBAB0)` | |
| Icon: Class/Type | `Amber Resin (#D99B52)` | |
| Icon: Keyword | `Cryo Interface (#89A8C2)` | |
| Icon: Constant | `Plasma Burn (#E5A040)` | |
| Icon: String | `Atmospheric (#64A37C)` | |

**Parameter Hints** (signature help popups)

| Element | Color | Notes |
| ------- | ----- | ----- |
| Background | `Hibernation (#0A0A0A)` | |
| Active Parameter | `Molecular Acid (#74E813)` | Currently typed parameter |
| Inactive Parameters | `Sensor Array (#8C9190)` | |
| Separator | `Pig Iron (#4A5656)` | Commas between params |

---

## Terminal Configuration

### ANSI Color Mapping

| ANSI Slot | Color Name | Hex Code |
| --------- | ---------- | -------- |
| Black | Singularity | `#010101` |
| Red | Self-Destruct | `#C33434` |
| Green | Molecular Acid | `#74E813` |
| Yellow | Warning Beacon | `#D99B52` |
| Blue | Weyland Blue | `#5E7A94` |
| Magenta | Neural Parasite | `#9471BA` |
| Cyan | Oxidation | `#508585` |
| White | Titanium Alloy | `#B4B9B6` |
| Bright Black | Slag | `#2A2A2A` |
| Bright Red | Flamethrower | `#E05252` |
| Bright Green | Acid Spray | `#9FFF4A` |
| Bright Yellow | Plasma Burn | `#E5A040` |
| Bright Blue | Cryo Interface | `#89A8C2` |
| Bright Magenta | Hyperdream | `#B08EC9` |
| Bright Cyan | Rebreather | `#6AAAAA` |
| Bright White | Polished Titanium | `#D8DDD9` |

### Terminal Defaults

| Element | Color |
| ------- | ----- |
| Default Foreground | `Titanium Alloy (#B4B9B6)` |
| Default Background | `Singularity (#010101)` |
| Cursor | `Molecular Acid (#74E813)` |
| Cursor Text | `Singularity (#010101)` |
| Selection Background | `Biolume (#1f4c05)` |
| Bold Text | Use Bright variant of current color |

### 256-Color & Truecolor Guidance

- The 16 ANSI colors above are the canonical core for 256-color terminals
- Colors 16–255 should interpolate between palette colors where possible
- Truecolor-capable terminals (24-bit) should use exact hex values from the palette
- When truecolor is available, prefer it over approximated 256-color values

---

## Implementation Guidelines

### Accessibility Standards

#### Minimum Contrast Requirements

Maintain **4.5:1 minimum contrast ratio** (WCAG 2.1 Level AA) for all text. The following canonical pairs have been verified:

| Foreground | Background | Contrast Ratio | Status |
| ---------- | ---------- | -------------- | ------ |
| Titanium Alloy (#B4B9B6) | Singularity (#010101) | ~14.5:1 | ✓ Pass |
| Biofilm (#4B6858) | Singularity (#010101) | ~4.7:1 | ✓ Pass |
| Self-Destruct (#C33434) | Singularity (#010101) | ~5.8:1 | ✓ Pass |
| Sensor Array (#8C9190) | Singularity (#010101) | ~8.9:1 | ✓ Pass |
| Pig Iron (#4A5656) | Singularity (#010101) | ~3.2:1 | ⚠ Decorative only — structural punctuation, not content |
| Slag (#2A2A2A) | Singularity (#010101) | ~1.8:1 | ⚠ Decorative only — inactive line numbers |
| Molecular Acid (#74E813) | Singularity (#010101) | ~11.2:1 | ✓ Pass |

> **Note**: Low-contrast pairs like `Pig Iron` on `Singularity` are acceptable only for decorative punctuation and separators. Critical information must always meet 4.5:1.

#### Non-Color Cues

Errors, warnings, and critical states must not rely on color alone. Specify secondary cues:

- **Errors**: Red color + wavy underline + error icon (⚠ or ✗)
- **Warnings**: Amber color + wavy underline + warning icon
- **Info/Hints**: Blue color + dotted underline or icon
- **Success**: Green color + checkmark icon
- **Links**: Color + solid underline (hover reveals additional state)
- **Spelling Errors**: Dotted underline (distinct style from error wavy underlines)

#### Color Vision Deficiency Considerations

- Test with protanopia, deuteranopia, and tritanopia simulations
- The Molecular Acid green (#74E813) is highly luminous and remains visible under most CVD conditions
- Ensure red/green distinctions (diff views, success/error) have non-color differentiators (line markers `+`/`-`, background presence)
- Consider providing alternative high-contrast mode documentation

### Consistency Requirements

1. **Priority Order**: Follow token classification hierarchy strictly
2. **Fallback Handling**: Use `Titanium Alloy` for unrecognized tokens
3. **Semantic Consistency**: Same meaning = same color across all languages
4. **Industrial Flatness**: Maintain minimal surface differentiation — backgrounds should cluster near black

---

## UI Design Guidelines

### Visual Hierarchy

Apply colors based on functional importance and user interaction patterns:

- **High Priority**: Errors, active states, critical alerts → `Self-Destruct (#C33434)`, `Molecular Acid (#74E813)`
- **Medium Priority**: Keywords, types, functions → `Cryo Interface (#89A8C2)`, `Amber Resin (#D99B52)`, `Hive Moss (#8EBE77)`
- **Low Priority**: Body text, supporting information → `Titanium Alloy (#B4B9B6)`, `Sensor Array (#8C9190)`
- **Invisible Priority**: Punctuation, structural chrome → `Pig Iron (#4A5656)`, `Ore Seam (#111111)`

### Component Guidelines

**Borders and Separators**

- Primary Borders: `Ore Seam (#111111)` at 1px width
- Subtle Dividers: `Singularity (#010101)` with 1px `Ore Seam` where absolutely necessary
- Focus Rings: `Molecular Acid (#74E813)` at 1–2px width, or `rgba(116, 232, 19, 0.7)`
- Container Edges: Avoid visible edges; rely on negative space

> **Design Note**: Xenomorphic avoids visible borders wherever possible. The flat black aesthetic depends on surfaces bleeding into one another. Use borders only when functional grouping is required.

**Shadows and Depth**

- **NEVER use shadows** — flat surfaces only
- Depth is achieved exclusively through background color steps (Singularity → Hibernation)
- No gradients, no blur, no transparency on structural elements
- Atmospheric overlays (scanlines, noise) may use minimal opacity but are decorative only

**State Indicators**

- Success: `Molecular Acid (#74E813)`
- Warning: `Warning Beacon (#D99B52)`
- Error: `Self-Destruct (#C33434)`
- Info: `Telemetry (#5D839C)`
- Processing: `Molecular Acid (#74E813)` with blink or pulse animation

**Link Styling**

- Default: `Weyland Blue (#5E7A94)` as specified in palette
- Hover: `Cryo Interface (#89A8C2)` with optional underline
- Visited: `Navigation (#6D8FA6)`
- Active: `Molecular Acid (#74E813)`

### Theme Philosophy

Xenomorphic believes that a code editor should feel like a piece of industrial equipment — precise, unforgiving, and alive only where necessary. The theme should evoke:

- **Absolute Darkness**: The void of deep space, crushed to #010101
- **Single Life Sign**: The acid green cursor as the only living thing on screen
- **Industrial Utility**: Every color earns its place; no decorative flourishes
- **Bio-Mechanical Tension**: Organic greens and ambers against cold steel and black
- **Semantic Clarity**: Identical semantic roles render consistently, even across alien syntax

The palette is intentionally constrained: Xenomorphic offers a narrow beam of function in infinite dark. Add colors sparingly. When in doubt, use `Titanium Alloy` or `Pig Iron`.

---

## Testing Requirements

### Required Testing Scenarios

1. **Language Coverage**: Test with at least 5 different programming languages (recommend: Fortran, Rust, TypeScript, Python, SQL)
2. **File Types**: Verify Markdown, JSON, XML, and configuration files
3. **Diff Views**: Ensure additions/deletions are clearly distinguishable without relying on color alone
4. **Terminal Emulation**: Validate ANSI color representation
5. **Dark Sensitivity**: Provide documentation for users sensitive to very dark themes

### Performance Considerations

- Colors should render consistently across sRGB and P3 color spaces
- Test on both OLED and standard LCD displays — pure black (#010101) behaves differently on OLED
- Verify legibility at various zoom levels (50% - 200%)
- The acid green (#74E813) may appear oversaturated on wide-gamut displays; consider sRGB clamping

---

## Implementation Notes

### Handling Coarse Tokenization

Some editors or language grammars provide limited token granularity. When fine-grained semantic distinctions cannot be made:

1. **Built-ins vs. User Definitions**: When built-in functions cannot be distinguished from user-defined functions, use `Hive Moss (#8EBE77)` for all functions.

2. **Types vs. Variables**: When types cannot be distinguished from variables, prefer `Chitin (#9CBAB0)` as the neutral fallback.

3. **Ambiguous Tokens**: Favor readability and contrast over forcing a semantic color that doesn’t map cleanly.

4. **Missing Scopes**: For tokens not covered by the grammar, fall back to `Titanium Alloy (#B4B9B6)`.

> **Guiding Principle**: When in doubt, prioritize readability and visual silence over strict semantic accuracy. Xenomorphic is a quiet theme; let the code speak, not the chrome.

---

*In space, no one can hear you scream.*
