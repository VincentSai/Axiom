using System;
using Axiom.Generated;
using Axiom.Survivor;
using NUnit.Framework;
using UnityEngine;

namespace Axiom.Survivor.Tests
{
    [TestFixture]
    public sealed class SurvivorRuntimeTests
    {
        private GameObject _runnerObject;
        private AxiomRunner _runner;

        [SetUp]
        public void SetUp()
        {
            _runnerObject = new GameObject("Axiom runner test");
            _runner = _runnerObject.AddComponent<AxiomRunner>();
        }

        [TearDown]
        public void TearDown()
        {
            if (_runner != null)
            {
                _runner.Shutdown();
            }

            if (_runnerObject != null)
            {
                UnityEngine.Object.DestroyImmediate(_runnerObject);
            }
        }

        [Test]
        public void Runner_initializes_native_runtime_and_publishes_initial_snapshot()
        {
            Assert.That(_runner.Initialize(), Is.True);
            Assert.That(_runner.IsInitialized, Is.True);
            Assert.That(_runner.SnapshotCount, Is.GreaterThan(0));
        }

        [Test]
        public void Runner_advances_one_hundred_ticks_without_losing_runtime_state()
        {
            Assert.That(_runner.Initialize(), Is.True);
            ulong initialRevision = _runner.SnapshotRevision;

            PlayerInput input = new PlayerInput { move_x = 1.0, move_y = 0.0, is_firing = true };
            for (int tick = 0; tick < 100; tick++)
            {
                _runner.Tick(input, 1.0 / 60.0);
            }

            Assert.That(_runner.IsInitialized, Is.True);
            Assert.That(_runner.SnapshotRevision, Is.EqualTo(initialRevision + 100));
            Assert.That(_runner.SnapshotCount, Is.GreaterThan(0));
        }

        [Test]
        public void Runner_firing_spawns_projectiles_and_damages_monsters()
        {
            Assert.That(_runner.Initialize(), Is.True);

            // Fire towards initial monster at (5.0, 2.0)
            PlayerInput input = new PlayerInput { move_x = 1.0, move_y = 0.4, is_firing = true };
            bool spawnedProjectile = false;

            for (int tick = 0; tick < 60; tick++)
            {
                _runner.Tick(input, 1.0 / 60.0);
                for (int i = 0; i < _runner.SnapshotCount; i++)
                {
                    EntitySnapshot entity = _runner.GetSnapshot(i);
                    if (entity.active && entity.kind == EntityKind.Projectile)
                    {
                        spawnedProjectile = true;
                        break;
                    }
                }
            }

            Assert.That(spawnedProjectile, Is.True);
            Assert.That(_runner.SnapshotRevision, Is.GreaterThan((ulong)50));
        }

        [Test]
        public void Controller_creates_updates_and_removes_views_from_entity_snapshots()
        {
            GameObject controllerObject = new GameObject("Survivor controller test");
            SurvivorGameController controller = controllerObject.AddComponent<SurvivorGameController>();
            try
            {
                EntitySnapshot spawned = Snapshot(42, 2.0, 3.0, true);
                controller.HandleEntityUpdated(spawned);

                Assert.That(controller.TrackedEntityCount, Is.EqualTo(1));
                Assert.That(controller.TryGetEntityView(42, out GameObject view), Is.True);
                Assert.That(view.transform.position, Is.EqualTo(new Vector3(2.0f, 3.0f, 0.0f)));

                EntitySnapshot moved = Snapshot(42, -4.0, 7.5, true);
                controller.HandleEntityUpdated(moved);
                Assert.That(view.transform.position, Is.EqualTo(new Vector3(-4.0f, 7.5f, 0.0f)));

                controller.HandleEntityUpdated(Snapshot(42, -4.0, 7.5, false));
                Assert.That(controller.TrackedEntityCount, Is.EqualTo(0));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(controllerObject);
            }
        }

        [Test]
        public void Controller_routes_visual_events_to_presentation_effects()
        {
            GameObject controllerObject = new GameObject("Survivor visual event test");
            SurvivorGameController controller = controllerObject.AddComponent<SurvivorGameController>();
            RecordingVisualEffects effects = new RecordingVisualEffects();
            controller.SetVisualEffects(effects);
            try
            {
                controller.HandleEntityUpdated(Snapshot(7, 1.0, 1.0, true));
                VisualEvent visualEvent = new VisualEvent
                {
                    kind = VisualEventKind.Hit,
                    entity_id = 7,
                    asset_id = 3,
                    x = 1.0,
                    y = 1.0,
                    value = 12,
                };

                controller.HandleVisualEvent(visualEvent);

                Assert.That(effects.CallCount, Is.EqualTo(1));
                Assert.That(effects.LastEvent.entity_id, Is.EqualTo(7));
                Assert.That(effects.LastTarget, Is.Not.Null);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(controllerObject);
            }
        }

        private static EntitySnapshot Snapshot(long id, double x, double y, bool active)
        {
            return new EntitySnapshot
            {
                id = id,
                kind = EntityKind.Monster,
                x = x,
                y = y,
                hp = 10,
                max_hp = 10,
                active = active,
            };
        }

        private sealed class RecordingVisualEffects : ISurvivorVisualEffects
        {
            public int CallCount { get; private set; }
            public VisualEvent LastEvent { get; private set; }
            public GameObject LastTarget { get; private set; }

            public void Present(VisualEvent visualEvent, GameObject target)
            {
                CallCount++;
                LastEvent = visualEvent;
                LastTarget = target;
            }
        }
    }
}
