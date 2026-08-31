using System;
using System.Linq;
using System.Reflection;
using NUnit.Framework;

internal static class Program
{
    private static int Main()
    {
        Type fixtureType = typeof(Axiom.Survivor.Tests.SurvivorRuntimeTests);
        MethodInfo setUp = fixtureType.GetMethods().Single(method => method.GetCustomAttribute<SetUpAttribute>() != null);
        MethodInfo tearDown = fixtureType.GetMethods().Single(method => method.GetCustomAttribute<TearDownAttribute>() != null);
        MethodInfo[] tests = fixtureType.GetMethods()
            .Where(method => method.GetCustomAttribute<TestAttribute>() != null)
            .OrderBy(method => method.Name, StringComparer.Ordinal)
            .ToArray();

        int failures = 0;
        foreach (MethodInfo test in tests)
        {
            object fixture = Activator.CreateInstance(fixtureType);
            try
            {
                setUp.Invoke(fixture, null);
                test.Invoke(fixture, null);
                Console.WriteLine($"PASS {test.Name}");
            }
            catch (TargetInvocationException exception)
            {
                failures++;
                Console.Error.WriteLine($"FAIL {test.Name}: {exception.InnerException}");
            }
            finally
            {
                tearDown.Invoke(fixture, null);
            }
        }

        Console.WriteLine($"{tests.Length - failures}/{tests.Length} survivor runtime tests passed.");
        return failures == 0 ? 0 : 1;
    }
}
