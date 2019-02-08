use crate::worker::component::{self, Component, ComponentId, ComponentDatabase};
use spatialos_sdk_sys::worker::{Worker_ComponentData, Worker_Entity};
use std::collections::HashMap;
use std::ptr;

//#[derive(Debug)]
pub struct Entity<'a> {
    components: HashMap<ComponentId, Worker_ComponentData>,
    database: &'a ComponentDatabase
}

impl<'a> Entity<'a> {
    pub fn new(database: &'a ComponentDatabase) -> Self {
        Entity {
            components: HashMap::new(),
            database
        }
    }

    pub fn add<C: Component>(&mut self, component: C) {
        assert!(
            !self.components.contains_key(&C::ID),
            "Duplicate component added to `Entity`"
        );

        let data_ptr = component::handle_allocate(component);
        let raw_data = Worker_ComponentData {
            reserved: ptr::null_mut(),
            component_id: C::ID,
            schema_type: ptr::null_mut(),
            user_handle: data_ptr as *mut _,
        };

        self.components.insert(
            C::ID,
            raw_data
        );
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.components
            .get(&C::ID)
            .map(|data| unsafe { &*(data.user_handle as *const _) })
    }

    pub(crate) fn raw_component_data(&self) -> Vec<Worker_ComponentData> {
        self.components.values().map(|c| c.clone()).collect()
    }

    pub(crate) fn add_raw(&mut self, data: Worker_ComponentData) {
        self.components.insert(data.component_id, data);
    }
}

impl<'a> Drop for Entity<'a> {
    fn drop(&mut self) {
        for vtable in &self.database.component_vtables {
            let id = vtable.component_id;
            if self.components.contains_key(&id) {
                unsafe {
                    (vtable.component_data_free.unwrap())(0, ::std::ptr::null_mut(), self.components[&id].user_handle as *mut std::ffi::c_void)
                }
            }
        }
    }
}