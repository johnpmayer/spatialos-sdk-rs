use spatialos_sdk::worker::{component::*, schema::*, EntityId};
use std::collections::BTreeMap;

pub struct CustomComponent {
    pub name: String,
    pub count: i32,
    pub targets: Vec<EntityId>,
    pub target_names: BTreeMap<EntityId, String>,
    pub byte_collection: Vec<Vec<u8>>,
}

impl SchemaObjectType for CustomComponent {
    fn from_object(object: &SchemaObject) -> Self {
        Self {
            name: object.field::<String>(0),
            count: object.field::<SchemaSfixed32>(1),
            targets: object.field_array::<EntityId>(2),
            target_names: object.field::<BTreeMap<EntityId, String>>(3),
            byte_collection: object.field::<Vec<Vec<u8>>>(4),
        }
    }

    fn into_object<'a>(&'a self, object: &mut SchemaObject<'a>) {
        object.add_field::<String>(0, &self.name);
        object.add_field::<SchemaSfixed32>(1, &self.count);
        object.add_field_array::<EntityId>(2, &self.targets);
        object.add_field::<BTreeMap<EntityId, String>>(3, &self.target_names);
        object.add_field::<Vec<Vec<u8>>>(4, &self.byte_collection);
    }
}

impl Component for CustomComponent {
    const ID: ComponentId = 1234;
}

pub struct CustomComponentUpdate {
    pub name: Option<String>,
    pub count: Option<i32>,
    pub targets: Option<Vec<EntityId>>,
    pub target_names: Option<BTreeMap<EntityId, String>>,
    pub byte_collection: Option<Vec<Vec<u8>>>,
}

impl ComponentUpdate<CustomComponent> for CustomComponentUpdate {
    fn merge(&mut self, update: Self) {
        unimplemented!();
    }
}

impl SchemaObjectType for CustomComponentUpdate {
    fn from_object(object: &SchemaObject) -> Self {
        unimplemented!()
        // Self {
        //     name: object.field::<Option<String>>(0),
        //     count: object.field::<Option<SchemaSfixed32>>(1),
        //     targets: object.field_array::<Option<Vec<EntityId>>>(2),
        //     target_names: object.field::<Option<BTreeMap<EntityId, String>>>(3),
        //     byte_collection: object.field::<Option<Vec<Vec<u8>>>>(4),
        // }
    }

    fn into_object<'a>(&'a self, object: &mut SchemaObject<'a>) {
        unimplemented!();
        // object.add_field::<Option<String>>(0, &self.name);
        // object.add_field::<Option<SchemaSfixed32>>(1, &self.count);
        // object.add_field_array::<Option<Vec<EntityId>>>(2, &self.targets);
        // object.add_field::<Option<BTreeMap<EntityId, String>>>(3, &self.target_names);
        // object.add_field::<Optoin<Vec<Vec<u8>>>>(4, &self.byte_collection);
    }
}

pub enum CustomComponentRequest {}

pub enum CustomComponentResponse {}

fn main() {}
