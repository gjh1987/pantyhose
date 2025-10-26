# Shell Scripts for Build Tools

This directory contains Windows batch scripts for building and managing various tools in the Pantyhose Server project.

## Scripts Overview

### 1. build_protobuf_message_id.cmd
**Purpose**: Compiles the proto-id-tool utility from source.

**What it does**:
- Navigates to `tools/proto/protoIdTool` directory
- Runs `cargo build --release` to compile the Rust project
- Copies the resulting `proto-id-tool.exe` to `tools/bin/`

**Output**: `tools/bin/proto-id-tool.exe`

**Usage**:
```cmd
build_protobuf_message_id.cmd
```

### 2. generate_protobuf_message_id.cmd
**Purpose**: Generates Rust code from protobuf definitions.

**What it does**:
- Checks if `proto-id-tool.exe` exists in `tools/bin/`
- Reads `.proto` files from `tools/proto/config/`
- Generates Rust message structures and ID mappings
- Outputs files to `src/proto/messages/`

**Prerequisites**: Must run `build_protobuf_message_id.cmd` first

**Output**: 
- `src/proto/messages/protobuf_message_id.rs` - Message ID constants and factory
- `src/proto/messages/protobuf/message/*.rs` - Generated message structures

**Usage**:
```cmd
generate_protobuf_message_id.cmd
```

### 3. build_and_generate.cmd
**Purpose**: One-step build and code generation.

**What it does**:
- Calls `build_protobuf_message_id.cmd` to compile the tool
- Calls `generate_protobuf_message_id.cmd` to generate code
- Provides a single command for the complete workflow

**Usage**:
```cmd
build_and_generate.cmd
```

### 4. build_pantyhose_server_tools.cmd
**Purpose**: Builds the Pantyhose Server Tools GUI application.

**What it does**:
- Navigates to `tools/pantyhose_server_tools/src-tauri`
- Runs `cargo build --release` to compile the Tauri application
- Creates `tools/bin/` directory if it doesn't exist
- Copies `pantyhose_server_tool.exe` to `tools/bin/`

**Output**: `tools/bin/pantyhose_server_tool.exe` (Desktop GUI for server management)

**Usage**:
```cmd
build_pantyhose_server_tools.cmd
```

## Directory Structure

```
tools/
├── bin/                          # All compiled executables go here
│   ├── proto-id-tool.exe        # Protobuf ID generator tool
│   └── pantyhose_server_tool.exe # Server management GUI
├── proto/
│   ├── config/                   # Proto definition files
│   └── protoIdTool/              # Proto-id-tool source code
├── pantyhose_server_tools/       # Tauri GUI application source
└── shell/                        # Build scripts (this directory)
```

## Generated Files Locations

- **Protobuf message files**: `src/proto/messages/protobuf/message/`
- **Message ID mappings**: `src/proto/messages/protobuf_message_id.rs`

## Manual Tool Usage

After building, you can run the tools manually:

**Proto ID Tool**:
```cmd
tools\bin\proto-id-tool.exe --proto-path tools\proto\config --language rust --output-path src\proto\messages
```

**Server Management GUI**:
```cmd
tools\bin\pantyhose_server_tool.exe
```

## Requirements

- Rust toolchain (latest stable)
- Cargo build system
- Windows environment (for .cmd scripts)
- For Tauri app: Node.js and npm/yarn (for frontend dependencies)