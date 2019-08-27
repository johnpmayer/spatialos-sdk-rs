use crate::{connection_handler::*, opt::*};
use generated::{example, improbable};
use rand::Rng;
use spatialos_sdk::worker::{
    commands::{EntityQueryRequest, ReserveEntityIdsRequest},
    component::{Component, ComponentData, UpdateParameters},
    connection::{Connection, WorkerConnection},
    entity_builder::EntityBuilder,
    metrics::{HistogramMetric, Metrics},
    op::{StatusCode, WorkerOp},
    query::{EntityQuery, QueryConstraint, ResultType},
    {EntityId, InterestOverride, LogLevel},
};
use std::{collections::HashMap, f64};
use structopt::StructOpt;
use specs::{prelude::*};

mod connection_handler;
#[rustfmt::skip]
mod generated;
mod opt;

fn main() {
    println!("Go, server!");

    let mut world = World::new();

    let opt = Opt::from_args();
    let mut worker_connection = match get_connection(opt) {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };

    worker_connection.send_log_message(LogLevel::Info, "main", "Connected!", None);

    world.add_resource(ConnectionResource{connection: Some(worker_connection)});

    loop {
        ConnectionOpMux.run_now(&world);
    }
}

#[derive(Default)]
struct ConnectionResource {
    connection: Option<WorkerConnection>,
}

struct ConnectionOpMux;

impl<'a> System<'a> for ConnectionOpMux {
    type SystemData = (WriteExpect<'a, ConnectionResource>);

    fn run(&mut self, (mut connection_resource): Self::SystemData) {
        // connection_resource
        let mut connection: &mut WorkerConnection = connection_resource.connection.as_mut().expect("Connection resource is unset");
        let ops = connection.get_op_list(0); // FIXME: what's the zero here, a limit?
        for op in &ops {
            eprintln!("Recieved op: {:?}", op);
        }
    }
}