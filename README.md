# Lightweight Code Editor in Rust

This editor will have the following functions:

## Menus

### File

*   **New**
*   **Open**
*   **Open directory**
*   **Save**
*   **Save as**
*   **Close this file**
*   **Close all files**
*   **Exit**

### Edit

*   Search and replace (with regex support)
*   Cut
*   Copy
*   Paste
*   Indent
*   Outdent
*   Word wrap

### Settings

*   Preferences (dark or light theme; font and size in the UI)

### ?

*   About

## Features

*   **Syntax Highlighting:** Uses Syntect for Rust and Python. When the cursor is on an open parenthesis, it will show the corresponding closed one and vice versa. It will recognize Rust files by the `.rs` extension.

*   **Status Bar:** Will display the current row and column.

*   **Tabbed Interface:** Supports opening multiple files in separate tabs.

*   **Directory Tree:** Will have a tree view for the currently open directory.

<img width="800" height="437" alt="image" src="https://github.com/user-attachments/assets/c3986458-b3f4-4e12-a867-26d20835dbf1" />
