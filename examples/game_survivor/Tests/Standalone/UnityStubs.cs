using System;
using System.Collections.Generic;

namespace NUnit.Framework
{
    [AttributeUsage(AttributeTargets.Class)] public sealed class TestFixtureAttribute : Attribute { }
    [AttributeUsage(AttributeTargets.Method)] public sealed class TestAttribute : Attribute { }
    [AttributeUsage(AttributeTargets.Method)] public sealed class SetUpAttribute : Attribute { }
    [AttributeUsage(AttributeTargets.Method)] public sealed class TearDownAttribute : Attribute { }

    public static class Assert
    {
        public static void That<T>(T actual, Constraint constraint)
        {
            if (!constraint.Matches(actual))
            {
                throw new AssertionException($"Expected {constraint.Description}, but was {actual ?? (object)"null"}.");
            }
        }
    }

    public sealed class AssertionException : Exception
    {
        public AssertionException(string message) : base(message) { }
    }

    public abstract class Constraint
    {
        protected Constraint(string description) { Description = description; }
        public string Description { get; }
        public abstract bool Matches(object actual);
    }

    internal sealed class EqualConstraint : Constraint
    {
        private readonly object _expected;
        public EqualConstraint(object expected) : base($"equal to {expected}") { _expected = expected; }
        public override bool Matches(object actual)
        {
            if (IsNumeric(_expected) && IsNumeric(actual))
            {
                return Convert.ToDecimal(_expected).Equals(Convert.ToDecimal(actual));
            }
            return Equals(_expected, actual);
        }

        private static bool IsNumeric(object value)
        {
            if (value == null || value is bool || value is char)
            {
                return false;
            }
            TypeCode code = Type.GetTypeCode(value.GetType());
            return code >= TypeCode.SByte && code <= TypeCode.Decimal;
        }
    }

    internal sealed class GreaterThanConstraint : Constraint
    {
        private readonly IComparable _expected;
        public GreaterThanConstraint(IComparable expected) : base($"greater than {expected}") { _expected = expected; }
        public override bool Matches(object actual) => actual is IComparable comparable && comparable.CompareTo(_expected) > 0;
    }

    internal sealed class NullConstraint : Constraint
    {
        private readonly bool _expectNull;
        public NullConstraint(bool expectNull) : base(expectNull ? "null" : "not null") { _expectNull = expectNull; }
        public override bool Matches(object actual) => (actual == null) == _expectNull;
    }

    public static class Is
    {
        public static Constraint True => EqualTo(true);
        public static Constraint EqualTo(object expected) => new EqualConstraint(expected);
        public static Constraint GreaterThan(IComparable expected) => new GreaterThanConstraint(expected);
        public static Constraint NotNull => new NullConstraint(false);
        public static NotExpression Not => new NotExpression();
    }

    public sealed class NotExpression
    {
        public Constraint Null => new NullConstraint(false);
    }
}

namespace UnityEngine
{
    [AttributeUsage(AttributeTargets.Field)] public sealed class SerializeField : Attribute { }
    [AttributeUsage(AttributeTargets.Class)] public sealed class CreateAssetMenuAttribute : Attribute
    {
        public string fileName;
        public string menuName;
    }
    [AttributeUsage(AttributeTargets.Class, AllowMultiple = true)] public sealed class RequireComponentAttribute : Attribute
    {
        public RequireComponentAttribute(Type type) { }
    }
    [AttributeUsage(AttributeTargets.Field)] public sealed class HeaderAttribute : Attribute
    {
        public HeaderAttribute(string header) { }
    }

    public class Object
    {
        public static T Instantiate<T>(T original, Vector3 position, Quaternion rotation, Transform parent = null) where T : Object
        {
            if (typeof(T) == typeof(GameObject))
            {
                GameObject clone = new GameObject(((GameObject)(Object)original).name + " (Clone)");
                clone.transform.position = position;
                clone.transform.parent = parent;
                return (T)(Object)clone;
            }
            return original;
        }

        public static void Destroy(Object target) { }
        public static void DestroyImmediate(Object target) { }
    }

    public class Component : Object
    {
        public GameObject gameObject { get; internal set; }
        public Transform transform => gameObject.transform;
        public T GetComponent<T>() where T : Component => gameObject.GetComponent<T>();
    }

    public class Behaviour : Component { public bool enabled { get; set; } = true; }
    public class MonoBehaviour : Behaviour { }
    public class ScriptableObject : Object { }

    public sealed class GameObject : Object
    {
        private readonly Dictionary<Type, Component> _components = new Dictionary<Type, Component>();
        public GameObject(string name = "GameObject")
        {
            this.name = name;
            transform = AddComponent<Transform>();
        }
        public string name { get; set; }
        public Transform transform { get; }
        public bool activeSelf { get; private set; } = true;
        public void SetActive(bool active) { activeSelf = active; }
        public T AddComponent<T>() where T : Component, new()
        {
            T component = new T { gameObject = this };
            _components[typeof(T)] = component;
            return component;
        }
        public T GetComponent<T>() where T : Component
        {
            return _components.TryGetValue(typeof(T), out Component component) ? (T)component : null;
        }
    }

    public sealed class Transform : Component
    {
        public Vector3 position { get; set; }
        public Transform parent { get; set; }
        public void SetParent(Transform value, bool worldPositionStays) { parent = value; }
    }

    public readonly struct Vector3 : IEquatable<Vector3>
    {
        public Vector3(float x, float y, float z) { this.x = x; this.y = y; this.z = z; }
        public readonly float x;
        public readonly float y;
        public readonly float z;
        public bool Equals(Vector3 other) => x.Equals(other.x) && y.Equals(other.y) && z.Equals(other.z);
        public override bool Equals(object obj) => obj is Vector3 other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(x, y, z);
        public override string ToString() => $"({x}, {y}, {z})";
        public static Vector3 zero => new Vector3(0.0f, 0.0f, 0.0f);
    }

    public readonly struct Quaternion { public static Quaternion identity => new Quaternion(); }
    public static class Time { public static float fixedDeltaTime => 1.0f / 50.0f; }
    public static class Input
    {
        public static float GetAxisRaw(string name) => 0.0f;
        public static bool GetButton(string name) => false;
    }
    public static class Mathf { public static int RoundToInt(float value) => (int)Math.Round(value); }
    public static class Debug
    {
        public static void LogError(object message) => Console.Error.WriteLine(message);
        public static void LogWarning(object message) => Console.Error.WriteLine(message);
    }

    public sealed class Sprite : Object { }
    public sealed class SpriteRenderer : Component { public Sprite sprite { get; set; } }
    public sealed class RuntimeAnimatorController : Object { }
    public sealed class Animator : Component
    {
        public RuntimeAnimatorController runtimeAnimatorController { get; set; }
        public void SetTrigger(string name) { }
    }
    public sealed class AudioClip : Object { }
    public sealed class AudioSource : Behaviour
    {
        public void PlayOneShot(AudioClip clip) { }
    }
    public sealed class ParticleSystem : Component { public void Play() { } }
}

namespace UnityEngine.UI
{
    public sealed class Slider : UnityEngine.Component
    {
        public float minValue { get; set; }
        public float maxValue { get; set; }
        public float value { get; set; }
    }
}

namespace TMPro
{
    public class TMP_Text : UnityEngine.Component { public string text { get; set; } = string.Empty; }
}
