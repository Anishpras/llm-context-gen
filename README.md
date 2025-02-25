# LLM Context Generator

A command-line tool that processes a directory of source code files and creates a set of text files suitable for uploading as context to Large Language Models (LLMs).

## Features

- Traverses directory structures respecting `.gitignore` rules
- Generates text files with clear format: filename followed by content
- Creates a file tree visualization
- Skips binary files, large files, and common directories like `node_modules`
- Customizable ignore patterns

## Installation

### Using Cargo (recommended)

If you have Rust and Cargo installed:

```bash
cargo install llm-context-gen
```

### From Source

1. Clone the repository:

   ```bash
   git clone https://github.com/anishpras/llm-context-gen.git
   cd llm-context-gen
   ```

2. Build and install:
   ```bash
   cargo install --path .
   ```

## Usage

```bash
# Process the current directory and output to "llm-context"
llm-context-gen

# Process a specific directory
llm-context-gen -d /path/to/your/project

# Specify a custom output directory
llm-context-gen -o custom-output-dir

# Add additional directories to ignore
llm-context-gen -i "temp,logs,cache"

# See all options
llm-context-gen --help
```

## Output Format

The tool creates:

1. A text file for each source file with the format:

   ```
   filename.ext

   [file content]
   ```

2. A `file-tree.txt` showing the directory structure.

## License

MIT
# llm-context-gen
