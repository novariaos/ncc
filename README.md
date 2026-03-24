# ncc

C compiler targeting the NVM bytecode for [NovariaOS](https://github/novariaos/novariaos-src/).

## Overview

ncc compiles a subset of C to NVM assembly, then assembles it into `.bin` binaries that run on the NVM stack-based VM.

**Pipeline:** `C source -> preprocessor -> lexer -> parser -> AST -> codegen -> NVM asm -> nvma -> .bin`

## Building

```
cargo build --release
```

## Usage

```
ncc [options] <input.c>
```

| Option | Description |
|---|---|
| `-o <file>` | Output file (default: `input.bin`) |
| `--emit-asm` | Print generated NVM assembly to stdout |
| `-I <dir>` | Add include search directory |

The `include/` directory in the working directory is added automatically.

Assembling `.asm` files directly is also supported:

```
ncc input.asm -o output.bin
```

## Example

```c
#include "stdio.h"

int main() {
    printf("Hello, world!\n");
    return 0;
}
```

```
ncc examples/hello.c -o hello.bin
```

## Supported C Subset

### Types
- `int`, `char`, `void`
- Pointers (`int *p`)
- Arrays (`int arr[10]`)
- Structs (`struct Point { int x; int y; }`)

### Statements
- `if` / `else`
- `while`, `do...while`, `for`
- `switch` / `case` / `default` / `break`
- `return`
- Local variable declarations with initializers

### Expressions
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `&&`, `||`, `!`
- Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
- Increment/decrement: `++`, `--` (prefix and postfix)
- Function calls, array indexing, struct field access (`.`)
- `sizeof`, casts
- String and character literals with escape sequences

### Preprocessor
- `#include "file.h"` and `#include <file.h>`
- `#define NAME value`
- `#ifdef` / `#ifndef` / `#endif`
- `__attribute__((...))` stripping

### Standard Library Headers
- `stdio.h` — `printf` (compiler intrinsic), `putchar`, `print_int`, `print_char`, `print_ln`
- `stdlib.h` — `abs`, `min`, `max`
- `string.h` — `strlen`, `strcmp`
- `nvm.h` — direct NVM syscall wrappers

## NVM Target

- Stack-based VM, 32-bit signed integers, big-endian
- 1024 stack cells, 512 process-scoped local slots
- Frame-based calling convention: `enter` / `leave` / `ret` / `load_arg`
- Output goes through `/dev/tty` (opened at program startup)
- `printf` is a compiler intrinsic — format string is parsed at compile time

## Project Structure

```
src/
  main.rs          CLI entry point
  cc/              C compiler frontend
    token.rs       Token types
    lexer.rs       Tokenizer
    preprocess.rs  Minimal C preprocessor
    ast.rs         AST node types
    parser.rs      Recursive descent parser
    types.rs       Type resolution and struct layouts
    codegen.rs     AST to NVM assembly generation
  nvm/             NVM definitions
    asm.rs         Assembly text builder
    opcodes.rs     Opcode constants
  ffi/             FFI to nvma assembler
  nvma/            nvma C assembler (asm text -> binary)
include/           C standard library headers for NVM
```

## Limitations

- No bitwise or shift operators
- No pointer arithmetic or dereferencing (limited pointer support)
- No `enum`, `union`, `typedef`
- Variable-index array access uses switch-dispatch (max ~64 elements)
- `printf` format string must be a string literal
- `%s` in `printf` only works with string literal arguments
