use crate::worker::Authority::Authoritative;
use crate::worker::{
    component::internal::{ComponentData, CommandRequest, CommandResponse},
    component::{Component, ComponentId, DATABASE},
    op::*,
    Authority, EntityId, RequestId
};
use spatialos_sdk_sys::{
    worker::Worker_AcquireComponentData, worker::Worker_ComponentData,
    worker::Worker_ReleaseComponentData, worker::Worker_CommandRequest,
    worker::Worker_AcquireCommandRequest, worker::Worker_ReleaseCommandRequest,
    worker::Worker_CommandResponse, worker::Worker_AcquireCommandResponse,
    worker::Worker_ReleaseCommandResponse
};
use std::{
    collections::{hash_map::HashMap, HashSet},
    ops::Deref,
};
use crate::worker::commands::{IncomingCommandRequest, OutgoingCommandRequest};

pub struct View {
    data: HashMap<ComponentId, HashMap<EntityId, OwnedComponentData>>,
    authority: HashMap<ComponentId, HashMap<EntityId, Authority>>,
    entities: HashSet<EntityId>,

    entities_added: HashSet<EntityId>,
    entities_removed: HashSet<EntityId>,
    components_updated: HashMap<ComponentId, Vec<EntityId>>,

    command_requests: HashMap<ComponentId, HashMap<EntityId, Vec<(RequestId<IncomingCommandRequest>, OwnedCommandRequestData)>>>,
    command_responses: HashMap<ComponentId, HashMap<EntityId, Vec<(RequestId<OutgoingCommandRequest>, Result<OwnedCommandResponseData, String>)>>>
}

impl View {
    pub fn new() -> Self {
        let mut view = View {
            data: HashMap::new(),
            authority: HashMap::new(),
            entities: HashSet::new(),
            entities_added: HashSet::new(),
            entities_removed: HashSet::new(),
            components_updated: HashMap::new(),

            command_requests: HashMap::new(),
            command_responses: HashMap::new()
        };

        for id in DATABASE.get_registered_component_ids() {
            view.data.insert(id, HashMap::new());
            view.authority.insert(id, HashMap::new());
            view.components_updated.insert(id, Vec::new());
            view.command_requests.insert(id, HashMap::new());
            view.command_responses.insert(id, HashMap::new());
        }

        view
    }

    pub fn clear_transient_data(&mut self) {
        self.entities_added.clear();
        self.entities_removed.clear();
        for (_, vec) in &mut self.components_updated {
            vec.clear();
        }

        for (_, map) in &mut self.command_requests {
            map.clear();
        }

        for (_, map) in &mut self.command_responses {
            map.clear();
        }
    }

    pub fn process_ops(&mut self, ops: &OpList) {
        for op in ops.iter() {
            match op {
                WorkerOp::AddEntity(op) => self.add_entity(op.entity_id),
                WorkerOp::RemoveEntity(op) => self.remove_entity(op.entity_id),
                WorkerOp::AddComponent(op) => self.add_component(&op),
                WorkerOp::RemoveComponent(op) => self.remove_component(&op),
                WorkerOp::ComponentUpdate(op) => self.handle_component_update(&op),
                WorkerOp::AuthorityChange(op) => self.set_authority(&op),
                WorkerOp::CommandRequest(op) => self.handle_command_request(&op),
                WorkerOp::CommandResponse(op) => self.handle_command_response(&op),
                _ => {}
            }
        }
    }

    pub fn get_component<C: Component>(&self, entity_id: EntityId) -> Option<&C> {
        self.data
            .get(&C::ID)
            .expect("Error")
            .get(&entity_id)
            .map(|data| unsafe { (&*((*data).user_handle as *const C)) })
    }

    pub fn get_command_requests<C: Component>(&self, entity_id: EntityId) -> Option<Vec<(RequestId<IncomingCommandRequest>, &C::CommandRequest)>> {
        self.command_requests
            .get(&C::ID)
            .expect("Error")
            .get(&entity_id)
            .map(|vec| vec.iter().map(|(id, data)| unsafe {
                (*id, (&*((*data).user_handle as *const C::CommandRequest)))
            }).collect())
    }

