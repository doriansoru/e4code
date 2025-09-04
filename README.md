# Lightweight Code Editor in Rust

This editor will have the following functions:

## Menus

### File

*   **New** - Completed

*   **Open** - Completed

*   **Open directory** - Completed

*   **Close this file** - Completed

*   **Close all files** - Completed

*   **Save** - Completed

*   **Save as** - Completed

*   **Exit** - Completed

### Edit

*   Search and replace (with regex support)

*   Cut

*   Copy

*   Paste

*   Settings (dark or light theme; font and size in the UI)

### ?

*   About

## Features

*   **Syntax Highlighting:** Uses Syntect for Rust and Python. When the cursor is on an open parenthesis, it will show the corresponding closed one and vice versa. It will recognize Rust files by the `.rs` extension.

*   **Status Bar:** Will display the current row and column.

*   **Tabbed Interface:** Supports opening multiple files in separate tabs.

*   **Directory Tree:** Will have a tree view for the currently open directory.

*   **Unsaved Changes Prompt:** If a file is modified and the user tries to create a new file or close the current one, it will prompt to save changes.
