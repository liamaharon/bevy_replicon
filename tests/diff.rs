use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_replicon::{prelude::*, test_app::ServerTestAppExt};
use serde::{Deserialize, Serialize};

#[test]
fn component() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_diff::<Points>()
        .finish();
    }

    server_app.connect_client(&mut client_app);

    let server_entity = server_app
        .world_mut()
        .spawn((Replicated, Points(vec![0])))
        .id();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let mut points_query = client_app.world_mut().query::<&Points>();
    let points = points_query.single(client_app.world()).unwrap();
    assert_eq!(points.0, [0]);

    server_app
        .world_mut()
        .entity_mut(server_entity)
        .apply_diff::<Points>(AddPoint(1))
        .unwrap();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = points_query.single(client_app.world()).unwrap();
    assert_eq!(points.0, [0, 1]);
}

#[test]
fn resource() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_resource_diff::<Points>()
        .finish();
    }

    server_app.connect_client(&mut client_app);

    server_app.insert_resource(Points(vec![0]));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0, [0]);

    server_app
        .world_mut()
        .apply_resource_diff::<Points>(AddPoint(1))
        .unwrap();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0, [0, 1]);
}

#[test]
fn message_loss() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_resource_diff::<Points>()
        .finish();
    }

    server_app.connect_client(&mut client_app);

    server_app.insert_resource(Points(vec![0]));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0.len(), 1);

    server_app
        .world_mut()
        .apply_resource_diff::<Points>(AddPoint(1))
        .unwrap();

    server_app.update();

    let mut messages = server_app.world_mut().resource_mut::<ServerMessages>();
    assert_eq!(messages.drain_sent().len(), 1);

    server_app
        .world_mut()
        .apply_resource_diff::<Points>(AddPoint(2))
        .unwrap();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0, [0, 1, 2]);
}

#[test]
fn outside_of_history_window() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_resource_diff::<Points>()
        .finish();
    }

    server_app.connect_client(&mut client_app);

    server_app.insert_resource(Points(vec![0]));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0.len(), 1);

    for index in 1..=Points::HISTORY_LEN {
        server_app
            .world_mut()
            .apply_resource_diff::<Points>(AddPoint(index))
            .unwrap();
    }

    server_app.update();

    let mut messages = server_app.world_mut().resource_mut::<ServerMessages>();
    assert_eq!(messages.drain_sent().len(), 1);

    server_app
        .world_mut()
        .apply_resource_diff::<Points>(AddPoint(100))
        .unwrap();

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0, [0, 1, 2, 3, 4, 5, 100]);
    assert_eq!(points.0.len(), Points::HISTORY_LEN + 2);
}

#[test]
fn external_mutation() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_resource_diff::<Points>()
        .finish();
    }

    server_app.connect_client(&mut client_app);

    server_app.insert_resource(Points(vec![0]));

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0.len(), 1);

    let mut points = server_app.world_mut().resource_mut::<Points>();
    points.0.push(1);

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let points = client_app.world().resource::<Points>();
    assert_eq!(points.0, [0, 1]);
}

#[derive(Resource, Deserialize, Serialize, Debug, Clone)]
struct Points(Vec<usize>);

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct AddPoint(usize);

impl Diffable for Points {
    type Diff = AddPoint;
    const HISTORY_LEN: usize = 5;

    fn apply_diff(&mut self, diff: &Self::Diff) -> Result<()> {
        self.0.push(diff.0);
        Ok(())
    }
}

#[derive(Component, Serialize, Deserialize)]
struct Extra;

#[test]
fn snapshots_bootstrap_diff_cursors_for_every_recipient_and_atomic_updates() {
    let mut server = App::new();
    let mut first = App::new();
    let mut second = App::new();
    for app in [&mut server, &mut first, &mut second] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_diff::<Points>()
        .replicate::<Extra>()
        .finish();
    }
    server.connect_client(&mut first);
    server.connect_client(&mut second);
    let entity = server
        .world_mut()
        .spawn((Replicated, Points(vec![12345; 256])))
        .id();
    server.update();
    for client in [&mut first, &mut second] {
        server.exchange_with_client(client);
        client.update();
        server.exchange_with_client(client);
    }
    // Exactly the retained history length must still be diffable from the
    // initial reliable snapshot, including recipients reusing cached bytes.
    for index in 0..Points::HISTORY_LEN {
        server
            .world_mut()
            .entity_mut(entity)
            .apply_diff::<Points>(AddPoint(index))
            .unwrap();
    }
    for step in 0..3 {
        if step > 0 {
            server
                .world_mut()
                .entity_mut(entity)
                .apply_diff::<Points>(AddPoint(10 + step))
                .unwrap();
        }
        if step == 1 {
            // An insertion moves this entity's mutation onto the reliable
            // channel. The next diff must use that newly established base.
            server.world_mut().entity_mut(entity).insert(Extra);
        }
        server.update();
        let mut packets = 0;
        server
            .world_mut()
            .resource_mut::<ServerMessages>()
            .retain_sent(|(_, _, bytes)| {
                packets += 1;
                assert!(
                    bytes.len() < 128,
                    "full 256-element snapshot sent instead of diff: {} bytes",
                    bytes.len()
                );
                true
            });
        assert!(packets >= 2);
        for client in [&mut first, &mut second] {
            server.exchange_with_client(client);
            client.update();
            server.exchange_with_client(client);
            let actual = client
                .world_mut()
                .query::<&Points>()
                .single(client.world())
                .unwrap();
            assert_eq!(actual.0, server.world().get::<Points>(entity).unwrap().0);
        }
    }
}

