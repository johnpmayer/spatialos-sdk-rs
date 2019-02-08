use std::path::Path;

use crate::worker::EntityId;
use crate::worker::component::{Component, ComponentDatabase};
use crate::worker::component::internal::ComponentData;
use crate::worker::entity::Entity;
use crate::worker::parameters::SnapshotParameters;
use crate::worker::internal::utils::cstr_to_string;
use spatialos_sdk_sys::worker::*;
use std::ffi::CStr;
use std::ffi::CString;
use std::marker::PhantomData;
use crate::worker::vtable::PASSTHROUGH_VTABLE;
use crate::worker::internal::schema::SchemaComponentData;

pub struct SnapshotOutputStream {
    ptr: *mut Worker_SnapshotOutputStream,
}

impl SnapshotOutputStream {
    pub fn new<P: AsRef<Path>>(filename: P, params: &SnapshotParameters) -> Self {
        let filename_cstr = CString::new(filename.as_ref().to_str().unwrap()).unwrap();

        let ptr = unsafe {
            Worker_SnapshotOutputStream_Create(filename_cstr.as_ptr(), &params.to_worker_sdk())
        };

        SnapshotOutputStream { ptr: ptr }
    }

    pub fn write_entity(&self, id: EntityId, entity: &Entity) -> Result<(), String> {
        let components = entity.raw_component_data();

        let wrk_entity = Worker_Entity {
            entity_id: id.id,
            components: components.as_ptr(),
            component_count: components.len() as u32
        };

        let success = match unsafe {
            Worker_SnapshotOutputStream_WriteEntity(self.ptr, &wrk_entity)
        } {
            0 => false,
            1 => true,
            _ => panic!("What")
        };

        if success {
            Ok(())
        }
        else {
            let msg_cstr = unsafe { Worker_SnapshotOutputStream_GetError(self.ptr) };
            let msg = cstr_to_string(msg_cstr);
            Err(msg)
        }
    }
}

impl Drop for SnapshotOutputStream {
    fn drop(&mut self) {
        unsafe { Worker_SnapshotOutputStream_Destroy(self.ptr) };
    }
}

pub struct SnapshotInputStream<'a> {
    ptr: *mut Worker_SnapshotInputStream,
    database: &'a ComponentDatabase
}

impl<'a> SnapshotInputStream<'a> {
    pub fn new<P: AsRef<Path>>(filename: P, database: &'a ComponentDatabase) -> Result<Self, String> {
        let filename_cstr = CString::new(filename.as_ref().to_str().unwrap()).unwrap();

        let vtables = &database.component_vtables;

        let params = Worker_SnapshotParameters {
            component_vtable_count: 0,
            component_vtables: ::std::ptr::null(),
            default_component_vtable: &PASSTHROUGH_VTABLE
        };

        let ptr = unsafe { Worker_SnapshotInputStream_Create(filename_cstr.as_ptr(), &params)};

        let stream = SnapshotInputStream {
            ptr,
            database
        };

        let err_ptr = unsafe { Worker_SnapshotInputStream_GetError(ptr) };

        if !err_ptr.is_null() {
            return Err(cstr_to_string(err_ptr));
        }

        Ok(stream)
    }

    pub fn has_next(&mut self) -> bool {
        match unsafe { Worker_SnapshotInputStream_HasNext(self.ptr)} {
            0 => false,
            1 => true,
            _ => panic!("What")
        }
    }

    pub fn read_entity(&mut self) -> Result<Entity, String> {
        let wrk_entity = unsafe { *Worker_SnapshotInputStream_ReadEntity(self.ptr) };
        let mut entity = Entity::new();

        let component_data = unsafe {
            ::std::slice::from_raw_parts(wrk_entity.components, wrk_entity.component_count as usize)
        };

        for component in component_data {
            let c = unsafe { *Worker_AcquireComponentData(component) };
            let drop_fn = self.database.component_vtables
                .iter()
                .filter(|table| table.component_id == c.component_id)
                .nth(0)
                .unwrap()
                .component_data_free
                .unwrap();

            entity.add_raw(c, drop_fn);
        }

        Ok(entity)
    }
}

impl<'a> Drop for SnapshotInputStream<'a> {
    fn drop(&mut self) {
        unsafe { Worker_SnapshotInputStream_Destroy(self.ptr) }
    }
}
