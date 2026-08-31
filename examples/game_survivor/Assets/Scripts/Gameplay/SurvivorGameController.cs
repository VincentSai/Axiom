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
        [SerializeField] private bool autoFollowCamera = true;

        [Header("Entity presentation")]
        [SerializeField] private EntityVisualBinding[] entityVisuals = Array.Empty<EntityVisualBinding>();
        [SerializeField] private VisualEventBinding[] visualEvents = Array.Empty<VisualEventBinding>();
        [SerializeField] private AudioSource audioSource = null;

        [Header("HUD")]
        [SerializeField] private Slider healthBar = null;
        [SerializeField] private TMP_Text levelText = null;
        [SerializeField] private TMP_Text scoreText = null;

        private readonly Dictionary<long, GameObject> _entityViews = new Dictionary<long, GameObject>();
        private static readonly Dictionary<EntityKind, Sprite> _proceduralSprites = new Dictionary<EntityKind, Sprite>();
        private ISurvivorVisualEffects _visualEffects;
        private EntitySnapshot _lastPlayerSnapshot;
        private GameObject _playerView;

        private bool _isGameOver;

        public int TrackedEntityCount => _entityViews.Count;
        public bool IsGameOver => _isGameOver;

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
        private static void AutoBootstrapScene()
        {
            SurvivorGameController existing = UnityEngine.Object.FindObjectOfType<SurvivorGameController>();
            if (existing == null)
            {
                GameObject manager = new GameObject("Axiom_Survivor_GameManager");
                manager.AddComponent<AxiomRunner>();
                manager.AddComponent<SurvivorGameController>();
            }
        }

        private void Awake()
        {
            if (runner == null)
            {
                runner = GetComponent<AxiomRunner>();
                if (runner == null)
                {
                    runner = gameObject.AddComponent<AxiomRunner>();
                }
            }
        }

        private void OnEnable()
        {
            if (runner == null)
            {
                runner = GetComponent<AxiomRunner>();
                if (runner == null)
                {
                    runner = gameObject.AddComponent<AxiomRunner>();
                }
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

        private void Update()
        {
            if (_isGameOver && Input.GetKeyDown(KeyCode.R))
            {
                RestartGame();
            }
        }

        public void RestartGame()
        {
            _isGameOver = false;
            ClearEntityViews();
            if (runner != null)
            {
                runner.ResetGame();
            }
        }

        private void LateUpdate()
        {
            if (autoFollowCamera && _playerView != null && Camera.main != null)
            {
                Vector3 playerPos = _playerView.transform.position;
                Camera.main.transform.position = new Vector3(playerPos.x, playerPos.y, -10.0f);
            }
        }

        private void OnGUI()
        {
            if (healthBar != null && levelText != null)
            {
                return;
            }

            GUI.color = Color.white;
            GUI.Box(new Rect(10, 10, 280, 150), "★ Axiom Survivors 战斗状态");

            int hp = _lastPlayerSnapshot.max_hp > 0 ? (int)_lastPlayerSnapshot.hp : 100;
            int maxHp = _lastPlayerSnapshot.max_hp > 0 ? (int)_lastPlayerSnapshot.max_hp : 100;
            int level = _lastPlayerSnapshot.level > 0 ? (int)_lastPlayerSnapshot.level : 1;
            long score = _lastPlayerSnapshot.score;

            GUI.Label(new Rect(20, 35, 260, 25), $"💚 生命值: {hp} / {maxHp}");
            GUI.Label(new Rect(20, 60, 260, 25), $"⭐ 角色等级: Lv. {level}");
            GUI.Label(new Rect(20, 85, 260, 25), $"🏆 生存积分: {score}  |  👾 怪物: {TrackedEntityCount}");
            GUI.Label(new Rect(20, 110, 260, 25), "🎮 [WASD/方向键] 移动 | [空格/J] 射击");
            GUI.Label(new Rect(20, 130, 260, 20), _isGameOver ? "💀 [GAME OVER - 按 R 键重开]" : "⚡ [Axiom 确定性内核 60Hz 运行中]");

            // Game Over Popup Modal
            if (_isGameOver)
            {
                int screenW = Screen.width > 0 ? Screen.width : 800;
                int screenH = Screen.height > 0 ? Screen.height : 600;
                int boxW = 360;
                int boxH = 180;
                Rect modalRect = new Rect((screenW - boxW) / 2f, (screenH - boxH) / 2f, boxW, boxH);

                GUI.color = Color.red;
                GUI.Box(modalRect, "💀  Y O U   D I E D  💀");
                GUI.color = Color.white;
                GUI.Label(new Rect(modalRect.x + 30, modalRect.y + 45, 300, 30), $"最终生存积分: {score} 点");
                GUI.Label(new Rect(modalRect.x + 30, modalRect.y + 80, 300, 30), $"最终达成等级: Lv. {level}");
                GUI.Label(new Rect(modalRect.x + 30, modalRect.y + 125, 300, 30), "👉 请按键盘 [ R ] 键重新开始 👈");
            }
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
                _lastPlayerSnapshot = snapshot;
                _playerView = view;
                if (snapshot.hp <= 0)
                {
                    _isGameOver = true;
                }
                UpdateHud(snapshot);
            }
        }

        public void HandleVisualEvent(VisualEvent visualEvent)
        {
            if (visualEvent.kind == VisualEventKind.Death || visualEvent.kind == VisualEventKind.Despawn)
            {
                RemoveEntityView(visualEvent.entity_id);
            }
            else if (visualEvent.kind == VisualEventKind.GameOver)
            {
                _isGameOver = true;
            }

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
                    audioSource = GetComponent<AudioSource>();
                    if (audioSource == null)
                    {
                        audioSource = gameObject.AddComponent<AudioSource>();
                    }
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
                : new GameObject($"{snapshot.kind}_{snapshot.id}");
            if (entityRoot != null && view.transform.parent != entityRoot)
            {
                view.transform.SetParent(entityRoot, false);
            }

            EntityVisualBinding binding = FindEntityVisualBinding(snapshot.kind);
            SpriteRenderer renderer = view.GetComponent<SpriteRenderer>();
            if (renderer == null)
            {
                renderer = view.AddComponent<SpriteRenderer>();
            }

            if (binding.Sprite != null)
            {
                renderer.sprite = binding.Sprite;
            }
            else
            {
                renderer.sprite = GetOrCreateProceduralSprite(snapshot.kind);
            }

            if (binding.AnimatorController != null)
            {
                Animator animator = view.GetComponent<Animator>();
                if (animator == null)
                {
                    animator = view.AddComponent<Animator>();
                }
                animator.runtimeAnimatorController = binding.AnimatorController;
            }
            return view;
        }

        private static Sprite GetOrCreateProceduralSprite(EntityKind kind)
        {
            if (_proceduralSprites.TryGetValue(kind, out Sprite existing))
            {
                return existing;
            }

            int size = (kind == EntityKind.Projectile || kind == EntityKind.Gem) ? 16 : 32;
            Texture2D tex = new Texture2D(size, size);
            Color color = kind switch
            {
                EntityKind.Player => new Color(0.2f, 0.8f, 0.4f, 1.0f),
                EntityKind.Monster => new Color(0.9f, 0.2f, 0.2f, 1.0f),
                EntityKind.Projectile => new Color(1.0f, 0.9f, 0.2f, 1.0f),
                EntityKind.Gem => new Color(0.2f, 0.6f, 1.0f, 1.0f),
                _ => new Color(0.8f, 0.8f, 0.8f, 1.0f)
            };

            for (int y = 0; y < size; y++)
            {
                for (int x = 0; x < size; x++)
                {
                    tex.SetPixel(x, y, color);
                }
            }
            tex.Apply();

            Sprite sprite = Sprite.Create(tex, new Rect(0, 0, size, size), new Vector2(0.5f, 0.5f), 32.0f);
            _proceduralSprites[kind] = sprite;
            return sprite;
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
