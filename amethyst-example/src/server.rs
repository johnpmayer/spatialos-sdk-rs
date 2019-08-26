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

mod connection_handler;
#[rustfmt::skip]
mod generated;
mod opt;

fn main() {
    println!("Go, server!")
}

struct ConnectionResource {
    connection: WorkerConnection,
}
