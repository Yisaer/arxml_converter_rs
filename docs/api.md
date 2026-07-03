# API Reference

## `ArxmlCodec`

The top-level entry point. Auto-detects CP vs AP on `load()`.

### `ArxmlCodec::load`

```rust
pub fn load(path: impl AsRef<Path>) -> Result<Self, String>
```

Parses an AUTOSAR ARXML file and prepares the codec for type lookup and decoding.

- Returns an error if the file cannot be read or parsed.
- Returns an error if the file is not recognised as CP or AP.
- **CP detection** requires the top-level `AR-PACKAGES` to contain:
  `DataTypes`, `Communication`, `SoftwareTypes`, `System`, `Topology`,
  `DataTypeMappingSets`.
- **AP detection** requires the top-level `AR-PACKAGES` to contain:
  `dataTypes`, `interfaces`, `IAUTOSAR`.

### `resolve_cp`

```rust
pub fn resolve_cp(&self, service_id: u16, header_id: u32) -> Result<&DataType, String>
```

Follow the full CP 9-step lookup chain to find the `DataType` for a given
`(service_id, header_id)` pair.

Panics if called on an AP codec.

### `decode_cp`

```rust
pub fn decode_cp(
    &self,
    service_id: u16,
    header_id: u32,
    data: &[u8],
) -> Result<Value, String>
```

Resolve the CP type and decode `data` in one call. Equivalent to
`resolve_cp` followed by `Decoder::decode`.

Panics if called on an AP codec.

### `resolve_ap`

```rust
pub fn resolve_ap(&self, service_id: u16, event_id: u16) -> Result<&DataType, String>
```

AP lookup chain: `serviceId → Service → ServiceInterface → Event/Field → TypeRef → DataType`.

Panics if called on a CP codec.

### `decode_ap`

```rust
pub fn decode_ap(
    &self,
    service_id: u16,
    event_id: u16,
    data: &[u8],
) -> Result<Value, String>
```

Resolve the AP type and decode `data` in one call.

Panics if called on a CP codec.

---

## `DataType`

```rust
pub struct DataType {
    pub short_name: String,
    pub category: String,
    pub kind: DataTypeKind,
}
```

`category` is one of `"TYPE_REFERENCE"`, `"VALUE"`, `"STRING"`,
`"ARRAY"`, `"STRUCTURE"` (CP) or `"TYPE_REFERENCE"`, `"VECTOR"`,
`"ARRAY"`, `"STRUCTURE"` (AP).

### `DataTypeKind`

```rust
pub enum DataTypeKind {
    TypeReference(TypeReference),
    Array(ArrayType),
    Vector(VectorType),
    Structure(StructureType),
}
```

### `TypeReference`

```rust
pub struct TypeReference {
    pub type_name: String,       // e.g. "uint32", "/BaseTypes/float"
    pub string_size: Option<u64>, // Some(n) for fixed-length strings
}
```

### `ArrayType`

```rust
pub struct ArrayType {
    pub size: u64,           // number of elements (0 = variable-length)
    pub in_place: bool,
    pub element_ref: String, // path to element type
}
```

### `VectorType`

```rust
pub struct VectorType {
    pub element_ref: String, // path to element type
}
```

### `StructureType`

```rust
pub struct StructureType {
    pub fields: Vec<StructureField>,
}

pub struct StructureField {
    pub name: String,    // SHORT-NAME
    pub type_ref: String,// TYPE-TREF or TYPE-REFERENCE-REF path
    pub in_place: bool,
}
```

---

## `Value`

Produced by `decode_cp` / `decode_ap`. Designed for trivial conversion to
veloFlux's `Collection` / `Tuple` / `Value` types.

```rust
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    /// Variable-length or fixed-length UTF-8 string.
    Str(String),
    /// A structured record with ordered, named fields.
    Struct(Vec<(String, Value)>),
    /// Ordered list (arrays, vectors, sequences).
    Array(Vec<Value>),
    /// Opaque bytes.
    Bytes(Vec<u8>),
}
```

All integers are big-endian (AUTOSAR SOME/IP convention).

---

## `Decoder`

Low-level decoder that operates directly on `&[u8]` + `&DataType`.

```rust
pub struct Decoder<'a> { /* ... */ }

impl Decoder<'_> {
    pub fn new(types: &HashMap<String, DataType>) -> Decoder<'_>;
    pub fn decode(&self, data: &[u8], dt: &DataType) -> Result<(usize, Value), String>;
}
```

`decode` returns `(bytes_consumed, value)`, enabling callers to track the
read offset when decoding multiple consecutive values from a single buffer.

The type map is keyed by **lowercased** short-name.

---

## Utility functions

### `extract_last`

```rust
pub fn extract_last(r: &str) -> &str;
```

Returns the last `/`-separated segment of a path: `"/A/B/C"` → `"C"`.

### `merge_u16_to_u32`

```rust
pub fn merge_u16_to_u32(high: u16, low: u16) -> u32;
```

Merges two `u16` values into a single `u32` (used in CP header-ID
construction).

### `to_u16`, `to_u32`, `to_u64`, `to_i64`

```rust
pub fn to_u16(raw: &str) -> Result<u16, ParseIntError>;
pub fn to_u32(raw: &str) -> Result<u32, ParseIntError>;
pub fn to_u64(raw: &str) -> Result<u64, ParseIntError>;
pub fn to_i64(raw: &str) -> Result<i64, ParseIntError>;
```

Parse decimal strings into numeric types.

### XML helpers (`util::xml`)

```rust
pub fn find_child(node: Node, tag: &str) -> Option<Node>;
pub fn require_child(node: Node, tag: &str) -> Result<Node, String>;
pub fn child_text(node: Node, tag: &str) -> Option<&str>;
pub fn require_child_text(node: Node, tag: &str) -> Result<&str, String>;
pub fn get_shortname(node: Node) -> Result<&str, String>;
pub fn get_category(node: Node) -> Result<&str, String>;
pub fn get_elements(node: Node) -> Result<Node, String>;
pub fn get_array_size_semantics(node: Node) -> Result<bool, String>;
pub fn has_local_name(node: Node, tag: &str) -> bool;
```

All tag comparisons use **local names** (namespace-agnostic), so they work
with both `xmlns`-equipped and plain ARXML.