#[test]
fn entity_references_are_mapped_in_snapshots_and_ordered_diffs() {
    use bevy::ecs::entity::{EntityMapper, MapEntities};
    use bevy_replicon::shared::server_entity_map::ServerEntityMap;

    #[derive(Component, Serialize, Deserialize)]
    struct Links(#[entities] Vec<Entity>);

    #[derive(Serialize, Deserialize, MapEntities)]
    enum LinkDiff {
        Append(#[entities] Entity),
        Remove(#[entities] Entity),
    }

    impl Diffable for Links {
        type Diff = LinkDiff;

        fn map_diff_entities(diff: &mut Self::Diff, mapper: &mut impl EntityMapper) {
            diff.map_entities(mapper);
        }

        fn apply_diff(&mut self, diff: &Self::Diff) -> Result<()> {
            match diff {
                LinkDiff::Append(entity) => self.0.push(*entity),
                LinkDiff::Remove(entity) => self.0.retain(|link| link != entity),
            }
            Ok(())
        }
    }

    let mut server = App::new();
    let mut client = App::new();
    for app in [&mut server, &mut client] {
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        ))
        .replicate_diff::<Links>()
        .finish();
    }
    for _ in 0..12 {
        client.world_mut().spawn_empty();
    }
    let first = server.world_mut().spawn(Replicated).id();
    let second = server.world_mut().spawn(Replicated).id();
    let list = server
        .world_mut()
        .spawn((Replicated, Links(vec![first])))
        .id();
    server.connect_client(&mut client);
    server.update();
    server.exchange_with_client(&mut client);
    client.update();
    server.exchange_with_client(&mut client);
    server.update();
    let map = client.world().resource::<ServerEntityMap>();
    let mapped_first = *map.to_client().get(&first).unwrap();
    let mapped_second = *map.to_client().get(&second).unwrap();
    let mapped_list = *map.to_client().get(&list).unwrap();
    assert_ne!(mapped_first, first);
    assert_ne!(mapped_second, second);
    assert_eq!(
        client.world().get::<Links>(mapped_list).unwrap().0,
        [mapped_first]
    );

    // Deliver duplicate packets in reverse order. Mapping must affect only new
    // diffs, never previously mapped component references or discarded diffs.
    let mut packets = Vec::new();
    for diff in [
        LinkDiff::Append(second),
        LinkDiff::Append(first),
        LinkDiff::Remove(second),
    ] {
        server
            .world_mut()
            .entity_mut(list)
            .apply_diff::<Links>(diff)
            .unwrap();
        server.update();
        packets.extend(
            server
                .world_mut()
                .resource_mut::<ServerMessages>()
                .drain_sent(),
        );
    }
    for (_, channel, bytes) in packets.into_iter().rev() {
        let mut messages = client.world_mut().resource_mut::<ClientMessages>();
        messages.insert_received(channel, bytes.clone());
        messages.insert_received(channel, bytes);
    }
    client.update();
    assert_eq!(
        client.world().get::<Links>(mapped_list).unwrap().0,
        [mapped_first, mapped_first]
    );
    assert_eq!(
        client
            .world()
            .resource::<ServerEntityMap>()
            .to_client()
            .len(),
        3
    );

    // A direct mutation forces another snapshot without remapping old values twice.
    server.world_mut().get_mut::<Links>(list).unwrap().0 = vec![second];
    server.update();
    server.exchange_with_client(&mut client);
    client.update();
    assert_eq!(
        client.world().get::<Links>(mapped_list).unwrap().0,
        [mapped_second]
    );
}
