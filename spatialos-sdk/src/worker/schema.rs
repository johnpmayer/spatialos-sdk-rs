use crate::worker::component::ComponentId;
use spatialos_sdk_sys::worker::*;
use std::collections::BTreeMap;

pub type FieldId = u32;

#[derive(Debug)]
pub struct SchemaComponentUpdate {
    pub component_id: ComponentId,
    pub internal: *mut Schema_ComponentUpdate,
}

impl SchemaComponentUpdate {
    pub fn new(component_id: ComponentId) -> SchemaComponentUpdate {
        SchemaComponentUpdate {
            component_id,
            internal: unsafe { Schema_CreateComponentUpdate(component_id) },
        }
    }

    pub fn component_id(&self) -> ComponentId {
        unsafe { Schema_GetComponentUpdateComponentId(self.internal) }
    }

    pub fn fields(&self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentUpdateFields(self.internal) },
        }
    }

    pub fn fields_mut(&mut self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentUpdateFields(self.internal) },
        }
    }

    pub fn events(&self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentUpdateEvents(self.internal) },
        }
    }

    pub fn events_mut(&mut self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentUpdateEvents(self.internal) },
        }
    }

    // TODO: Cleared fields.
}

#[derive(Debug)]
pub struct SchemaComponentData {
    pub component_id: ComponentId,
    pub internal: *mut Schema_ComponentData,
}

impl SchemaComponentData {
    pub fn new(component_id: ComponentId) -> SchemaComponentData {
        SchemaComponentData {
            component_id,
            internal: unsafe { Schema_CreateComponentData(component_id) },
        }
    }

    pub fn component_id(&self) -> ComponentId {
        unsafe { Schema_GetComponentDataComponentId(self.internal) }
    }

    pub fn fields(&self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentDataFields(self.internal) },
        }
    }

    pub fn fields_mut(&mut self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetComponentDataFields(self.internal) },
        }
    }
}

#[derive(Debug)]
pub struct SchemaCommandRequest {
    pub component_id: ComponentId,
    pub internal: *mut Schema_CommandRequest,
}

impl SchemaCommandRequest {
    pub fn new(component_id: ComponentId, command_index: FieldId) -> SchemaCommandRequest {
        SchemaCommandRequest {
            component_id,
            internal: unsafe { Schema_CreateCommandRequest(component_id, command_index) },
        }
    }

    pub fn component_id(&self) -> ComponentId {
        unsafe { Schema_GetCommandRequestComponentId(self.internal) }
    }

    pub fn command_index(&self) -> FieldId {
        unsafe { Schema_GetCommandRequestCommandIndex(self.internal) }
    }

    pub fn object(&self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetCommandRequestObject(self.internal) },
        }
    }

    pub fn object_mut(&mut self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetCommandRequestObject(self.internal) },
        }
    }
}

#[derive(Debug)]
pub struct SchemaCommandResponse {
    pub component_id: ComponentId,
    pub internal: *mut Schema_CommandResponse,
}

impl SchemaCommandResponse {
    pub fn new(component_id: u32, command_index: u32) -> SchemaCommandResponse {
        SchemaCommandResponse {
            component_id,
            internal: unsafe { Schema_CreateCommandResponse(component_id, command_index) },
        }
    }

    pub fn component_id(&self) -> ComponentId {
        unsafe { Schema_GetCommandResponseComponentId(self.internal) }
    }

    pub fn command_index(&self) -> FieldId {
        unsafe { Schema_GetCommandResponseCommandIndex(self.internal) }
    }

    pub fn object(&self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetCommandResponseObject(self.internal) },
        }
    }

    pub fn object_mut(&mut self) -> SchemaObject {
        SchemaObject {
            internal: unsafe { Schema_GetCommandResponseObject(self.internal) },
        }
    }
}

#[derive(Debug)]
pub struct SchemaObject {
    internal: *mut Schema_Object,
}

impl SchemaObject {
    pub unsafe fn field<T: SchemaType>(&self, field: FieldId) -> T::RustType {
        T::from_field(self, field)
    }

    pub unsafe fn deserialize<T: SchemaObjectType>(&self) -> T {
        T::from_schema_object(self)
    }
}

// =================================================================================================
// Schema Conversion Traits
// =================================================================================================

pub trait SchemaType: Sized {
    type RustType: Sized;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType;
}

pub trait SchemaIndexType: SchemaType {
    fn field_count(schema_object: &SchemaObject, field: FieldId) -> u32;

    fn index_field(schema_object: &SchemaObject, field: FieldId, index: u32) -> Self::RustType;
}

pub trait SchemaListType: SchemaIndexType {
    fn get_field_list(schema_object: &SchemaObject, field: FieldId, data: &mut Vec<Self::RustType>);
}

/// A type that can be deserialized from an entire `SchemaObject`.
pub trait SchemaObjectType: Sized {
    fn from_schema_object(schema_object: &SchemaObject) -> Self;
}

