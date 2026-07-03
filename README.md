# arxml_converter_rs

A Rust port of [arxml-converter](https://github.com/yisaer/arxml-converter), designed
to serve [veloFlux](https://github.com/yisaer/veloFlux)'s stream-format field by
decoding SOME/IP binary payloads according to AUTOSAR ARXML type schemas.

```
ARXML file → arxml_converter_rs → type schema + decoder → veloFlux stream format
```

## Features

- **CP (Classic Platform)** — full 9-step lookup chain from `(serviceId, headerId)`
  → PDU → signal → data-type
- **AP (Adaptive Platform)** — service-interface event/field → data-type lookup
- **Auto-detection** — `ArxmlCodec::load()` detects CP vs AP automatically
- **Big-endian binary decoding** — decodes `&[u8]` into structured `Value` trees
  with zero-copy where possible
- **Real-world tested** — validated against 14 MB production ARXML files

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
arxml_converter_rs = { git = "https://github.com/yisaer/arxml_converter_rs" }
```

```rust
use arxml_converter_rs::ArxmlCodec;

// Load any AUTOSAR ARXML — CP or AP is auto-detected
let codec = ArxmlCodec::load("path/to/file.arxml")?;

// CP: resolve and decode by (service_id, header_id)
let value = codec.decode_cp(10, 100, &[0x00, 0x00, 0x00, 0x2A])?;
// → Value::U32(42)

// AP: resolve and decode by (service_id, event_id)
let value = codec.decode_ap(100, 1, &payload)?;
```

## API overview

| Method | Platform | Parameters | Returns |
|---|---|---|---|
| `ArxmlCodec::load(path)` | CP / AP | file path | `Result<ArxmlCodec>` |
| `resolve_cp(svc, header)` | CP | `u16`, `u32` | `Result<&DataType>` |
| `decode_cp(svc, header, data)` | CP | `u16`, `u32`, `&[u8]` | `Result<Value>` |
| `resolve_ap(svc, event)` | AP | `u16`, `u16` | `Result<&DataType>` |
| `decode_ap(svc, event, data)` | AP | `u16`, `u16`, `&[u8]` | `Result<Value>` |

The `Value` enum covers all AUTOSAR basic and composite types:

```rust
pub enum Value {
    U8(u8), U16(u16), U32(u32), U64(u64),
    I8(i8), I16(i16), I32(i32), I64(i64),
    F32(f32), F64(f64),
    Bool(bool),
    Str(String),
    Struct(Vec<(String, Value)>),
    Array(Vec<Value>),
    Bytes(Vec<u8>),
}
```

See [docs/api.md](docs/api.md) for the full API reference.

## Project structure

```
src/
├── lib.rs              # ArxmlCodec — public entry point
├── ast/                # DataType, TypeReference, BasicType resolver
├── parser/
│   ├── cp.rs           # CP parser + 9-step lookup chain
│   ├── datatypes.rs    # CP data-type parser
│   ├── topology.rs     # Service-ID / Header-ID / PDU mappings
│   ├── communication.rs# PDU → signal mappings
│   ├── system.rs       # System-level operation refs
│   ├── software_types.rs # Interface refs
│   └── ap/             # AP parser (interfaces, data-types, IAUTOSAR)
├── decoder/
│   ├── mod.rs          # Binary decoder (11 primitives + composite recursion)
│   └── value.rs        # Decoded Value enum
└── util/
    ├── convert.rs      # Numeric & ID conversions
    └── xml.rs          # roxmltree query helpers (namespace-aware)
```

## Development

```bash
make build    # debug build
make test     # run all tests (81 tests, including 14 MB production ARXML)
make fmt      # format code
make clippy   # static analysis (-D warnings)
```

See [AGENTS.md](AGENTS.md) for contributing guidelines and architecture notes.
