using UnityEngine;

namespace Synapse.Utils
{
    /// <summary>
    /// Utilities and extension methods for working with the <see cref="Transform"/>
    /// component.
    /// </summary>
    public static class TransformUtil
    {
        /// <summary>
        /// Destroys all children of the specified transform.
        /// </summary>
        ///
        /// <param name="transform">The transform to target.</param>
        public static void DestroyChildren(this Transform transform)
        {
            foreach (Transform child in transform)
            {
                Object.Destroy(child.gameObject);
            }
        }
    }
}