    pub fn get_command_responses<C: Component>(&self, entity_id: EntityId) -> Option<Vec<(RequestId<OutgoingCommandRequest>, Result<&C::CommandResponse, String>)>> {
        self.command_responses
            .get(&C::ID)
            .expect("Error")
            .get(&entity_id)
            .map(|vec|
                     vec.iter().map(|(id, result)| match result {
                         Ok(ref data) => (*id, Ok(unsafe {
                             (&*((*data).user_handle as *const C::CommandResponse)) })),
                         Err(e) => (*id, Err(e.clone()))
                     }).collect()
            )
    }

    pub fn get_authority<C: Component>(&self, entity_id: EntityId) -> Option<Authority> {
        self.authority
            .get(&C::ID)
            .expect("Error")
            .get(&entity_id)
            .map(|data| data.clone())
    }

    pub fn is_authoritative<C: Component>(&self, entity_id: EntityId) -> bool {
        self.get_authority::<C>(entity_id)
            .map_or(false, |auth| auth == Authoritative)
    }

    pub fn iter_entities(&self) -> ViewEntityIterator {
        ViewEntityIterator::new(self)
    }

    pub fn has_entity(&self, entity_id: EntityId) -> bool {
        self.entities.contains(&entity_id)
    }

    pub fn was_entity_added(&self, entity_id: EntityId) -> bool {
        self.entities_added.contains(&entity_id)
    }

    pub fn was_entity_removed(&self, entity_id: EntityId) -> bool {
        self.entities_removed.contains(&entity_id)
    }

    pub fn has_command_requests<C: Component>(&self, entity_id: EntityId) -> bool {
        self.command_requests.get(&C::ID).expect("Error").contains_key(&entity_id)
    }

    pub fn has_command_responses<C: Component>(&self, entity_id: EntityId) -> bool {
        self.command_responses.get(&C::ID).expect("Error").contains_key(&entity_id)
    }

    pub fn iter_entities_removed(&self) -> impl Iterator<Item = &EntityId> {
        self.entities_removed.iter()
    }

    pub fn query<'a, T: ViewQuery<'a>>(&'a self) -> (impl Iterator<Item = T> + 'a) {
        self.iter_entities()
            .filter(move |id| T::filter(self, **id))
            .map(move |id| T::select(self, *id))
    }

    fn add_entity(&mut self, entity_id: EntityId) {
        self.entities.insert(entity_id);
        self.entities_added.insert(entity_id);
    }

    fn remove_entity(&mut self, entity_id: EntityId) {
        self.entities.remove(&entity_id);
        self.entities_removed.insert(entity_id);
    }

    fn add_component(&mut self, op: &AddComponentOp) {
        let data = OwnedComponentData::from(op.data());
        self.data
            .get_mut(&op.component_id)
            .expect("Error")
            .insert(op.entity_id, data);
    }

    fn remove_component(&mut self, op: &RemoveComponentOp) {
        let mut _data = self
            .data
            .get_mut(&op.component_id)
            .expect("Error")
            .remove(&op.entity_id)
            .expect("Error");

        self.authority
            .get_mut(&op.component_id)
            .expect("Error")
            .remove(&op.entity_id);
    }

    fn set_authority(&mut self, op: &AuthorityChangeOp) {
        self.authority
            .get_mut(&op.component_id)
            .expect("Error")
            .insert(op.entity_id, op.authority);
    }

    fn handle_component_update(&mut self, op: &ComponentUpdateOp) {
        let merge = DATABASE.get_merge_method(op.component_id).expect("Error");
        let data = self
            .data
            .get_mut(&op.component_id)
            .expect("Error")
            .get_mut(&op.entity_id)
            .expect("Error");
        unsafe { merge(data.user_handle, op.component_update.user_handle) };
        self.components_updated.get_mut(&op.component_id).expect("Error").push(op.entity_id);
    }