impl<T: SchemaObjectType> SchemaType for T {
    type RustType = Self;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        let field_object = unsafe { Schema_GetObject(schema_object.internal, field) };
        T::from_schema_object(&SchemaObject {
            internal: field_object,
        })
    }
}

impl<T: SchemaObjectType> SchemaIndexType for T {
    fn field_count(schema_object: &SchemaObject, field: FieldId) -> u32 {
        unsafe { Schema_GetObjectCount(schema_object.internal, field) }
    }

    fn index_field(schema_object: &SchemaObject, field: FieldId, index: u32) -> Self::RustType {
        let field_object = unsafe { Schema_IndexObject(schema_object.internal, field, index) };
        T::from_schema_object(&SchemaObject {
            internal: field_object,
        })
    }
}

// =================================================================================================
// Schema Conversion Implementations for Primitive Types
// =================================================================================================

macro_rules! impl_primitive_field {
    (
        $rust_type:ty,
        $schema_type:ident,
        $schema_get:ident,
        $schema_index:ident,
        $schema_count:ident,
        $schema_add:ident,
        $schema_add_list:ident,
        $schema_get_list:ident,
    ) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $schema_type;

        impl SchemaType for $schema_type {
            type RustType = $rust_type;

            fn from_field(input: &SchemaObject, field: FieldId) -> Self::RustType {
                unsafe { $schema_get(input.internal, field) }
            }
        }

        impl SchemaIndexType for $schema_type {
            fn field_count(input: &SchemaObject, field: FieldId) -> u32 {
                unsafe { $schema_count(input.internal, field) }
            }

            fn index_field(
                schema_object: &SchemaObject,
                field: FieldId,
                index: u32,
            ) -> Self::RustType {
                unsafe { $schema_index(schema_object.internal, field, index) }
            }
        }

        impl SchemaListType for $schema_type {
            fn get_field_list(
                schema_object: &SchemaObject,
                field: FieldId,
                data: &mut Vec<Self::RustType>,
            ) {
                let count = Self::field_count(schema_object, field) as usize;

                // Ensure that there is enough capacity for the elements in the schema field.
                if data.capacity() < count {
                    data.reserve(count - data.capacity());
                }

                // Replace the contents of `data` with the list of values in the schema field.
                unsafe {
                    data.set_len(count);
                    $schema_get_list(schema_object.internal, field, data.as_mut_ptr());
                }
            }
        }
    };
}

impl_primitive_field!(
    f32,
    SchemaFloat,
    Schema_GetFloat,
    Schema_IndexFloat,
    Schema_GetFloatCount,
    Schema_AddFloat,
    Schema_AddFloatList,
    Schema_GetFloatList,
);
impl_primitive_field!(
    f64,
    SchemaDouble,
    Schema_GetDouble,
    Schema_IndexDouble,
    Schema_GetDoubleCount,
    Schema_AddDouble,
    Schema_AddDoubleList,
    Schema_GetDoubleList,
);
impl_primitive_field!(
    i32,
    SchemaInt32,
    Schema_GetInt32,
    Schema_IndexInt32,
    Schema_GetInt32Count,
    Schema_AddInt32,
    Schema_AddInt32List,
    Schema_GetInt32List,
);
impl_primitive_field!(
    i64,
    SchemaInt64,
    Schema_GetInt64,
    Schema_IndexInt64,
    Schema_GetInt64Count,
    Schema_AddInt64,
    Schema_AddInt64List,
    Schema_GetInt64List,
);
impl_primitive_field!(
    u32,
    SchemaUint32,
    Schema_GetUint32,
    Schema_IndexUint32,
    Schema_GetUint32Count,
    Schema_AddUint32,
    Schema_AddUint32List,
    Schema_GetUint32List,
);
impl_primitive_field!(
    u64,
    SchemaUint64,
    Schema_GetUint64,
    Schema_IndexUint64,
    Schema_GetUint64Count,
    Schema_AddUint64,
    Schema_AddUint64List,
    Schema_GetUint64List,
);
impl_primitive_field!(
    i32,
    SchemaSint32,
    Schema_GetSint32,
    Schema_IndexSint32,
    Schema_GetSint32Count,
    Schema_AddSint32,
    Schema_AddSint32List,
    Schema_GetSint32List,
);
impl_primitive_field!(
    i64,
    SchemaSint64,
    Schema_GetSint64,
    Schema_IndexSint64,
    Schema_GetSint64Count,
    Schema_AddSint64,
    Schema_AddSint64List,
    Schema_GetSint64List,
);
impl_primitive_field!(
    u32,
    SchemaFixed32,
    Schema_GetFixed32,
    Schema_IndexFixed32,
    Schema_GetFixed32Count,
    Schema_AddFixed32,
    Schema_AddFixed32List,
    Schema_GetFixed32List,
);
impl_primitive_field!(
    u64,
    SchemaFixed64,
    Schema_GetFixed64,
    Schema_IndexFixed64,
    Schema_GetFixed64Count,
    Schema_AddFixed64,
    Schema_AddFixed64List,
    Schema_GetFixed64List,
);
impl_primitive_field!(
    i32,
    SchemaSfixed32,
    Schema_GetSfixed32,
    Schema_IndexSfixed32,
    Schema_GetSfixed32Count,
    Schema_AddSfixed32,
    Schema_AddSfixed32List,
    Schema_GetSfixed32List,
);
impl_primitive_field!(
    i64,
    SchemaSfixed64,
    Schema_GetSfixed64,
    Schema_IndexSfixed64,
    Schema_GetSfixed64Count,
    Schema_AddSfixed64,
    Schema_AddSfixed64List,
    Schema_GetSfixed64List,
);
impl_primitive_field!(
    i64,
    SchemaEntityId,
    Schema_GetEntityId,
    Schema_IndexEntityId,
    Schema_GetEntityIdCount,
    Schema_AddEntityId,
    Schema_AddEntityIdList,
    Schema_GetEntityIdList,
);
impl_primitive_field!(
    u32,
    SchemaEnum,
    Schema_GetEnum,
    Schema_IndexEnum,
    Schema_GetEnumCount,
    Schema_AddEnum,
    Schema_AddEnumList,
    Schema_GetEnumList,
);

