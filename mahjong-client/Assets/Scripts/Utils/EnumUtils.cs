using System;
using System.Collections.Generic;
using System.Linq;

namespace Synapse.Utils
{
    /// <summary>
    /// Utilities for working with enum types.
    /// </summary>
    // TODO: Move this into a package. It's not game-specific, and could easily be reused.
    public static class EnumUtils
    {
        /// <summary>
        /// Creates an iterator over the values of an enum types.
        /// </summary>
        ///
        /// <typeparam name="T">An enumeration type.</typeparam>
        ///
        /// <returns>
        /// An iterator yielding the values of <typeparamref name="T"/>.
        /// </returns>
        public static IEnumerable<T> GetValues<T>() where T : Enum
        {
            return Enum.GetValues(typeof(T)).Cast<T>();
        }
    }
}
