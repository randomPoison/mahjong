using System.Collections.Generic;
using System.Linq;

namespace Synapse.Utils
{
    /// <summary>
    /// Utilities for working with iterators.
    /// </summary>
    // TODO: Move this into a package. It's not game-specific, and could easily be reused.
    public static class IteratorUtils
    {
        /// <summary>
        /// Creates an iterator which gives the current iteration count with each value.
        /// </summary>
        ///
        /// <typeparam name="T">The type of element yielded by the iterator</typeparam>
        ///
        /// <param name="iter">A sequence of values to enumerate.</param>
        ///
        /// <returns>
        /// An iterator that yields each element of <paramref name="iter"/>, along with
        /// the index of the element within the iterator.
        /// </returns>
        public static IEnumerable<(int index, T item)> Enumerate<T>(this IEnumerable<T> iter)
        {
            var counter = 0;
            return iter.Select(item =>
            {
                var index = counter;
                counter += 1;
                return (index, item);
            });
        }
    }
}
