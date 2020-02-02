using System.Collections.Generic;

public static class TupleUtils
{
    /// <summary>
    /// Allows for tuple destructing of <see cref="KeyValuePair<TKey, TValue>"/> object.
    /// </summary>
    ///
    /// <param name="kvp">The key-value pair object to destructure.</param>
    /// <param name="key">The key extracted from the pair.</param>
    /// <param name="val">The value extracted from the pair.</param>
    /// <typeparam name="TKey">The key type.</typeparam>
    /// <typeparam name="TValue">The value type.</typeparam>
    ///
    /// <remarks>
    /// Allows for the use of tuple destructuring when iterating over a dictionary or
    /// any other map type.
    /// </remarks>
    public static void Deconstruct<TKey, TValue>(this KeyValuePair<TKey, TValue> kvp, out TKey key, out TValue val)
    {
        key = kvp.Key;
        val = kvp.Value;
    }
}
