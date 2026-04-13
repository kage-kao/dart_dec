use crate::types::*;
use crate::SnapshotError;
use ahash::AHashMap;
use byteorder::{ByteOrder, LittleEndian};
use dart_dec_profiles::DartProfile;
use tracing::{debug, trace, warn};

/// Collection of all parsed objects from the snapshot
#[derive(Debug, Clone)]
pub struct ObjectPool {
    pub objects: AHashMap<SnapshotAddr, DartObject>,
    pub object_order: Vec<SnapshotAddr>,
}

impl ObjectPool {
    pub fn new() -> Self {
        Self {
            objects: AHashMap::new(),
            object_order: Vec::new(),
        }
    }

    pub fn get(&self, addr: &SnapshotAddr) -> Option<&DartObject> {
        self.objects.get(addr)
    }

    pub fn insert(&mut self, addr: SnapshotAddr, obj: DartObject) {
        self.object_order.push(addr);
        self.objects.insert(addr, obj);
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Iterate over all objects in order
    pub fn iter(&self) -> impl Iterator<Item = (&SnapshotAddr, &DartObject)> {
        self.object_order
            .iter()
            .filter_map(move |addr| self.objects.get(addr).map(|obj| (addr, obj)))
    }

    /// Get all functions
    pub fn functions(&self) -> Vec<(&SnapshotAddr, &DartFunction)> {
        self.objects
            .iter()
            .filter_map(|(addr, obj)| match obj {
                DartObject::Function(f) => Some((addr, f)),
                _ => None,
            })
            .collect()
    }

    /// Get all classes
    pub fn classes(&self) -> Vec<(&SnapshotAddr, &DartClass)> {
        self.objects
            .iter()
            .filter_map(|(addr, obj)| match obj {
                DartObject::Class(c) => Some((addr, c)),
                _ => None,
            })
            .collect()
    }

    /// Get all strings
    pub fn strings(&self) -> Vec<(&SnapshotAddr, &DartString)> {
        self.objects
            .iter()
            .filter_map(|(addr, obj)| match obj {
                DartObject::String(s) => Some((addr, s)),
                _ => None,
            })
            .collect()
    }
}

/// Parse the object pool from raw snapshot data
pub fn parse_object_pool(
    data: &[u8],
    base_offset: usize,
    profile: &DartProfile,
    pointer_size: u32,
    compressed: bool,
) -> Result<ObjectPool, SnapshotError> {
    let mut pool = ObjectPool::new();
    let tags_config = &profile.object_tags;
    let class_id_mask = tags_config.class_id_mask_value();
    let class_id_shift = tags_config.class_id_shift;
    let size_tag_mask = tags_config.size_tag_mask_value();
    let size_tag_shift = tags_config.size_tag_shift;

    let tag_size = if pointer_size == 4 { 4usize } else { 8usize };
    let effective_ptr = if compressed { 4usize } else { pointer_size as usize };

    let mut offset = base_offset;
    let data_len = data.len();
    let mut object_count = 0u64;
    let max_objects = 500_000u64; // Safety limit

    while offset + tag_size <= data_len && object_count < max_objects {
        // Read object tag
        let raw_tag = if tag_size == 4 {
            LittleEndian::read_u32(&data[offset..offset + 4]) as u64
        } else {
            LittleEndian::read_u64(&data[offset..offset + 8])
        };

        // Extract class_id and size from tag
        let class_id = ((raw_tag >> class_id_shift) & class_id_mask) as u16;
        let size_from_tag = ((raw_tag & size_tag_mask) >> size_tag_shift) as u32;

        // Skip null/zero tags
        if raw_tag == 0 || class_id == 0 {
            offset += tag_size;
            continue;
        }

        let is_canonical = (raw_tag & (1 << tags_config.canonical_bit)) != 0;
        let is_old = (raw_tag & (1 << tags_config.old_and_not_marked_bit)) != 0;

        let addr = SnapshotAddr(offset as u64);
        let obj_data_start = offset + tag_size;

        // Determine object size
        let obj_size = if size_from_tag > 0 {
            size_from_tag as usize
        } else {
            // Read variable size from the object header
            if obj_data_start + 4 <= data_len {
                LittleEndian::read_u32(&data[obj_data_start..obj_data_start + 4]) as usize
            } else {
                break;
            }
        };

        // Ensure we have enough data
        if obj_data_start + obj_size > data_len {
            trace!(
                "Object at 0x{:x} exceeds data bounds (size {}), stopping",
                offset, obj_size
            );
            break;
        }

        let obj_bytes = &data[obj_data_start..obj_data_start + obj_size.min(data_len - obj_data_start)];

        // Parse object based on class_id
        let dart_obj = parse_object_by_class_id(
            class_id,
            addr,
            obj_bytes,
            profile,
            effective_ptr,
            ObjectTag {
                class_id,
                size: obj_size as u32,
                is_canonical,
                is_old,
            },
        );

        pool.insert(addr, dart_obj);
        object_count += 1;

        // Advance to next object (aligned)
        let alignment = if pointer_size == 4 { 8usize } else { 16usize };
        let total_size = tag_size + obj_size;
        let aligned_size = (total_size + alignment - 1) & !(alignment - 1);
        offset += aligned_size.max(tag_size);
    }

    debug!("Parsed {} objects from object pool", pool.len());
    Ok(pool)
}

fn parse_object_by_class_id(
    class_id: u16,
    addr: SnapshotAddr,
    data: &[u8],
    profile: &DartProfile,
    ptr_size: usize,
    _tag: ObjectTag,
) -> DartObject {
    // Match against known class IDs from the profile
    let ids = &profile.class_ids;

    if Some(&class_id) == ids.get("OneByteString") || Some(&class_id) == ids.get("TwoByteString") {
        return parse_string_object(addr, data, class_id, profile);
    }

    if Some(&class_id) == ids.get("Class") {
        return parse_class_object(addr, data, profile, ptr_size);
    }

    if Some(&class_id) == ids.get("Function") {
        return parse_function_object(addr, data, profile, ptr_size);
    }

    if Some(&class_id) == ids.get("Field") {
        return parse_field_object(addr, data, profile, ptr_size);
    }

    if Some(&class_id) == ids.get("Code") {
        return parse_code_object(addr, data, profile, ptr_size);
    }

    if Some(&class_id) == ids.get("Mint") {
        return parse_mint(data);
    }

    if Some(&class_id) == ids.get("Double") {
        return parse_double(data);
    }

    if Some(&class_id) == ids.get("Bool") {
        return parse_bool(data);
    }

    if Some(&class_id) == ids.get("Null") {
        return DartObject::Null;
    }

    if Some(&class_id) == ids.get("Array") || Some(&class_id) == ids.get("ImmutableArray") {
        let is_immutable = Some(&class_id) == ids.get("ImmutableArray");
        return parse_array_object(addr, data, ptr_size, is_immutable);
    }

    if Some(&class_id) == ids.get("Type") {
        return parse_type_object(addr, data, ptr_size);
    }

    if Some(&class_id) == ids.get("Closure") {
        return parse_closure_object(addr, data, ptr_size);
    }

    if Some(&class_id) == ids.get("Record") {
        return parse_record_object(addr, data, ptr_size);
    }

    // Unknown object — store raw bytes for later analysis
    DartObject::Unknown {
        class_id,
        raw: data.to_vec(),
    }
}

fn parse_string_object(
    addr: SnapshotAddr,
    data: &[u8],
    class_id: u16,
    profile: &DartProfile,
) -> DartObject {
    let layout = profile.class_layout.get("RawString");
    let (length_off, _hash_off) = if let Some(l) = layout {
        let len_off = l.fields.get("length_offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize;
        let hash_off = l.fields.get("hash_offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(12) as usize;
        (len_off, hash_off)
    } else {
        (8usize, 12usize)
    };

    let is_one_byte = profile.class_ids.get("OneByteString") == Some(&class_id);

    let length = if length_off + 4 <= data.len() {
        LittleEndian::read_u32(&data[length_off..length_off + 4]) as usize
    } else if data.len() >= 4 {
        LittleEndian::read_u32(&data[0..4]) as usize
    } else {
        0
    };

    let header_size = layout
        .and_then(|l| l.fields.get("header_size"))
        .and_then(|v| v.as_u64())
        .unwrap_or(16) as usize;

    let str_data_start = header_size.min(data.len());
    let str_data_end = (str_data_start + length).min(data.len());

    let value = if is_one_byte {
        String::from_utf8_lossy(&data[str_data_start..str_data_end]).to_string()
    } else {
        // Two-byte string (UTF-16)
        let mut chars = Vec::new();
        let mut i = str_data_start;
        while i + 1 < str_data_end {
            let code = LittleEndian::read_u16(&data[i..i + 2]);
            chars.push(code);
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    };

    DartObject::String(DartString {
        addr,
        value,
        is_one_byte,
    })
}

fn parse_class_object(
    addr: SnapshotAddr,
    data: &[u8],
    profile: &DartProfile,
    ptr_size: usize,
) -> DartObject {
    let layout = profile.class_layout.get("RawClass");
    let get_off = |field: &str, default: usize| -> usize {
        layout
            .and_then(|l| l.fields.get(field))
            .and_then(|v| v.as_u64())
            .unwrap_or(default as u64) as usize
    };

    let name_off = get_off("name_offset", 16);
    let super_off = get_off("super_type_offset", 24);
    let id_off = get_off("id_offset", 72);
    let state_off = get_off("state_bits_offset", 68);

    let name_addr = read_pointer(data, name_off, ptr_size);
    let super_addr = read_pointer(data, super_off, ptr_size);
    let class_id_val = read_u16(data, id_off);
    let state_bits = read_u32(data, state_off);

    let is_abstract = state_bits & 0x1 != 0;
    let is_sealed = state_bits & 0x2 != 0;
    let is_mixin = state_bits & 0x4 != 0;
    let is_enum = state_bits & 0x8 != 0;

    DartObject::Class(DartClass {
        addr,
        name: format!("Class@{}", addr),
        library: String::new(),
        super_class: if super_addr != 0 {
            Some(SnapshotAddr(super_addr))
        } else {
            None
        },
        interfaces: vec![],
        fields: vec![],
        functions: vec![],
        type_parameters: vec![],
        is_abstract,
        is_sealed,
        is_mixin,
        is_enum,
        class_id: class_id_val,
    })
}

fn parse_function_object(
    addr: SnapshotAddr,
    data: &[u8],
    profile: &DartProfile,
    ptr_size: usize,
) -> DartObject {
    let layout = profile.class_layout.get("RawFunction");
    let get_off = |field: &str, default: usize| -> usize {
        layout
            .and_then(|l| l.fields.get(field))
            .and_then(|v| v.as_u64())
            .unwrap_or(default as u64) as usize
    };

    let name_off = get_off("name_offset", 8);
    let owner_off = get_off("owner_offset", 16);
    let code_off = get_off("code_offset", 24);
    let kind_off = get_off("kind_bits_offset", 32);

    let _name_addr = read_pointer(data, name_off, ptr_size);
    let owner_addr = read_pointer(data, owner_off, ptr_size);
    let code_addr = read_pointer(data, code_off, ptr_size);
    let kind_bits = read_u32(data, kind_off);

    let kind = match kind_bits & 0xF {
        0 => FunctionKind::RegularFunction,
        1 => FunctionKind::Getter,
        2 => FunctionKind::Setter,
        3 => FunctionKind::Constructor,
        4 => FunctionKind::FactoryConstructor,
        5 => FunctionKind::ImplicitGetter,
        6 => FunctionKind::ImplicitSetter,
        7 => FunctionKind::ClosureFunction,
        8 => FunctionKind::AsyncClosure,
        _ => FunctionKind::RegularFunction,
    };

    let is_static = kind_bits & 0x10 != 0;
    let is_async = kind_bits & 0x20 != 0;
    let is_generator = kind_bits & 0x40 != 0;

    DartObject::Function(DartFunction {
        addr,
        name: format!("func@{}", addr),
        owner_class: if owner_addr != 0 {
            Some(SnapshotAddr(owner_addr))
        } else {
            None
        },
        code_addr: SnapshotAddr(code_addr),
        kind,
        is_static,
        is_async,
        is_generator,
        parameter_names: vec![],
        return_type: None,
    })
}

fn parse_field_object(
    addr: SnapshotAddr,
    data: &[u8],
    _profile: &DartProfile,
    ptr_size: usize,
) -> DartObject {
    let _name_addr = read_pointer(data, 8, ptr_size);
    let owner_addr = read_pointer(data, 16, ptr_size);
    let flags = read_u32(data, 24);

    DartObject::Field(DartField {
        addr,
        name: format!("field@{}", addr),
        owner_class: if owner_addr != 0 {
            Some(SnapshotAddr(owner_addr))
        } else {
            None
        },
        field_type: None,
        is_static: flags & 0x1 != 0,
        is_final: flags & 0x2 != 0,
        is_const: flags & 0x4 != 0,
        is_late: flags & 0x8 != 0,
    })
}

fn parse_code_object(
    addr: SnapshotAddr,
    data: &[u8],
    profile: &DartProfile,
    ptr_size: usize,
) -> DartObject {
    let layout = profile.class_layout.get("RawCode");
    let get_off = |field: &str, default: usize| -> usize {
        layout
            .and_then(|l| l.fields.get(field))
            .and_then(|v| v.as_u64())
            .unwrap_or(default as u64) as usize
    };

    let instr_off = get_off("instructions_offset", 8);
    let pool_off = get_off("object_pool_offset", 16);

    let instr_addr = read_pointer(data, instr_off, ptr_size);
    let pool_addr = read_pointer(data, pool_off, ptr_size);

    DartObject::Code(DartCode {
        addr,
        instructions_offset: instr_addr,
        instructions_size: 0, // will be resolved later
        object_pool_addr: if pool_addr != 0 {
            Some(SnapshotAddr(pool_addr))
        } else {
            None
        },
        pc_descriptors: vec![],
    })
}

fn parse_mint(data: &[u8]) -> DartObject {
    if data.len() >= 8 {
        DartObject::Mint(LittleEndian::read_i64(&data[0..8]))
    } else if data.len() >= 4 {
        DartObject::Mint(LittleEndian::read_i32(&data[0..4]) as i64)
    } else {
        DartObject::Mint(0)
    }
}

fn parse_double(data: &[u8]) -> DartObject {
    if data.len() >= 8 {
        DartObject::Double(LittleEndian::read_f64(&data[0..8]))
    } else {
        DartObject::Double(0.0)
    }
}

fn parse_bool(data: &[u8]) -> DartObject {
    DartObject::Bool(!data.is_empty() && data[0] != 0)
}

fn parse_array_object(
    addr: SnapshotAddr,
    data: &[u8],
    ptr_size: usize,
    is_immutable: bool,
) -> DartObject {
    let length = if data.len() >= ptr_size {
        read_pointer(data, 0, ptr_size) as usize
    } else {
        0
    };

    let mut elements = Vec::with_capacity(length.min(10000));
    let start = ptr_size; // after length field
    for i in 0..length {
        let elem_off = start + i * ptr_size;
        if elem_off + ptr_size > data.len() {
            break;
        }
        let elem_addr = read_pointer(data, elem_off, ptr_size);
        elements.push(SnapshotAddr(elem_addr));
    }

    DartObject::Array(DartArray {
        addr,
        elements,
        is_immutable,
        type_args: None,
    })
}

fn parse_type_object(
    addr: SnapshotAddr,
    data: &[u8],
    ptr_size: usize,
) -> DartObject {
    let _name_addr = read_pointer(data, 0, ptr_size);
    let flags = if data.len() >= ptr_size + 4 {
        read_u32(data, ptr_size)
    } else {
        0
    };

    DartObject::Type(DartType {
        addr,
        name: format!("type@{}", addr),
        is_nullable: flags & 0x1 != 0,
        type_arguments: vec![],
    })
}

fn parse_closure_object(
    addr: SnapshotAddr,
    data: &[u8],
    ptr_size: usize,
) -> DartObject {
    let func_addr = read_pointer(data, 0, ptr_size);
    let ctx_addr = read_pointer(data, ptr_size, ptr_size);

    DartObject::Closure(DartClosure {
        addr,
        function: SnapshotAddr(func_addr),
        context: if ctx_addr != 0 {
            Some(SnapshotAddr(ctx_addr))
        } else {
            None
        },
    })
}

fn parse_record_object(
    addr: SnapshotAddr,
    data: &[u8],
    ptr_size: usize,
) -> DartObject {
    let num_fields = if data.len() >= 4 {
        LittleEndian::read_u32(&data[0..4])
    } else {
        0
    };

    let mut field_values = Vec::new();
    let start = 4usize;
    for i in 0..num_fields as usize {
        let off = start + i * ptr_size;
        if off + ptr_size > data.len() {
            break;
        }
        field_values.push(SnapshotAddr(read_pointer(data, off, ptr_size)));
    }

    DartObject::Record(DartRecord {
        addr,
        num_fields,
        field_names: vec![],
        field_values,
    })
}

// Helper functions
fn read_pointer(data: &[u8], offset: usize, ptr_size: usize) -> u64 {
    if offset + ptr_size > data.len() {
        return 0;
    }
    if ptr_size == 4 {
        LittleEndian::read_u32(&data[offset..offset + 4]) as u64
    } else {
        LittleEndian::read_u64(&data[offset..offset + 8])
    }
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() {
        return 0;
    }
    LittleEndian::read_u32(&data[offset..offset + 4])
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    LittleEndian::read_u16(&data[offset..offset + 2])
}