impl<T: SchemaIndexType> SchemaType for Option<T> {
    type RustType = Option<T::RustType>;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        let count = T::field_count(schema_object, field);
        match count {
            0 => None,
            1 => Some(T::from_field(schema_object, field)),
            _ => panic!(
                "Invalid count {} for `option` schema field {}",
                count, field
            ),
        }
    }
}

impl<K, V> SchemaType for BTreeMap<K, V>
where
    K: SchemaIndexType,
    V: SchemaIndexType,
    K::RustType: Ord,
{
    type RustType = BTreeMap<K::RustType, V::RustType>;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        // Get the map's schema object from the specified field on `schema_object`.
        let schema_object = &SchemaObject {
            internal: unsafe { Schema_GetObject(schema_object.internal, field) },
        };

        // Load each of the key-value pairs from the map object.
        let count = K::field_count(schema_object, SCHEMA_MAP_KEY_FIELD_ID);
        let mut result = BTreeMap::new();
        for index in 0..count {
            let key = K::index_field(schema_object, SCHEMA_MAP_KEY_FIELD_ID, index);
            let value = V::index_field(schema_object, SCHEMA_MAP_VALUE_FIELD_ID, index);
            result.insert(key, value);
        }

        result
    }
}

impl<T: SchemaIndexType> SchemaType for Vec<T> {
    type RustType = Vec<T::RustType>;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        let count = T::field_count(schema_object, field);

        // TODO: Provide a specialized version for types implementing `SchemaListType`.
        let mut result = Vec::with_capacity(count as usize);
        for index in 0..count {
            result.push(T::index_field(schema_object, field, index));
        }

        result
    }
}

impl SchemaType for String {
    type RustType = Self;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        let bytes = get_bytes(schema_object, field);
        std::str::from_utf8(bytes)
            .expect("Schema string was invalid UTF-8")
            .into()
    }
}

impl SchemaIndexType for String {
    fn field_count(schema_object: &SchemaObject, field: FieldId) -> u32 {
        unsafe { Schema_GetBytesCount(schema_object.internal, field) }
    }

    fn index_field(schema_object: &SchemaObject, field: FieldId, index: u32) -> Self::RustType {
        let bytes = index_bytes(schema_object, field, index);
        std::str::from_utf8(bytes)
            .expect("Schema string was invalid UTF-8")
            .into()
    }
}

impl SchemaType for Vec<u8> {
    type RustType = Self;

    fn from_field(schema_object: &SchemaObject, field: FieldId) -> Self::RustType {
        get_bytes(schema_object, field).into()
    }
}

impl SchemaIndexType for Vec<u8> {
    fn field_count(schema_object: &SchemaObject, field: FieldId) -> u32 {
        unsafe { Schema_GetBytesCount(schema_object.internal, field) }
    }

    fn index_field(schema_object: &SchemaObject, field: FieldId, index: u32) -> Self::RustType {
        index_bytes(schema_object, field, index).into()
    }
}

fn get_bytes(object: &SchemaObject, field: FieldId) -> &[u8] {
    unsafe {
        let data = Schema_GetBytes(object.internal, field);
        let len = Schema_GetBytesLength(object.internal, field);
        std::slice::from_raw_parts(data, len as usize)
    }
}

fn index_bytes(object: &SchemaObject, field: FieldId, index: u32) -> &[u8] {
    unsafe {
        let data = Schema_IndexBytes(object.internal, field, index);
        let len = Schema_IndexBytesLength(object.internal, field, index);
        std::slice::from_raw_parts(data, len as usize)
    }
}
