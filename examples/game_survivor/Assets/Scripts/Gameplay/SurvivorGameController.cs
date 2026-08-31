using System;
using System.Collections.Generic;
using Axiom.Generated;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Axiom.Survivor
{
    public interface ISurvivorVisualEffects
    {
        void Present(VisualEvent visualEvent, GameObject target);
    }

    [Serializable]
    public struct EntityVisualBinding
    {
        public EntityKind Kind;
        public Sprite Sprite;
        public RuntimeAnimatorController AnimatorController;
    }

    [Serializable]
    public struct VisualEventBinding
    {
        public VisualEventKind Kind;
        public string AnimatorTrigger;
        public AudioClip AudioClip;
        public ParticleSystem ParticlePrefab;
    }

    public sealed class SurvivorGameController : MonoBehaviour, ISurvivorVisualEffects
    {
        [Header("Runtime")]
        [SerializeField] private AxiomRunner runner = null;
        [SerializeField] private Transform entityRoot = null;

        [Header("Entity presentation")]
        [SerializeField] private EntityVisualBinding[] entityVisuals = Array.Empty<EntityVisualBinding>();
        [SerializeField] private VisualEventBinding[] visualEvents = Array.Empty<VisualEventBinding>();
        [SerializeField] private AudioSource audioSource = null;

        [Header("HUD")]
        [SerializeField] private Slider healthBar = null;
        [SerializeField] private TMP_Text levelText = null;
        [SerializeField] private TMP_Text scoreText = null;

        private readonly Dictionary<long, GameObject> _entityViews = new Dictionary<long, GameObject>();
        private ISurvivorVisualEffects _visualEffects;

        public int TrackedEntityCount => _entityViews.Count;

        private void OnEnable()
        {
            if (runner == null)
            {
                runner = GetComponent<AxiomRunner>();
            }

            if (runner != null)
            {
                runner.OnEntityUpdated += HandleEntityUpdated;
                runner.OnVisualEvent += HandleVisualEvent;
                runner.Initialize();
            }
        }

        private void OnDisable()
        {
            if (runner != null)
            {
                runner.OnEntityUpdated -= HandleEntityUpdated;
                runner.OnVisualEvent -= HandleVisualEvent;
            }

            ClearEntityViews();
        }

        public void SetVisualEffects(ISurvivorVisualEffects visualEffects)
        {
            _visualEffects = visualEffects;
        }

        public void HandleEntityUpdated(EntitySnapshot snapshot)
        {
            if (!snapshot.active)
            {
                RemoveEntityView(snapshot.id);
                return;
            }

            if (!_entityViews.TryGetValue(snapshot.id, out GameObject view))
            {
                view = CreateEntityView(snapshot);
                _entityViews.Add(snapshot.id, view);
            }

            view.transform.position = new Vector3((float)snapshot.x, (float)snapshot.y, 0.0f);
            if (snapshot.kind == EntityKind.Player)
            {
                UpdateHud(snapshot);
            }
        }

        public void HandleVisualEvent(VisualEvent visualEvent)
        {
            _entityViews.TryGetValue(visualEvent.entity_id, out GameObject target);
            (_visualEffects ?? this).Present(visualEvent, target);
        }

        public bool TryGetEntityView(long entityId, out GameObject view)
        {
            return _entityViews.TryGetValue(entityId, out view);
        }

        public void Present(VisualEvent visualEvent, GameObject target)
        {
            VisualEventBinding binding = FindVisualEventBinding(visualEvent.kind);
            if (target != null && !string.IsNullOrEmpty(binding.AnimatorTrigger))
            {
                Animator animator = target.GetComponent<Animator>();
                if (animator != null)
                {
                    animator.SetTrigger(binding.AnimatorTrigger);
                }
            }

            AudioClip clip = binding.AudioClip;
            if (clip == null && runner != null && runner.AssetRegistry != null)
            {
                runner.AssetRegistry.TryGetAudioClip(visualEvent.kind, out clip);
            }
            if (clip != null)
            {
                if (audioSource == null)
                {
                    audioSource = GetComponent<AudioSource>() ?? gameObject.AddComponent<AudioSource>();
                }
                audioSource.PlayOneShot(clip);
            }

            if (binding.ParticlePrefab != null)
            {
                ParticleSystem particles = Instantiate(
                    binding.ParticlePrefab,
                    new Vector3((float)visualEvent.x, (float)visualEvent.y, 0.0f),
                    Quaternion.identity,
                    entityRoot);
                particles.Play();
            }
        }

        private GameObject CreateEntityView(EntitySnapshot snapshot)
        {
            GameObject prefab = null;
            if (runner != null && runner.AssetRegistry != null)
            {
                runner.AssetRegistry.TryGetPrefab(snapshot.kind, out prefab);
            }

            GameObject view = prefab != null
                ? Instantiate(prefab, Vector3.zero, Quaternion.identity, entityRoot)
                : new GameObject($"{snapshot.kind} {snapshot.id}");
            if (entityRoot != null && view.transform.parent != entityRoot)
            {
                view.transform.SetParent(entityRoot, false);
            }

            EntityVisualBinding binding = FindEntityVisualBinding(snapshot.kind);
            if (binding.Sprite != null)
            {
                SpriteRenderer renderer = view.GetComponent<SpriteRenderer>() ?? view.AddComponent<SpriteRenderer>();
                renderer.sprite = binding.Sprite;
            }
            if (binding.AnimatorController != null)
            {
                Animator animator = view.GetComponent<Animator>() ?? view.AddComponent<Animator>();
                animator.runtimeAnimatorController = binding.AnimatorController;
            }
            return view;
        }

        private void UpdateHud(EntitySnapshot player)
        {
            if (healthBar != null)
            {
                healthBar.minValue = 0.0f;
                healthBar.maxValue = Math.Max(1.0f, (float)player.max_hp);
                healthBar.value = (float)player.hp;
            }
            if (levelText != null)
            {
                levelText.text = $"Lv. {player.level}";
            }
            if (scoreText != null)
            {
                scoreText.text = player.score.ToString();
            }
        }

        private EntityVisualBinding FindEntityVisualBinding(EntityKind kind)
        {
            for (int index = 0; index < entityVisuals.Length; index++)
            {
                if (entityVisuals[index].Kind == kind)
                {
                    return entityVisuals[index];
                }
            }
            return default(EntityVisualBinding);
        }

        private VisualEventBinding FindVisualEventBinding(VisualEventKind kind)
        {
            for (int index = 0; index < visualEvents.Length; index++)
            {
                if (visualEvents[index].Kind == kind)
                {
                    return visualEvents[index];
                }
            }
            return default(VisualEventBinding);
        }

        private void RemoveEntityView(long entityId)
        {
            if (!_entityViews.TryGetValue(entityId, out GameObject view))
            {
                return;
            }
            _entityViews.Remove(entityId);
            Destroy(view);
        }

        private void ClearEntityViews()
        {
            foreach (GameObject view in _entityViews.Values)
            {
                Destroy(view);
            }
            _entityViews.Clear();
        }
    }
}
