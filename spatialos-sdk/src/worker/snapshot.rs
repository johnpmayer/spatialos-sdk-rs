use std::path::Path;

use crate::worker::EntityId;
use crate::worker::entity::Entity;
use crate::worker::parameters::SnapshotParameters;
use crate::worker::internal::utils::cstr_to_string;
use spatialos_sdk_sys::worker::*;
use std::ffi::CStr;
use std::ffi::CString;

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

        let success = unsafe {
            Worker_SnapshotOutputStream_WriteEntity(self.ptr, &wrk_entity) as bool
        };

        if success {
            Ok(())
        }
        else {
            let msg_cstr = unsafe { Worker_SnapshotOutputStream_GetError(self.ptr) };
            let msg = cstr_to_string(msg_cstr);
            Err(msg);
        }
    }
}

impl Drop for SnapshotOutputStream {
    fn drop(&mut self) {
        unsafe { Worker_SnapshotOutputStream_Destroy(self.ptr) };
    }
}

pub struct SnapshotInputStream {
    ptr: *mut Worker_SnapshotInputStream
}

impl SnapshotInputStream {
    pub fn new<P: AsRef<Path>>(filename: P, params: &SnapshotParameters) -> Result<Self, String> {
        let filename_cstr = CString::new(filename.as_ref().to_str().unwrap()).unwrap();

        let ptr = unsafe { Worker_SnapshotInputStream_Create(filename_cstr.as_ptr(), &params.to_worker_sdk())};

        let stream = SnapshotInputStream {
            ptr
        };

        let err_ptr = unsafe { Worker_SnapshotInputStream_GetError(ptr) };

        if !err_ptr.is_null() {
            Err(cstr_to_string(err_ptr))
        }

        Ok(stream)
    }

    pub fn has_next(&mut self) -> bool {
        unsafe { Worker_SnapshotInputStream_HasNext(self.ptr) } as bool
    }

    pub fn read_entity(&mut self) -> Entity {
        let wrk_entity = unsafe { *Worker_SnapshotInputStream_ReadEntity(self.ptr) };

        let entity = Entity::new();

        // TODO: Need to think about this.
    }
}

impl Drop for SnapshotInputStream {
    fn drop(&mut self) {
        unsafe { Worker_SnapshotInputStream_Destroy(self.ptr) }
    }
}