    fn handle_command_request(&mut self, op: &CommandRequestOp) {
        let data = OwnedCommandRequestData::from(op.data());
        let entry = self.command_requests
            .get_mut(&op.component_id)
            .expect("Error")
            .entry(op.entity_id)
            .or_default()
            .push((op.request_id, data));
    }

    fn handle_command_response(&mut self, op: &CommandResponseOp) {
        let data = match &op.response {
            StatusCode::Success(resp) => Ok(OwnedCommandResponseData::from(resp.data())),
            StatusCode::Timeout(message) => Err(message.clone()),
            StatusCode::NotFound(message) => Err(message.clone()),
            StatusCode::AuthorityLost(message) => Err(message.clone()),
            StatusCode::PermissionDenied(message) => Err(message.clone()),
            StatusCode::ApplicationError(message) => Err(message.clone()),
            StatusCode::InternalError(message) => Err(message.clone()),
        };

        self.command_responses
            .get_mut(&op.component_id)
            .expect("Error")
            .entry(op.entity_id)
            .or_default()
            .push((op.request_id, data));
    }
}

pub struct ViewEntityIterator<'a> {
    iter: ::std::collections::hash_set::Iter<'a, EntityId>,
    view: &'a View,
}

impl<'a> ViewEntityIterator<'a> {
    fn new(view: &'a View) -> Self {
        ViewEntityIterator {
            iter: view.entities.iter(),
            view,
        }
    }
}

pub trait ViewQuery<'a> {
    fn filter(view: &View, entity_id: EntityId) -> bool;
    fn select(view: &'a View, entity_id: EntityId) -> Self;
}

impl<'a> Iterator for ViewEntityIterator<'a> {
    type Item = &'a EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

struct OwnedComponentData {
    data: Worker_ComponentData,
}

impl<'a> From<&'a ComponentData<'a>> for OwnedComponentData {
    fn from(data: &ComponentData) -> Self {
        let mut internal = Worker_ComponentData {
            reserved: data.reserved,
            component_id: data.component_id,
            schema_type: data.schema_type.internal,
            user_handle: data.user_handle as *mut _,
        };

        OwnedComponentData {
            data: unsafe { *Worker_AcquireComponentData(&mut internal) },
        }
    }
}

impl Deref for OwnedComponentData {
    type Target = Worker_ComponentData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Drop for OwnedComponentData {
    fn drop(&mut self) {
        unsafe { Worker_ReleaseComponentData(&mut self.data) };
    }
}


struct OwnedCommandRequestData {
    data: Worker_CommandRequest,
}

impl<'a> From<&'a CommandRequest<'a>> for OwnedCommandRequestData {
    fn from(data: &CommandRequest) -> Self {
        let mut internal = Worker_CommandRequest {
            reserved: data.reserved,
            component_id: data.component_id,
            schema_type: data.schema_type.internal,
            user_handle: data.user_handle as *mut _,
        };

        OwnedCommandRequestData {
            data: unsafe { *Worker_AcquireCommandRequest(&mut internal) },
        }
    }
}

impl Deref for OwnedCommandRequestData {
    type Target = Worker_CommandRequest;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Drop for OwnedCommandRequestData {
    fn drop(&mut self) {
        unsafe { Worker_ReleaseCommandRequest(&mut self.data) };
    }
}


struct OwnedCommandResponseData {
    data: Worker_CommandResponse,
}

impl From<&'_ CommandResponse<'_>> for OwnedCommandResponseData {
    fn from(data: &CommandResponse) -> Self {
        let mut internal = Worker_CommandResponse {
            reserved: data.reserved,
            component_id: data.component_id,
            schema_type: data.schema_type.internal,
            user_handle: data.user_handle as *mut _,
        };

        OwnedCommandResponseData {
            data: unsafe { *Worker_AcquireCommandResponse(&mut internal) },
        }
    }
}

impl Deref for OwnedCommandResponseData {
    type Target = Worker_CommandResponse;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Drop for OwnedCommandResponseData {
    fn drop(&mut self) {
        unsafe { Worker_ReleaseCommandResponse(&mut self.data) };
    }
}
