using NUnit.Framework;
using Synapse.Utils;
using System.Collections;
using UnityEngine;
using UnityEngine.TestTools;

namespace Synapse.Mahjong.Tests
{
    public class TransformUtilTests
    {
        [UnityTest]
        public IEnumerator DestroyNoChildren()
        {
            // Create a root object with no children.
            var root = new GameObject().transform;

            Assert.AreEqual(0, root.childCount);

            yield return null;

            root.DestroyChildren();

            yield return null;
            Assert.AreEqual(0, root.childCount);
        }

        [UnityTest]
        public IEnumerator DestroyMultipleChildren()
        {
            // Create a root object with some children.
            var root = new GameObject().transform;
            for (var count = 0; count < 10; count += 1)
            {
                new GameObject().transform.SetParent(root, false);
            }

            Assert.AreEqual(10, root.childCount);

            yield return null;

            root.DestroyChildren();

            yield return null;
            Assert.AreEqual(0, root.childCount);
        }
    }
}
